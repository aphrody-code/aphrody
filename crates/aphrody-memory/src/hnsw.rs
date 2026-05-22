// SPDX-License-Identifier: Apache-2.0
//! # `HnswBackend`
//!
//! An in-process approximate nearest-neighbour memory backend.
//!
//! ## Implementation note — brute-force fallback
//!
//! The intended implementation uses the [`instant-distance`] crate
//! (HNSW, pure Rust, no FFI, v0.7+).  However, `instant-distance` is not
//! present in the offline Cargo registry cache for this workspace.  Until the
//! registry is reachable and the crate is added to `[workspace.dependencies]`,
//! this module implements **exact cosine-similarity search** over a
//! `BTreeMap<String, MemoryRecord>` index.  Exact search is O(n) per query
//! but is correct, safe, and requires zero external crates.
//!
//! ### Migration path to HNSW
//!
//! 1. Add to `Cargo.toml` (workspace dep + crate dep): ```toml instant-distance = { version =
//!    "0.7", features = ["serde"] } ```
//! 2. Replace the `index` field type with `instant_distance::HnswMap<EmbeddingPoint, String>`.
//! 3. Implement [`instant_distance::Point`] for a newtype around `Vec<f32>`.
//! 4. Replace `brute_force_search` with `hnsw.search(…)`.
//! 5. Replace JSON persistence with `bincode` + `serde` on the HNSW map.
//!
//! The public `HnswBackend` API is **identical** before and after this swap.
//!
//! ## Persistence
//!
//! The in-memory index is serialised to `<root>/index.bin` as newline-delimited
//! JSON (same format as the JSONL backend) on every `put` and `delete`.  On
//! construction the index is reloaded from that file if it exists.
//!
//! ## Thread safety
//!
//! `HnswBackend` is `Send + Sync`.  Callers must wrap it in a `Mutex` or
//! `RwLock` when shared across tasks.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use serde_json::Deserializer;
use tokio::fs;
use tracing::{debug, trace};

use crate::{Embedding, MemoryBackend, MemoryError, MemoryRecord};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Filename inside `root` where the full index is persisted.
const INDEX_FILE: &str = "index.bin";

// ---------------------------------------------------------------------------
// Backend struct
// ---------------------------------------------------------------------------

/// In-process nearest-neighbour memory backend.
///
/// Uses brute-force cosine similarity (O(n)) until `instant-distance` is
/// integrated.  See the module documentation for the HNSW migration path.
pub struct HnswBackend {
    root: PathBuf,
    /// In-memory index: `key -> record`.
    index: BTreeMap<String, MemoryRecord>,
}

