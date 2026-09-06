// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Portable contracts shared by Aphrody OCR backends and consumers.
//!
//! This crate owns serialised facts only: page identity, provenance, text
//! blocks, quality outcomes and attempts. It deliberately owns no image codec,
//! ONNX Runtime session or llama.cpp process, so parsers and schema consumers
//! can compile for every Aphrody target.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]

/// Quality audit for plain-text and structured OCR output.
pub mod audit;
/// `DocTags` parsing and deterministic cleanup for document VLM output.
pub mod doctags;
/// Versioned OCR result and JSONL compatibility types.
pub mod result;

pub use doctags::{Block, Document};
pub use result::{
    Attempt, AttemptStatus, ImageIdentity, OcrBlock, OcrResult, OcrStatus, Polygon, Quality,
    RESULT_SCHEMA_V2, RunProvenance,
};
