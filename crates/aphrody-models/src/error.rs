// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Typed errors for the local-model store.

use std::path::PathBuf;

/// Result alias for every fallible operation in this crate.
pub type Result<T> = core::result::Result<T, ModelError>;

/// Everything that can go wrong while resolving, fetching, inspecting or
/// evicting a local model artefact.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModelError {
    /// A model reference string could not be parsed.
    #[error("invalid model reference `{input}`: {reason}")]
    BadRef {
        /// The offending input, verbatim.
        input: String,
        /// Human-readable explanation of the parse failure.
        reason: &'static str,
    },

    /// The aphrody state directory could not be resolved (no `$APHRODY_HOME`,
    /// no `$HOME`, no `%USERPROFILE%`, no platform home).
    #[error("cannot resolve the aphrody state directory: set $APHRODY_HOME")]
    NoStateDir,

    /// A filesystem operation failed. Carries the path for actionable output.
    #[error("i/o error on `{path}`: {source}")]
    Io {
        /// Path the failing operation targeted.
        path: PathBuf,
        /// Underlying OS error.
        #[source]
        source: std::io::Error,
    },

    /// The on-disk registry could not be decoded (corrupt or wrong schema).
    #[error("model registry at `{path}` is corrupt: {source}")]
    Registry {
        /// Path to `registry.json`.
        path: PathBuf,
        /// Underlying serde failure.
        #[source]
        source: serde_json::Error,
    },

    /// A lookup targeted a model that is not installed.
    #[error("model `{0}` is not installed")]
    NotInstalled(String),

    /// The requested catalog entry does not exist.
    #[error("unknown catalog id `{0}`")]
    UnknownCatalogId(String),

    /// A download completed but the digest did not match the expectation.
    #[error("checksum mismatch for `{model}`: expected sha256:{expected}, got sha256:{actual}")]
    ChecksumMismatch {
        /// Model reference being fetched.
        model: String,
        /// Digest declared by the caller / catalog.
        expected: String,
        /// Digest actually computed over the received bytes.
        actual: String,
    },

    /// A transport failure while downloading weights.
    #[error("download of `{url}` failed: {reason}")]
    Download {
        /// Fully-resolved URL that was requested.
        url: String,
        /// Status line or transport error description.
        reason: String,
    },

    /// The artefact bytes do not match any format this crate can inspect, or
    /// the header is truncated / malformed.
    #[error("cannot inspect `{path}`: {reason}")]
    Inspect {
        /// Path of the artefact under inspection.
        path: PathBuf,
        /// Why the parse gave up.
        reason: String,
    },

    /// The operation requires the host (filesystem + network) and is not
    /// available on `wasm32-unknown-unknown`.
    #[error("`{0}` is unavailable on this target (host-only operation)")]
    UnsupportedTarget(&'static str),
}

impl ModelError {
    /// Build an [`ModelError::Io`] from a path and an [`std::io::Error`].
    ///
    /// Host-only: the wasm build has no filesystem module to call it, and an
    /// uncallable constructor would warn as dead code.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }
}
