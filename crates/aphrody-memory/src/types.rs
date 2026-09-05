// SPDX-License-Identifier: Apache-2.0
//! Tier-1 memory provider types — shared DTOs and error envelope.
//!
//! This module defines the surface consumed by [`crate::provider::MemoryProvider`]
//! implementations (Mem0, Honcho, SqliteLocal). The shapes here are intentionally
//! provider-agnostic: each backend maps its native JSON / table layout onto
//! [`MemoryRecord`] at the seam.
//!
//! ## Rationale
//!
//! The original [`crate::MemoryBackend`] trait targets vector recall (key/value
//! + embedding search). The Tier-1 [`crate::provider::MemoryProvider`] trait
//! targets *agent long-term context*: tag-filtered keyword search across runs,
//! synced to a hosted service (Mem0, Honcho) or a local sqlite for offline
//! mode. The two traits coexist — different consumers want different shapes.
//!
//! ## Stability
//!
//! Field names are stable across providers. Provider-specific extras live in
//! the open-ended `metadata` `serde_json::Value` map so a Mem0 categories
//! array or a Honcho session id can round-trip without a schema change.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// MemoryRecord — provider-agnostic long-term memory entry.
// ─────────────────────────────────────────────────────────────────────────────

/// A single long-term memory entry as exchanged through a [`MemoryProvider`].
///
/// Field-by-field semantics:
///
/// - `id`: provider-assigned (HTTP providers) or UUID v4 (local sqlite). Stable
///   for `get` / `delete`.
/// - `agent_id`: logical owner — either a Mem0 `user_id`, a Honcho `user_id`,
///   or any opaque caller-defined string for sqlite.
/// - `content`: free-form text (often a single conversational turn).
/// - `tags`: caller-chosen labels (`"fact"`, `"preference"`, `"todo"`, …).
///   Backends without native tag support encode this list inside `metadata`.
/// - `created_at_unix_ms`: epoch milliseconds — wall clock at insertion time.
///   Chosen over RFC 3339 strings for cheaper sort + range queries.
/// - `metadata`: open-ended JSON blob. Providers stash native fields here
///   (`session_id`, `categories`, etc.) so callers can drill in without
///   losing fidelity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Stable identifier — provider-assigned or locally generated.
    pub id: String,
    /// Logical owner (user_id / session owner).
    pub agent_id: String,
    /// Human-readable / machine-parseable content body.
    pub content: String,
    /// Caller-chosen labels for filtering at search time.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Wall-clock creation timestamp (epoch milliseconds, UTC).
    pub created_at_unix_ms: u64,
    /// Provider-specific extras and any caller-supplied metadata.
    #[serde(default = "default_metadata")]
    pub metadata: serde_json::Value,
}

fn default_metadata() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl MemoryRecord {
    /// Convenience constructor that sets `created_at_unix_ms` to the current
    /// wall-clock time and leaves `metadata` as an empty object.
    ///
    /// `id` is left empty — Mem0 / Honcho assign theirs on the server side,
    /// and [`crate::sqlite_local::SqliteLocalProvider::add`] generates a UUID
    /// v4 locally. Pass any string for offline construction (tests, fixtures).
    #[must_use]
    pub fn new(id: impl Into<String>, agent_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            agent_id: agent_id.into(),
            content: content.into(),
            tags: Vec::new(),
            created_at_unix_ms: now_unix_ms(),
            metadata: default_metadata(),
        }
    }

    /// Builder helper — attach tag list, replacing any existing tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder helper — attach metadata blob.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Wall-clock now in epoch milliseconds.
///
/// Saturates at `u64::MAX` if `SystemTime::now()` is somehow before the Unix
/// epoch (clock skew on embedded targets). Never panics.
#[must_use]
pub fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryQuery — search filter envelope.
// ─────────────────────────────────────────────────────────────────────────────

