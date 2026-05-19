// SPDX-License-Identifier: Apache-2.0
//! aphrody-search — in-memory full-text search index for the aphrody runtime.
//!
//! The crate ships a small, dependency-light search engine the wider workspace
//! reuses to index three orthogonal corpora without taking a Tantivy / Meili
//! dependency:
//!
//! * **`aphrody-memory`** records — agent conversations, persisted skill
//!   triggers, marketplace artifact descriptions.
//! * **`aphrody-marketplace`** entries — discoverable skills / MCP / hooks
//!   currently surface a `find_by_tag` linear scan; this crate provides a
//!   real inverted-index BM25-lite ranking the marketplace UI can consume.
//! * **`aphrody-cron`** & **`aphrody-skills-forge`** — both persist JSON
//!   bundles to disk; this crate's [`InvertedIndex::save`] / [`load`] follow
//!   the same atomic temp-file + rename pattern as
//!   `aphrody_cron::JsonFileJobStore`.
//!
//! ## Design
//!
//! * Tokenizer is a regex-free, allocation-conscious lowercase splitter that
//!   strips configurable stop words (EN + FR defaults) and length-bounded
//!   tokens. The `regex` crate is wired in `Cargo.toml` so consumers can
//!   plug a custom tokenizer via [`TokenizerConfig::custom_stopwords`] +
//!   their own regex pre-pass.
//! * Postings are stored as a `HashMap<Token, Vec<Posting>>` so document
//!   removal is O(unique-terms-in-doc). The per-document length cache is
//!   updated in lock-step so BM25 stays accurate after removals.
//! * Scoring is a BM25 variant with `k1 = 1.5`, `b = 0.75` — the default
//!   Lucene values — but the IDF is the simple `ln((N - df + 0.5) / (df + 0.5) + 1.0)`
//!   variant ("BM25+") to keep scores non-negative even when terms appear
//!   in the majority of documents.
//! * Snippet generation extracts ±`radius` chars around the first
//!   case-insensitive match and wraps the match in `<<>>` markers so the
//!   caller stays free to render them however they want (terminal bold,
//!   HTML `<mark>`, JSON output, …).
//!
//! ## Example
//!
//! ```no_run
//! use aphrody_search::{Document, DocId, InvertedIndex, SearchQuery, TokenizerConfig};
//! use chrono::Utc;
//!
//! let mut idx = InvertedIndex::new(TokenizerConfig::default());
//! idx.index_document(Document {
//!     id: DocId::from("doc-1"),
//!     title: "Rust async runtime".into(),
//!     body: "Tokio is a popular async runtime for Rust.".into(),
//!     tags: vec!["rust".into(), "async".into()],
//!     metadata: serde_json::json!({}),
//!     indexed_at: Utc::now(),
//! }).expect("index");
//!
//! let hits = idx.search(&SearchQuery {
//!     terms: vec!["rust".into()],
//!     tags: vec![],
//!     limit: 10,
//! });
//! assert!(!hits.is_empty());
//! ```

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All recoverable failures surfaced by the search index.
#[derive(Debug, Error)]
pub enum SearchError {
    /// Filesystem failure while loading / persisting the index.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON (de)serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// `remove_document` was called with an unknown id.
    #[error("document not found: {0}")]
    DocumentNotFound(String),
    /// `index_document` was called with an id that already exists. Use
    /// [`InvertedIndex::reindex_document`] (which re-indexes in place) to
    /// update an existing entry.
    #[error("duplicate document id: {0}")]
    DuplicateDocument(String),
    /// The provided [`SearchQuery`] is structurally invalid (empty terms +
    /// no tag filter would match every document).
    #[error("invalid query: {0}")]
    InvalidQuery(String),
}

// ---------------------------------------------------------------------------
// Newtypes
// ---------------------------------------------------------------------------

/// Stable string identifier for an indexed [`Document`].
///
/// Implemented as a `String` newtype so the type system distinguishes a
/// document id from a free-form tag or token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocId(pub String);

