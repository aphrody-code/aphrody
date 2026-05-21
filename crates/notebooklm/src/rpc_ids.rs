// SPDX-License-Identifier: Apache-2.0
//! NotebookLM batchexecute RPC identifiers.
//!
//! Captured from the live `notebooklm.google.com` web UI traffic and ported
//! verbatim from the upstream icebear0828/notebooklm-client `rpc-ids.ts`
//! catalogue. Each constant is a stable identifier the Boq router uses to
//! dispatch a single RPC method; the inner payload schema is documented in
//! `crate::notebooks`, `crate::sources`, `crate::chat`, `crate::artifacts`
//! and `crate::research`.

// ── Notebooks ───────────────────────────────────────────────────────────────
pub const CREATE_NOTEBOOK: &str = "CCqFvf";
pub const LIST_NOTEBOOKS: &str = "wXbhsf";
pub const GET_NOTEBOOK: &str = "rLM1Ne";
pub const RENAME_NOTEBOOK: &str = "s0tc2d";
pub const DELETE_NOTEBOOK: &str = "WWINqb";
pub const REMOVE_RECENTLY_VIEWED: &str = "fejl7e";

// ── Sources ─────────────────────────────────────────────────────────────────
pub const ADD_SOURCE: &str = "izAoDd";
pub const ADD_SOURCE_FILE: &str = "o4cbdc";
pub const GET_SOURCE_CONTENT: &str = "hizoJc";
pub const GET_SOURCE_SUMMARY: &str = "tr032e";
pub const DELETE_SOURCE: &str = "tGMBJ";
pub const REFRESH_SOURCE: &str = "FLmJqe";
pub const UPDATE_SOURCE: &str = "b7Wfje";

// ── Research ────────────────────────────────────────────────────────────────
pub const CREATE_WEB_SEARCH: &str = "Ljjv0c";
pub const CREATE_DEEP_RESEARCH: &str = "QA9ei";
pub const POLL_RESEARCH: &str = "e3bVqc";
pub const IMPORT_RESEARCH: &str = "LBwxtb";

// ── Artifacts ───────────────────────────────────────────────────────────────
pub const GENERATE_ARTIFACT: &str = "R7cb6c";
pub const GET_ARTIFACTS_FILTERED: &str = "gArtLc";
pub const DELETE_ARTIFACT: &str = "V5N4be";
pub const RENAME_ARTIFACT: &str = "rc3d8d";
pub const GET_INTERACTIVE_HTML: &str = "v9rmvd";
pub const EXPORT_ARTIFACT: &str = "Krh3pd";
pub const SHARE_ARTIFACT: &str = "RGP97b";
pub const GET_STUDIO_CONFIG: &str = "sqTeoe";

// ── Notes & Mind Maps ──────────────────────────────────────────────────────
pub const CREATE_NOTE: &str = "CYK0Xb";
pub const GET_NOTES: &str = "cFji9";
pub const UPDATE_NOTE: &str = "cYAfTb";
pub const DELETE_NOTE: &str = "AH0mwd";

// ── Chat ────────────────────────────────────────────────────────────────────
pub const LIST_CHAT_THREADS: &str = "hPTbtc";
pub const DELETE_CHAT_THREAD: &str = "J7Gthc";

// ── Sharing ─────────────────────────────────────────────────────────────────
pub const GET_SHARE_STATUS: &str = "JFMDGd";
pub const SHARE_NOTEBOOK: &str = "QDyure";

// ── Settings / Account ─────────────────────────────────────────────────────
pub const GET_ACCOUNT_INFO: &str = "ZwVcOc";
pub const SET_USER_SETTINGS: &str = "hT54vc";
pub const GET_NOTEBOOK_SUMMARY: &str = "VfAZjd";
pub const GET_RECOMMENDED_TOPICS: &str = "otmP3b";
pub const GET_UI_CONFIG: &str = "ozz5Z";
pub const REPORT_PLAY_PROGRESS: &str = "Fxmvse";

// ── Artifact type discriminants ────────────────────────────────────────────
//
// These integers are the `type` field the GENERATE_ARTIFACT RPC payload sets;
// they also surface in the `gArtLc` listing rows so callers can filter.
pub const ARTIFACT_TYPE_AUDIO: u32 = 1;
pub const ARTIFACT_TYPE_REPORT: u32 = 2;
pub const ARTIFACT_TYPE_VIDEO: u32 = 3;
pub const ARTIFACT_TYPE_QUIZ: u32 = 4;
pub const ARTIFACT_TYPE_MIND_MAP: u32 = 5;
pub const ARTIFACT_TYPE_FLASHCARDS: u32 = 6;
pub const ARTIFACT_TYPE_INFOGRAPHIC: u32 = 7;
pub const ARTIFACT_TYPE_SLIDE_DECK: u32 = 8;
pub const ARTIFACT_TYPE_DATA_TABLE: u32 = 9;

// ── URL constants ──────────────────────────────────────────────────────────
pub const URL_BASE: &str = "https://notebooklm.google.com";
pub const URL_DASHBOARD: &str = "https://notebooklm.google.com/";
pub const URL_BATCH_EXECUTE: &str =
    "https://notebooklm.google.com/_/LabsTailwindUi/data/batchexecute";
pub const URL_CHAT_STREAM: &str =
    "https://notebooklm.google.com/_/LabsTailwindUi/data/google.internal.labs.tailwind.orchestration.v1.LabsTailwindOrchestrationService/GenerateFreeFormStreamed";
pub const URL_UPLOAD: &str = "https://notebooklm.google.com/upload/_/";

/// Tagged tuple inserted as the first element of artifact + research payloads.
/// Mirrors `DEFAULT_USER_CONFIG` in `notebooklm-client`.
pub const DEFAULT_USER_CONFIG: &str =
    r#"[2,null,null,[1,null,null,null,null,null,null,null,null,null,[1]],[[2,1,3]]]"#;

/// Platform discriminator surfaced in every `f.req` envelope ("web").
pub const PLATFORM_WEB: &str = "[2]";
