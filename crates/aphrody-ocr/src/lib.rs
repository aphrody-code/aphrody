// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Image-to-text for aphrody: drive a local vision-language model over one
//! image or a whole directory, and turn its output into markdown.
//!
//! This is the task layer of the local-inference toolbox. `aphrody-models`
//! puts verified weights on disk, `aphrody-infer` decides what runs them, and
//! this crate turns "a folder of scanned plates" into "text, or an explicit
//! nothing".
//!
//! # The distinction that matters
//!
//! [`PageText::None`] is a first-class result, not an error. A vision model
//! shown a full-page illustration will happily describe it; recording that
//! description as a transcription silently corrupts a corpus. So a page whose
//! decoded blocks are all pictures or page furniture yields `None`, and a
//! caller can forward that as an explicit null rather than as prose.
//!
//! # Backend
//!
//! GGUF vision models run through llama.cpp's `llama-mtmd-cli`, resolved by
//! [`aphrody_infer::llama`]. aphrody spawns it rather than linking llama.cpp
//! (see that module for why).

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]

/// Portable, versioned result records shared with non-native consumers.
pub use aphrody_ocr_core as ocr_core;
/// Quality audit over a batch of transcriptions.
pub use aphrody_ocr_core::audit;
/// `DocTags` parsing: the serialisation document VLMs emit.
pub use aphrody_ocr_core::doctags;
pub use aphrody_ocr_core::{Block, Document};
/// Deterministic PP-OCR ONNX backend for local text geometry and CTC decoding.
pub use aphrody_ocr_onnx as onnx;
pub use aphrody_ocr_onnx::{
    DetectedRegion, DetectionMap, PpOcr, PpOcrDetector, PpOcrRecognizer, RecognisedRegion,
};
pub use aphrody_ocr_vlm::{
    OcrError, OcrOptions, PageResult, PageText, Result, ServerRunner, VlmRunner, list_images_sorted,
};