impl DocId {
    /// Borrow the inner identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DocId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for DocId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for DocId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A single lowercase token emitted by [`tokenize`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Token(pub String);

impl Token {
    /// Borrow the underlying token text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Token {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Token {
    fn from(s: String) -> Self {
        Self(s)
    }
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

/// Default English + French stop-word list (~30 entries). Kept short on
/// purpose: more aggressive lists hurt recall on technical corpora where
/// terms like "a" (Rust attribute name) or "is" (assertion) carry meaning.
const DEFAULT_STOPWORDS: &[&str] = &[
    // English
    "the", "a", "an", "and", "or", "but", "is", "are", "was", "were", "be",
    "to", "of", "in", "on", "at", "for", "with", "by", "as", "it", "this",
    "that", "from",
    // French
    "le", "la", "les", "de", "du", "des", "et", "ou", "un", "une", "ce",
    "ces", "est", "sont",
];

/// Tunable knobs for [`tokenize`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerConfig {
    /// Lowercase tokens before emitting them (default `true`).
    #[serde(default = "default_true")]
    pub lowercase: bool,
    /// Strip the default stop-word list + any [`Self::custom_stopwords`]
    /// (default `true`).
    #[serde(default = "default_true")]
    pub strip_stopwords: bool,
    /// Minimum byte length of a token to be emitted (default `2`). Drops
    /// single-character tokens that are usually punctuation residue.
    #[serde(default = "default_min_len")]
    pub min_token_len: usize,
    /// Maximum byte length of a token to be emitted (default `30`). Drops
    /// pathological hash-like tokens that explode the postings map.
    #[serde(default = "default_max_len")]
    pub max_token_len: usize,
    /// Caller-provided stop words appended to the default list when
    /// [`Self::strip_stopwords`] is `true`. Stored lowercase.
    #[serde(default)]
    pub custom_stopwords: Vec<String>,
}

fn default_true() -> bool { true }
fn default_min_len() -> usize { 2 }
fn default_max_len() -> usize { 30 }

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            lowercase: true,
            strip_stopwords: true,
            min_token_len: 2,
            max_token_len: 30,
            custom_stopwords: Vec::new(),
        }
    }
}

impl TokenizerConfig {
    fn stopword_set(&self) -> HashSet<String> {
        let mut set: HashSet<String> = DEFAULT_STOPWORDS.iter().map(|s| (*s).to_string()).collect();
        for word in &self.custom_stopwords {
            set.insert(word.to_lowercase());
        }
        set
    }
}

/// Split `text` into lowercase tokens using the supplied configuration.
///
/// The splitter walks the input once, accumulating runs of ASCII
/// alphanumerics. Non-ASCII chars (e.g. accented Latin glyphs) are kept
/// verbatim *after* the lowercase normalisation so French content still
/// indexes meaningfully.
///
/// # Examples
///
/// ```
/// use aphrody_search::{tokenize, TokenizerConfig, Token};
/// let toks = tokenize("Rust + Tokio = async!", &TokenizerConfig::default());
/// assert!(toks.contains(&Token::from("rust")));
/// assert!(toks.contains(&Token::from("tokio")));
/// assert!(toks.contains(&Token::from("async")));
/// ```
#[must_use]
pub fn tokenize(text: &str, cfg: &TokenizerConfig) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let stopwords = if cfg.strip_stopwords {
        cfg.stopword_set()
    } else {
        HashSet::new()
    };

    let push = |current: &mut String, tokens: &mut Vec<Token>| {
        if current.is_empty() {
            return;
        }
        let lowered = if cfg.lowercase {
            current.to_lowercase()
        } else {
            std::mem::take(current)
        };
        let len = lowered.len();
        if len >= cfg.min_token_len
            && len <= cfg.max_token_len
            && !stopwords.contains(&lowered)
        {
            tokens.push(Token(lowered));
        }
        current.clear();
    };

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            current.push(ch);
        } else {
            push(&mut current, &mut tokens);
        }
    }
    push(&mut current, &mut tokens);
    tokens
}

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// A single indexable record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// Stable id (e.g. `skill:web-design-guidelines:v0.3`).
    pub id: DocId,
    /// Short heading boosted at index time (currently weighted 2× tf).
    pub title: String,
    /// Free-form body — markdown, README excerpts, JSON manifests, …
    pub body: String,
    /// Tags consumed by [`SearchQuery::tags`] for facet filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Caller-defined metadata round-tripped verbatim. Not indexed.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Wall-clock at index time. Round-trips via RFC 3339.
    pub indexed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Inverted index
// ---------------------------------------------------------------------------

/// One postings-list entry: a document id + the term frequency inside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posting {
    /// Identifier of the document the term appears in.
    pub doc_id: DocId,
    /// Number of occurrences of the term within that document
    /// (title hits weighted 2× when accumulated in [`InvertedIndex::index_document`]).
    pub tf: u32,
}

