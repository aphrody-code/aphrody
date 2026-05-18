// SPDX-License-Identifier: Apache-2.0
//! Streaming artifact pipeline for the aphrody design sidecar.
//!
//! Agents stream `<artifact type="..." id="..." title="...">…</artifact>`
//! blocks into the sidecar process. Each artifact is:
//!
//! 1. **Parsed** out of the stream chunk-by-chunk ([`parser`]).
//! 2. **Normalized** by a per-format adapter ([`adapters`]) — HTML strips
//!    script tags, Markdown is CommonMark-validated, ZIP rejects path
//!    traversal entries, PDF/PPTX validate magic bytes.
//! 3. **Merged** when multiple stream chunks refer to the same artifact id
//!    ([`merge`]).
//! 4. **Validated** against the schema (50 MiB cap, MIME whitelist,
//!    type/format coherence) ([`validate`]).
//! 5. **Digested** into a SHA-256 manifest with deterministic ordering
//!    ([`digest`]).
//! 6. **Driven** end-to-end by [`pipeline::Pipeline::run`], which falls back
//!    to a degraded partial result if the parser hard-fails halfway through
//!    a stream (the manifest is still emitted for whatever artifacts have
//!    been completed, plus a `partial: true` flag).
//!
//! Path / IPC patterns mirror the upstream sidecar contract (descriptor
//! defaults + namespace + base resolution) — see [`resolve`] for the
//! cross-platform host path helpers we reuse from that contract.

#![forbid(unsafe_code)]
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
///
/// Hard cap at 50 MiB — anything bigger is almost certainly a runaway
/// streaming buffer (or an attempt to OOM the sidecar). Mirrors the limit
/// that the upstream sidecar IPC framing enforces on the host side.
pub const MAX_ARTIFACT_SIZE: usize = 50 * 1024 * 1024;
