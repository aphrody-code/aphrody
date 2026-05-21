// SPDX-License-Identifier: Apache-2.0
//! Native Magika file-type classification (Google's deep-learning content
//! detector), wired through the official [`magika`] crate.
//!
//! This replaces the Python `magika` shell-out used during forensic passes
//! with an in-process, pure-Rust call path (the model itself runs on the
//! ONNX Runtime via the [`ort`] crate, linked into the final binary).
//!
//! ## Why it is feature-gated
//!
//! The [`magika`] crate transitively depends on `ort` (ONNX Runtime). Pulling
//! a usable backend means enabling `ort`'s `download-binaries` feature in the
//! final binary, which fetches a prebuilt runtime **at build time** — this is
//! not compatible with the hermetic-offline workspace build (`cargo
//! ci-offline`) nor with `wasm32` targets. The whole module therefore lives
//! behind the opt-in `magika` cargo feature and is host-only:
//!
//! ```bash
//! cargo build -p aphrody --features magika
//! aphrody re classify path/to/file
//! ```
//!
//! When the feature is off, the CLI sub-command degrades to a clear error
//! pointing at the rebuild flag — no silent fallback to a stale Python tool.
//!
//! ## Output
//!
//! [`classify_bytes`] / [`Classifier::classify_path`] return a
//! [`MagikaClass`], a fully-owned, `serde`-serializable projection of
//! `magika`'s `FileType` + `TypeInfo` (label, MIME, group, description,
//! extensions, text flag, score, and the overwrite reason when the model's
//! raw inference was overridden by a confidence threshold or the canonical
//! overwrite map).

use serde::Serialize;

/// A single Magika classification result, owned and serializable.
///
/// Mirrors the fields exposed by `magika`'s `FileType` + `TypeInfo`, copied
/// out of the crate's `'static` references so the value can outlive the
/// [`Classifier`] / model session and be serialized to JSON directly.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MagikaClass {
    /// Stable label identifying the content type, e.g. `"rust"`, `"pebin"`,
    /// `"json"`, `"sqlite"`. This is the canonical Magika label.
    pub label: String,
    /// MIME type, e.g. `"text/x-rust"`, `"application/x-executable"`.
    pub mime_type: String,
    /// Coarse group, e.g. `"code"`, `"executable"`, `"document"`.
    pub group: String,
    /// Human-readable description of the content type.
    pub description: String,
    /// Common file extensions associated with the content type (no leading
    /// dot), e.g. `["rs"]`.
    pub extensions: Vec<String>,
    /// Whether Magika considers this a text type (vs. binary).
    pub is_text: bool,
    /// Model confidence in `[0, 1]`. `1.0` when the type was decided by a
    /// deterministic rule (directory, symlink, small/empty content) rather
    /// than the neural model.
    pub score: f32,
    /// How the type was decided: `"inferred"` (neural model),
    /// `"ruled"` (deterministic rule), `"directory"`, or `"symlink"`.
    pub kind: &'static str,
    /// When the model's raw inference was overridden, why: `"low-confidence"`
    /// (score below the per-type threshold) or `"overwrite-map"` (the raw
    /// label is not the canonical one). `None` when the inferred type stands
    /// or when the type was ruled rather than inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwrite_reason: Option<&'static str>,
}

impl MagikaClass {
    fn from_file_type(ft: &magika::FileType) -> Self {
        use magika::{FileType, OverwriteReason};

        let info = ft.info();
        let (kind, overwrite_reason) = match ft {
            FileType::Directory => ("directory", None),
            FileType::Symlink => ("symlink", None),
            FileType::Ruled(_) => ("ruled", None),
            FileType::Inferred(inferred) => {
                let reason = match &inferred.content_type {
                    Some((_, OverwriteReason::LowConfidence)) => Some("low-confidence"),
                    Some((_, OverwriteReason::OverwriteMap)) => Some("overwrite-map"),
                    None => None,
                };
                ("inferred", reason)
            },
        };

        Self {
            label: info.label.to_owned(),
            mime_type: info.mime_type.to_owned(),
            group: info.group.to_owned(),
            description: info.description.to_owned(),
            extensions: info.extensions.iter().map(|s| (*s).to_owned()).collect(),
            is_text: info.is_text,
            score: ft.score(),
            kind,
            overwrite_reason,
        }
    }
}

