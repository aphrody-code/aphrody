// SPDX-License-Identifier: Apache-2.0
//! # `SqliteBackend`
//!
//! A durable [`MemoryBackend`] backed by a single-file SQLite database.
//!
//! ## Schema
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS memory_items (
//!   id          TEXT PRIMARY KEY,
//!   ns          TEXT NOT NULL,
//!   content     TEXT NOT NULL,
//!   embedding   BLOB,                          -- little-endian f32 packed array
//!   metadata    TEXT NOT NULL,                 -- JSON object
//!   created_at  INTEGER NOT NULL,              -- unix millis
//!   updated_at  INTEGER NOT NULL               -- unix millis
//! );
//! CREATE INDEX IF NOT EXISTS idx_memory_items_created_at
//!     ON memory_items(created_at);
//! CREATE INDEX IF NOT EXISTS idx_memory_items_ns
//!     ON memory_items(ns);
//! ```
//!
//! The caller-supplied storage key (see [`MemoryBackend::put`]) is used as the
//! `id` column verbatim — this matches the `JsonlBackend` convention of
//! `"<ns>/<uuid>"` keys while remaining schema-agnostic.
//!
//! ## Similarity search
//!
//! SQLite has no native vector index.  [`SqliteBackend::search`] performs a
//! full table scan, deserialises each embedding from its `BLOB` row, and
//! computes cosine similarity in Rust (same helper as the JSONL backend).  This
//! is acceptable up to ~100k rows.  For larger corpora use the LanceDB
//! backend, which carries a real ANN index.
//!
//! ## Concurrency
//!
//! `rusqlite::Connection` is `!Send` *between threads*, so this backend wraps
//! the connection in an `Arc<tokio::sync::Mutex<_>>` and dispatches every call
//! through [`tokio::task::spawn_blocking`].  The backend is therefore
//! `Send + Sync` and safe to share across Tokio tasks via `Arc<…>` /
//! `RwLock<…>` without further wrapping.
//!
//! ## Bundled SQLite
//!
//! The `rusqlite` workspace dependency declares the `bundled` feature, so this
//! backend does **not** depend on a system libsqlite3 install — it ships a
//! statically linked copy.  This keeps cross-compilation deterministic on
//! Linux / Windows / macOS (cf. `CLAUDE.md` §0 platform priority).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use tokio::{sync::Mutex, task};
use tracing::{debug, trace};
use uuid::Uuid;

use crate::{Embedding, MemoryBackend, MemoryError, MemoryRecord, cosine_similarity};

// ---------------------------------------------------------------------------
// Schema constants
// ---------------------------------------------------------------------------

const SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS memory_items (
    id          TEXT PRIMARY KEY,
    ns          TEXT NOT NULL,
    content     TEXT NOT NULL,
    embedding   BLOB,
    metadata    TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_memory_items_created_at ON memory_items(created_at);
CREATE INDEX IF NOT EXISTS idx_memory_items_ns         ON memory_items(ns);
";

// ---------------------------------------------------------------------------
// Backend struct
// ---------------------------------------------------------------------------

/// SQLite-backed memory store.
///
/// The on-disk file lives at the path supplied to [`Self::open`].  Use
/// `":memory:"` for an ephemeral in-process database (handy in tests).
pub struct SqliteBackend {
    path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl SqliteBackend {
    /// Open (or create) a SQLite-backed memory store at `path`.
    ///
    /// Pass `":memory:"` as `path` for an ephemeral database.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Internal`] when the connection or schema
    /// initialisation fails.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let path = path.as_ref().to_owned();
        let path_clone = path.clone();
        let conn = task::spawn_blocking(move || -> Result<Connection, rusqlite::Error> {
            let conn = if path_clone.as_os_str() == ":memory:" {
                Connection::open_in_memory()?
            } else {
                if let Some(parent) = path_clone.parent()
                    && !parent.as_os_str().is_empty()
                {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                    })?;
                }
                Connection::open(&path_clone)?
            };
            // Pragmas: WAL for concurrent reads + sane synchronous mode.
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "synchronous", "NORMAL")?;
            conn.execute_batch(SCHEMA_SQL)?;
            Ok(conn)
        })
        .await
        .map_err(|e| MemoryError::Internal { message: format!("join error: {e}") })?
        .map_err(|e| MemoryError::Internal { message: format!("sqlite open: {e}") })?;

        debug!(path = %path.display(), "opened sqlite backend");
        Ok(Self { path, conn: Arc::new(Mutex::new(conn)) })
    }

    /// Filesystem path the backend was opened with (mostly for diagnostics).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Convenience helper for callers wanting a fresh in-memory store.
    ///
    /// # Errors
    ///
    /// Same conditions as [`Self::open`].
    pub async fn in_memory() -> Result<Self, MemoryError> {
        Self::open(":memory:").await
    }
}