/// Search filter passed to [`crate::provider::MemoryProvider::search`].
///
/// All fields except `agent_id` are optional and combine with AND semantics:
/// a record matches iff its `agent_id` equals `MemoryQuery::agent_id` **and**
/// (`q` is `None` or `content` contains `q` case-insensitively) **and**
/// (every tag in `MemoryQuery::tags` is present on the record).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Required — restrict search to this logical owner.
    pub agent_id: String,
    /// Optional keyword (substring/contains semantics on `content`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    /// Optional tag filter — every tag must be present on the record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Optional maximum number of records to return (server-side capped).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl MemoryQuery {
    /// Build the minimal query: just an `agent_id`, no filters.
    #[must_use]
    pub fn for_agent(agent_id: impl Into<String>) -> Self {
        Self { agent_id: agent_id.into(), q: None, tags: Vec::new(), limit: None }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ProviderKind — discriminator returned by `MemoryProvider::provider_kind`.
// ─────────────────────────────────────────────────────────────────────────────

/// Identifier for the concrete provider behind a `dyn MemoryProvider`.
///
/// Used for logging, metrics labelling, and conditional fallback (e.g. the CLI
/// chooses SqliteLocal when no API key is set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Hosted Mem0 cloud (<https://mem0.ai>).
    Mem0,
    /// Honcho v1 multi-tenant memory backend.
    Honcho,
    /// Local rusqlite-backed offline store.
    SqliteLocal,
    /// Embedded LanceDB vector store (via [`crate::lancedb_adapter::LanceDbAdapter`]).
    LanceDb,
}

impl ProviderKind {
    /// Stable lowercase identifier suitable for logs and metrics labels.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mem0 => "mem0",
            Self::Honcho => "honcho",
            Self::SqliteLocal => "sqlite_local",
            Self::LanceDb => "lancedb",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryConfig — provider selection + connection params.
// ─────────────────────────────────────────────────────────────────────────────

/// Extension config for the Tier-1 memory layer.
///
/// Intended to be folded into a higher-level configuration object (the
/// `aphrody-terminal-config` `TerminalConfig`, the CLI top-level settings, …)
/// via composition. Standalone so this crate is consumable without pulling
/// the terminal-config schema crate into its dep graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Which provider to instantiate.
    pub provider: ProviderKind,
    /// Name of the environment variable holding the API key (Mem0 / Honcho).
    /// Ignored by `SqliteLocal`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Override the provider's default base URL (self-hosted endpoints, tests).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Filesystem path for `SqliteLocal`; ignored by HTTP providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sqlite_path: Option<PathBuf>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { provider: ProviderKind::SqliteLocal, api_key_env: None, base_url: None, sqlite_path: None }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryError — unified error envelope.
// ─────────────────────────────────────────────────────────────────────────────

/// All failures surfacable from a [`crate::provider::MemoryProvider`] call.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// Required configuration value (env var, field…) is missing.
    #[error("missing configuration: {0}")]
    MissingConfig(&'static str),

    /// HTTP transport-level failure (DNS, TCP, TLS, decode).
    #[error("HTTP transport error: {0}")]
    Http(String),

    /// Remote provider returned an explicit non-2xx envelope.
    #[error("provider API error ({status}): {message}")]
    ApiError {
        /// HTTP status code as returned by the provider.
        status: u16,
        /// Human-readable explanation lifted from the response body.
        message: String,
    },

    /// A record was requested by id but does not exist.
    #[error("record not found: {id}")]
    NotFound {
        /// The id that yielded no match.
        id: String,
    },

    /// JSON (de)serialisation failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// SQLite driver error.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// I/O error from the local filesystem.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Caller supplied an invalid argument.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Catch-all for unexpected provider payloads.
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

impl From<reqwest::Error> for MemoryError {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_new_sets_timestamp() {
        let rec = MemoryRecord::new("id-1", "agent-A", "hello");
        assert_eq!(rec.id, "id-1");
        assert_eq!(rec.agent_id, "agent-A");
        assert_eq!(rec.content, "hello");
        assert!(rec.tags.is_empty());
        assert!(rec.created_at_unix_ms > 0);
    }

    #[test]
    fn record_with_tags_chains() {
        let rec = MemoryRecord::new("", "agent-A", "x")
            .with_tags(vec!["fact".into(), "preference".into()]);
        assert_eq!(rec.tags, vec!["fact".to_string(), "preference".to_string()]);
    }

    #[test]
    fn query_for_agent_minimal() {
        let q = MemoryQuery::for_agent("agent-Z");
        assert_eq!(q.agent_id, "agent-Z");
        assert!(q.q.is_none());
        assert!(q.tags.is_empty());
        assert!(q.limit.is_none());
    }

    #[test]
    fn provider_kind_str_is_snake_case() {
        assert_eq!(ProviderKind::Mem0.as_str(), "mem0");
        assert_eq!(ProviderKind::Honcho.as_str(), "honcho");
        assert_eq!(ProviderKind::SqliteLocal.as_str(), "sqlite_local");
        assert_eq!(ProviderKind::LanceDb.as_str(), "lancedb");
    }

    #[test]
    fn memory_config_default_is_offline() {
        let cfg = MemoryConfig::default();
        assert_eq!(cfg.provider, ProviderKind::SqliteLocal);
        assert!(cfg.api_key_env.is_none());
    }
}
