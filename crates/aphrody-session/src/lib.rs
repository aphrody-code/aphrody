// SPDX-License-Identifier: Apache-2.0
//! aphrody-session — conversation session tracker for the aphrody runtime.
//!
//! A `Session` captures the full audit trail of an interactive conversation:
//!
//! * every **turn** (`User` / `Assistant` / `Tool` / `System`) with the model
//!   token counts and per-turn USD cost;
//! * every **tool call** with its JSON arguments, JSON result or error
//!   message, and a wall-clock duration in milliseconds;
//! * **session lifecycle** events (`SessionStart`, `SessionEnd`) carrying the
//!   start / end timestamps and the rolled-up totals.
//!
//! Each of these is modelled as a [`SessionEvent`] variant. The events are
//! kept in insertion order on the [`Session`] and serialized as one JSON
//! object per line — the **JSON Lines** format — so the log is append-friendly
//! and trivially streamable through `jq`, `grep`, or any line-oriented tool.
//!
//! ## Cohesion with the wider workspace
//!
//! This crate intentionally complements three sibling crates without
//! overlapping their responsibilities:
//!
//! * **`aphrody-cost`** owns the *pricing tables* and the *budget guard*; the
//!   per-turn `cost_usd` here is supplied by the caller (typically after
//!   computing it with `aphrody_cost::estimate`).
//! * **`aphrody-hooks`** dispatches `PreToolUse` / `PostToolUse` hooks; the
//!   [`ToolCall`] DTO here is the post-hoc *audit record* of what actually
//!   ran, regardless of whether a hook short-circuited it.
//! * **`aphrody-context`** maintains the *live* working set fed back to the
//!   model; `Session` is the *cold* archive that survives process exit.
//!
//! The on-disk format mirrors the atomic write pattern from
//! `aphrody-cron::JsonFileJobStore` (tmp-file + rename), the DTO shape from
//! `aphrody-memory::types::MemoryRecord` (serde with `#[serde(default)]` and
//! `serde_json::Value` extension points), and the error envelope shape from
//! `aphrody-marketplace::MarketplaceError` (thiserror enum, `Io` / `Json`
//! transparent forwards + domain-specific variants).
//!
//! ## Example
//!
//! ```no_run
//! use aphrody_session::{
//!     JsonlFileStore, Session, SessionId, SessionStore, Turn, TurnRole,
//! };
//! use chrono::Utc;
//!
//! # async fn run() -> Result<(), aphrody_session::SessionError> {
//! let store = JsonlFileStore::new("./var/sessions");
//! let id = SessionId::generate();
//! let mut session = Session::new(id.clone(), serde_json::json!({"model": "claude-opus-4-7"}));
//! session.add_turn(Turn {
//!     id: "t1".into(),
//!     role: TurnRole::User,
//!     content: "hello".into(),
//!     ts: Utc::now(),
//!     prompt_tokens: 0,
//!     completion_tokens: 0,
//!     cost_usd: 0.0,
//! });
//! store.save_session(&session).await?;
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs as afs;
use tokio::io::AsyncWriteExt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All recoverable failures emitted by this crate.
///
/// Shape modelled after [`aphrody_marketplace::MarketplaceError`] — opaque
/// transparent forwards for IO and JSON, named variants for domain errors.
#[derive(Debug, Error)]
pub enum SessionError {
    /// Filesystem error while loading / saving / listing sessions.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON (de)serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// The requested session id is not present in the backing store.
    #[error("session not found: {0}")]
    NotFound(String),
    /// A JSONL line failed to parse cleanly; payload carries the line number
    /// and the underlying decoder error message.
    #[error("invalid jsonl: {0}")]
    InvalidJsonl(String),
    /// A `Session` was constructed without any events and an operation that
    /// requires at least one (e.g. serialization of a finalized log) was
    /// attempted.
    #[error("session is empty")]
    EmptySession,
}

// ---------------------------------------------------------------------------
// SessionId — opaque ULID-shaped identifier (no `uuid` crate dep on purpose)
// ---------------------------------------------------------------------------

