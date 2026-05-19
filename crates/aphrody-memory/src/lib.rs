// SPDX-License-Identifier: Apache-2.0
//! # aphrody-memory
//!
//! Async [`MemoryBackend`] trait and four production-ready implementations:
//!
//! - [`jsonl::JsonlBackend`] — durable JSONL file store whose envelope shape mirrors the
//!   `.coord/inbox-from-aphrody.jsonl` A2A protocol so that a [`MemoryRecord`] **is** a
//!   coordination envelope.
//! - [`hnsw::HnswBackend`] — in-process approximate nearest-neighbour index backed by brute-force
//!   cosine similarity (pure Rust, zero FFI).  Persists the index to `<root>/index.bin` via
//!   `serde_json`.
//! - [`sqlite::SqliteBackend`] — single-file SQLite store via `rusqlite` (bundled). Brute-force
//!   cosine search over a `BLOB` embedding column; acceptable up to ~100k rows.
//! - [`lancedb::LanceDbBackend`] — Arrow-native vector database via the `lancedb` crate, ported
//!   from `openclaw/extensions/memory-lancedb` (TypeScript). Scales past 100k rows with ANN.
//!
//! ## Design invariants
//!
//! - Every public function is `async` and returns `Result<_, MemoryError>`.
//! - No `unwrap` / `expect` in production paths.
//! - The crate is `#![forbid(unsafe_code)]`.
//! - All backends implement [`MemoryBackend`] and can be used interchangeably.

#![forbid(unsafe_code)]

pub mod hnsw;
pub mod jsonl;
pub mod lancedb;
pub mod sqlite;

// ─────────────────────────────────────────────────────────────────────────────
// Tier-1 provider surface (Mem0, Honcho, SqliteLocal).
// Added 2026-05-19 as part of the hermes-agent audit close-out (Sprint C).
// Coexists with the legacy `MemoryBackend` trait above: providers expose
// an agent-id + tags + keyword surface, while `MemoryBackend` exposes a
// key-value + vector surface. Different consumers, different shapes.
// ─────────────────────────────────────────────────────────────────────────────
pub mod eviction;
pub mod honcho;
pub mod mem0;
pub mod migrate;
pub mod provider;
pub mod schema;
pub mod sqlite_local;
pub mod types;

pub use crate::eviction::EvictionPolicy;
pub use crate::hnsw::HnswBackend;
pub use crate::honcho::HonchoProvider;
pub use crate::lancedb::LanceDbBackend;
pub use crate::mem0::Mem0Provider;
pub use crate::migrate::{MAX_ID_PREVIEW, MigrationDiff, migrate as migrate_provider};
pub use crate::provider::MemoryProvider;
pub use crate::schema::{AnyMemoryRecord, MemoryRecordV1, MemoryRecordV2, MigrationReport, SchemaVersion, upgrade_jsonl};
pub use crate::sqlite::SqliteBackend;
pub use crate::sqlite_local::SqliteLocalProvider;
// Tier-1 provider DTOs: re-exported under disambiguated names so they don't
// collide with the legacy `MemoryBackend` types defined further down in this
// file. Tests and downstream consumers can either use these aliases or the
// fully-qualified `aphrody_memory::types::*` paths.
pub use crate::types::{
    MemoryConfig, MemoryError as ProviderMemoryError, MemoryQuery,
    MemoryRecord as ProviderMemoryRecord, ProviderKind,
};

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A floating-point embedding vector.
pub type Embedding = Vec<f32>;

/// A single memory record stored by any [`MemoryBackend`].
///
/// The field layout deliberately mirrors the `.coord` A2A envelope schema
/// (`id`, `ns` ≈ `from`, `content` ≈ `body`) so records can be serialised
/// to or deserialized from the coordination mailboxes without a conversion
/// layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Stable unique identifier (UUID v4).
    pub id: Uuid,
    /// Namespace — logical partition, e.g. `"sessions"`, `"coord"`, `"facts"`.
    pub ns: String,
    /// Human-readable / machine-parseable content body.
    pub content: String,
    /// Optional embedding vector.  `None` means the record has not been
    /// embedded yet; [`MemoryBackend::search`] implementations must skip
    /// un-embedded records rather than panic.
    pub embedding: Option<Embedding>,
    /// Arbitrary key/value metadata (type/subject/re fields from A2A envelope,
    /// etc.).
    pub metadata: HashMap<String, serde_json::Value>,
    /// Wall-clock timestamp when this record was first persisted (RFC 3339).
    pub created_at: DateTime<Utc>,
    /// Wall-clock timestamp of the most recent mutation.
    pub updated_at: DateTime<Utc>,
}

