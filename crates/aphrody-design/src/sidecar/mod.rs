// SPDX-License-Identifier: Apache-2.0
//! Streaming artifact pipeline for the aphrody design sidecar.
//!
//! Agents stream `<artifact type="..." id="..." title="...">...</artifact>`
//! blocks into the sidecar process. Each artifact is parsed, normalized,
//! merged, validated, and digested into a SHA-256 manifest.

#![deny(rust_2018_idioms)]

pub mod adapters;
pub mod digest;
pub mod error;
pub mod merge;
pub mod parser;
pub mod pipeline;
pub mod resolve;
pub mod validate;

pub use adapters::{ArtifactFormat, FormatAdapter, NormalizedArtifact, adapter_for};
pub use digest::{ArtifactDigest, DigestManifest, digest_artifact, digest_manifest};
pub use error::{SidecarError, SidecarResult};
pub use merge::{merge_chunks, merge_into};
pub use parser::{Artifact, ArtifactParser, ParserEvent};
pub use pipeline::{Pipeline, PipelineOutcome};
pub use validate::{ValidationReport, validate_artifact, validate_manifest};

/// Maximum accepted size for a single artifact's raw payload, in bytes.
pub const MAX_ARTIFACT_SIZE: usize = 50 * 1024 * 1024;
