// SPDX-License-Identifier: Apache-2.0
//! Schema validation for parsed + normalized artifacts.
//!
//! Hard rules enforced here (a violation is a hard reject, *not* a degraded
//! pass — the artifact is dropped from the manifest):
//!
//! * Total payload size ≤ [`MAX_ARTIFACT_SIZE`] (50 MiB).
//! * `type_` is in the whitelist (see [`ArtifactFormat::from_mime`]).
//! * Declared `type_` matches the adapter-resolved [`NormalizedArtifact::mime`]
//!   (no spoofing a PDF as `text/markdown`).
//! * `id` is non-empty and shorter than 256 bytes.

use serde::{Deserialize, Serialize};

use crate::sidecar::MAX_ARTIFACT_SIZE;
use crate::sidecar::adapters::{ArtifactFormat, NormalizedArtifact};
use crate::sidecar::digest::DigestManifest;
use crate::sidecar::error::{SidecarError, SidecarResult};
use crate::sidecar::parser::Artifact;

/// Per-artifact validation outcome with the failure reason (if any).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Artifact id being validated.
    pub id: String,
    /// Whether the artifact passed all checks.
    pub ok: bool,
    /// Failure reason, populated iff `ok == false`.
    pub reason: Option<String>,
}

/// Validate a single artifact's structural metadata plus the normalized
/// payload it produced.
///
/// Returns `Ok(())` on pass, or [`SidecarError::Validation`] on fail.
///
/// # Errors
///
/// [`SidecarError::Validation`] when one of the hard rules is violated.
pub fn validate_artifact(
    artifact: &Artifact,
    normalized: &NormalizedArtifact,
) -> SidecarResult<()> {
    if artifact.id.is_empty() {
        return Err(SidecarError::Validation("artifact id is empty".into()));
    }
    if artifact.id.len() > 256 {
        return Err(SidecarError::Validation(format!(
            "artifact id too long ({} > 256)",
            artifact.id.len()
        )));
    }
    if normalized.bytes.len() > MAX_ARTIFACT_SIZE {
        return Err(SidecarError::Validation(format!(
            "artifact {} size {} exceeds cap {}",
            artifact.id,
            normalized.bytes.len(),
            MAX_ARTIFACT_SIZE
        )));
    }

    // Declared MIME must match the adapter's resolved format.
    let declared = ArtifactFormat::from_mime(&artifact.type_)
        .ok_or_else(|| SidecarError::Validation(format!(
            "unsupported artifact type: {}",
            artifact.type_
        )))?;
    if declared != normalized.format {
        return Err(SidecarError::Validation(format!(
            "declared type {} disagrees with resolved format {:?}",
            artifact.type_, normalized.format
        )));
    }
    Ok(())
}

/// Validate a fully built manifest (cross-row checks: unique ids, total size).
///
/// # Errors
///
/// [`SidecarError::Validation`] if any cross-row invariant is broken.
pub fn validate_manifest(manifest: &DigestManifest) -> SidecarResult<()> {
    let mut seen = std::collections::HashSet::new();
    let mut total: u64 = 0;
    for row in &manifest.artifacts {
        if !seen.insert(&row.id) {
            return Err(SidecarError::Validation(format!(
                "duplicate artifact id in manifest: {}",
                row.id
            )));
        }
        total = total.saturating_add(row.size as u64);
    }
    // Soft total cap: 4x the per-artifact cap so a manifest can hold a few
    // large blobs but not 100x of them.
    let cap = (MAX_ARTIFACT_SIZE as u64) * 4;
    if total > cap {
        return Err(SidecarError::Validation(format!(
            "manifest total size {total} exceeds 4x per-artifact cap"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidecar::adapters::{ArtifactFormat, NormalizedArtifact};
    use crate::sidecar::digest::{ArtifactDigest, DigestManifest};

    fn norm(bytes: Vec<u8>, fmt: ArtifactFormat) -> NormalizedArtifact {
        NormalizedArtifact {
            format: fmt,
            mime: fmt.canonical_mime().to_string(),
            bytes,
            notes: Vec::new(),
        }
    }

    fn art(id: &str, type_: &str) -> Artifact {
        Artifact {
            id: id.to_string(),
            type_: type_.to_string(),
            title: id.to_string(),
            content: String::new(),
            mime: type_.to_string(),
        }
    }

    #[test]
    fn size_cap_rejects_oversized_artifact() {
        let big = norm(vec![0u8; MAX_ARTIFACT_SIZE + 1], ArtifactFormat::Markdown);
        let err = validate_artifact(&art("x", "text/markdown"), &big).unwrap_err();
        assert!(matches!(err, SidecarError::Validation(ref s) if s.contains("exceeds cap")));
    }

    #[test]
    fn unknown_mime_rejected() {
        let n = norm(b"hi".to_vec(), ArtifactFormat::Markdown);
        let err = validate_artifact(&art("x", "application/x-not-real"), &n).unwrap_err();
        assert!(matches!(err, SidecarError::Validation(ref s) if s.contains("unsupported")));
    }

    #[test]
    fn type_format_mismatch_rejected() {
        let n = norm(b"hi".to_vec(), ArtifactFormat::Html);
        let err = validate_artifact(&art("x", "text/markdown"), &n).unwrap_err();
        assert!(matches!(err, SidecarError::Validation(ref s) if s.contains("disagrees")));
    }

    #[test]
    fn manifest_dedup_check() {
        let manifest = DigestManifest {
            artifacts: vec![
                ArtifactDigest {
                    id: "a".into(),
                    sha256: "x".into(),
                    size: 1,
                    format: ArtifactFormat::Markdown,
                },
                ArtifactDigest {
                    id: "a".into(),
                    sha256: "y".into(),
                    size: 1,
                    format: ArtifactFormat::Markdown,
                },
            ],
            total_sha256: "z".into(),
            partial: false,
        };
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(matches!(err, SidecarError::Validation(ref s) if s.contains("duplicate")));
    }
}
