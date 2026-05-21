// SPDX-License-Identifier: Apache-2.0
//! Strongly-typed data model for the NotebookLM RPC surface.
//!
//! Each struct here is the Rust mirror of a `wrb.fr` envelope value the Boq
//! parser hands back from a successful `batchexecute` call. The shapes were
//! derived from the icebear0828/notebooklm-client + teng-lin/notebooklm-py
//! references — both reverse-engineered against the live web UI.

use serde::{Deserialize, Serialize};

/// A NotebookLM workspace ("notebook"). Returned by `wXbhsf` (list) and
/// indirectly by `rLM1Ne` (detail) which fills in `sources`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notebook {
    /// Stable opaque identifier (e.g. `"abcdef12-3456-7890-abcd-ef1234567890"`).
    pub id: String,
    /// User-visible title; defaults to `""` for fresh notebooks.
    pub title: String,
    /// Number of attached sources. `None` when listing returns a compact row
    /// without the count column.
    pub source_count: Option<u32>,
    /// Created-at + updated-at expressed as `[seconds, nanos]` UNIX tuples;
    /// the upstream wire format uses `int64` pairs here.
    pub created_at: Option<(i64, i32)>,
    pub updated_at: Option<(i64, i32)>,
}

/// Kind discriminator for a `Source` row.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Remote URL (HTML page, PDF link, …). Added via `izAoDd`.
    Url,
    /// Inline plain-text snippet pasted by the user.
    Text,
    /// File upload (PDF/DOCX/MP3/…). Uses the Scotty resumable upload protocol.
    File,
    /// YouTube video URL — NotebookLM grabs transcripts on its side.
    YouTube,
}

/// A source attached to a notebook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub kind: SourceKind,
    /// Indexed word count; `None` while ingestion is still pending.
    pub word_count: Option<u32>,
    /// Final HTTP status if the upstream was a URL fetch (200 / 404 / …).
    pub status_code: Option<u16>,
    /// Originating URL for `Url` / `YouTube` sources.
    pub url: Option<String>,
}

/// Plain "thread" record returned by `hPTbtc`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatThread {
    pub id: String,
    pub notebook_id: String,
}

/// Free-form text response from the chat stream parser.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatReply {
    pub text: String,
    pub thread_id: String,
    pub response_id: Option<String>,
}

/// Discriminator the GENERATE_ARTIFACT RPC uses to pick a workflow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Audio overview (podcast-style narration). Wire id `1`.
    Audio,
    /// Long-form Markdown report (briefing doc / study guide / blog post). Wire id `2`.
    Report,
    /// Video overview (Veo-powered). Wire id `3`.
    Video,
    /// Multiple-choice quiz. Wire id `4`.
    Quiz,
    /// Mind map (SVG/HTML interactive). Wire id `5`.
    MindMap,
    /// Flashcards deck. Wire id `6`.
    Flashcards,
    /// Static infographic image. Wire id `7`.
    Infographic,
    /// Slide deck (PPTX). Wire id `8`.
    SlideDeck,
    /// Tabular data export (CSV). Wire id `9`.
    DataTable,
}

impl ArtifactKind {
    /// Convert to the wire discriminant the Boq router expects.
    pub fn as_wire_id(self) -> u32 {
        use crate::rpc_ids::{
            ARTIFACT_TYPE_AUDIO, ARTIFACT_TYPE_DATA_TABLE, ARTIFACT_TYPE_FLASHCARDS,
            ARTIFACT_TYPE_INFOGRAPHIC, ARTIFACT_TYPE_MIND_MAP, ARTIFACT_TYPE_QUIZ,
            ARTIFACT_TYPE_REPORT, ARTIFACT_TYPE_SLIDE_DECK, ARTIFACT_TYPE_VIDEO,
        };
        match self {
            ArtifactKind::Audio => ARTIFACT_TYPE_AUDIO,
            ArtifactKind::Report => ARTIFACT_TYPE_REPORT,
            ArtifactKind::Video => ARTIFACT_TYPE_VIDEO,
            ArtifactKind::Quiz => ARTIFACT_TYPE_QUIZ,
            ArtifactKind::MindMap => ARTIFACT_TYPE_MIND_MAP,
            ArtifactKind::Flashcards => ARTIFACT_TYPE_FLASHCARDS,
            ArtifactKind::Infographic => ARTIFACT_TYPE_INFOGRAPHIC,
            ArtifactKind::SlideDeck => ARTIFACT_TYPE_SLIDE_DECK,
            ArtifactKind::DataTable => ARTIFACT_TYPE_DATA_TABLE,
        }
    }

