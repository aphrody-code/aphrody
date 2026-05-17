// SPDX-License-Identifier: Apache-2.0
//! Session memory pane — semantic recall for past terminal commands and
//! sub-agent transcripts.
//!
//! Matrix row `T-INT-memory` integration point. The pane delegates **all**
//! storage and similarity search to the [`aphrody_memory`] crate:
//!
//! - [`aphrody_memory::jsonl::JsonlBackend`] — durable append-on-rewrite store,
//!   one `<root>/<ns>.jsonl` file per namespace (the same envelope shape used
//!   by the `.coord/inbox-from-aphrody.jsonl` A2A mailboxes).
//! - [`aphrody_memory::MemoryBackend::search`] — exact cosine similarity over
//!   embeddings (brute-force fallback for the HNSW backend per the crate's
//!   migration note).
//!
//! ### Embedding strategy
//!
//! Terminal commands are short (typically 5–200 characters) and rarely benefit
//! from a learned embedding model — most useful recall is **character-level
//! lexical similarity** (`git log` matches `git log -p`, `cargo nextest run`
//! matches `cargo nextest list`, etc.). This pane uses a deterministic
//! character-bigram hashing embedder ("hashing trick", a well-established IR
//! baseline) so the pane works fully offline, has zero ML deps, and produces
//! real cosine scores on real f32 vectors that exercise the backend's
//! search code path end-to-end.
//!
//! The embedder is deterministic — the same string always hashes to the same
//! vector — so persisted records remain query-compatible across restarts.
//!
//! ### Publish surface
//!
//! After every successful `recall`, the pane can [`publish_to`] an
//! [`EventBus`] which broadcasts an [`LlmEvent::SessionRecall`] envelope for
//! any subscribed renderer (Ctrl+R fuzzy finder, history pane, command
//! palette).
//!
//! [`publish_to`]: SessionMemoryPane::publish_to

use std::{
    path::Path,
    sync::Arc,
};

use anyhow::{Context, Result};
use tokio::sync::Mutex;

use aphrody_memory::{Embedding, MemoryBackend, MemoryRecord, jsonl::JsonlBackend};

use crate::{EventBus, LlmEvent};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Namespace used for terminal command history records inside the backing
/// store. Matches the `.coord` envelope `ns` convention so the JSONL file can
/// be consumed by other tools that expect the standard envelope.
pub const SESSIONS_NS: &str = "sessions";

/// Dimensionality of the character-bigram hashing embedder. 256 is a sweet
/// spot for short command strings: collisions remain rare (birthday bound
/// ~16 distinct bigrams before 50% collision likelihood) while keeping
/// per-record memory at 1 KiB (256 × f32).
const EMBED_DIM: usize = 256;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single session-memory match returned by [`SessionMemoryPane::recall`].
///
/// The struct is a thin view over an [`aphrody_memory::MemoryRecord`] —
/// callers receive the raw command text plus the cosine-similarity score
/// without having to know about the backend's internal envelope schema.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SessionMemoryEntry {
    /// Verbatim command (or sub-agent transcript) recorded earlier.
    pub command: String,
    /// Cosine similarity in `[-1.0, 1.0]`. Higher = better match.
    pub score: f32,
}

/// Terminal session-memory pane.
///
/// Owns a [`JsonlBackend`] behind a [`tokio::sync::Mutex`] so the pane can be
/// shared across tasks (the renderer loop, the OSC parser, the command
/// palette) without race conditions on the in-memory index. All public
/// methods are async to keep the contract identical to the underlying
/// [`MemoryBackend`] trait.
#[derive(Clone)]
pub struct SessionMemoryPane {
    memory: Arc<Mutex<JsonlBackend>>,
}

impl SessionMemoryPane {
    /// Open or create the session-memory store rooted at `root`.
    ///
    /// On disk this materialises `<root>/sessions.jsonl`. Reopening the same
    /// root rehydrates all previously-recorded entries; recall works
    /// immediately, no warm-up phase.
    ///
    /// # Errors
    ///
    /// Propagates any I/O or deserialisation error from [`JsonlBackend::open`].
    pub async fn open(root: impl AsRef<Path>) -> Result<Self> {
        let backend = JsonlBackend::open(root)
            .await
            .context("open JsonlBackend for session-memory pane")?;
        Ok(Self {
            memory: Arc::new(Mutex::new(backend)),
        })
    }