// ---------------------------------------------------------------------------
// Embedding (de)serialisation — little-endian packed f32 blob.
// ---------------------------------------------------------------------------

fn encode_embedding(emb: &Embedding) -> Vec<u8> {
    let mut out = Vec::with_capacity(emb.len() * 4);
    for f in emb {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

fn decode_embedding(bytes: &[u8]) -> Result<Embedding, MemoryError> {
    if !bytes.len().is_multiple_of(4) {
        return Err(MemoryError::Internal {
            message: format!("embedding blob length {} is not a multiple of 4", bytes.len()),
        });
    }
    let mut out = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        // chunks_exact guarantees length 4, infallible array conversion.
        let arr: [u8; 4] = chunk.try_into().map_err(|_| MemoryError::Internal {
            message: "embedding blob chunk size mismatch".into(),
        })?;
        out.push(f32::from_le_bytes(arr));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Row → MemoryRecord adapter
// ---------------------------------------------------------------------------

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRow> {
    Ok(RawRow {
        id: row.get::<_, String>(0)?,
        ns: row.get::<_, String>(1)?,
        content: row.get::<_, String>(2)?,
        embedding: row.get::<_, Option<Vec<u8>>>(3)?,
        metadata: row.get::<_, String>(4)?,
        created_at: row.get::<_, i64>(5)?,
        updated_at: row.get::<_, i64>(6)?,
    })
}

/// Plain-data carrier so we can do the JSON / embedding decoding outside the
/// row-mapper closure (where `?` against `MemoryError` would not type-check).
struct RawRow {
    id: String,
    ns: String,
    content: String,
    embedding: Option<Vec<u8>>,
    metadata: String,
    created_at: i64,
    updated_at: i64,
}

fn raw_to_record(raw: RawRow) -> Result<MemoryRecord, MemoryError> {
    let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&raw.metadata)?;
    let embedding = match raw.embedding {
        Some(bytes) => Some(decode_embedding(&bytes)?),
        None => None,
    };
    // The `id` column is the caller's storage key (e.g. "ns/uuid").  We try to
    // recover the trailing UUID for the record's `id` field; if parsing fails
    // we fall back to a fresh v4 — this only affects external `MemoryRecord`
    // consumers that read `record.id` (the storage key remains stable).
    let parsed_id = raw
        .id
        .rsplit('/')
        .next()
        .and_then(|tail| Uuid::parse_str(tail).ok())
        .unwrap_or_else(Uuid::new_v4);
    let created_at = millis_to_dt(raw.created_at);
    let updated_at = millis_to_dt(raw.updated_at);
    Ok(MemoryRecord {
        id: parsed_id,
        ns: raw.ns,
        content: raw.content,
        embedding,
        metadata,
        created_at,
        updated_at,
    })
}

fn dt_to_millis(dt: DateTime<Utc>) -> i64 {
    dt.timestamp_millis()
}

fn millis_to_dt(millis: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(millis).single().unwrap_or_else(Utc::now)
}

// ---------------------------------------------------------------------------
// MemoryBackend impl
// ---------------------------------------------------------------------------

#[async_trait]
impl MemoryBackend for SqliteBackend {
    async fn put(&mut self, key: &str, value: MemoryRecord) -> Result<(), MemoryError> {
        let conn = Arc::clone(&self.conn);
        let key = key.to_owned();
        let metadata = serde_json::to_string(&value.metadata)?;
        let embedding = value.embedding.as_ref().map(encode_embedding);
        let ns = value.ns.clone();
        let content = value.content.clone();
        let created_at = dt_to_millis(value.created_at);
        let updated_at = dt_to_millis(value.updated_at);

        task::spawn_blocking(move || -> Result<(), rusqlite::Error> {
            let guard = conn.blocking_lock();
            guard.execute(
                "INSERT INTO memory_items \
                 (id, ns, content, embedding, metadata, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(id) DO UPDATE SET \
                    ns         = excluded.ns,         \
                    content    = excluded.content,    \
                    embedding  = excluded.embedding,  \
                    metadata   = excluded.metadata,   \
                    updated_at = excluded.updated_at",
                params![key, ns, content, embedding, metadata, created_at, updated_at],
            )?;
            Ok(())
        })
        .await
        .map_err(|e| MemoryError::Internal { message: format!("join error: {e}") })?
        .map_err(|e| MemoryError::Internal { message: format!("sqlite put: {e}") })?;
        trace!("sqlite put ok");
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        let conn = Arc::clone(&self.conn);
        let key = key.to_owned();
        let raw = task::spawn_blocking(move || -> Result<Option<RawRow>, rusqlite::Error> {
            let guard = conn.blocking_lock();
            let mut stmt = guard.prepare(
                "SELECT id, ns, content, embedding, metadata, created_at, updated_at \
                 FROM memory_items WHERE id = ?1",
            )?;
            stmt.query_row(params![key], row_to_record).optional()
        })
        .await
        .map_err(|e| MemoryError::Internal { message: format!("join error: {e}") })?
        .map_err(|e| MemoryError::Internal { message: format!("sqlite get: {e}") })?;
        match raw {
            Some(r) => Ok(Some(raw_to_record(r)?)),
            None => Ok(None),
        }
    }

    async fn search(
        &self,
        query: &Embedding,
        top_k: usize,
    ) -> Result<Vec<(MemoryRecord, f32)>, MemoryError> {
        if top_k == 0 {
            return Ok(vec![]);
        }
        if query.is_empty() {
            return Err(MemoryError::InvalidArgument {
                message: "query embedding must not be empty".into(),
            });
        }

        let conn = Arc::clone(&self.conn);
        let raws = task::spawn_blocking(move || -> Result<Vec<RawRow>, rusqlite::Error> {
            let guard = conn.blocking_lock();
            let mut stmt = guard.prepare(
                "SELECT id, ns, content, embedding, metadata, created_at, updated_at \
                 FROM memory_items WHERE embedding IS NOT NULL",
            )?;
            let rows = stmt.query_map([], row_to_record)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| MemoryError::Internal { message: format!("join error: {e}") })?
        .map_err(|e| MemoryError::Internal { message: format!("sqlite search: {e}") })?;

        let query_len = query.len();
        let mut scored: Vec<(MemoryRecord, f32)> = Vec::with_capacity(raws.len());
        for raw in raws {
            let record = raw_to_record(raw)?;
            let Some(ref emb) = record.embedding else { continue };
            if emb.len() != query_len {
                continue;
            }
            let score = cosine_similarity(query, emb);
            scored.push((record, score));
        }
        scored
            .sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    async fn delete(&mut self, key: &str) -> Result<(), MemoryError> {
        let conn = Arc::clone(&self.conn);
        let key = key.to_owned();
        task::spawn_blocking(move || -> Result<(), rusqlite::Error> {
            let guard = conn.blocking_lock();
            guard.execute("DELETE FROM memory_items WHERE id = ?1", params![key])?;
            Ok(())
        })
        .await
        .map_err(|e| MemoryError::Internal { message: format!("join error: {e}") })?
        .map_err(|e| MemoryError::Internal { message: format!("sqlite delete: {e}") })?;
        Ok(())
    }

    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<MemoryRecord>, MemoryError> {
        let conn = Arc::clone(&self.conn);
        let pattern = format!("{}%", prefix.replace('%', "\\%").replace('_', "\\_"));
        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let raws = task::spawn_blocking(move || -> Result<Vec<RawRow>, rusqlite::Error> {
            let guard = conn.blocking_lock();
            let mut stmt = guard.prepare(
                "SELECT id, ns, content, embedding, metadata, created_at, updated_at \
                 FROM memory_items \
                 WHERE id LIKE ?1 ESCAPE '\\' \
                 ORDER BY created_at ASC \
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![pattern, limit_i64], row_to_record)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| MemoryError::Internal { message: format!("join error: {e}") })?
        .map_err(|e| MemoryError::Internal { message: format!("sqlite list: {e}") })?;

        let mut records = Vec::with_capacity(raws.len());
        for raw in raws {
            records.push(raw_to_record(raw)?);
        }
        Ok(records)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_backend() -> SqliteBackend {
        SqliteBackend::in_memory().await.expect("open in-memory sqlite")
    }

    #[tokio::test]
    async fn put_then_get_round_trip() {
        let mut b = make_backend().await;
        let mut rec = MemoryRecord::new("test", "hello sqlite")
            .with_embedding(vec![0.1, 0.2, 0.3, 0.4]);
        rec.metadata.insert("subject".into(), serde_json::Value::String("hi".into()));
        let key = format!("{}/{}", rec.ns, rec.id);
        b.put(&key, rec.clone()).await.expect("put");
        let fetched = b.get(&key).await.expect("get").expect("present");
        assert_eq!(fetched.content, "hello sqlite");
        assert_eq!(fetched.ns, "test");
        assert_eq!(fetched.embedding.as_deref(), Some([0.1_f32, 0.2, 0.3, 0.4].as_slice()));
        assert_eq!(fetched.metadata.get("subject").and_then(|v| v.as_str()), Some("hi"));
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let mut b = make_backend().await;
        let rec = MemoryRecord::new("del", "deleteme");
        let key = format!("{}/{}", rec.ns, rec.id);
        b.put(&key, rec).await.expect("put");
        b.delete(&key).await.expect("first delete");
        b.delete(&key).await.expect("second delete idempotent");
        assert!(b.get(&key).await.expect("get").is_none());
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let b = make_backend().await;
        assert!(b.get("nope").await.expect("get").is_none());
    }

    #[tokio::test]
    async fn search_returns_top_k_in_descending_order() {
        let mut b = make_backend().await;
        let near = MemoryRecord::new("vec", "near").with_embedding(vec![1.0, 0.0, 0.0]);
        let mid = MemoryRecord::new("vec", "mid").with_embedding(vec![0.7, 0.7, 0.0]);
        let far = MemoryRecord::new("vec", "far").with_embedding(vec![0.0, 1.0, 0.0]);
        b.put(&format!("vec/{}", near.id), near).await.unwrap();
        b.put(&format!("vec/{}", mid.id), mid).await.unwrap();
        b.put(&format!("vec/{}", far.id), far).await.unwrap();

        let q = vec![1.0_f32, 0.0, 0.0];
        let results = b.search(&q, 3).await.expect("search");
        assert_eq!(results.len(), 3);
        for w in results.windows(2) {
            assert!(w[0].1 >= w[1].1, "results not sorted: {} < {}", w[0].1, w[1].1);
        }
        assert_eq!(results[0].0.content, "near");
    }

    #[tokio::test]
    async fn search_with_k_greater_than_n_returns_all() {
        let mut b = make_backend().await;
        for i in 0..3u8 {
            let r = MemoryRecord::new("k", format!("item-{i}"))
                .with_embedding(vec![f32::from(i), 0.0]);
            b.put(&format!("k/{}", r.id), r).await.unwrap();
        }
        let r = b.search(&[1.0_f32, 0.0].to_vec(), 99).await.expect("search");
        assert_eq!(r.len(), 3, "k>n must return n results, got {}", r.len());
    }

    #[tokio::test]
    async fn empty_store_search_returns_empty() {
        let b = make_backend().await;
        let r = b.search(&[0.5_f32, 0.5].to_vec(), 5).await.expect("search");
        assert!(r.is_empty(), "empty store must return no results, got {}", r.len());
    }

    #[tokio::test]
    async fn search_skips_records_without_embedding() {
        let mut b = make_backend().await;
        let no_emb = MemoryRecord::new("vec", "no embedding");
        let with_emb = MemoryRecord::new("vec", "has embedding").with_embedding(vec![1.0, 0.0]);
        b.put(&format!("vec/{}", no_emb.id), no_emb).await.unwrap();
        b.put(&format!("vec/{}", with_emb.id), with_emb).await.unwrap();
        let r = b.search(&[1.0_f32, 0.0].to_vec(), 5).await.expect("search");
        assert_eq!(r.len(), 1, "only embedded records should appear, got {}", r.len());
        assert_eq!(r[0].0.content, "has embedding");
    }

    #[tokio::test]
    async fn list_with_prefix_filters() {
        let mut b = make_backend().await;
        let r1 = MemoryRecord::new("facts", "alpha");
        let r2 = MemoryRecord::new("facts", "beta");
        let r3 = MemoryRecord::new("sessions", "gamma");
        b.put(&format!("facts/{}", r1.id), r1).await.unwrap();
        b.put(&format!("facts/{}", r2.id), r2).await.unwrap();
        b.put(&format!("sessions/{}", r3.id), r3).await.unwrap();
        let facts = b.list("facts/", 10).await.expect("list");
        assert_eq!(facts.len(), 2, "expected 2 facts, got {}", facts.len());
        let all = b.list("", 10).await.expect("list-all");
        assert_eq!(all.len(), 3, "expected 3 total, got {}", all.len());
    }

    #[tokio::test]
    async fn persist_across_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mem.sqlite");
        let key = {
            let mut b = SqliteBackend::open(&path).await.expect("open");
            let rec = MemoryRecord::new("p", "durable");
            let k = format!("p/{}", rec.id);
            b.put(&k, rec).await.expect("put");
            k
        };
        let b2 = SqliteBackend::open(&path).await.expect("reopen");
        let fetched = b2.get(&key).await.expect("get").expect("survived reopen");
        assert_eq!(fetched.content, "durable");
    }
}
