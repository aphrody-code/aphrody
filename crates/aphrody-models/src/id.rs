// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Model reference parsing and cache-path derivation.
//
// A `ModelRef` names ONE artefact (a single file on disk), not a repository:
// local inference backends load a concrete `.gguf` / `.onnx` / `.safetensors`
// / `.bin` file, so that is the unit the store tracks, hashes and evicts.
//
// Grammar (all forms round-trip through `Display`):
//
//   hf:<owner>/<repo>/<file-path>[@<revision>]
//   https://<host>/<path>            (or  url:https://...)
//   file:<absolute-or-relative-path> (already-on-disk, never copied)
//
// The Hugging Face form is unambiguous because a repo id is ALWAYS exactly two
// path segments: the first two segments are the repo, everything after is the
// in-repo file path (which may itself contain slashes, e.g. `onnx/model.onnx`).

use core::fmt;
use std::path::PathBuf;

use crate::error::{ModelError, Result};

/// Default Hugging Face revision when none is pinned in the reference.
pub const DEFAULT_REVISION: &str = "main";

/// A parsed, canonical reference to exactly one model artefact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModelRef {
    /// A file inside a Hugging Face Hub repository.
    Hf {
        /// Repository owner / organisation (first path segment).
        owner: String,
        /// Repository name (second path segment).
        repo: String,
        /// In-repo path of the artefact, slash-separated, no leading slash.
        file: String,
        /// Git revision: branch, tag or commit sha. Defaults to `main`.
        revision: String,
    },
    /// A direct HTTP(S) download.
    Url(String),
    /// An artefact that already exists on the local filesystem. The store
    /// never copies these; it records them so `list` / `info` can see them.
    Local(PathBuf),
}

impl ModelRef {
    /// Parse a reference string.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::BadRef`] when the string matches none of the
    /// three supported grammars.
    pub fn parse(input: &str) -> Result<Self> {
        let raw = input.trim();
        if raw.is_empty() {
            return Err(ModelError::BadRef { input: input.to_owned(), reason: "empty reference" });
        }

        if let Some(rest) = raw.strip_prefix("hf:") {
            return Self::parse_hf(input, rest);
        }
        if let Some(rest) = raw.strip_prefix("url:") {
            return Self::parse_url(input, rest);
        }
        if raw.starts_with("https://") || raw.starts_with("http://") {
            return Self::parse_url(input, raw);
        }
        if let Some(rest) = raw.strip_prefix("file:") {
            if rest.is_empty() {
                return Err(ModelError::BadRef {
                    input: input.to_owned(),
                    reason: "`file:` needs a path",
                });
            }
            return Ok(Self::Local(PathBuf::from(rest)));
        }

        // Bare `owner/repo/file` is accepted as a Hugging Face shorthand: it is
        // the overwhelmingly common case and matches how the catalog spells
        // its entries.
        Self::parse_hf(input, raw)
    }

    fn parse_url(input: &str, candidate: &str) -> Result<Self> {
        if !(candidate.starts_with("https://") || candidate.starts_with("http://")) {
            return Err(ModelError::BadRef {
                input: input.to_owned(),
                reason: "url reference must start with http:// or https://",
            });
        }
        // Reject a scheme with no authority (`https://`) or no path at all.
        let after_scheme = candidate.split_once("//").map_or("", |(_, rest)| rest);
        if after_scheme.is_empty() || after_scheme.starts_with('/') {
            return Err(ModelError::BadRef {
                input: input.to_owned(),
                reason: "url reference has no host",
            });
        }
        Ok(Self::Url(candidate.to_owned()))
    }