/// Opaque newtype around a session identifier.
///
/// `SessionId::generate()` produces a 32-character lowercase hex string built
/// from the current monotonic system time (nanoseconds since the UNIX epoch)
/// concatenated with a small per-call counter that survives process
/// re-entry — collision-resistant enough for human-driven conversation logs
/// without pulling in the `uuid` crate when callers don't need it.
///
/// The wire format is intentionally URL-safe: hex characters only, no
/// padding, no dashes — so a `SessionId` can be slotted straight into a file
/// name (`{id}.jsonl`) or a URL path segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    /// Wrap an existing string. The caller is responsible for ensuring it is
    /// a sensible filename — this constructor performs no validation beyond
    /// rejecting the empty string.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::NotFound`] (re-purposed as a validation
    /// failure) if the input is empty.
    pub fn new(raw: impl Into<String>) -> Result<Self, SessionError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(SessionError::NotFound("<empty session id>".into()));
        }
        Ok(Self(raw))
    }

    /// Generate a fresh identifier from the current wall clock + a per-call
    /// monotonic counter. Format: 32 lowercase hex characters.
    ///
    /// The two halves of the hex string are:
    ///
    /// 1. The current `SystemTime` rendered as nanoseconds since the UNIX
    ///    epoch (16 hex chars, big-endian u64).
    /// 2. A process-local atomic counter rendered as 16 hex chars
    ///    (big-endian u64). The counter starts at a random-ish seed derived
    ///    from the same nanosecond timestamp so it is non-zero on the very
    ///    first call.
    #[must_use]
    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let now_ns: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| {
                // u64::MAX nanoseconds = ~584 years from epoch; safe wrap.
                let secs = d.as_secs();
                let nanos = u64::from(d.subsec_nanos());
                secs.wrapping_mul(1_000_000_000).wrapping_add(nanos)
            })
            .unwrap_or(0);
        // Seed the counter on first use so collisions across two close
        // `generate()` calls in the same nanosecond bucket are still rare.
        let _ = COUNTER.compare_exchange(0, now_ns | 1, Ordering::Relaxed, Ordering::Relaxed);
        let bump = COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        Self(format!("{now_ns:016x}{bump:016x}"))
    }

    /// Borrow the raw identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for SessionId {
    type Err = SessionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Turn / TurnRole / ToolCall DTOs
// ---------------------------------------------------------------------------

/// Originator of a conversation turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TurnRole {
    /// Human / driver-side message.
    User,
    /// Assistant / model reply (may have triggered tool calls).
    Assistant,
    /// Result emitted by a tool the assistant invoked.
    Tool,
    /// System message (kick-off prompt, mid-session pinned instructions).
    System,
}

/// A single conversation turn.
///
/// The `prompt_tokens` and `completion_tokens` fields are filled in by the
/// caller using whatever token counter the underlying provider exposes; this
/// crate is **provider-agnostic** and never invokes a tokenizer itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    /// Stable identifier within the session (`t1`, `t2`, …, or a UUID).
    pub id: String,
    /// Who produced this turn.
    pub role: TurnRole,
    /// Free-form textual content.
    pub content: String,
    /// Wall-clock timestamp of when the turn was finalized.
    pub ts: DateTime<Utc>,
    /// Prompt-side tokens billed to this turn (caller-supplied).
    #[serde(default)]
    pub prompt_tokens: u32,
    /// Completion-side tokens billed to this turn (caller-supplied).
    #[serde(default)]
    pub completion_tokens: u32,
    /// USD cost of this turn (caller-supplied; typically from `aphrody-cost`).
    #[serde(default)]
    pub cost_usd: f64,
}

/// A single tool invocation by the assistant.
///
/// Both `result` and `error` are optional: a successful call yields
/// `result = Some(...)` / `error = None`, a failed call swaps them. A call
/// that is still in-flight at session close yields `result = None` and
/// `error = None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Stable identifier within the session (`call_abc123`).
    pub id: String,
    /// Tool name (matches the MCP tool name).
    pub name: String,
    /// JSON arguments handed to the tool.
    pub args: serde_json::Value,
    /// JSON result emitted by the tool on success.
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    /// Error message emitted by the tool on failure.
    #[serde(default)]
    pub error: Option<String>,
    /// Wall-clock timestamp of when the call was initiated.
    pub ts: DateTime<Utc>,
    /// Wall-clock duration in milliseconds (start → result/error).
    #[serde(default)]
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// SessionEvent — the union of everything that lands on the JSONL log
// ---------------------------------------------------------------------------