/// In-memory inverted index with BM25-lite scoring + atomic JSON persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvertedIndex {
    /// `Token -> Vec<Posting>` map. Postings inside a list are *not* sorted
    /// at insertion time; [`search`] iterates the full list per term.
    pub postings: HashMap<Token, Vec<Posting>>,
    /// Total token count per document (used as the denominator inside BM25).
    pub doc_lengths: HashMap<DocId, u32>,
    /// Full document storage so [`search`] can materialise snippets without
    /// requiring callers to re-supply the corpus on every query.
    pub documents: HashMap<DocId, Document>,
    /// Tokenizer settings frozen at construction time.
    pub tokenizer: TokenizerConfig,
}

impl InvertedIndex {
    /// Create an empty index using `tokenizer` for both indexing and queries.
    #[must_use]
    pub fn new(tokenizer: TokenizerConfig) -> Self {
        Self {
            postings: HashMap::new(),
            doc_lengths: HashMap::new(),
            documents: HashMap::new(),
            tokenizer,
        }
    }

    /// Tokenize and store `doc`. Returns an error if a document with the
    /// same id is already present — call [`Self::reindex_document`] for
    /// in-place updates.
    ///
    /// Title tokens are counted twice so heading hits float to the top.
    pub fn index_document(&mut self, doc: Document) -> Result<(), SearchError> {
        if self.documents.contains_key(&doc.id) {
            return Err(SearchError::DuplicateDocument(doc.id.0.clone()));
        }
        self.insert_document_inner(doc);
        Ok(())
    }

    /// Convenience: remove then re-insert a document, preserving the
    /// caller's stable id. Returns `Ok(())` even if the document was not
    /// previously indexed.
    pub fn reindex_document(&mut self, doc: Document) -> Result<(), SearchError> {
        if self.documents.contains_key(&doc.id) {
            self.remove_document(&doc.id)?;
        }
        self.insert_document_inner(doc);
        Ok(())
    }

    fn insert_document_inner(&mut self, doc: Document) {
        let title_tokens = tokenize(&doc.title, &self.tokenizer);
        let body_tokens = tokenize(&doc.body, &self.tokenizer);

        // Accumulate term frequencies, weighting title tokens 2x.
        let mut tf_map: HashMap<Token, u32> = HashMap::new();
        for tok in &title_tokens {
            *tf_map.entry(tok.clone()).or_insert(0) += 2;
        }
        for tok in &body_tokens {
            *tf_map.entry(tok.clone()).or_insert(0) += 1;
        }

        let doc_len = (title_tokens.len() as u32) + (body_tokens.len() as u32);
        self.doc_lengths.insert(doc.id.clone(), doc_len);

        for (tok, tf) in tf_map {
            self.postings
                .entry(tok)
                .or_default()
                .push(Posting { doc_id: doc.id.clone(), tf });
        }

        self.documents.insert(doc.id.clone(), doc);
    }

    /// Remove `id` and every posting that references it. Returns
    /// [`SearchError::DocumentNotFound`] if the id is unknown.
    pub fn remove_document(&mut self, id: &DocId) -> Result<(), SearchError> {
        if !self.documents.contains_key(id) {
            return Err(SearchError::DocumentNotFound(id.0.clone()));
        }
        self.documents.remove(id);
        self.doc_lengths.remove(id);
        // Drop the doc from every postings list, then drop empty lists.
        let empty_tokens: Vec<Token> = self
            .postings
            .iter_mut()
            .filter_map(|(tok, list)| {
                list.retain(|p| p.doc_id != *id);
                if list.is_empty() {
                    Some(tok.clone())
                } else {
                    None
                }
            })
            .collect();
        for tok in empty_tokens {
            self.postings.remove(&tok);
        }
        Ok(())
    }

    /// Number of indexed documents.
    #[must_use]
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// `true` if no document has been indexed yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Sum of every document length — useful for diagnostics.
    #[must_use]
    pub fn total_terms(&self) -> u64 {
        self.doc_lengths.values().map(|n| u64::from(*n)).sum()
    }

    /// Mean document length, in tokens. Returns `0.0` on an empty index.
    #[must_use]
    pub fn avg_doc_length(&self) -> f32 {
        if self.documents.is_empty() {
            return 0.0;
        }
        let total = self.total_terms() as f32;
        total / (self.documents.len() as f32)
    }

