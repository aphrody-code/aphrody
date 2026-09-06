// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

use std::path::PathBuf;

/// Errors from strict PP-OCR configuration decoding.
#[derive(Debug, thiserror::Error)]
pub enum OnnxOcrError {
    /// The export configuration could not be read.
    #[error("could not read PP-OCR export configuration {path}: {source}")]
    ReadConfig {
        /// Configuration path that could not be read.
        path: PathBuf,
        /// I/O failure returned by the filesystem.
        source: std::io::Error,
    },
    /// The configuration was not valid YAML.
    #[error("invalid PP-OCR export configuration {path}: {source}")]
    ParseConfig {
        /// Configuration path whose YAML was invalid.
        path: PathBuf,
        /// YAML parser failure.
        source: serde_yaml::Error,
    },
    /// The export lacks an explicit CTC character dictionary.
    #[error("PP-OCR export configuration {path} has no usable PostProcess.character_dict")]
    MissingCharacterDict {
        /// Configuration path without the required dictionary.
        path: PathBuf,
    },
    /// The model's actual class count disagrees with its decoder configuration.
    #[error("PP-OCR recognizer class count mismatch: model={model}, decoder={decoder}")]
    ClassCountMismatch {
        /// Number of classes reported by the ONNX tensor.
        model: usize,
        /// Number of classes implied by the export dictionary.
        decoder: usize,
    },
    /// The recognizer output was not a single `[batch, time, class]` tensor.
    #[error("unexpected PP-OCR recognizer output shape {shape:?}")]
    UnexpectedOutputShape {
        /// Shape reported by ONNX Runtime.
        shape: Vec<usize>,
    },
    /// An ONNX output could not be represented as contiguous CTC rows.
    #[error("PP-OCR recognizer output is not contiguous")]
    NonContiguousOutput,
    /// The ONNX recognizer did not expose exactly one required tensor slot.
    #[error("PP-OCR recognizer expected at least one {kind}, found {count}")]
    MissingTensorSlot {
        /// Whether the missing slot is an input or output.
        kind: &'static str,
        /// Number of slots reported by ONNX Runtime.
        count: usize,
    },
    /// A detected quadrilateral projected to an empty image crop.
    #[error("PP-OCR detector region projected to an empty crop")]
    EmptyCrop,
    /// The source image could not be decoded.
    #[error(transparent)]
    Image(#[from] image::ImageError),
    /// The selected catalogue entry lacks a required OCR artefact role.
    #[error("catalogue entry {entry} has no {role} artefact")]
    MissingCatalogRole {
        /// Catalogue entry identifier.
        entry: String,
        /// Required artefact role.
        role: String,
    },
    /// The shared model catalogue or local model store failed.
    #[error(transparent)]
    Model(#[from] aphrody_models::ModelError),
    /// ONNX Runtime rejected an input or an output extraction.
    #[error(transparent)]
    Ort(#[from] ort::Error),
    /// The shared inference layer could not load the selected session.
    #[error(transparent)]
    Infer(#[from] aphrody_infer::InferError),
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, OnnxOcrError>;
