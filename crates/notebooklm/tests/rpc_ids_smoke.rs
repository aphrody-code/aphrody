// SPDX-License-Identifier: Apache-2.0
//! Sanity check on the verbatim-ported RPC ID catalogue.

use notebooklm::rpc_ids;

#[test]
fn all_rpc_ids_are_exported_and_non_empty() {
    let all = [
        rpc_ids::CREATE_NOTEBOOK,
        rpc_ids::LIST_NOTEBOOKS,
        rpc_ids::GET_NOTEBOOK,
        rpc_ids::RENAME_NOTEBOOK,
        rpc_ids::DELETE_NOTEBOOK,
        rpc_ids::REMOVE_RECENTLY_VIEWED,
        rpc_ids::ADD_SOURCE,
        rpc_ids::ADD_SOURCE_FILE,
        rpc_ids::GET_SOURCE_CONTENT,
        rpc_ids::GET_SOURCE_SUMMARY,
        rpc_ids::DELETE_SOURCE,
        rpc_ids::REFRESH_SOURCE,
        rpc_ids::UPDATE_SOURCE,
        rpc_ids::CREATE_WEB_SEARCH,
        rpc_ids::CREATE_DEEP_RESEARCH,
        rpc_ids::POLL_RESEARCH,
        rpc_ids::IMPORT_RESEARCH,
        rpc_ids::GENERATE_ARTIFACT,
        rpc_ids::GET_ARTIFACTS_FILTERED,
        rpc_ids::DELETE_ARTIFACT,
        rpc_ids::RENAME_ARTIFACT,
        rpc_ids::GET_INTERACTIVE_HTML,
        rpc_ids::EXPORT_ARTIFACT,
        rpc_ids::SHARE_ARTIFACT,
        rpc_ids::GET_STUDIO_CONFIG,
        rpc_ids::CREATE_NOTE,
        rpc_ids::GET_NOTES,
        rpc_ids::UPDATE_NOTE,
        rpc_ids::DELETE_NOTE,
        rpc_ids::LIST_CHAT_THREADS,
        rpc_ids::DELETE_CHAT_THREAD,
        rpc_ids::GET_SHARE_STATUS,
        rpc_ids::SHARE_NOTEBOOK,
        rpc_ids::GET_ACCOUNT_INFO,
        rpc_ids::SET_USER_SETTINGS,
        rpc_ids::GET_NOTEBOOK_SUMMARY,
        rpc_ids::GET_RECOMMENDED_TOPICS,
        rpc_ids::GET_UI_CONFIG,
        rpc_ids::REPORT_PLAY_PROGRESS,
    ];
    assert_eq!(all.len(), 39, "wrong RPC count");
    for id in all {
        assert!(!id.is_empty(), "empty RPC id");
        assert!(
            id.len() >= 5 && id.len() <= 8,
            "RPC id {id:?} should be 5-8 chars (Google Boq style)",
        );
        assert!(
            id.chars().all(|c| c.is_ascii_alphanumeric()),
            "RPC id {id:?} must be alphanumeric",
        );
    }
}

#[test]
fn artifact_type_constants_in_expected_range() {
    let kinds = [
        rpc_ids::ARTIFACT_TYPE_AUDIO,
        rpc_ids::ARTIFACT_TYPE_REPORT,
        rpc_ids::ARTIFACT_TYPE_VIDEO,
        rpc_ids::ARTIFACT_TYPE_QUIZ,
        rpc_ids::ARTIFACT_TYPE_MIND_MAP,
        rpc_ids::ARTIFACT_TYPE_FLASHCARDS,
        rpc_ids::ARTIFACT_TYPE_INFOGRAPHIC,
        rpc_ids::ARTIFACT_TYPE_SLIDE_DECK,
        rpc_ids::ARTIFACT_TYPE_DATA_TABLE,
    ];
    assert_eq!(kinds.len(), 9);
    for k in kinds {
        assert!(k >= 1 && k <= 9, "artifact kind discriminant out of range: {k}");
    }
}

#[test]
fn url_constants_point_at_notebooklm_google_com() {
    assert!(rpc_ids::URL_BASE.starts_with("https://notebooklm.google.com"));
    assert!(rpc_ids::URL_DASHBOARD.starts_with("https://notebooklm.google.com"));
    assert!(rpc_ids::URL_BATCH_EXECUTE.contains("/batchexecute"));
    assert!(rpc_ids::URL_CHAT_STREAM.contains("GenerateFreeFormStreamed"));
    assert!(rpc_ids::URL_UPLOAD.contains("/upload/"));
}

#[test]
fn artifact_kind_round_trips_through_wire_id() {
    use notebooklm::ArtifactKind;
    for k in [
        ArtifactKind::Audio,
        ArtifactKind::Report,
        ArtifactKind::Video,
        ArtifactKind::Quiz,
        ArtifactKind::MindMap,
        ArtifactKind::Flashcards,
        ArtifactKind::Infographic,
        ArtifactKind::SlideDeck,
        ArtifactKind::DataTable,
    ] {
        let wire = k.as_wire_id();
        let parsed = ArtifactKind::from_wire_id(wire).expect("round-trip");
        assert_eq!(k, parsed);
    }
}

#[test]
fn artifact_kind_parses_string_aliases() {
    use notebooklm::ArtifactKind;
    use std::str::FromStr;
    assert_eq!(ArtifactKind::from_str("audio").unwrap(), ArtifactKind::Audio);
    assert_eq!(ArtifactKind::from_str("Podcast").unwrap(), ArtifactKind::Audio);
    assert_eq!(ArtifactKind::from_str("slides").unwrap(), ArtifactKind::SlideDeck);
    assert_eq!(ArtifactKind::from_str("CSV").unwrap(), ArtifactKind::DataTable);
    assert!(ArtifactKind::from_str("never-heard-of-it").is_err());
}