    /// Record `entry` (a terminal command or sub-agent transcript line) into
    /// the session-memory store, attaching a character-bigram embedding so it
    /// becomes recall-able by [`recall`][Self::recall].
    ///
    /// # Errors
    ///
    /// Propagates the backend's `put` error (I/O / serialisation failure).
    pub async fn record(&self, entry: &str) -> Result<()> {
        let embedding = embed(entry);
        let record = MemoryRecord::new(SESSIONS_NS, entry).with_embedding(embedding);
        let key = format!("{}/{}", record.ns, record.id);

        let mut guard = self.memory.lock().await;
        guard
            .put(&key, record)
            .await
            .context("put session-memory record into JsonlBackend")
    }

    /// Recall the `limit` most-similar past entries for `query`, ordered by
    /// descending cosine score.
    ///
    /// Delegates to [`MemoryBackend::search`] on the underlying
    /// [`JsonlBackend`], which iterates every embedded record and computes
    /// the cosine similarity against the query vector. Records without an
    /// embedding are silently skipped (matches the backend contract).
    ///
    /// `limit == 0` returns an empty vec without touching the backend.
    ///
    /// # Errors
    ///
    /// Propagates the backend's `search` error (e.g.
    /// [`aphrody_memory::MemoryError::InvalidArgument`] for empty queries).
    pub async fn recall(&self, query: &str, limit: usize) -> Result<Vec<SessionMemoryEntry>> {
        if limit == 0 || query.is_empty() {
            return Ok(Vec::new());
        }
        let query_vec = embed(query);

        let guard = self.memory.lock().await;
        let scored = guard
            .search(&query_vec, limit)
            .await
            .context("search JsonlBackend for session-memory recall")?;

        Ok(scored
            .into_iter()
            .map(|(rec, score)| SessionMemoryEntry {
                command: rec.content,
                score,
            })
            .collect())
    }