    /// Iterate every indexed document in arbitrary order.
    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }

    // -------------------------------------------------------------------
    // Search
    // -------------------------------------------------------------------

    /// Execute `query` against the index. Returns up to `query.limit` hits
    /// sorted by descending BM25-lite score.
    ///
    /// * Empty terms + non-empty tags → tag-only filter, score = 0.
    /// * Empty terms + empty tags → empty result (no implicit "match all").
    #[must_use]
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchHit> {
        if query.terms.is_empty() && query.tags.is_empty() {
            return Vec::new();
        }
        if query.limit == 0 {
            return Vec::new();
        }

        let q_tokens: Vec<Token> = query
            .terms
            .iter()
            .flat_map(|t| tokenize(t, &self.tokenizer))
            .collect();

        let n = self.documents.len() as f32;
        let avgdl = self.avg_doc_length().max(1.0);
        const K1: f32 = 1.5;
        const B: f32 = 0.75;

        // Pre-filter the candidate doc set by tag membership.
        let tag_filter: Option<HashSet<String>> = if query.tags.is_empty() {
            None
        } else {
            Some(query.tags.iter().map(|s| s.to_lowercase()).collect())
        };
        let passes_tag_filter = |doc: &Document| -> bool {
            match &tag_filter {
                None => true,
                Some(needed) => needed.iter().all(|wanted| {
                    doc.tags.iter().any(|have| have.to_lowercase() == *wanted)
                }),
            }
        };

        // Tag-only query: return every matching doc with score 0.0, no snippet.
        // Only enter this branch when the caller didn't supply any term at all
        // — if every supplied term tokenized to nothing (e.g. all custom stop
        // words), treat as an empty result instead of leaking the full corpus.
        if q_tokens.is_empty() {
            if !query.terms.is_empty() {
                return Vec::new();
            }
            let mut hits: Vec<SearchHit> = self
                .documents
                .values()
                .filter(|d| passes_tag_filter(d))
                .map(|d| SearchHit {
                    doc_id: d.id.clone(),
                    score: 0.0,
                    matched_terms: Vec::new(),
                    snippet: snippet(&d.body, &[], 60),
                })
                .collect();
            hits.sort_by(|a, b| a.doc_id.0.cmp(&b.doc_id.0));
            hits.truncate(query.limit);
            return hits;
        }

        // Accumulate per-doc score + matched terms.
        let mut acc: HashMap<DocId, (f32, HashSet<String>)> = HashMap::new();
        for tok in &q_tokens {
            let Some(list) = self.postings.get(tok) else { continue };
            let df = list.len() as f32;
            // BM25+ IDF variant: always non-negative.
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            for posting in list {
                let Some(doc) = self.documents.get(&posting.doc_id) else { continue };
                if !passes_tag_filter(doc) {
                    continue;
                }
                let dl = f32::from(self.doc_lengths.get(&posting.doc_id).copied().unwrap_or(0) as u16);
                let tf = posting.tf as f32;
                let num = tf * (K1 + 1.0);
                let den = tf + K1 * (1.0 - B + B * (dl / avgdl));
                let contrib = idf * (num / den.max(f32::EPSILON));
                let entry = acc.entry(posting.doc_id.clone()).or_insert((0.0, HashSet::new()));
                entry.0 += contrib;
                entry.1.insert(tok.0.clone());
            }
        }

        let mut hits: Vec<SearchHit> = acc
            .into_iter()
            .filter_map(|(doc_id, (score, matched))| {
                let doc = self.documents.get(&doc_id)?;
                let matched_terms: Vec<String> = matched.into_iter().collect();
                let snip = snippet(&doc.body, &matched_terms, 60);
                Some(SearchHit { doc_id, score, matched_terms, snippet: snip })
            })
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.doc_id.0.cmp(&b.doc_id.0))
        });
        hits.truncate(query.limit);
        hits
    }

    // -------------------------------------------------------------------
    // Persistence — atomic temp + rename (mirrors aphrody_cron::JsonFileJobStore).
    // -------------------------------------------------------------------

    /// Persist the full index to `path` as pretty-printed JSON.
    /// Writes to `<path>.tmp` first, then renames atomically.
    pub async fn save(&self, path: &Path) -> Result<(), SearchError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).await?;
            }
        }
        let json = serde_json::to_vec_pretty(self)?;
        let tmp: PathBuf = path.with_extension("json.tmp");
        let mut file = fs::File::create(&tmp).await?;
        file.write_all(&json).await?;
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        fs::rename(&tmp, path).await?;
        tracing::debug!(target: "aphrody_search", path = %path.display(), docs = self.len(), "search index persisted");
        Ok(())
    }

    /// Load an index previously written via [`Self::save`].
    pub async fn load(path: &Path) -> Result<Self, SearchError> {
        let bytes = fs::read(path).await?;
        let idx: Self = serde_json::from_slice(&bytes)?;
        Ok(idx)
    }
}

// ---------------------------------------------------------------------------
// Query + hit
// ---------------------------------------------------------------------------