impl HnswBackend {
    /// Open (or create) an `HnswBackend` rooted at `root`.
    ///
    /// If `<root>/index.bin` exists its contents are loaded into the
    /// in-memory index.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Io`] if the directory cannot be created or read,
    /// and [`MemoryError::Serde`] if the index file is malformed.
    pub async fn open(root: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let root = root.as_ref().to_owned();
        fs::create_dir_all(&root).await?;

        let index_path = root.join(INDEX_FILE);
        let index: BTreeMap<String, MemoryRecord> = if index_path.exists() {
            debug!(path = %index_path.display(), "loading hnsw index");
            let bytes = fs::read(&index_path).await?;
            let mut map = BTreeMap::new();
            for result in Deserializer::from_slice(&bytes).into_iter::<(String, MemoryRecord)>() {
                let (key, record) = result?;
                trace!(key, "loaded hnsw record");
                map.insert(key, record);
            }
            map
        } else {
            BTreeMap::new()
        };

        Ok(Self { root, index })
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Flush the entire index to `<root>/index.bin`.
    ///
    /// Each entry is written as a JSON-encoded `(key, MemoryRecord)` tuple on
    /// its own line (newline-delimited JSON).
    async fn flush(&self) -> Result<(), MemoryError> {
        let index_path = self.root.join(INDEX_FILE);
        let tmp_path = self.root.join(format!(".{INDEX_FILE}.tmp"));

        let mut lines = Vec::with_capacity(self.index.len());
        for (key, record) in &self.index {
            let pair = (key.as_str(), record);
            lines.push(serde_json::to_string(&pair)?);
        }

        let content =
            if lines.is_empty() { String::new() } else { format!("{}\n", lines.join("\n")) };

        fs::write(&tmp_path, content.as_bytes()).await?;
        fs::rename(&tmp_path, &index_path).await?;
        debug!(
            path = %index_path.display(),
            records = self.index.len(),
            "flushed hnsw index"
        );
        Ok(())
    }

    /// Insert `batch` records into the index and flush **once** at the end.
    ///
    /// Equivalent to calling [`MemoryBackend::put`] in a loop, but pays a
    /// single disk flush rather than `batch.len()`. Useful for bulk imports
    /// (migration tool R3.4) and benchmarks (R3.7 recall bench) where the
    /// per-record fsync cost would dominate.
    ///
    /// # Errors
    ///
    /// Surfaces a single [`MemoryError::Io`] / [`MemoryError::Serde`] from
    /// the final flush; partial inserts before the failure remain in the
    /// in-memory index but are not persisted.
    pub async fn put_many(
        &mut self,
        batch: Vec<(String, MemoryRecord)>,
    ) -> Result<(), MemoryError> {
        for (key, value) in batch {
            self.index.insert(key, value);
        }
        self.flush().await
    }

    /// Brute-force nearest-neighbour scan over embedded records.
    ///
    /// Exact (not approximate) for any `n`. Time complexity is `O(n · d)` for
    /// scoring plus `O(n)` quickselect for the top-`k` cut — never the
    /// `O(n · log n)` of a full sort.
    ///
    /// Three constant-factor wins over the naive "score-clone-sort-all" form
    /// keep the 100k-record p95 under the R3.7 budget without an ANN index:
    ///
    /// 1. The **query norm is hoisted out of the loop** (`cosine_similarity`
    ///    recomputes it per record; here it is computed once and the per-record
    ///    work is one fused dot-and-norm pass — see [`Self::scaled_cosine`]).
    /// 2. **Scoring borrows** records (`&MemoryRecord`) rather than cloning all
    ///    `n`; only the `k` survivors are cloned at the very end.
    /// 3. **`select_nth_unstable_by`** (quickselect) partitions the `k` best in
    ///    linear time; only that `k`-slice is then sorted for stable ordering.
    fn brute_force_search(&self, query: &Embedding, top_k: usize) -> Vec<(MemoryRecord, f32)> {
        // Hoist the query norm: identical for every record, so paying it once
        // turns the per-record cost from two norms + a dot into one norm + a
        // dot. A zero-magnitude query yields all-zero cosines (matching
        // `cosine_similarity`), so there is nothing to rank.
        let query_norm = l2_norm(query);
        if query_norm == 0.0 {
            return Vec::new();
        }

        let mut scored: Vec<(f32, &MemoryRecord)> = Vec::with_capacity(self.index.len());
        for r in self.index.values() {
            let Some(emb) = r.embedding.as_deref() else { continue };
            if emb.len() != query.len() {
                continue;
            }
            scored.push((Self::scaled_cosine(query, query_norm, emb), r));
        }

        // Descending order: best (highest similarity) first. NaN scores are
        // treated as equal, matching the previous `partial_cmp` fallback.
        let desc =
            |a: &(f32, &MemoryRecord), b: &(f32, &MemoryRecord)| {
                b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
            };

        let k = top_k.min(scored.len());
        if k == 0 {
            return Vec::new();
        }
        // Quickselect partitions so the k highest-scoring entries occupy
        // [0, k) (unordered among themselves) in O(n); only that prefix is
        // then sorted, so the full O(n log n) sort is avoided for large n.
        if k < scored.len() {
            scored.select_nth_unstable_by(k - 1, desc);
        }
        let top = &mut scored[..k];
        top.sort_unstable_by(desc);
        top.iter().map(|(score, r)| ((*r).clone(), *score)).collect()
    }

    /// Cosine similarity with the query's L2 norm supplied by the caller.
    ///
    /// `query_norm` must equal `l2_norm(query)` and be non-zero. Folds the dot
    /// product and the candidate norm into a single pass over `embedding`.
    #[inline]
    fn scaled_cosine(query: &[f32], query_norm: f32, embedding: &[f32]) -> f32 {
        let mut dot = 0.0_f32;
        let mut norm_sq = 0.0_f32;
        for (q, e) in query.iter().zip(embedding.iter()) {
            dot += q * e;
            norm_sq += e * e;
        }
        if norm_sq == 0.0 { 0.0 } else { dot / (query_norm * norm_sq.sqrt()) }
    }
}

/// L2 (Euclidean) norm of a vector.
#[inline]
fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

// ---------------------------------------------------------------------------
// MemoryBackend impl
// ---------------------------------------------------------------------------

#[async_trait]
impl MemoryBackend for HnswBackend {
    async fn put(&mut self, key: &str, value: MemoryRecord) -> Result<(), MemoryError> {
        self.index.insert(key.to_owned(), value);
        self.flush().await
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
        Ok(self.brute_force_search(query, top_k))
    }

