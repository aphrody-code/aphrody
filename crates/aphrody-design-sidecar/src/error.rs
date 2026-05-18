// SPDX-License-Identifier: Apache-2.0
//! Unified error type for the streaming artifact pipeline.

use thiserror::Error;

/// Convenience alias for `Result<T, SidecarError>`.
pub type SidecarResult<T> = std::result::Result<T, SidecarError>;

/// All recoverable failures surfaced by the design sidecar pipeline.
///
/// The pipeline degrades to a partial result on most variants (the failed
/// artifact is dropped, but the rest of the stream is still digested).
/// Only [`SidecarError::Io`] aborts the run completely.
#[derive(Debug, Error)]
pub enum SidecarError {
    /// The streaming parser produced malformed XML / unbalanced tags.
    #[error("artifact parse failure: {0}")]
    Parse(String),

    /// A required `<artifact>` attribute was missing or invalid.
    #[error("artifact attribute error: {0}")]
    Attribute(String),

    /// The format adapter rejected the raw payload as non-conformant.
    #[error("format normalization failed: {0}")]
    Normalize(String),

    /// Schema validation rejected the artifact (size cap, MIME mismatch,
    /// unknown type, etc.).
    #[error("validation failed: {0}")]
    Validation(String),

    /// SHA-256 digest computation could not complete (e.g. mid-stream cut).
    #[error("digest failure: {0}")]
    Digest(String),

    /// Multi-chunk merge produced an inconsistent state (ordering / dedup).
    #[error("merge failure: {0}")]
    Merge(String),

    /// Underlying IO error (file read / stdin slurp / stdout flush).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization error from the validate / digest layer.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// XML decoding error from the streaming parser.
    #[error(transparent)]
    Xml(#[from] quick_xml::Error),
}