impl MemoryRecord {
    /// Construct a new record with sane defaults.
    ///
    /// - `id` is generated via `Uuid::new_v4()`.
    /// - `created_at` and `updated_at` are set to `Utc::now()`.
    /// - `embedding` defaults to `None`.
    /// - `metadata` defaults to an empty map.
    pub fn new(ns: impl Into<String>, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            ns: ns.into(),
            content: content.into(),
            embedding: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Attach an embedding to an existing record, bumping `updated_at`.
    pub fn with_embedding(mut self, emb: Embedding) -> Self {
        self.embedding = Some(emb);
        self.updated_at = Utc::now();
        self
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// All errors that can be returned by a [`MemoryBackend`] implementation.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// An I/O error from the underlying filesystem or network.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialisation failure.
    #[error("serialisation error: {0}")]
    Serde(#[from] serde_json::Error),

    /// A record was requested by key but does not exist in the backend.
    #[error("record not found: {key}")]
    NotFound { key: String },

    /// The backend is not initialised or has been closed.
    #[error("backend not ready: {reason}")]
    NotReady { reason: String },

    /// The caller supplied an invalid argument.
    #[error("invalid argument: {message}")]
    InvalidArgument { message: String },

    /// An unexpected internal error not covered by the variants above.
    #[error("internal error: {message}")]
    Internal { message: String },
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Async key/value + vector memory storage trait.
///
/// Implementations must be `Send + Sync` so they can be shared across Tokio
/// tasks via `Arc<Mutex<dyn MemoryBackend>>` or similar patterns.
///
/// ### Key semantics
///
/// Keys are opaque UTF-8 strings.  The convention used by both built-in
/// backends is `"<ns>/<uuid>"`, but callers may choose any scheme as long as
/// it does not contain path separators that would confuse filesystem backends.
///
/// ### Search semantics
///
/// [`search`][MemoryBackend::search] uses cosine similarity by default.
/// Records without an [`Embedding`] are silently skipped.  Results are
/// returned in descending score order (highest similarity first).
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Persist `value` under `key`, replacing any existing record.
    async fn put(&mut self, key: &str, value: MemoryRecord) -> Result<(), MemoryError>;

    /// Retrieve the record stored under `key`, or `Ok(None)` if absent.
    async fn get(&self, key: &str) -> Result<Option<MemoryRecord>, MemoryError>;

    /// Return up to `top_k` records ordered by descending cosine similarity
    /// to `query`.  Records without an embedding are excluded.
    async fn search(
        &self,
        query: &Embedding,
        top_k: usize,
    ) -> Result<Vec<(MemoryRecord, f32)>, MemoryError>;

    /// Remove the record stored under `key`.  Returns `Ok(())` whether or not
    /// the key existed (idempotent delete).
    async fn delete(&mut self, key: &str) -> Result<(), MemoryError>;

    /// List up to `limit` records whose key starts with `prefix`.
    /// An empty `prefix` matches all records.
    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<MemoryRecord>, MemoryError>;
}

// ---------------------------------------------------------------------------
// Shared math utilities (used by both backends)
// ---------------------------------------------------------------------------

/// Compute cosine similarity between two equal-length vectors.
///
/// Returns a value in `[-1.0, 1.0]`.  Returns `0.0` if either vector has
/// zero magnitude to avoid division by zero.
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "embedding dimension mismatch");
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0_f32, 0.0, 0.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6, "identical vectors must have sim=1.0, got {sim}");
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0_f32, 0.0, 0.0];
        let b = vec![0.0_f32, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "orthogonal vectors must have sim=0.0, got {sim}");
    }

    #[test]
    fn cosine_zero_vector() {
        let a = vec![0.0_f32, 0.0, 0.0];
        let b = vec![1.0_f32, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "zero-vector must return 0.0");
    }

    #[test]
    fn memory_record_new_has_no_embedding() {
        let r = MemoryRecord::new("test-ns", "hello");
        assert!(r.embedding.is_none());
        assert_eq!(r.ns, "test-ns");
        assert_eq!(r.content, "hello");
    }

    #[test]
    fn memory_record_with_embedding_sets_field() {
        let emb = vec![0.1, 0.2, 0.3];
        let r = MemoryRecord::new("ns", "body").with_embedding(emb.clone());
        assert_eq!(r.embedding.as_deref(), Some(emb.as_slice()));
    }
}
