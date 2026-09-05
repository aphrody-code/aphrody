// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Typed errors for the local-embeddings surface.

use std::path::PathBuf;

/// Result alias for the crate's public surface.
pub type Result<T> = std::result::Result<T, EmbedError>;

/// Errors surfaced by the local embeddings engine.
///
/// The set is intentionally small and stable: callers can match on
/// [`EmbedError::Unsupported`] to detect a build/target where local
/// embeddings are unavailable (e.g. wasm32, or a build without the
/// `embeddings` feature) and fall back to a remote provider, without
/// pulling in `fastembed`'s `anyhow`-flavoured error type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EmbedError {
    /// Local embeddings are not available on this build/target.
    ///
    /// Returned by every public entry point when the crate is compiled
    /// without the `embeddings` feature, or for `wasm32-*` targets where the
    /// ONNX Runtime backend cannot link. The message names the concrete
    /// reason so the caller can log it and degrade gracefully.
    #[error("local embeddings unavailable: {0}")]
    Unsupported(&'static str),

    /// The model cache directory could not be resolved or created.
    #[error("could not prepare model cache directory {path}: {source}")]
    CacheDir {
        /// The directory we tried to resolve/create.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// The home directory could not be resolved (HOME / USERPROFILE unset).
    #[error("could not resolve a home directory for the model cache (HOME / USERPROFILE unset)")]
    NoHome,

    /// Model initialisation failed (download, ONNX session build, tokenizer).
    ///
    /// Wraps the underlying engine error as a string so the public type does
    /// not leak the `fastembed`/`anyhow` dependency across the API boundary.
    #[error("embedding model initialisation failed: {0}")]
    ModelInit(String),

    /// Inference failed while embedding the provided texts.
    #[error("embedding inference failed: {0}")]
    Inference(String),

    /// The model returned a vector whose dimension did not match the model's
    /// advertised dimension. Defensive: should never happen with a healthy
    /// model, but we assert it so downstream vector stores (`LanceDB` fixed-size
    /// list columns) never silently get a ragged batch.
    #[error("embedding dimension mismatch: model advertises {expected}, got {actual}")]
    DimensionMismatch {
        /// Dimension the model's metadata advertises.
        expected: usize,
        /// Dimension actually produced for at least one vector.
        actual: usize,
    },
}