/// Discriminated union of everything that can land on a session's log.
///
/// Serialized as an untagged enum: every variant carries enough structural
/// information for serde to disambiguate on read. `SessionStart` /
/// `SessionEnd` use the `kind` field as a structural discriminator; `Turn`
/// and `ToolCall` rely on the presence of `role` vs. `name` / `args`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SessionEvent {
    /// Lifecycle marker — emitted as the first event of a fresh session.
    SessionStart {
        /// `"session_start"` discriminator (set automatically by `Session::new`).
        kind: SessionStartTag,
        /// Owning session id.
        session_id: SessionId,
        /// Wall-clock start time.
        ts: DateTime<Utc>,
        /// Free-form metadata carried over from `Session::new`.
        metadata: serde_json::Value,
    },
    /// Lifecycle marker — emitted by `Session::end` with final totals.
    SessionEnd {
        /// `"session_end"` discriminator.
        kind: SessionEndTag,
        /// Owning session id.
        session_id: SessionId,
        /// Wall-clock end time.
        ts: DateTime<Utc>,
        /// Total prompt + completion tokens consumed across every turn.
        total_tokens: u32,
        /// Total USD cost rolled up from every turn.
        total_cost_usd: f64,
    },
    /// One conversation turn.
    Turn(Turn),
    /// One tool invocation.
    ToolCall(ToolCall),
}

/// Phantom-typed `"session_start"` literal used as a serde discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStartTag {
    /// The single literal `"session_start"`.
    #[serde(rename = "session_start")]
    SessionStart,
}

/// Phantom-typed `"session_end"` literal used as a serde discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionEndTag {
    /// The single literal `"session_end"`.
    #[serde(rename = "session_end")]
    SessionEnd,
}

// ---------------------------------------------------------------------------
// Session — the in-memory aggregate
// ---------------------------------------------------------------------------

/// In-memory aggregate of one conversation session.
///
/// `events` is the canonical insertion-ordered log. `add_turn` / `add_tool_call`
/// append to it. The first event is always a `SessionStart` (stamped by
/// `Session::new`). Callers append a closing `SessionEnd` via `end()` before
/// persisting if they want the totals materialized on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// Stable identifier (used as filename in `JsonlFileStore`).
    pub id: SessionId,
    /// Wall-clock start.
    pub started_at: DateTime<Utc>,
    /// In-order log of every recorded event.
    pub events: Vec<SessionEvent>,
    /// Free-form metadata supplied at construction time.
    pub metadata: serde_json::Value,
}

impl Session {
    /// Build a fresh session. Stamps `started_at = Utc::now()` and pushes a
    /// `SessionStart` event onto the log.
    #[must_use]
    pub fn new(id: SessionId, metadata: serde_json::Value) -> Self {
        let started_at = Utc::now();
        let events = vec![SessionEvent::SessionStart {
            kind: SessionStartTag::SessionStart,
            session_id: id.clone(),
            ts: started_at,
            metadata: metadata.clone(),
        }];
        Self { id, started_at, events, metadata }
    }

    /// Append a [`Turn`] to the event log.
    pub fn add_turn(&mut self, turn: Turn) {
        tracing::debug!(target: "aphrody_session", id = %self.id, turn_id = %turn.id, "turn appended");
        self.events.push(SessionEvent::Turn(turn));
    }

    /// Append a [`ToolCall`] to the event log.
    pub fn add_tool_call(&mut self, call: ToolCall) {
        tracing::debug!(target: "aphrody_session", id = %self.id, call_id = %call.id, name = %call.name, "tool call appended");
        self.events.push(SessionEvent::ToolCall(call));
    }

