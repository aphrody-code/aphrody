// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for the aphrody-search crate.

use aphrody_search::{
    DocId, Document, InvertedIndex, SearchQuery, TokenizerConfig, tokenize,
};
use chrono::Utc;
use serde_json::json;
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn doc(id: &str, title: &str, body: &str, tags: &[&str]) -> Document {
    Document {
        id: DocId::from(id),
        title: title.into(),
        body: body.into(),
        tags: tags.iter().map(|s| (*s).to_string()).collect(),
        metadata: json!({}),
        indexed_at: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// 1. tokenizer drops stop words + punctuation
// ---------------------------------------------------------------------------

#[test]
fn tokenize_strips_stopwords_and_punctuation() {
    let cfg = TokenizerConfig::default();
    let toks = tokenize("The Rust async runtime is fast, and reliable!", &cfg);
    let strs: Vec<&str> = toks.iter().map(|t| t.as_str()).collect();

    // Stop words gone.
    assert!(!strs.contains(&"the"), "the not stripped: {strs:?}");
    assert!(!strs.contains(&"is"), "is not stripped: {strs:?}");
    assert!(!strs.contains(&"and"), "and not stripped: {strs:?}");

    // Real words kept (lowercased).
    assert!(strs.contains(&"rust"), "rust missing: {strs:?}");
    assert!(strs.contains(&"async"), "async missing: {strs:?}");
    assert!(strs.contains(&"runtime"), "runtime missing: {strs:?}");
    assert!(strs.contains(&"fast"), "fast missing: {strs:?}");
    assert!(strs.contains(&"reliable"), "reliable missing: {strs:?}");
}

// ---------------------------------------------------------------------------
// 2. indexing grows len
// ---------------------------------------------------------------------------

#[test]
fn index_document_increases_len() {
    let mut idx = InvertedIndex::new(TokenizerConfig::default());
    assert_eq!(idx.len(), 0);
    assert!(idx.is_empty());

    idx.index_document(doc("a", "T1", "body one", &["t"])).unwrap();
    assert_eq!(idx.len(), 1);
    assert!(!idx.is_empty());

    idx.index_document(doc("b", "T2", "body two", &["t"])).unwrap();
    assert_eq!(idx.len(), 2);

    // Duplicate id rejected.
    let err = idx.index_document(doc("a", "again", "body again", &[])).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("duplicate"), "expected duplicate err, got {msg}");
}

// ---------------------------------------------------------------------------
// 3. basic recall
// ---------------------------------------------------------------------------

#[test]
fn search_finds_documents_by_term() {
    let mut idx = InvertedIndex::new(TokenizerConfig::default());
    idx.index_document(doc("d1", "Rust", "Rust is systems programming", &["lang"])).unwrap();
    idx.index_document(doc("d2", "Python", "Python is dynamic", &["lang"])).unwrap();
    idx.index_document(doc("d3", "JavaScript", "JS runs in browsers", &["web"])).unwrap();

    let hits = idx.search(&SearchQuery {
        terms: vec!["rust".into()],
        tags: vec![],
        limit: 10,
    });
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].doc_id.as_str(), "d1");
    assert!(hits[0].matched_terms.iter().any(|t| t == "rust"));
}

// ---------------------------------------------------------------------------
// 4. tf-based ranking
// ---------------------------------------------------------------------------

#[test]
fn search_ranks_higher_tf_first() {
    let mut idx = InvertedIndex::new(TokenizerConfig::default());
    idx.index_document(doc(
        "high",
        "stuff",
        "rust rust rust rust rust async programming",
        &[],
    ))
    .unwrap();
    idx.index_document(doc("low", "stuff", "rust appears once here", &[])).unwrap();

    let hits = idx.search(&SearchQuery {
        terms: vec!["rust".into()],
        tags: vec![],
        limit: 10,
    });
    assert_eq!(hits.len(), 2, "expected both docs to match: {hits:?}");
    assert_eq!(
        hits[0].doc_id.as_str(),
        "high",
        "high-tf doc should rank first: scores={:?}",
        hits.iter().map(|h| (h.doc_id.as_str().to_string(), h.score)).collect::<Vec<_>>()
    );
    assert!(hits[0].score >= hits[1].score);
}

