// SPDX-License-Identifier: Apache-2.0
//! SHA-256 manifest digest.
//!
//! For each normalized artifact we compute:
//!
//! ```json
//! { "id": "dash-1", "sha256": "…", "size": 12345, "format": "html" }
//! ```
//!
//! The manifest is the deterministic sort-by-id list of those entries,
//! plus a `total_sha256` covering the concatenation of every per-artifact
//! digest hex string (so consumers can diff two manifests in O(1)).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::adapters::{ArtifactFormat, NormalizedArtifact};
use crate::parser::Artifact;

/// One row in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDigest {
    /// Artifact id (must be unique inside the manifest).
    pub id: String,
    /// Lowercase hex SHA-256 of [`NormalizedArtifact::bytes`].
    pub sha256: String,
    /// Size in bytes of [`NormalizedArtifact::bytes`].
    pub size: usize,
    /// Resolved format.
    pub format: ArtifactFormat,
}

/// Full manifest emitted at the end of a pipeline run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigestManifest {
    /// All artifacts, sorted by id.
    pub artifacts: Vec<ArtifactDigest>,
    /// SHA-256 of the concatenation `id1\nsha1\nid2\nsha2\n…`.
    pub total_sha256: String,
    /// `true` when the pipeline ran in fallback mode and the manifest may
    /// be incomplete relative to the original stream.
    #[serde(default)]
    pub partial: bool,
}

/// Compute one artifact's digest row.
#[must_use]
pub fn digest_artifact(artifact: &Artifact, normalized: &NormalizedArtifact) -> ArtifactDigest {
    let mut hasher = Sha256::new();
    hasher.update(&normalized.bytes);
    let hash = hasher.finalize();
    ArtifactDigest {
        id: artifact.id.clone(),
        sha256: hex(&hash),
        size: normalized.bytes.len(),
        format: normalized.format,
    }
}

/// Build the full manifest from a list of per-artifact digests.
///
/// The input order is preserved on entry but the output is **sorted by
/// `id`** so the final `total_sha256` is deterministic regardless of
/// stream-arrival order.
#[must_use]
pub fn digest_manifest(rows: Vec<ArtifactDigest>, partial: bool) -> DigestManifest {
    let mut artifacts = rows;
    artifacts.sort_by(|a, b| a.id.cmp(&b.id));
    let mut hasher = Sha256::new();
    for row in &artifacts {
        hasher.update(row.id.as_bytes());
        hasher.update(b"\n");
        hasher.update(row.sha256.as_bytes());
        hasher.update(b"\n");
    }
    DigestManifest {
        artifacts,
        total_sha256: hex(&hasher.finalize()),
        partial,
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_norm(payload: &[u8], format: ArtifactFormat) -> NormalizedArtifact {
        NormalizedArtifact {
            format,
            mime: format.canonical_mime().to_string(),
            bytes: payload.to_vec(),
            notes: Vec::new(),
        }
    }

    fn fake_art(id: &str) -> Artifact {
        Artifact {
            id: id.to_string(),
            type_: "text/markdown".to_string(),
            title: id.to_string(),
            content: String::new(),
            mime: "text/markdown".to_string(),
        }
    }

    #[test]
    fn digest_is_deterministic_regardless_of_input_order() {
        let n1 = fake_norm(b"alpha", ArtifactFormat::Markdown);
        let n2 = fake_norm(b"beta", ArtifactFormat::Markdown);
        let d1 = digest_artifact(&fake_art("a"), &n1);
        let d2 = digest_artifact(&fake_art("b"), &n2);

        let m_ab = digest_manifest(vec![d1.clone(), d2.clone()], false);
        let m_ba = digest_manifest(vec![d2, d1], false);
        assert_eq!(m_ab.total_sha256, m_ba.total_sha256);
        assert_eq!(m_ab.artifacts, m_ba.artifacts);
    }

    #[test]
    fn known_sha256_smoke() {
        // SHA-256("hi") = 8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4
        let n = fake_norm(b"hi", ArtifactFormat::Markdown);
        let d = digest_artifact(&fake_art("x"), &n);
        assert_eq!(
            d.sha256,
            "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4"
        );
        assert_eq!(d.size, 2);
    }
}
