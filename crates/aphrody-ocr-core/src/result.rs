// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Stable, append-only result records for OCR batch outputs.

use std::path::PathBuf;

/// Identifier emitted by every newly produced result record.
pub const RESULT_SCHEMA_V2: &str = "aphrody.ocr.result/v2";

/// A quadrilateral in original image pixel coordinates, clockwise from the
/// top-left corner after normalisation.
pub type Polygon = [[f32; 2]; 4];

/// The semantic outcome of processing one page.
///
/// The states are deliberately more precise than the legacy `none` result:
/// corpus consumers must never treat an unreadable page as a request to erase
/// an existing transcription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OcrStatus {
    /// Text was extracted and can be considered by a consumer after auditing.
    Text,
    /// The backend completed and found no credible text region.
    NoText,
    /// Text likely exists but could not be read with sufficient confidence.
    Unreadable,
    /// A completed result conflicts with a quality rule and needs review.
    NeedsReview,
    /// The input or backend failed before an OCR result could be produced.
    ProcessingError,
}

/// Execution state of a single backend attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttemptStatus {
    /// The backend returned an OCR outcome.
    Completed,
    /// The backend timed out.
    TimedOut,
    /// The backend failed before an outcome was available.
    Failed,
}

/// Identity and diagnostic properties of the input image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImageIdentity {
    /// Path recorded for backwards-compatible resume behavior.
    pub path: PathBuf,
    /// Digest of the exact bytes read, when the caller computed one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Detected media type rather than an extension-derived claim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

impl ImageIdentity {
    /// Build the minimum safe identity from a path.
    #[must_use]
    pub fn from_path(path: PathBuf) -> Self {
        Self { path, sha256: None, media_type: None }
    }
}

/// Provenance needed to reproduce or safely compare an OCR run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunProvenance {
    /// Identifier of the selected model catalog entry.
    pub model_id: String,
    /// Backend that performed the attempt, such as `onnx-runtime` or
    /// `llama-cpp`.
    pub backend: String,
    /// Runtime provider that actually executed the model, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Digest of the selected model/configuration set, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_digest: Option<String>,
    /// Digest of a prompt, used only by generative OCR backends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_digest: Option<String>,
}

/// Aggregate and per-block quality information.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Quality {
    /// Mean recognition confidence in the inclusive range `[0, 1]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean_confidence: Option<f32>,
    /// Human- and machine-readable reasons that prevented automatic acceptance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

impl Default for Quality {
    fn default() -> Self {
        Self { mean_confidence: None, reasons: Vec::new() }
    }
}

/// One recognised region in reading order.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OcrBlock {
    /// Text recognised in this region.
    pub text: String,
    /// Region geometry in original-pixel coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polygon: Option<Polygon>,
    /// Recognition confidence for this region.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Optional structural role, e.g. `title`, `table` or `text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Evidence from one backend invocation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Attempt {
    /// Backend provenance for this invocation.
    pub run: RunProvenance,
    /// Whether the invocation completed, timed out or failed.
    pub status: AttemptStatus,
    /// Wall-clock time consumed by the invocation.
    pub elapsed_ms: u128,
    /// Backend result quality, if it completed.
    #[serde(default)]
    pub quality: Quality,
    /// Error retained for a failed attempt without misclassifying it as no text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A schema-versioned OCR page result suitable for JSONL.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OcrResult {
    /// Schema discriminator. New records use [`RESULT_SCHEMA_V2`].
    pub schema: String,
    /// Stable page identity within the caller's batch or corpus.
    pub page_id: String,
    /// Input identity and diagnostic properties.
    pub image: ImageIdentity,
    /// Backend attempts in execution order.
    #[serde(default)]
    pub attempts: Vec<Attempt>,
    /// Final semantic status of the page.
    pub status: OcrStatus,
    /// Markdown produced by a layout-aware backend, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Ordered recognised regions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<OcrBlock>,
    /// Final quality gate and reasons.
    #[serde(default)]
    pub quality: Quality,
    /// Raw model output retained when the caller requested auditability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

impl OcrResult {
    /// Create a record with no recognised text.
    #[must_use]
    pub fn no_text(page_id: String, image: ImageIdentity) -> Self {
        Self {
            schema: RESULT_SCHEMA_V2.to_owned(),
            page_id,
            image,
            attempts: Vec::new(),
            status: OcrStatus::NoText,
            markdown: None,
            blocks: Vec::new(),
            quality: Quality::default(),
            raw: None,
        }
    }

    /// Return whether the final state may be considered for automatic deposit.
    #[must_use]
    pub const fn is_depositable(&self) -> bool {
        matches!(self.status, OcrStatus::Text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_text_is_not_depositable() {
        let result =
            OcrResult::no_text("page-1".into(), ImageIdentity::from_path("page-1.jpg".into()));
        assert_eq!(result.schema, RESULT_SCHEMA_V2);
        assert_eq!(result.status, OcrStatus::NoText);
        assert!(!result.is_depositable());
    }

    #[test]
    fn result_round_trips_without_eliding_status() {
        let result = OcrResult {
            schema: RESULT_SCHEMA_V2.into(),
            page_id: "lot-001/1.jpg".into(),
            image: ImageIdentity::from_path("1.jpg".into()),
            attempts: Vec::new(),
            status: OcrStatus::NeedsReview,
            markdown: Some("uncertain".into()),
            blocks: Vec::new(),
            quality: Quality {
                mean_confidence: Some(0.42),
                reasons: vec!["low-confidence".into()],
            },
            raw: None,
        };
        let encoded = serde_json::to_string(&result).unwrap();
        let decoded: OcrResult = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, result);
        assert!(!decoded.is_depositable());
    }
}