    async fn delete(&mut self, key: &str) -> Result<(), MemoryError> {
        self.index.remove(key);
        self.flush().await
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

    async fn make_backend() -> (HnswBackend, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = HnswBackend::open(dir.path()).await.expect("open");
        (backend, dir)
    }

    #[tokio::test]
    async fn put_then_get_round_trip() {
        let (mut b, _dir) = make_backend().await;
        let rec = MemoryRecord::new("hnsw-test", "hello from hnsw");
        let key = format!("{}/{}", rec.ns, rec.id);
        b.put(&key, rec.clone()).await.expect("put");
        let fetched = b.get(&key).await.expect("get").expect("record present");
        assert_eq!(fetched.id, rec.id);
        assert_eq!(fetched.content, "hello from hnsw");
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let (mut b, _dir) = make_backend().await;
        let rec = MemoryRecord::new("del", "delete me");
        let key = format!("{}/{}", rec.ns, rec.id);
        b.put(&key, rec).await.unwrap();
        b.delete(&key).await.unwrap();
        b.delete(&key).await.unwrap();
        assert!(b.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn search_top_k_ordering() {
        let (mut b, _dir) = make_backend().await;

        let near = MemoryRecord::new("vec", "near").with_embedding(vec![1.0, 0.0, 0.0]);
        let mid = MemoryRecord::new("vec", "mid").with_embedding(vec![0.7, 0.7, 0.0]);
        let far = MemoryRecord::new("vec", "far").with_embedding(vec![0.0, 1.0, 0.0]);

        b.put(&format!("vec/{}", near.id), near).await.unwrap();
        b.put(&format!("vec/{}", mid.id), mid).await.unwrap();
        b.put(&format!("vec/{}", far.id), far).await.unwrap();

        let query = vec![1.0_f32, 0.0, 0.0];
        let results = b.search(&query, 3).await.unwrap();

        assert_eq!(results.len(), 3);
        for w in results.windows(2) {
            assert!(w[0].1 >= w[1].1, "results not sorted: {} < {}", w[0].1, w[1].1);
        }
        assert_eq!(results[0].0.content, "near");
    }

    #[tokio::test]
    async fn search_top_k_truncates() {
        let (mut b, _dir) = make_backend().await;
        for i in 0..5u8 {
            let r = MemoryRecord::new("vec", format!("r{i}")).with_embedding(vec![i as f32, 0.0]);
            b.put(&format!("vec/{}", r.id), r).await.unwrap();
        }
        let results = b.search(&[1.0_f32, 0.0].to_vec(), 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn search_skips_unembedded_records() {
        let (mut b, _dir) = make_backend().await;
        let bare = MemoryRecord::new("vec", "no embedding");
        let emb = MemoryRecord::new("vec", "has embedding").with_embedding(vec![1.0, 0.0]);
        b.put(&format!("vec/{}", bare.id), bare).await.unwrap();
        b.put(&format!("vec/{}", emb.id), emb).await.unwrap();
        let results = b.search(&[1.0_f32, 0.0].to_vec(), 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn list_prefix_and_limit() {
        let (mut b, _dir) = make_backend().await;
        for i in 0..4u8 {
            let r = MemoryRecord::new("ns-a", format!("item {i}"));
            b.put(&format!("ns-a/{}", r.id), r).await.unwrap();
        }
        let r_b = MemoryRecord::new("ns-b", "other");
        b.put(&format!("ns-b/{}", r_b.id), r_b).await.unwrap();

        let a_all = b.list("ns-a/", 100).await.unwrap();
        assert_eq!(a_all.len(), 4);
        let limited = b.list("ns-a/", 2).await.unwrap();
        assert_eq!(limited.len(), 2);
        let b_only = b.list("ns-b/", 100).await.unwrap();
        assert_eq!(b_only.len(), 1);
    }

    #[tokio::test]
    async fn persist_across_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key = {
            let mut b = HnswBackend::open(dir.path()).await.unwrap();
            let rec =
                MemoryRecord::new("persist", "durable content").with_embedding(vec![0.5, 0.5]);
            let k = format!("{}/{}", rec.ns, rec.id);
            b.put(&k, rec).await.unwrap();
            k
        };
        let b2 = HnswBackend::open(dir.path()).await.unwrap();
        let fetched = b2.get(&key).await.unwrap().expect("must survive reopen");
        assert_eq!(fetched.content, "durable content");
        assert!(fetched.embedding.is_some());
    }

    #[test]
    fn scaled_cosine_matches_reference() {
        // The optimised per-record scorer must agree with the canonical
        // `cosine_similarity` so the quickselect path returns identical ranks.
        let cases: &[(&[f32], &[f32])] = &[
            (&[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]),
            (&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]),
            (&[0.3, 0.7, 0.1], &[0.9, 0.2, 0.4]),
            (&[2.0, 2.0, 2.0], &[1.0, 1.0, 1.0]),
        ];
        for (q, e) in cases {
            let reference = crate::cosine_similarity(q, e);
            let scaled = HnswBackend::scaled_cosine(q, l2_norm(q), e);
            assert!(
                (reference - scaled).abs() < 1e-6,
                "scaled_cosine diverged: ref={reference} scaled={scaled} for {q:?} vs {e:?}",
            );
        }
    }

    #[tokio::test]
    async fn zero_query_returns_empty() {
        // A zero-magnitude query has undefined direction; cosine is 0 for every
        // record, so there is nothing meaningful to rank.
        let (mut b, _dir) = make_backend().await;
        let r = MemoryRecord::new("vec", "x").with_embedding(vec![1.0, 2.0, 3.0]);
        b.put(&format!("vec/{}", r.id), r).await.unwrap();
        let results = b.search(&vec![0.0_f32, 0.0, 0.0], 5).await.unwrap();
        assert!(results.is_empty(), "zero query must rank nothing, got {results:?}");
    }

    #[tokio::test]
    async fn empty_query_returns_error() {
        let (b, _dir) = make_backend().await;
        let err = b.search(&vec![], 5).await.unwrap_err();
        assert!(
            matches!(err, MemoryError::InvalidArgument { .. }),
            "expected InvalidArgument, got {err:?}"
        );
    }
}