// ---------------------------------------------------------------------------
// 5. tag filter
// ---------------------------------------------------------------------------

#[test]
fn search_filters_by_tag() {
    let mut idx = InvertedIndex::new(TokenizerConfig::default());
    idx.index_document(doc("rs", "Doc A", "shared keyword body", &["rust"])).unwrap();
    idx.index_document(doc("py", "Doc B", "shared keyword body", &["python"])).unwrap();
    idx.index_document(doc("js", "Doc C", "shared keyword body", &["javascript"])).unwrap();

    let hits = idx.search(&SearchQuery {
        terms: vec!["shared".into()],
        tags: vec!["rust".into()],
        limit: 10,
    });
    assert_eq!(hits.len(), 1, "tag filter failed: {hits:?}");
    assert_eq!(hits[0].doc_id.as_str(), "rs");

    // AND semantics: two tags require both to be present.
    let hits2 = idx.search(&SearchQuery {
        terms: vec!["shared".into()],
        tags: vec!["rust".into(), "missing".into()],
        limit: 10,
    });
    assert!(hits2.is_empty(), "AND filter leaked: {hits2:?}");
}

// ---------------------------------------------------------------------------
// 6. snippet rendering
// ---------------------------------------------------------------------------

#[test]
fn search_returns_snippet_with_match() {
    let mut idx = InvertedIndex::new(TokenizerConfig::default());
    idx.index_document(doc(
        "snip",
        "title",
        "Aphrody is built in rust with strict cross-platform guarantees.",
        &[],
    ))
    .unwrap();

    let hits = idx.search(&SearchQuery {
        terms: vec!["rust".into()],
        tags: vec![],
        limit: 5,
    });
    assert_eq!(hits.len(), 1, "{hits:?}");
    let snip = &hits[0].snippet;
    assert!(
        snip.to_lowercase().contains("<<rust>>"),
        "expected snippet to wrap match: {snip}"
    );
}

// ---------------------------------------------------------------------------
// 7. removal hides documents
// ---------------------------------------------------------------------------

#[test]
fn remove_document_excludes_from_results() {
    let mut idx = InvertedIndex::new(TokenizerConfig::default());
    idx.index_document(doc("keep", "T", "alpha beta gamma", &[])).unwrap();
    idx.index_document(doc("drop", "T", "alpha beta gamma", &[])).unwrap();

    assert_eq!(idx.len(), 2);
    idx.remove_document(&DocId::from("drop")).unwrap();
    assert_eq!(idx.len(), 1);

    let hits = idx.search(&SearchQuery {
        terms: vec!["alpha".into()],
        tags: vec![],
        limit: 5,
    });
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].doc_id.as_str(), "keep");

    // Removing again surfaces a not-found error.
    let err = idx.remove_document(&DocId::from("drop")).unwrap_err();
    assert!(format!("{err}").contains("not found"));
}

// ---------------------------------------------------------------------------
// 8. JSON save / load round trip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn index_save_and_load_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("nested").join("index.json");

    let mut idx = InvertedIndex::new(TokenizerConfig::default());
    idx.index_document(doc("a", "Async runtime", "tokio rust async runtime", &["rust"]))
        .unwrap();
    idx.index_document(doc("b", "Web", "web servers in rust", &["rust", "web"])).unwrap();
    idx.index_document(doc("c", "Other", "python async io", &["python"])).unwrap();

    idx.save(&path).await.expect("save");
    assert!(path.exists(), "index file not written: {}", path.display());

    let loaded = InvertedIndex::load(&path).await.expect("load");
    assert_eq!(loaded.len(), 3);

    let hits = loaded.search(&SearchQuery {
        terms: vec!["rust".into(), "async".into()],
        tags: vec![],
        limit: 10,
    });
    assert!(!hits.is_empty(), "post-load search empty");
    // Top hit should still mention rust+async.
    let top = &hits[0];
    assert!(top.matched_terms.iter().any(|t| t == "rust"));
}

// ---------------------------------------------------------------------------
// 9. BM25 rewards rare terms in short documents
// ---------------------------------------------------------------------------

