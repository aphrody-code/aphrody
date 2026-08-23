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

/// Quality audit over a batch of transcriptions.
pub mod audit;
/// `DocTags` parsing: the serialisation document VLMs emit.
pub mod doctags;
/// Japanese script normalisation, dictionary-free and wasm-safe.
pub mod kana;
/// Closed-lexicon correction: measured wrong forms, and only those.
pub mod lexique;
/// A minimal HTTP/1.1 client for the loopback llama-server.
#[cfg(not(target_arch = "wasm32"))]
pub mod http;
/// Resident-model backend: one `llama-server` for a whole batch.
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
/// Running a vision-language model over images.
#[cfg(not(target_arch = "wasm32"))]
pub mod vlm;
/// Speech-balloon detection, for comics pages a document model reads as blank.
#[cfg(feature = "bulles")]
pub mod bulles;

pub use doctags::{Block, Document};

// Ces deux-là pilotent un processus llama.cpp : ils n'existent pas pour wasm,
// et la ré-exportation doit être gardée comme le module l'est. Elle ne l'était
// pas, ce qui faisait échouer `cargo check --target wasm32-unknown-unknown` sur
// tout le crate — alors que le parsing, la normalisation et l'audit, eux,
// compilent pour wasm sans rien demander.
#[cfg(not(target_arch = "wasm32"))]
pub use server::ServerRunner;
#[cfg(not(target_arch = "wasm32"))]
pub use vlm::{OcrOptions, PageResult, PageText, VlmRunner, list_images_sorted};

/// The crate's error type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OcrError {
    /// No llama.cpp multimodal binary could be found.
    #[error(
        "llama.cpp `llama-mtmd-cli` not found — unpack a release under ~/.aphrody/runtimes/llama-<build>/ or set $APHRODY_LLAMA_DIR"
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

#[cfg(feature = "japanese")]
pub mod japonais;