    /// Roll-up of `prompt_tokens + completion_tokens` across every `Turn` in
    /// the log. `ToolCall` events do not contribute (tools have no token
    /// usage in our model — the surrounding assistant turn carries it).
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        let mut total: u32 = 0;
        for ev in &self.events {
            if let SessionEvent::Turn(t) = ev {
                total = total
                    .saturating_add(t.prompt_tokens)
                    .saturating_add(t.completion_tokens);
            }
        }
        total
    }

    /// Roll-up of `cost_usd` across every `Turn` in the log.
    #[must_use]
    pub fn total_cost_usd(&self) -> f64 {
        let mut total: f64 = 0.0;
        for ev in &self.events {
            if let SessionEvent::Turn(t) = ev {
                total += t.cost_usd;
            }
        }
        total
    }

    /// Elapsed wall-clock time between `started_at` and the timestamp of the
    /// latest event (or `started_at` itself if no other events were ever
    /// recorded — yielding `Duration::ZERO`).
    #[must_use]
    pub fn duration(&self) -> Duration {
        let latest = self.events.iter().rev().find_map(event_ts).unwrap_or(self.started_at);
        let delta = latest - self.started_at;
        // chrono::Duration -> std::time::Duration — clamp negatives to zero.
        delta
            .to_std()
            .unwrap_or(Duration::ZERO)
    }

    /// Finalize the session by appending a `SessionEnd` event carrying the
    /// rolled-up totals. Idempotent — calling twice appends two ends, which
    /// is intentional (you can mark milestones inside a longer session).
    pub fn end(&mut self) {
        let total_tokens = self.total_tokens();
        let total_cost_usd = self.total_cost_usd();
        self.events.push(SessionEvent::SessionEnd {
            kind: SessionEndTag::SessionEnd,
            session_id: self.id.clone(),
            ts: Utc::now(),
            total_tokens,
            total_cost_usd,
        });
    }

    /// Render the session as a JSON-Lines string: one event per line,
    /// terminated by a newline character. Order is insertion order.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::EmptySession`] if the log is empty, and
    /// [`SessionError::Json`] if any event fails to serialize.
    pub fn to_jsonl(&self) -> Result<String, SessionError> {
        if self.events.is_empty() {
            return Err(SessionError::EmptySession);
        }
        let mut out = String::new();
        for ev in &self.events {
            let line = serde_json::to_string(ev)?;
            out.push_str(&line);
            out.push('\n');
        }
        Ok(out)
    }

    /// Parse a JSON-Lines string back into a `Session`.
    ///
    /// The first line MUST decode to a `SessionStart` event — that is how we
    /// recover the session id, the start time, and the original metadata
    /// blob without needing a sidecar manifest.
    ///
    /// # Errors
    ///
    /// * [`SessionError::EmptySession`] if the input has no non-blank lines.
    /// * [`SessionError::InvalidJsonl`] if any line fails to decode, or if
    ///   the first event is not a `SessionStart`.
    pub fn from_jsonl(raw: &str) -> Result<Self, SessionError> {
        let mut events: Vec<SessionEvent> = Vec::new();
        for (idx, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let ev: SessionEvent = serde_json::from_str(trimmed)
                .map_err(|e| SessionError::InvalidJsonl(format!("line {}: {e}", idx + 1)))?;
            events.push(ev);
        }
        if events.is_empty() {
            return Err(SessionError::EmptySession);
        }
        let (id, started_at, metadata) = match &events[0] {
            SessionEvent::SessionStart { session_id, ts, metadata, .. } => {
                (session_id.clone(), *ts, metadata.clone())
            }
            _ => {
                return Err(SessionError::InvalidJsonl(
                    "first event must be `session_start`".into(),
                ));
            }
        };
        Ok(Self { id, started_at, events, metadata })
    }
}

/// Project the timestamp out of any [`SessionEvent`] variant.
fn event_ts(ev: &SessionEvent) -> Option<DateTime<Utc>> {
    Some(match ev {
        SessionEvent::SessionStart { ts, .. } => *ts,
        SessionEvent::SessionEnd { ts, .. } => *ts,
        SessionEvent::Turn(t) => t.ts,
        SessionEvent::ToolCall(c) => c.ts,
    })
}

// ---------------------------------------------------------------------------
// SessionStore trait + JsonlFileStore impl
// ---------------------------------------------------------------------------

/// Async persistence layer for [`Session`] aggregates.
///
/// Implementations are expected to be cheap to clone (or wrapped in `Arc`) so
/// the same handle can be shared across many tokio tasks.
#[allow(async_fn_in_trait)]
pub trait SessionStore: Send + Sync {
    /// Persist (or overwrite) a session.
    async fn save_session(&self, session: &Session) -> Result<(), SessionError>;

    /// Load a previously persisted session.
    async fn load_session(&self, id: &SessionId) -> Result<Session, SessionError>;

    /// Enumerate every session id currently held by the store.
    async fn list_sessions(&self) -> Result<Vec<SessionId>, SessionError>;

    /// Remove a session from the store.
    async fn delete_session(&self, id: &SessionId) -> Result<(), SessionError>;
}

/// JSON-Lines file backed store. One `{session_id}.jsonl` file per session.
///
/// All writes go through `tokio::fs` and are made atomic via the
/// tmp-file + rename pattern borrowed from
/// [`aphrody_cron::JsonFileJobStore`]. This guards against torn writes if
/// the process dies mid-save.
#[derive(Debug, Clone)]
pub struct JsonlFileStore {
    dir: PathBuf,
}

