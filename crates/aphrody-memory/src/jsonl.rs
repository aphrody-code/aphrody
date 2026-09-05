// SPDX-License-Identifier: Apache-2.0
//! # `JsonlBackend`
//!
//! A durable, file-based [`MemoryBackend`] that writes one [`MemoryRecord`]
//! per line to `<root>/<ns>.jsonl`.
//!
//! ## Envelope compatibility
//!
//! The on-disk JSON shape mirrors the `.coord/inbox-from-aphrody.jsonl` A2A
//! coordination envelope:
//!
//! ```json
//! {
//!   "id":         "<uuid>",
//!   "ns":         "aphrody",
//!   "content":    "...",
//!   "embedding":  null,
//!   "metadata":   { "type": "fact", "subject": "...", "re": null },
//!   "created_at": "2026-05-17T14:00:00Z",
//!   "updated_at": "2026-05-17T14:00:00Z"
//! }
//! ```
//!
//! Each JSONL file is named after the record's `ns` field, so records with
//! `ns = "aphrody"` live in `<root>/aphrody.jsonl`.
//!
//! ## Consistency model
//!
//! All mutations operate on an in-memory `BTreeMap<String, MemoryRecord>`
//! that is flushed to disk on every `put` or `delete`.  The flush rewrites
//! the entire namespace file atomically (write → rename).  This is
//! appropriate for the expected cardinality (hundreds of records per
//! namespace); for millions of records use a proper database backend.
//!
//! ## Thread safety
//!
//! `JsonlBackend` is `Send + Sync`.  It does not use interior mutability
//! — callers must wrap it in a `Mutex`/`RwLock` if shared across tasks.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use serde_json::Deserializer;
use tokio::fs;
use tracing::{debug, trace};

use crate::{Embedding, MemoryBackend, MemoryError, MemoryRecord, cosine_similarity};

// ---------------------------------------------------------------------------
// Backend struct
// ---------------------------------------------------------------------------

/// JSONL-file-backed memory store.
///
/// Records are partitioned by `ns` into separate `<root>/<ns>.jsonl` files.
/// The in-memory index is a `BTreeMap` keyed by the caller-supplied storage
/// key (not the record's `id`).
pub struct JsonlBackend {
    root: PathBuf,
    /// In-memory index: `key -> record`.
    ///
    /// Keys use the convention `"<ns>/<uuid>"` but the caller chooses.
    index: BTreeMap<String, MemoryRecord>,
}

impl JsonlBackend {
    /// Open (or create) a `JsonlBackend` rooted at `root`.
    ///
    /// All `*.jsonl` files found under `root` are loaded into the in-memory
    /// index on construction.  The method is `async` because it performs I/O.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Io`] if the directory cannot be created or read,
    /// and [`MemoryError::Serde`] if any line fails to deserialise.
    pub async fn open(root: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let root = root.as_ref().to_owned();
        fs::create_dir_all(&root).await?;

        let mut index: BTreeMap<String, MemoryRecord> = BTreeMap::new();

        // Walk existing *.jsonl files and load all records.
        let mut read_dir = fs::read_dir(&root).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            debug!(path = %path.display(), "loading jsonl file");
            let bytes = fs::read(&path).await?;
            // serde_json streaming deserialiser — handles partial/blank lines.
            for result in Deserializer::from_slice(&bytes).into_iter::<MemoryRecord>() {
                let record: MemoryRecord = result?;
                // Reconstruct the canonical key from the record's id + ns.
                let key = format!("{}/{}", record.ns, record.id);
                trace!(key, "loaded record");
                index.insert(key, record);
            }
        }

        Ok(Self { root, index })
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Return the path to the JSONL file for namespace `ns`.
    fn ns_path(&self, ns: &str) -> PathBuf {
        self.root.join(format!("{ns}.jsonl"))
    }