/// One search request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Raw query terms — each entry is re-tokenized through the same
    /// [`TokenizerConfig`] the index was built with.
    #[serde(default)]
    pub terms: Vec<String>,
    /// Tag filter (AND semantics — every tag must appear on the document).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Maximum number of hits to return. `0` returns nothing.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 10 }

impl Default for SearchQuery {
    fn default() -> Self {
        Self { terms: Vec::new(), tags: Vec::new(), limit: 10 }
    }
}

impl SearchQuery {
    /// Validate the query against the shape rules. Currently rejects only
    /// the degenerate `terms.len() == 0 && tags.len() == 0 && limit > 0`
    /// case so callers can rely on a non-trivial intent.
    pub fn validate(&self) -> Result<(), SearchError> {
        if self.terms.is_empty() && self.tags.is_empty() {
            return Err(SearchError::InvalidQuery(
                "query must carry at least one term or tag".into(),
            ));
        }
        Ok(())
    }
}

/// One result row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    /// Identifier of the matched document.
    pub doc_id: DocId,
    /// BM25-lite aggregated score (higher = better).
    pub score: f32,
    /// Subset of [`SearchQuery::terms`] that actually contributed to the score.
    pub matched_terms: Vec<String>,
    /// `±radius` chars around the first match, with the match wrapped in `<<>>`.
    pub snippet: String,
}

// ---------------------------------------------------------------------------
// Snippet
// ---------------------------------------------------------------------------

/// Find the first occurrence of any `matched_terms` (case-insensitive) inside
/// `body` and return `"…{prefix}<<{match}>>{suffix}…"` with `radius` chars on
/// each side. Returns the leading slice of `body` if no term matches.
#[must_use]
pub fn snippet(body: &str, matched_terms: &[String], radius: usize) -> String {
    let trimmed: String = body.chars().take(2_000).collect();
    if trimmed.is_empty() {
        return String::new();
    }
    let lower = trimmed.to_lowercase();
    let mut best: Option<(usize, usize, &str)> = None;
    for term in matched_terms {
        let needle = term.to_lowercase();
        if needle.is_empty() {
            continue;
        }
        if let Some(pos) = lower.find(&needle) {
            let len = needle.len();
            match best {
                Some((p, _, _)) if p <= pos => {}
                _ => best = Some((pos, len, term.as_str())),
            }
        }
    }

    let head_len = trimmed.chars().count().min(radius);
    let Some((pos, mlen, _)) = best else {
        let mut head: String = trimmed.chars().take(head_len).collect();
        if trimmed.chars().count() > head_len {
            head.push_str("...");
        }
        return head;
    };

    // Safe byte-aligned prefix / suffix windows.
    let start = pos.saturating_sub(radius);
    let end_byte = pos + mlen;
    let end = (end_byte + radius).min(trimmed.len());
    let start = align_to_char(&trimmed, start);
    let end = align_to_char(&trimmed, end);

    let prefix = &trimmed[start..pos];
    let matched = &trimmed[pos..end_byte];
    let suffix = &trimmed[end_byte..end];

    let mut out = String::with_capacity(prefix.len() + matched.len() + suffix.len() + 12);
    if start > 0 {
        out.push_str("...");
    }
    out.push_str(prefix);
    out.push_str("<<");
    out.push_str(matched);
    out.push_str(">>");
    out.push_str(suffix);
    if end < trimmed.len() {
        out.push_str("...");
    }
    out
}

fn align_to_char(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

// ---------------------------------------------------------------------------
// Unit tests (in-file, smoke is in tests/search_smoke.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> TokenizerConfig {
        TokenizerConfig::default()
    }

    #[test]
    fn tokenize_lowercases_and_splits() {
        let toks = tokenize("Hello, World! 42 RUST", &cfg());
        let strs: Vec<&str> = toks.iter().map(Token::as_str).collect();
        assert!(strs.contains(&"hello"));
        assert!(strs.contains(&"world"));
        assert!(strs.contains(&"42"));
        assert!(strs.contains(&"rust"));
    }

    #[test]
    fn snippet_wraps_match() {
        let s = snippet("The quick brown fox jumps over the lazy dog", &["fox".into()], 8);
        assert!(s.contains("<<fox>>"), "snippet was: {s}");
    }

    #[test]
    fn empty_index_search_is_empty() {
        let idx = InvertedIndex::new(cfg());
        let hits = idx.search(&SearchQuery {
            terms: vec!["whatever".into()],
            tags: vec![],
            limit: 5,
        });
        assert!(hits.is_empty());
    }
}
