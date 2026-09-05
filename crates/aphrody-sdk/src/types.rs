// SPDX-License-Identifier: Apache-2.0
//! Public DTOs for the aphrody SDK.
//!
//! Every type in this module is part of the **public SDK surface**: bumping or
//! breaking any of them is a semver-major event from v1.0 onward. We keep them
//! free of dependencies on the heavy `aphrody-chat` orchestrator so the SDK
//! stays usable even when the chat loop is being rebuilt (cf. the mega-batch
//! note in `Cargo.toml`).
//!
//! TS provenance: `packages/gemini-cli/packages/sdk/src/types.ts`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// SdkError
// ---------------------------------------------------------------------------

/// Failure surface of the SDK. Aggregates the underlying error types from
/// every wired building block so consumers only ever need to match on this
/// one enum.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Error)]
pub enum SdkError {
    /// The SDK was constructed with an invalid configuration (empty model
    /// name, working directory that does not exist, etc.).
    #[error("invalid sdk configuration: {0}")]
    InvalidConfig(String),

    /// A required runtime dependency is missing on the host. The most common
    /// case is the `gemini` CLI binary not being on `PATH` when the
    /// `GeminiBackend` is selected.
    #[error("runtime dependency missing: {0}")]
    MissingRuntime(String),

    /// I/O failure (filesystem read/write, child process spawn, ...).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Forwarded from [`aphrody_session::SessionError`].
    #[error("session error: {0}")]
    Session(#[from] aphrody_session::SessionError),

    /// Forwarded from [`aphrody_tools::ToolsError`].
    #[error("tools error: {0}")]
    Tools(#[from] aphrody_tools::ToolsError),

    /// The agent backend reported a turn-level failure (network 5xx, model
    /// safety block, malformed stream-json line, ...).
    #[error("backend error: {0}")]
    Backend(String),

    /// Path access denied by the configured policy.
    #[error("access denied: {0}")]
    AccessDenied(String),

    /// A shell command requested interactive confirmation but the SDK runs
    /// headless and cannot serve a prompt.
    #[error("interactive confirmation required for: {0}")]
    InteractiveRequired(String),

    /// Operation not supported on the current target. WASM builds in
    /// particular cannot perform direct filesystem or shell operations.
    #[error("operation unsupported on this platform: {0}")]
    Unsupported(&'static str),
}

/// Result alias used pervasively across the SDK.
///
/// PUBLIC API — semver stable from v1.0.
pub type SdkResult<T> = Result<T, SdkError>;

// ---------------------------------------------------------------------------
// SdkConfig
// ---------------------------------------------------------------------------

/// Top-level configuration for [`crate::AphrodySdk::new`].
///
/// Mirrors the TS `GeminiCliAgentOptions` (`packages/gemini-cli/packages/sdk/
/// src/types.ts`) but trims the JS-specific fields (Node `process.cwd()`,
/// `recordResponses`/`fakeResponses` JSON-replay) that do not map cleanly to
/// the Rust runtime.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkConfig {
    /// System instructions sent to the model on every turn. Empty string by
    /// default — callers MUST set this to a non-empty value for production
    /// usage.
    #[serde(default)]
    pub instructions: String,

    /// Model identifier used by the underlying backend. Defaults to
    /// `gemini-2.5-pro` for the Gemini backend, ignored by the stub backend.
    #[serde(default = "default_model")]
    pub model: String,

    /// Working directory the agent should treat as its sandbox root. Tool
    /// filesystem / shell operations are anchored here. Defaults to the
    /// process working directory on native targets.
    ///
    /// On WASM (`wasm32-unknown-unknown`), this field is honoured only by
    /// in-memory virtual file system hooks consumers wire themselves — direct
    /// filesystem access is unavailable.
    #[serde(default)]
    pub cwd: Option<PathBuf>,

    /// Verbose tracing toggle. When `true`, the SDK emits `tracing::debug!`
    /// events on every step of the turn loop (backend select, tool call,
    /// session persistence, ...).
    #[serde(default)]
    pub debug: bool,

    /// Skill directories to discover at construction time. Each path is
    /// recursively walked for `SKILL.md` files via
    /// [`aphrody_skills::runtime::discover_skills_in_path`].
    #[serde(default)]
    pub skill_dirs: Vec<PathBuf>,

    /// Optional session persistence directory. When `Some(dir)`, every turn
    /// recorded by the agent is appended to `{dir}/{session_id}.jsonl` via
    /// [`aphrody_session::JsonlFileStore`]. When `None`, sessions live only
    /// in memory and vanish on drop.
    #[serde(default)]
    pub session_dir: Option<PathBuf>,
}

fn default_model() -> String {
    "gemini-2.5-pro".to_string()
}

impl Default for SdkConfig {
    fn default() -> Self {
        Self {
            instructions: String::new(),
            model: default_model(),
            cwd: None,
            debug: false,
            skill_dirs: Vec::new(),
            session_dir: None,
        }
    }
}

impl SdkConfig {
    /// Convenience constructor used by tests and quick-start examples.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn with_instructions(instructions: impl Into<String>) -> Self {
        Self {
            instructions: instructions.into(),
            ..Self::default()
        }
    }

    /// Chainable: override the working directory.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    /// Chainable: enable debug tracing.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn debug(mut self, on: bool) -> Self {
        self.debug = on;
        self
    }

