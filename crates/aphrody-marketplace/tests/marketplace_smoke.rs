// SPDX-License-Identifier: Apache-2.0
//! Marketplace smoke tests — embedded index sanity, loader round-trips,
//! semver query semantics, integrity verification.

use std::fs;

use aphrody_marketplace::{
    EmbeddedIndexLoader, ExtensionEntry, ExtensionKind, FileIndexLoader, IndexLoader,
    MarketplaceError, MarketplaceIndex, verify_sha256,
};
use semver::{Version, VersionReq};
use tempfile::TempDir;

#[tokio::test]
async fn embedded_index_loads_non_empty() {
    let loader = EmbeddedIndexLoader::default();
    let idx = loader.load_index().await.expect("embedded load");
    assert!(!idx.entries.is_empty(), "embedded catalog must seed entries");
    assert!(idx.entries.len() >= 3, "spec mandates >=3 baseline rows");
    // Every entry must declare an aphrody-code author (anonymisation rule).
    for e in &idx.entries {
        assert_eq!(e.author, "aphrody-code");
        assert!(!e.id.is_empty());
        assert_eq!(e.sha256.len(), 64, "sha256 must be 64 hex chars");
    }
}

#[tokio::test]
async fn find_by_id_returns_expected_entry() {
    let idx = EmbeddedIndexLoader::default().load_index().await.unwrap();
    let hits = idx.find_by_id("aphrody-terminal-vt");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].kind, ExtensionKind::Skill);
    assert!(idx.find_by_id("does-not-exist").is_empty());
}

#[tokio::test]
async fn find_by_kind_filters() {
    let idx = EmbeddedIndexLoader::default().load_index().await.unwrap();
    let mcps = idx.find_by_kind(ExtensionKind::Mcp);
    assert!(
        mcps.iter().all(|e| e.kind == ExtensionKind::Mcp),
        "kind filter must be pure"
    );
    assert!(
        mcps.iter().any(|e| e.id == "anthropic-mcp"),
        "anthropic mcp expected in baseline"
    );
    let themes = idx.find_by_kind(ExtensionKind::Theme);
    assert!(!themes.is_empty(), "theme baseline missing");
}

#[tokio::test]
async fn latest_version_picks_highest_semver() {
    let idx = EmbeddedIndexLoader::default().load_index().await.unwrap();
    // aphrody-cron seeds two versions (0.1.0 + 0.2.0) — the helper must
    // return the higher one.
    let latest = idx.latest_version("aphrody-cron").expect("present");
    assert_eq!(latest.version, Version::new(0, 2, 0));
    assert!(idx.latest_version("absent-id").is_none());

    let compat = idx.compatible_with("aphrody-cron", &VersionReq::parse("^0.1").unwrap());
    // ^0.1 matches 0.1.0 only (0.2.0 is a new minor for 0.x).
    assert_eq!(compat.len(), 1);
    assert_eq!(compat[0].version, Version::new(0, 1, 0));
}

#[tokio::test]
async fn find_by_tag_matches_exact_tag() {
    let idx = EmbeddedIndexLoader::default().load_index().await.unwrap();
    let cron_rows = idx.find_by_tag("cron");
    assert!(!cron_rows.is_empty(), "cron tag should resolve");
    assert!(idx.find_by_tag("Cron").is_empty(), "tag match must be case-sensitive");
}

#[tokio::test]
async fn file_loader_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("marketplace.json");

    let seed = MarketplaceIndex::new(vec![ExtensionEntry {
        id: "round-trip".to_string(),
        name: "Round Trip".to_string(),
        version: Version::new(1, 2, 3),
        description: "fixture".to_string(),
        kind: ExtensionKind::Skill,
        source_url: "https://example.invalid/x.tar.gz".to_string(),
        sha256: "f".repeat(64),
        author: "aphrody-code".to_string(),
        tags: vec!["fixture".to_string()],
        runtime: "rust".to_string(),
        dependencies: vec![],
    }]);

    fs::write(&path, serde_json::to_vec(&seed).unwrap()).unwrap();

    let loader = FileIndexLoader::new(&path);
    let loaded = loader.load_index().await.expect("file load");
    assert_eq!(loaded.entries.len(), 1);
    assert_eq!(loaded.entries[0].id, "round-trip");
    assert_eq!(loaded.entries[0].version, Version::new(1, 2, 3));
}

#[tokio::test]
async fn file_loader_missing_path_surfaces_io_error() {
    let loader = FileIndexLoader::new("does-not-exist-12345.json");
    let err = loader.load_index().await.unwrap_err();
    assert!(matches!(err, MarketplaceError::Io { .. }));
}

#[test]
fn integrity_mismatch_returns_error() {
    let err = verify_sha256(b"payload", &"a".repeat(64)).unwrap_err();
    match err {
        MarketplaceError::IntegrityMismatch { expected, got } => {
            assert_eq!(expected.len(), 64);
            assert_eq!(got.len(), 64);
            assert_ne!(expected, got);
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    // And a real match must succeed.
    let known = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    verify_sha256(b"abc", known).expect("match");
}
