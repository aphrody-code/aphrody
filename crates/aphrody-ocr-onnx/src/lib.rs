// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Deterministic PP-OCR ONNX recognition primitives.
//!
//! The PP-OCRv5 recognizer has 18,383 configured glyphs plus CTC blank and
//! space classes. The mapping is loaded from the downloaded export, never
//! embedded approximately.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]

mod ctc;
mod detector;
mod error;
mod pipeline;
mod recognizer;

pub use ctc::{CtcDecoder, DecodedText};
pub use detector::{DetectedRegion, DetectionMap, PpOcrDetector};
pub use error::{OnnxOcrError, Result};
pub use pipeline::{PpOcr, RecognisedRegion};
pub use recognizer::{PpOcrRecognizer, Recognition};