    /// Chainable: push a skill discovery root.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn add_skill_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.skill_dirs.push(dir.into());
        self
    }

    /// Chainable: configure session persistence.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn session_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.session_dir = Some(dir.into());
        self
    }

    /// Validate the config — called from [`crate::AphrodySdk::new`]. Empty
    /// instructions are *allowed* (the stub backend ignores them) but the
    /// `model` field MUST be non-empty.
    ///
    /// # Errors
    /// Returns [`SdkError::InvalidConfig`] when validation fails.
    pub fn validate(&self) -> SdkResult<()> {
        if self.model.trim().is_empty() {
            return Err(SdkError::InvalidConfig("model must be non-empty".into()));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SessionContext
// ---------------------------------------------------------------------------

/// Contextual snapshot handed to tools when they execute.
///
/// Mirrors the TS `SessionContext` struct (`packages/gemini-cli/packages/sdk/
/// src/types.ts`). The full `transcript` is intentionally kept off this struct
/// in v1 — tools that need conversation history can fetch it via the
/// `SessionManager` re-exported by [`crate::session`].
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// Owning session identifier (matches [`aphrody_session::SessionId`]).
    pub session_id: String,
    /// Current working directory of the agent.
    pub cwd: PathBuf,
    /// RFC-3339 timestamp at the moment the context was minted.
    pub timestamp: String,
}

impl SessionContext {
    /// Construct a context for the given session id / cwd, stamping the
    /// timestamp from the system clock.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn new(session_id: impl Into<String>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            session_id: session_id.into(),
            cwd: cwd.into(),
            timestamp: chrono_like_iso8601(),
        }
    }
}

/// Lightweight RFC-3339 timestamp from `std::time` (avoids pulling
/// `chrono` into the public SDK API surface for a single formatting need).
fn chrono_like_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // RFC-3339 in UTC, second precision (sufficient for context timestamps).
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    // 1970-01-01 + days → YYYY-MM-DD via the civil-from-days algorithm.
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days` (proleptic Gregorian, public domain).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y } as i32;
    (y, m, d)
}

// ---------------------------------------------------------------------------
// TurnResult
// ---------------------------------------------------------------------------

/// Outcome of a single agent turn.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnResult {
    /// Assistant-side reply (concatenated text from every model chunk).
    pub assistant: String,
    /// Number of tool calls invoked during the turn (0 for pure-text turns).
    #[serde(default)]
    pub tool_call_count: u32,
    /// Wall-clock duration of the turn, in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_validates() {
        let cfg = SdkConfig::default();
        cfg.validate().expect("default config must validate");
        assert_eq!(cfg.model, "gemini-2.5-pro");
    }

    #[test]
    fn validate_rejects_empty_model() {
        let mut cfg = SdkConfig::default();
        cfg.model = String::new();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn builder_chain_round_trips() {
        let cfg = SdkConfig::with_instructions("be helpful")
            .debug(true)
            .add_skill_dir("/tmp/skills")
            .session_dir("/tmp/sessions");
        assert_eq!(cfg.instructions, "be helpful");
        assert!(cfg.debug);
        assert_eq!(cfg.skill_dirs.len(), 1);
        assert_eq!(cfg.session_dir.as_deref(), Some(std::path::Path::new("/tmp/sessions")));
    }

    #[test]
    fn iso8601_timestamp_is_sane() {
        let ctx = SessionContext::new("sid-1", "/tmp");
        // Format: YYYY-MM-DDTHH:MM:SSZ → 20 chars.
        assert_eq!(ctx.timestamp.len(), 20);
        assert!(ctx.timestamp.ends_with('Z'));
        assert_eq!(ctx.session_id, "sid-1");
    }
}