    fn parse_hf(input: &str, rest: &str) -> Result<Self> {
        // Split the optional `@revision` suffix first: revisions never contain
        // `/`, and a bare `@` is not legal inside a Hugging Face repo id.
        let (path_part, revision) = match rest.rsplit_once('@') {
            Some((p, rev)) if !rev.is_empty() && !rev.contains('/') => (p, rev.to_owned()),
            _ => (rest, DEFAULT_REVISION.to_owned()),
        };

        let mut segments = path_part.split('/');
        let owner = segments.next().unwrap_or_default();
        let repo = segments.next().unwrap_or_default();
        let file = segments.collect::<Vec<_>>().join("/");

        if owner.is_empty() || repo.is_empty() {
            return Err(ModelError::BadRef {
                input: input.to_owned(),
                reason: "expected `owner/repo/file-path` (a repo id is two segments)",
            });
        }
        if file.is_empty() {
            return Err(ModelError::BadRef {
                input: input.to_owned(),
                reason: "missing in-repo file path after `owner/repo/`",
            });
        }
        if path_part.contains("..") {
            return Err(ModelError::BadRef {
                input: input.to_owned(),
                reason: "path traversal (`..`) is not allowed in a reference",
            });
        }

        Ok(Self::Hf { owner: owner.to_owned(), repo: repo.to_owned(), file, revision })
    }

    /// The resolvable download URL for this reference, when one exists.
    ///
    /// Returns `None` for [`ModelRef::Local`], which is never downloaded.
    #[must_use]
    pub fn download_url(&self) -> Option<String> {
        match self {
            Self::Hf { owner, repo, file, revision } => Some(format!(
                "https://huggingface.co/{owner}/{repo}/resolve/{revision}/{file}?download=true"
            )),
            Self::Url(u) => Some(u.clone()),
            Self::Local(_) => None,
        }
    }

    /// Path of this artefact RELATIVE to the store root.
    ///
    /// Deterministic and collision-free: Hugging Face artefacts key on
    /// `owner/repo/revision/file`, direct URLs key on a truncated digest of
    /// the URL (so two different URLs sharing a basename never collide).
    /// Returns `None` for [`ModelRef::Local`] (nothing is stored).
    #[must_use]
    pub fn relative_path(&self) -> Option<PathBuf> {
        match self {
            Self::Hf { owner, repo, file, revision } => {
                let mut p = PathBuf::from("hf");
                p.push(sanitize(owner));
                p.push(sanitize(repo));
                p.push(sanitize(revision));
                for seg in file.split('/') {
                    p.push(sanitize(seg));
                }
                Some(p)
            }
            Self::Url(u) => {
                let digest = crate::digest::sha256_hex(u.as_bytes());
                let mut p = PathBuf::from("url");
                p.push(&digest[..16]);
                p.push(sanitize(&self.basename()));
                Some(p)
            }
            Self::Local(_) => None,
        }
    }

    /// Short human label: the artefact basename.
    #[must_use]
    pub fn basename(&self) -> String {
        match self {
            Self::Hf { file, .. } => file.rsplit('/').next().unwrap_or(file).to_owned(),
            Self::Url(u) => u
                .rsplit('/')
                .next()
                .and_then(|s| s.split(['?', '#']).next())
                .filter(|s| !s.is_empty())
                .unwrap_or("artifact.bin")
                .to_owned(),
            Self::Local(p) => p
                .file_name()
                .map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned()),
        }
    }
}

impl fmt::Display for ModelRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hf { owner, repo, file, revision } => {
                write!(f, "hf:{owner}/{repo}/{file}")?;
                if revision != DEFAULT_REVISION {
                    write!(f, "@{revision}")?;
                }
                Ok(())
            }
            Self::Url(u) => write!(f, "{u}"),
            Self::Local(p) => write!(f, "file:{}", p.display()),
        }
    }
}