#[test]
fn bm25_scores_short_docs_higher_for_rare_terms() {
    let mut idx = InvertedIndex::new(TokenizerConfig::default());

    // Common term "common" appears everywhere; rare term "lighthouse" only
    // in one short doc. The short doc should outscore long noise docs.
    idx.index_document(doc("rare_short", "Lighthouse", "lighthouse keyword", &[])).unwrap();
    // Three noise docs to make "common" frequent (lower idf).
    for i in 0..5 {
        let id = format!("noise{i}");
        idx.index_document(doc(
            &id,
            "noise",
            "common common common common common filler filler filler filler filler filler",
            &[],
        ))
        .unwrap();
    }

    let hits = idx.search(&SearchQuery {
        terms: vec!["lighthouse".into(), "common".into()],
        tags: vec![],
        limit: 10,
    });
    assert!(!hits.is_empty(), "no hits");
    assert_eq!(
        hits[0].doc_id.as_str(),
        "rare_short",
        "rare term should dominate ranking: {:?}",
        hits.iter().map(|h| (h.doc_id.as_str().to_string(), h.score)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// 10. custom stop words excluded
// ---------------------------------------------------------------------------

#[test]
fn custom_stopwords_excluded_from_index() {
    let mut cfg = TokenizerConfig::default();
    cfg.custom_stopwords = vec!["banana".into(), "kiwi".into()];

    let mut idx = InvertedIndex::new(cfg.clone());
    idx.index_document(doc("d", "fruit", "banana kiwi apple cherry", &[])).unwrap();

    // Apple and cherry should match.
    let h1 = idx.search(&SearchQuery {
        terms: vec!["apple".into()],
        tags: vec![],
        limit: 5,
    });
    assert_eq!(h1.len(), 1, "{h1:?}");

    // Banana / kiwi must NOT match (filtered both at index + query time).
    let h2 = idx.search(&SearchQuery {
        terms: vec!["banana".into()],
        tags: vec![],
        limit: 5,
    });
    assert!(h2.is_empty(), "stop word leaked into postings: {h2:?}");

    let h3 = idx.search(&SearchQuery {
        terms: vec!["kiwi".into()],
        tags: vec![],
        limit: 5,
    });
    assert!(h3.is_empty(), "stop word leaked: {h3:?}");
}

// ---------------------------------------------------------------------------
// 11. tokenizer min/max length boundaries
// ---------------------------------------------------------------------------

#[test]
fn tokenizer_respects_length_bounds() {
    let mut cfg = TokenizerConfig::default();
    cfg.min_token_len = 4;
    cfg.max_token_len = 8;

    let toks = tokenize("hi rust verylongtokenname rust", &cfg);
    let strs: Vec<&str> = toks.iter().map(|t| t.as_str()).collect();
    assert!(!strs.contains(&"hi"), "min_token_len failed");
    assert!(strs.contains(&"rust"));
    assert!(!strs.contains(&"verylongtokenname"), "max_token_len failed: {strs:?}");
}

// ---------------------------------------------------------------------------
// 12. reindex_document replaces an existing entry
// ---------------------------------------------------------------------------

#[test]
fn reindex_document_replaces_entry() {
    let mut idx = InvertedIndex::new(TokenizerConfig::default());
    idx.index_document(doc("x", "v1", "alpha", &[])).unwrap();
    let before = idx
        .search(&SearchQuery {
            terms: vec!["alpha".into()],
            tags: vec![],
            limit: 5,
        })
        .len();
    assert_eq!(before, 1);

    idx.reindex_document(doc("x", "v2", "beta", &[])).unwrap();
    let after_alpha = idx
        .search(&SearchQuery {
            terms: vec!["alpha".into()],
            tags: vec![],
            limit: 5,
        })
        .len();
    assert_eq!(after_alpha, 0, "old body should be gone");

    let after_beta = idx.search(&SearchQuery {
        terms: vec!["beta".into()],
        tags: vec![],
        limit: 5,
    });
    assert_eq!(after_beta.len(), 1);
    assert_eq!(after_beta[0].doc_id.as_str(), "x");
}