    /// Parse from the wire discriminant; unknown ids fall back to `Report`.
    pub fn from_wire_id(wire: u32) -> Option<Self> {
        use crate::rpc_ids::{
            ARTIFACT_TYPE_AUDIO, ARTIFACT_TYPE_DATA_TABLE, ARTIFACT_TYPE_FLASHCARDS,
            ARTIFACT_TYPE_INFOGRAPHIC, ARTIFACT_TYPE_MIND_MAP, ARTIFACT_TYPE_QUIZ,
            ARTIFACT_TYPE_REPORT, ARTIFACT_TYPE_SLIDE_DECK, ARTIFACT_TYPE_VIDEO,
        };
        match wire {
            x if x == ARTIFACT_TYPE_AUDIO => Some(Self::Audio),
            x if x == ARTIFACT_TYPE_REPORT => Some(Self::Report),
            x if x == ARTIFACT_TYPE_VIDEO => Some(Self::Video),
            x if x == ARTIFACT_TYPE_QUIZ => Some(Self::Quiz),
            x if x == ARTIFACT_TYPE_MIND_MAP => Some(Self::MindMap),
            x if x == ARTIFACT_TYPE_FLASHCARDS => Some(Self::Flashcards),
            x if x == ARTIFACT_TYPE_INFOGRAPHIC => Some(Self::Infographic),
            x if x == ARTIFACT_TYPE_SLIDE_DECK => Some(Self::SlideDeck),
            x if x == ARTIFACT_TYPE_DATA_TABLE => Some(Self::DataTable),
            _ => None,
        }
    }
}

impl std::str::FromStr for ArtifactKind {
    type Err = crate::error::NotebookError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "audio" | "podcast" | "overview" => Ok(Self::Audio),
            "report" | "doc" | "briefing" => Ok(Self::Report),
            "video" | "veo" => Ok(Self::Video),
            "quiz" => Ok(Self::Quiz),
            "mind_map" | "mindmap" | "map" => Ok(Self::MindMap),
            "flashcards" | "cards" => Ok(Self::Flashcards),
            "infographic" | "graphic" => Ok(Self::Infographic),
            "slide_deck" | "slides" | "deck" | "pptx" => Ok(Self::SlideDeck),
            "data_table" | "table" | "csv" => Ok(Self::DataTable),
            other => Err(crate::error::NotebookError::Parse(format!(
                "unknown artifact kind: {other}"
            ))),
        }
    }
}

/// Artifact row returned by `GET_ARTIFACTS_FILTERED`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    pub id: String,
    pub title: String,
    pub kind: ArtifactKind,
    /// Direct media download URL (resolved once the artifact reaches the
    /// `ARTIFACT_STATUS_COMPLETE` state). `None` while pending.
    pub download_url: Option<String>,
    /// Optional HLS / DASH stream URL (set for audio + video artifacts).
    pub stream_url: Option<String>,
    /// Duration in seconds when the artifact carries playable media.
    pub duration_seconds: Option<u32>,
    /// Source ids the artifact was generated from.
    pub source_ids: Vec<String>,
}

/// One research hit returned by the deep / web research pollers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchHit {
    pub url: String,
    pub title: String,
    pub description: String,
}

/// Aggregated quota + plan info for the active Google account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountInfo {
    /// 1=free, 6=plus/ultra, …
    pub plan_type: u32,
    pub notebook_limit: u32,
    pub source_limit: u32,
    pub source_word_limit: u64,
    pub is_plus: bool,
}
