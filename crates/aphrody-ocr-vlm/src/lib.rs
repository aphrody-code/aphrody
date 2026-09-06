// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! llama.cpp-backed VLM OCR for Aphrody.
//!
//! This crate owns only generative vision-language model execution. Shared
//! document cleanup, audit rules and serialised result contracts belong to
//! `aphrody-ocr-core`; deterministic ONNX OCR belongs to a separate backend.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]

/// Resident-model backend: one `llama-server` for a whole batch.
pub mod server;
/// Running a VLM over individual images.
pub mod vlm;

pub use server::ServerRunner;
pub use vlm::{OcrOptions, PageResult, PageText, VlmRunner, list_images_sorted};

/// Error returned by the llama.cpp OCR backends.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OcrError {
    /// No llama.cpp multimodal binary could be found.
    #[error(
        "llama.cpp `llama-mtmd-cli` not found — unpack a release under \
         ~/.aphrody/runtimes/llama-<build>/ or set $APHRODY_LLAMA_DIR"
    )]
    NoRunner,

    /// The catalog entry is missing an artefact the runner needs.
    #[error("{0}")]
    Infer(#[from] aphrody_infer::InferError),

    /// A model referenced by the pipeline is not installed.
    #[error("{0}")]
    Model(#[from] aphrody_models::ModelError),

    /// The model process failed.
    #[error("`{command}` failed ({status}): {stderr}")]
    Process {
        /// The binary that was invoked.
        command: String,
        /// Its exit status, rendered.
        status: String,
        /// Tail of its standard error.
        stderr: String,
    },

    /// A filesystem operation failed.
    #[error("i/o error on `{path}`: {source}")]
    Io {
        /// The path involved.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
}

/// Result alias for this crate.
pub type Result<T> = core::result::Result<T, OcrError>;