impl core::str::FromStr for ModelRef {
    type Err = ModelError;
    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl serde::Serialize for ModelRef {
    fn serialize<S: serde::Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for ModelRef {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// Reduce a string to a single filesystem-safe path component.
///
/// The reference grammar already rejects `..`, but revisions and URL basenames
/// are attacker-influenced strings, so every component is additionally reduced
/// to `[A-Za-z0-9._-]` before it reaches the filesystem.
fn sanitize(component: &str) -> String {
    let mut out: String = component
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '_' })
        .collect();
    // A component of nothing but dots would still resolve to `.` / `..`.
    if out.is_empty() || out.chars().all(|c| c == '.') {
        out = "_".repeat(out.len().max(1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hf_with_nested_file_path() {
        let r = ModelRef::parse("hf:BAAI/bge-small-en-v1.5/onnx/model.onnx").unwrap();
        assert_eq!(
            r,
            ModelRef::Hf {
                owner: "BAAI".into(),
                repo: "bge-small-en-v1.5".into(),
                file: "onnx/model.onnx".into(),
                revision: "main".into(),
            }
        );
        assert_eq!(r.basename(), "model.onnx");
    }

    #[test]
    fn parses_pinned_revision() {
        let r = ModelRef::parse("ggerganov/whisper.cpp/ggml-base.en.bin@v1.5.4").unwrap();
        let ModelRef::Hf { revision, file, .. } = &r else { panic!("expected hf") };
        assert_eq!(revision, "v1.5.4");
        assert_eq!(file, "ggml-base.en.bin");
        assert_eq!(r.to_string(), "hf:ggerganov/whisper.cpp/ggml-base.en.bin@v1.5.4");
    }

    #[test]
    fn default_revision_is_elided_by_display() {
        let r = ModelRef::parse("hf:a/b/c.onnx").unwrap();
        assert_eq!(r.to_string(), "hf:a/b/c.onnx");
        assert_eq!(ModelRef::parse(&r.to_string()).unwrap(), r);
    }

    #[test]
    fn hf_download_url_targets_resolve_endpoint() {
        let r = ModelRef::parse("hf:BAAI/bge-small-en-v1.5/onnx/model.onnx@abc123").unwrap();
        assert_eq!(
            r.download_url().unwrap(),
            "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/abc123/onnx/model.onnx?download=true"
        );
    }

    #[test]
    fn rejects_repo_without_file() {
        assert!(ModelRef::parse("hf:BAAI/bge-small-en-v1.5").is_err());
        assert!(ModelRef::parse("hf:onlyowner").is_err());
        assert!(ModelRef::parse("").is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(ModelRef::parse("hf:a/b/../../etc/passwd").is_err());
    }

    #[test]
    fn url_refs_key_on_digest_not_basename() {
        let a = ModelRef::parse("https://example.com/one/model.gguf").unwrap();
        let b = ModelRef::parse("url:https://example.com/two/model.gguf").unwrap();
        assert_ne!(a.relative_path(), b.relative_path());
        assert_eq!(a.basename(), "model.gguf");
    }

    #[test]
    fn url_query_string_is_stripped_from_basename() {
        let r = ModelRef::parse("https://example.com/m.gguf?download=true").unwrap();
        assert_eq!(r.basename(), "m.gguf");
    }

    #[test]
    fn url_without_host_is_rejected() {
        assert!(ModelRef::parse("https:///nohost.gguf").is_err());
    }

    #[test]
    fn local_refs_are_never_stored() {
        let r = ModelRef::parse("file:/opt/models/x.gguf").unwrap();
        assert!(r.relative_path().is_none());
        assert!(r.download_url().is_none());
    }

    #[test]
    fn slash_bearing_suffix_is_not_a_revision() {
        let r = ModelRef::parse("hf:a/b/c.onnx@refs/pr/1").unwrap();
        let ModelRef::Hf { revision, .. } = &r else { panic!() };
        assert_eq!(revision, "main");
    }

    #[test]
    fn revision_is_sanitized_into_the_cache_path() {
        let r = ModelRef::parse("hf:a/b/c.onnx@we!rd").unwrap();
        let p = r.relative_path().unwrap();
        assert!(p.to_string_lossy().contains("we_rd"), "{p:?}");
    }

    #[test]
    fn serde_round_trip() {
        let r = ModelRef::parse("hf:a/b/c.onnx@v2").unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"hf:a/b/c.onnx@v2\"");
        assert_eq!(serde_json::from_str::<ModelRef>(&json).unwrap(), r);
    }
}