impl JsonlFileStore {
    /// Create a new store rooted at `dir`. The directory is created on first
    /// write — `new` itself never touches the filesystem.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Borrow the backing directory path.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn file_for(&self, id: &SessionId) -> PathBuf {
        self.dir.join(format!("{}.jsonl", id.as_str()))
    }
}

impl SessionStore for JsonlFileStore {
    async fn save_session(&self, session: &Session) -> Result<(), SessionError> {
        let jsonl = session.to_jsonl()?;
        afs::create_dir_all(&self.dir).await?;
        let target = self.file_for(&session.id);
        let tmp = target.with_extension("jsonl.tmp");
        // Atomic write: write-tmp + flush + rename-over-target.
        {
            let mut f = afs::File::create(&tmp).await?;
            f.write_all(jsonl.as_bytes()).await?;
            f.flush().await?;
            f.sync_all().await?;
        }
        afs::rename(&tmp, &target).await?;
        tracing::debug!(
            target: "aphrody_session",
            id = %session.id,
            events = session.events.len(),
            "session persisted",
        );
        Ok(())
    }

    async fn load_session(&self, id: &SessionId) -> Result<Session, SessionError> {
        let path = self.file_for(id);
        if !path.exists() {
            return Err(SessionError::NotFound(id.to_string()));
        }
        let bytes = afs::read(&path).await?;
        let raw = String::from_utf8_lossy(&bytes);
        Session::from_jsonl(&raw)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionId>, SessionError> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut out: HashSet<SessionId> = HashSet::new();
        let mut rd = afs::read_dir(&self.dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if ext != "jsonl" {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if let Ok(id) = SessionId::new(stem.to_string()) {
                out.insert(id);
            }
        }
        let mut ids: Vec<SessionId> = out.into_iter().collect();
        ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(ids)
    }

    async fn delete_session(&self, id: &SessionId) -> Result<(), SessionError> {
        let path = self.file_for(id);
        if !path.exists() {
            return Err(SessionError::NotFound(id.to_string()));
        }
        afs::remove_file(&path).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// replay — synchronous deterministic walk
// ---------------------------------------------------------------------------

/// Walk every event of `session` in insertion order, calling `on_event` once
/// per event. Returns the number of events visited.
///
/// Pure-sync on purpose: replay is consumed by debuggers / test harnesses /
/// CLI inspectors that have no async runtime around them. Callers that want
/// to fire off async work per event should collect first and then spawn.
pub fn replay<F: FnMut(&SessionEvent)>(session: &Session, mut on_event: F) -> usize {
    let mut n: usize = 0;
    for ev in &session.events {
        on_event(ev);
        n += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// In-crate unit tests (additional smoke coverage lives under tests/)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_generate_is_unique_per_call() {
        let a = SessionId::generate();
        let b = SessionId::generate();
        assert_ne!(a, b);
        assert_eq!(a.as_str().len(), 32, "expected 32 lowercase hex chars");
        assert!(a.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn session_id_from_str_rejects_empty() {
        let r: Result<SessionId, _> = "".parse();
        assert!(r.is_err());
    }

    #[test]
    fn session_id_display_roundtrips_via_from_str() {
        let id = SessionId::generate();
        let s = id.to_string();
        let back: SessionId = s.parse().unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn new_session_pushes_session_start_event() {
        let id = SessionId::generate();
        let s = Session::new(id.clone(), serde_json::json!({"k": "v"}));
        assert_eq!(s.events.len(), 1);
        match &s.events[0] {
            SessionEvent::SessionStart { session_id, metadata, .. } => {
                assert_eq!(session_id, &id);
                assert_eq!(metadata, &serde_json::json!({"k": "v"}));
            }
            other => panic!("unexpected first event: {other:?}"),
        }
    }

    #[test]
    fn end_session_appends_session_end_with_totals() {
        let mut s = Session::new(SessionId::generate(), serde_json::json!({}));
        s.add_turn(Turn {
            id: "t1".into(),
            role: TurnRole::Assistant,
            content: "hi".into(),
            ts: Utc::now(),
            prompt_tokens: 10,
            completion_tokens: 5,
            cost_usd: 0.0001,
        });
        s.end();
        match s.events.last() {
            Some(SessionEvent::SessionEnd { total_tokens, total_cost_usd, .. }) => {
                assert_eq!(*total_tokens, 15);
                assert!((*total_cost_usd - 0.0001).abs() < 1e-12);
            }
            other => panic!("expected SessionEnd, got {other:?}"),
        }
    }
}