    /// Recall `query` and publish the matches as an
    /// [`LlmEvent::SessionRecall`] envelope on `bus`.
    ///
    /// Returns the number of subscribers that received the event (mirrors the
    /// [`EventBus::publish`] contract used by [`crate::DocsPreviewPane`]). A
    /// recall with zero matches still publishes an event with an empty
    /// `entries` vec so renderers can clear stale results.
    ///
    /// # Errors
    ///
    /// Propagates either the recall failure or the publish failure (no
    /// subscribers, channel closed, etc.).
    pub async fn publish_to(
        &self,
        bus: &EventBus,
        query: &str,
        limit: usize,
    ) -> Result<usize> {
        let matches = self.recall(query, limit).await?;
        let entries: Vec<String> = matches.into_iter().map(|m| m.command).collect();
        let count = bus
            .publish(LlmEvent::SessionRecall {
                query: query.to_owned(),
                entries,
            })
            .map_err(|e| anyhow::anyhow!("event bus has no subscribers: {e}"))?;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// Embedder — deterministic character-bigram hashing
// ---------------------------------------------------------------------------

/// Hash `text` into a fixed-dimension f32 vector using the character-bigram
/// hashing trick.
///
/// For each consecutive `(c_i, c_{i+1})` pair in the lowercased input we
/// compute a 64-bit hash, fold it into `[0, EMBED_DIM)` and increment that
/// bucket by 1.0. The unigrams (each individual character) are also folded in
/// with weight 0.5 so single-token queries (`"ls"`, `"git"`) still produce
/// signal. The resulting vector is L2-normalised so cosine similarity reduces
/// to a plain dot product on the backend side.
///
/// This is a well-known IR baseline ("hashing trick" / "feature hashing",
/// Weinberger et al. 2009). It is **not** a learned embedding — it is a
/// content-addressable fingerprint that is:
///
/// - Deterministic (same input → same vector, even across restarts).
/// - Dependency-free (uses only `std::hash`).
/// - Fast (O(n) in input length, no allocation beyond the output vec).
/// - Effective for short shell commands where lexical overlap dominates.
fn embed(text: &str) -> Embedding {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut v = vec![0.0_f32; EMBED_DIM];
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();

    // Unigram features (weight 0.5) — guarantee non-empty signal for inputs
    // shorter than 2 chars.
    for ch in &chars {
        let mut h = DefaultHasher::new();
        ('u', *ch).hash(&mut h);
        let bucket = (h.finish() as usize) % EMBED_DIM;
        v[bucket] += 0.5;
    }

    // Bigram features (weight 1.0) — dominant signal.
    for w in chars.windows(2) {
        let mut h = DefaultHasher::new();
        ('b', w[0], w[1]).hash(&mut h);
        let bucket = (h.finish() as usize) % EMBED_DIM;
        v[bucket] += 1.0;
    }

    // L2 normalise so the backend's cosine similarity reduces to a dot
    // product. Guard against the all-zero edge case (empty input).
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_memory_pane_recall_returns_match() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pane = SessionMemoryPane::open(dir.path()).await.expect("open pane");

        pane.record("cargo nextest run --workspace").await.expect("record 1");
        pane.record("git log --oneline -n 20").await.expect("record 2");
        pane.record("cargo clippy --workspace -- -D warnings").await.expect("record 3");

        let hits = pane.recall("cargo nextest", 3).await.expect("recall");

        assert!(!hits.is_empty(), "recall must return at least one match");
        // Best match must be the cargo-nextest record (substring overlap).
        assert!(
            hits[0].command.contains("nextest"),
            "expected nextest record at rank 1, got: {:?}",
            hits[0].command
        );
        // Scores must be non-increasing.
        for w in hits.windows(2) {
            assert!(
                w[0].score >= w[1].score,
                "recall must return scores in descending order: {} < {}",
                w[0].score,
                w[1].score
            );
        }
    }

    #[tokio::test]
    async fn session_memory_pane_publishes_event() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pane = SessionMemoryPane::open(dir.path()).await.expect("open pane");
        pane.record("ls -lah /tmp").await.expect("record");
        pane.record("ls /var/log").await.expect("record");

        let bus = EventBus::new(8);
        let mut rx = bus.subscribe();

        let receivers = pane
            .publish_to(&bus, "ls /tmp", 5)
            .await
            .expect("publish recall to event bus");
        assert!(receivers >= 1, "expected at least one subscriber, got {receivers}");

        let ev = rx.recv().await.expect("recv recall event");
        match ev {
            LlmEvent::SessionRecall { query, entries } => {
                assert_eq!(query, "ls /tmp");
                assert!(!entries.is_empty(), "entries must not be empty");
                assert!(
                    entries.iter().any(|e| e.starts_with("ls ")),
                    "at least one entry must be an `ls` command, got: {entries:?}"
                );
            },
            other => panic!("expected LlmEvent::SessionRecall, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn session_memory_recall_zero_limit_returns_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pane = SessionMemoryPane::open(dir.path()).await.expect("open pane");
        pane.record("anything").await.expect("record");

        let hits = pane.recall("anything", 0).await.expect("recall");
        assert!(hits.is_empty(), "limit=0 must short-circuit to empty");
    }

    #[tokio::test]
    async fn session_memory_persists_across_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        {
            let pane = SessionMemoryPane::open(dir.path()).await.expect("open pane");
            pane.record("rustc --version").await.expect("record");
        }
        // Reopen on the same root — recorded entry must survive.
        let pane2 = SessionMemoryPane::open(dir.path()).await.expect("reopen pane");
        let hits = pane2.recall("rustc", 5).await.expect("recall after reopen");
        assert!(
            hits.iter().any(|h| h.command.contains("rustc")),
            "rustc entry must survive reopen, got hits: {hits:?}"
        );
    }

    #[test]
    fn embed_is_deterministic_and_normalised() {
        let a = embed("git status");
        let b = embed("git status");
        assert_eq!(a, b, "embedding must be deterministic");
        let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "embedding must be L2-normalised, got |v|={norm}");
    }

    #[test]
    fn embed_similar_commands_score_higher_than_dissimilar() {
        let q = embed("cargo build");
        let near = embed("cargo build --release");
        let far = embed("rm -rf node_modules");

        let dot_near: f32 = q.iter().zip(near.iter()).map(|(x, y)| x * y).sum();
        let dot_far: f32 = q.iter().zip(far.iter()).map(|(x, y)| x * y).sum();

        assert!(
            dot_near > dot_far,
            "similar command must score higher than dissimilar one: near={dot_near}, far={dot_far}"
        );
    }
}