/// Errors surfaced by the Magika classification path.
#[derive(Debug, thiserror::Error)]
pub enum MagikaError {
    /// The ONNX model session failed to initialize or run inference.
    #[error("magika session error: {0}")]
    Session(#[from] magika::Error),
}

/// A reusable Magika classifier wrapping a single loaded model session.
///
/// Construction loads the embedded ONNX model and is comparatively expensive,
/// so prefer building one [`Classifier`] and reusing it across many files
/// (e.g. when sweeping a directory) over the one-shot [`classify_bytes`] /
/// [`classify_path`] helpers.
pub struct Classifier {
    session: magika::Session,
}

impl Classifier {
    /// Build a classifier, loading the embedded Magika model.
    ///
    /// # Errors
    ///
    /// Returns [`MagikaError::Session`] if the ONNX Runtime cannot initialize
    /// the model (e.g. the runtime shared library is unavailable).
    pub fn new() -> Result<Self, MagikaError> {
        Ok(Self { session: magika::Session::new()? })
    }

    /// Classify a file by path.
    ///
    /// Magika applies deterministic rules first (directory, symlink, empty /
    /// tiny content) and only runs the neural model on regular files large
    /// enough to feature-extract.
    ///
    /// # Errors
    ///
    /// Returns [`MagikaError::Session`] on I/O or inference failure.
    pub fn classify_path<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<MagikaClass, MagikaError> {
        let ft = self.session.identify_file_sync(path)?;
        Ok(MagikaClass::from_file_type(&ft))
    }

    /// Classify raw bytes already held in memory.
    ///
    /// # Errors
    ///
    /// Returns [`MagikaError::Session`] on inference failure.
    pub fn classify_bytes(&mut self, content: &[u8]) -> Result<MagikaClass, MagikaError> {
        let ft = self.session.identify_content_sync(content)?;
        Ok(MagikaClass::from_file_type(&ft))
    }
}

/// One-shot classification of a file by path.
///
/// Builds a throwaway [`Classifier`] and classifies a single file. For more
/// than one file, build a [`Classifier`] once and reuse it.
///
/// # Errors
///
/// Returns [`MagikaError`] if the model fails to load or inference fails.
pub fn classify_path<P: AsRef<std::path::Path>>(path: P) -> Result<MagikaClass, MagikaError> {
    Classifier::new()?.classify_path(path)
}

/// One-shot classification of raw bytes.
///
/// # Errors
///
/// Returns [`MagikaError`] if the model fails to load or inference fails.
pub fn classify_bytes(content: &[u8]) -> Result<MagikaClass, MagikaError> {
    Classifier::new()?.classify_bytes(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The model session requires the ONNX Runtime to be available at runtime,
    // which is provided by the final binary's `ort` feature, not by the test
    // harness here. These tests therefore exercise the pure projection logic
    // and the public surface without forcing a model load in unit tests.

    #[test]
    fn magika_class_is_serializable_and_skips_none_reason() {
        let class = MagikaClass {
            label: "rust".to_owned(),
            mime_type: "text/x-rust".to_owned(),
            group: "code".to_owned(),
            description: "Rust source".to_owned(),
            extensions: vec!["rs".to_owned()],
            is_text: true,
            score: 1.0,
            kind: "ruled",
            overwrite_reason: None,
        };
        let json = serde_json::to_string(&class).unwrap();
        assert!(json.contains("\"label\":\"rust\""));
        assert!(json.contains("\"is_text\":true"));
        // overwrite_reason is None => omitted from the JSON.
        assert!(!json.contains("overwrite_reason"));
    }

    #[test]
    fn magika_class_serializes_overwrite_reason_when_present() {
        let class = MagikaClass {
            label: "txt".to_owned(),
            mime_type: "text/plain".to_owned(),
            group: "text".to_owned(),
            description: "Generic text".to_owned(),
            extensions: vec!["txt".to_owned()],
            is_text: true,
            score: 0.42,
            kind: "inferred",
            overwrite_reason: Some("low-confidence"),
        };
        let json = serde_json::to_string(&class).unwrap();
        assert!(json.contains("\"overwrite_reason\":\"low-confidence\""));
        assert!(json.contains("\"kind\":\"inferred\""));
    }
}
