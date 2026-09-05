// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Typed errors for the local inference backend.

use std::path::PathBuf;

/// Result alias for every fallible operation in this crate.
pub type Result<T> = core::result::Result<T, InferError>;

/// Everything that can go wrong loading or running a local model.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InferError {
    /// The crate was built without the `onnx` feature, so no ONNX backend
    /// exists in this binary.
    #[error(
        "`{0}` needs the `onnx` feature (ONNX Runtime). Rebuild with: cargo build -p aphrody --features infer"
    )]
    BackendUnavailable(&'static str),

    /// No home directory could be resolved for `~/.aphrody/runtimes`.
    #[error("cannot resolve the aphrody state directory: set $APHRODY_HOME")]
    NoStateDir,

    /// The ONNX Runtime shared library could not be loaded.
    #[error("cannot load the ONNX Runtime from {}: {reason}", .path.as_ref().map_or_else(|| "the system search path".to_owned(), |p| p.display().to_string()))]
    RuntimeLoad {
        /// Path that was attempted, when one was named.
        path: Option<PathBuf>,
        /// Underlying loader message.
        reason: String,
    },

    /// A session could not be built for a model file.
    #[error("cannot load model `{path}`: {reason}")]
    SessionBuild {
        /// The model file.
        path: PathBuf,
        /// Underlying ONNX Runtime message.
        reason: String,
    },

    /// A model needed for this operation is not installed.
    #[error("{0}")]
    Model(#[from] aphrody_models::ModelError),

    /// A catalog entry does not declare the role a pipeline asked for.
    #[error("catalog entry `{entry}` has no `{role}` artefact")]
    MissingRole {
        /// The catalog id.
        entry: String,
        /// The role that was requested.
        role: String,
    },

    /// Inference itself failed.
    #[error("inference failed: {0}")]
    Run(String),
}