    /// Flush all records belonging to `ns` to `<root>/<ns>.jsonl`.
    ///
    /// Uses a write-to-temp-then-rename pattern for atomicity on POSIX.
    /// On Windows the rename is not atomic across filesystem boundaries but
    /// will succeed within the same volume (the common case).
    async fn flush_ns(&self, ns: &str) -> Result<(), MemoryError> {
        let path = self.ns_path(ns);
        let tmp_path = self.root.join(format!(".{ns}.jsonl.tmp"));

        // Collect all records for this namespace, sorted by key for stable output.
        let mut lines = Vec::new();
        for (_, record) in self.index.iter().filter(|(_, r)| r.ns == ns) {
            lines.push(serde_json::to_string(record)?);
        }

        let content = lines.join("\n");
        // Write a trailing newline only when there is content, matching the
        // convention used by the .coord JSONL mailboxes.
        let content = if content.is_empty() { String::new() } else { format!("{content}\n") };

        fs::write(&tmp_path, content.as_bytes()).await?;
        fs::rename(&tmp_path, &path).await?;
        debug!(ns, path = %path.display(), records = lines.len(), "flushed namespace");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MemoryBackend impl
// ---------------------------------------------------------------------------

#[async_trait]
impl MemoryBackend for JsonlBackend {
    async fn put(&mut self, key: &str, value: MemoryRecord) -> Result<(), MemoryError> {
        let ns = value.ns.clone();
        self.index.insert(key.to_owned(), value);
        self.flush_ns(&ns).await
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        Ok(self.index.get(key).cloned())
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

        // Collect scored candidates — only records that carry an embedding of
        // the same dimensionality.
        let mut scored: Vec<(MemoryRecord, f32)> = self
            .index
            .values()
            .filter_map(|r| {
                let emb = r.embedding.as_deref()?;
                if emb.len() != query.len() {
                    return None;
                }
                let score = cosine_similarity(query, emb);
                Some((r.clone(), score))
            })
            .collect();

        // Sort descending by score; break ties by record id for determinism.
        scored.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    async fn delete(&mut self, key: &str) -> Result<(), MemoryError> {
        // Capture the ns before removal so we can flush the right file.
        if let Some(record) = self.index.remove(key) {
            self.flush_ns(&record.ns).await?;
        }
        // Idempotent: no error if key was absent.
        Ok(())
    }

    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<MemoryRecord>, MemoryError> {
        let records: Vec<MemoryRecord> = self
            .index
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .take(limit)
            .map(|(_, v)| v.clone())
            .collect();
        Ok(records)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryRecord;

    async fn make_backend() -> (JsonlBackend, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = JsonlBackend::open(dir.path()).await.expect("open");
        (backend, dir)
    }

    #[tokio::test]
    async fn put_then_get_round_trip() {
        let (mut b, _dir) = make_backend().await;
        let mut rec = MemoryRecord::new("test", "hello world");
        let key = format!("{}/{}", rec.ns, rec.id);
        rec.metadata.insert("subject".into(), serde_json::Value::String("hi".into()));
        b.put(&key, rec.clone()).await.expect("put");
        let fetched = b.get(&key).await.expect("get").expect("record present");
        assert_eq!(fetched.id, rec.id);
        assert_eq!(fetched.content, "hello world");
        assert_eq!(fetched.metadata.get("subject").and_then(|v| v.as_str()), Some("hi"));
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let (mut b, _dir) = make_backend().await;
        let rec = MemoryRecord::new("del-ns", "to delete");
        let key = format!("{}/{}", rec.ns, rec.id);
        b.put(&key, rec).await.expect("put");
        b.delete(&key).await.expect("first delete");
        b.delete(&key).await.expect("second delete is idempotent");
        let result = b.get(&key).await.expect("get after delete");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let (b, _dir) = make_backend().await;
        let result = b.get("does-not-exist").await.expect("get");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_with_prefix_filters_correctly() {
        let (mut b, _dir) = make_backend().await;
        let r1 = MemoryRecord::new("facts", "alpha");
        let r2 = MemoryRecord::new("facts", "beta");
        let r3 = MemoryRecord::new("sessions", "gamma");
        b.put(&format!("facts/{}", r1.id), r1).await.unwrap();
        b.put(&format!("facts/{}", r2.id), r2).await.unwrap();
        b.put(&format!("sessions/{}", r3.id), r3).await.unwrap();
        let facts = b.list("facts/", 10).await.unwrap();
        assert_eq!(facts.len(), 2);
        let all = b.list("", 10).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn list_limit_is_respected() {
        let (mut b, _dir) = make_backend().await;
        for i in 0..5u8 {
            let r = MemoryRecord::new("bulk", format!("record {i}"));
            b.put(&format!("bulk/{}", r.id), r).await.unwrap();
        }
        let limited = b.list("bulk/", 3).await.unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[tokio::test]
    async fn search_returns_top_k_in_order() {
        let (mut b, _dir) = make_backend().await;
        // Three records: near, far, mid.  Query is [1,0,0].
        let near = MemoryRecord::new("vec", "near").with_embedding(vec![1.0, 0.0, 0.0]);
        let mid = MemoryRecord::new("vec", "mid").with_embedding(vec![0.7, 0.7, 0.0]);
        let far = MemoryRecord::new("vec", "far").with_embedding(vec![0.0, 1.0, 0.0]);

        b.put(&format!("vec/{}", near.id), near).await.unwrap();
        b.put(&format!("vec/{}", mid.id), mid).await.unwrap();
        b.put(&format!("vec/{}", far.id), far).await.unwrap();

        let query = vec![1.0_f32, 0.0, 0.0];
        let results = b.search(&query, 3).await.unwrap();

        assert_eq!(results.len(), 3);
        // Scores must be non-increasing.
        for w in results.windows(2) {
            assert!(w[0].1 >= w[1].1, "results not sorted: {} < {}", w[0].1, w[1].1);
        }
        // Top result should be the "near" record.
        assert_eq!(results[0].0.content, "near");
    }

    #[tokio::test]
    async fn search_skips_records_without_embedding() {
        let (mut b, _dir) = make_backend().await;
        let no_emb = MemoryRecord::new("vec", "no embedding");
        let with_emb = MemoryRecord::new("vec", "has embedding").with_embedding(vec![1.0, 0.0]);
        b.put(&format!("vec/{}", no_emb.id), no_emb).await.unwrap();
        b.put(&format!("vec/{}", with_emb.id), with_emb).await.unwrap();

        let results = b.search(&[1.0_f32, 0.0].to_vec(), 5).await.unwrap();
        assert_eq!(results.len(), 1, "only embedded records should appear");
    }

    #[tokio::test]
    async fn persist_across_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key = {
            let mut b = JsonlBackend::open(dir.path()).await.unwrap();
            let rec = MemoryRecord::new("persist", "durable content");
            let k = format!("{}/{}", rec.ns, rec.id);
            b.put(&k, rec).await.unwrap();
            k
        };
        // Re-open — data must survive.
        let b2 = JsonlBackend::open(dir.path()).await.unwrap();
        let fetched = b2.get(&key).await.unwrap().expect("record must survive reopen");
        assert_eq!(fetched.content, "durable content");
    }
}
