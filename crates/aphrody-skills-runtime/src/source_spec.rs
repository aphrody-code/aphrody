// SPDX-License-Identifier: Apache-2.0
//
// Polymorphic source location parser — supports GitHub shorthand, full Git
// URLs (https/git/ssh), GitLab URLs, GitHub subtree URLs, and local paths.
//
// Contract matches `vercel-labs/skills::source-parser.ts` but extends it with
// subpath + ref support harvested from GitHub `tree/<branch>/<path>` URLs.
// Fuses three sources:
//   1. `sources.rs::SourceSpec` (existing static registry → enum shape)
//   2. `vercel-labs/skills::source-parser.ts` (URL shorthand semantics)
//   3. `manifest.rs::ensure_home_exists` (PathBuf semantics for local entries)

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

/// Where a skill source physically lives. Resolved to a local path by
/// `cache::ensure_cached`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SourceLocation {
    /// Owner/repo on github.com, optional ref (branch/tag/sha) + subpath.
    GitHub {
        owner: String,
        repo: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subpath: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none", rename = "ref")]
        ref_: Option<String>,
    },
    /// Any other Git URL (gitlab, bitbucket, git@, ssh://, git://, https://).
    GitUrl(String),
    /// Absolute or relative path to an existing local directory.
    Local(PathBuf),
}

impl SourceLocation {
    /// Stable, human-friendly slug for cache + lockfile keys.
    #[must_use]
    pub fn slug_hint(&self) -> String {
        match self {
            Self::GitHub { owner, repo, subpath, .. } => match subpath {
                Some(sp) if !sp.is_empty() => {
                    let leaf = sp.rsplit('/').next().unwrap_or("");
                    if leaf.is_empty() {
                        format!("{owner}-{repo}")
                    } else {
                        format!("{owner}-{repo}-{leaf}")
                    }
                }
                _ => format!("{owner}-{repo}"),
            },
            Self::GitUrl(u) => u
                .trim_end_matches('/')
                .trim_end_matches(".git")
                .rsplit(['/', ':'])
                .next()
                .unwrap_or("git-source")
                .to_string(),
            Self::Local(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "local".to_string()),
        }
    }

    /// Best-effort canonical string (used for hashing in `cache::cache_key_for`).
    #[must_use]
    pub fn canonical(&self) -> String {
        match self {
            Self::GitHub { owner, repo, subpath, ref_ } => {
                let mut s = format!("github:{owner}/{repo}");
                if let Some(r) = ref_ {
                    s.push('#');
                    s.push_str(r);
                }
                if let Some(p) = subpath {
                    s.push(':');
                    s.push_str(p);
                }
                s
            }
            Self::GitUrl(u) => format!("git:{u}"),
            Self::Local(p) => format!("local:{}", p.display()),
        }
    }
}

/// Parse a user-supplied source string into a `SourceLocation`.
///
/// Recognised shapes (probed in order):
/// 1. Empty → error.
/// 2. Existing local directory (relative or absolute path).
/// 3. `https://github.com/<owner>/<repo>[.git]`
/// 4. `https://github.com/<owner>/<repo>/tree/<ref>/<subpath>`
/// 5. `https://gitlab.com/...`, `https://bitbucket.org/...` → GitUrl.
/// 6. `git@host:owner/repo.git`, `ssh://...`, `git://...` → GitUrl.
/// 7. `<owner>/<repo>` shorthand → GitHub (no ref, no subpath).
/// 8. Anything else → error.
pub fn parse_source(input: &str) -> Result<SourceLocation> {
    let s = input.trim();
    if s.is_empty() {
        bail!("source spec is empty");
    }

    // 2. Local path heuristic — `.`, `..`, `./`, `../`, absolute, or contains
    //    a path separator and the dir exists on disk.
    if is_local_path_shape(s) {
        let p = PathBuf::from(s);
        if p.exists() && p.is_dir() {
            return Ok(SourceLocation::Local(p));
        }
        // Path-shaped but missing: keep going (might still be a shorthand).
        if s.starts_with('.') || s.starts_with('/') || has_drive_letter(s) {
            bail!("local path `{s}` does not exist or is not a directory");
        }
    }

    // 3+4. GitHub HTTPS URLs (with optional tree/<ref>/<subpath>).
    if let Some(rest) = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
    {
        return parse_github_url_tail(rest);
    }

    // 5. Other host HTTPS — GitLab, Bitbucket, self-hosted.
    if s.starts_with("https://") || s.starts_with("http://") {
        return Ok(SourceLocation::GitUrl(s.to_string()));
    }

    // 6. SSH / git / ssh URLs.
    if s.starts_with("git@")
        || s.starts_with("ssh://")
        || s.starts_with("git://")
        || s.ends_with(".git")
    {
        return Ok(SourceLocation::GitUrl(s.to_string()));
    }

    // 7. <owner>/<repo> shorthand (exactly one '/', both halves non-empty, no
    //    leading dot, no scheme-like prefix).
    if let Some((owner, repo)) = s.split_once('/') {
        if !owner.is_empty()
            && !repo.is_empty()
            && !owner.contains('/')
            && !repo.contains('/')
            && !owner.starts_with('.')
            && !owner.contains(':')
            && is_safe_ident(owner)
            && is_safe_ident(repo)
        {
            return Ok(SourceLocation::GitHub {
                owner: owner.to_string(),
                repo: repo.trim_end_matches(".git").to_string(),
                subpath: None,
                ref_: None,
            });
        }
    }

    Err(anyhow!(
        "unrecognised source spec `{s}` — expected `owner/repo`, a Git URL, or a local path"
    ))
}

fn parse_github_url_tail(rest: &str) -> Result<SourceLocation> {
    let rest = rest.trim_end_matches('/');
    let mut parts = rest.split('/');
    let owner = parts.next().unwrap_or("").to_string();
    let repo = parts.next().unwrap_or("").trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        bail!("github url is missing owner/repo");
    }
    let mut ref_: Option<String> = None;
    let mut subpath: Option<String> = None;
    // Optional `/tree/<ref>/<subpath...>` or `/blob/<ref>/<subpath...>`.
    if let Some(kind) = parts.next() {
        if kind == "tree" || kind == "blob" {
            if let Some(r) = parts.next() {
                ref_ = Some(r.to_string());
                let sp: Vec<&str> = parts.collect();
                if !sp.is_empty() {
                    subpath = Some(sp.join("/"));
                }
            }
        }
    }
    Ok(SourceLocation::GitHub { owner, repo, subpath, ref_ })
}

fn is_local_path_shape(s: &str) -> bool {
    s == "."
        || s == ".."
        || s.starts_with("./")
        || s.starts_with("../")
        || s.starts_with('/')
        || s.starts_with('~')
        || has_drive_letter(s)
        || Path::new(s).is_absolute()
}

fn has_drive_letter(s: &str) -> bool {
    // `C:/...` or `C:\...` on Windows. Two-char form `C:` is also possible.
    let b = s.as_bytes();
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

fn is_safe_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_owner_repo_shorthand() {
        let loc = parse_source("vercel-labs/agent-skills").unwrap();
        assert_eq!(
            loc,
            SourceLocation::GitHub {
                owner: "vercel-labs".to_string(),
                repo: "agent-skills".to_string(),
                subpath: None,
                ref_: None,
            }
        );
    }

    #[test]
    fn parses_full_github_https_url() {
        let loc = parse_source("https://github.com/anthropics/skills").unwrap();
        assert!(matches!(loc, SourceLocation::GitHub { ref owner, ref repo, .. }
            if owner == "anthropics" && repo == "skills"));
    }

    #[test]
    fn parses_github_tree_subtree_url() {
        let loc = parse_source(
            "https://github.com/vercel-labs/skills/tree/main/agents/claude-code",
        )
        .unwrap();
        match loc {
            SourceLocation::GitHub { owner, repo, ref_, subpath } => {
                assert_eq!(owner, "vercel-labs");
                assert_eq!(repo, "skills");
                assert_eq!(ref_.as_deref(), Some("main"));
                assert_eq!(subpath.as_deref(), Some("agents/claude-code"));
            }
            other => panic!("expected GitHub variant, got {other:?}"),
        }
    }

    #[test]
    fn parses_ssh_git_url() {
        let loc = parse_source("git@github.com:owner/repo.git").unwrap();
        assert!(matches!(loc, SourceLocation::GitUrl(ref u) if u.contains("git@github.com")));
    }

    #[test]
    fn parses_gitlab_https_url() {
        let loc = parse_source("https://gitlab.com/org/repo").unwrap();
        assert!(matches!(loc, SourceLocation::GitUrl(ref u) if u.contains("gitlab.com")));
    }

    #[test]
    fn parses_local_path() {
        let td = tempdir().unwrap();
        let inner = td.path().join("my-skills");
        fs::create_dir(&inner).unwrap();
        let loc = parse_source(inner.to_str().unwrap()).unwrap();
        assert!(matches!(loc, SourceLocation::Local(ref p) if p == &inner));
    }

    #[test]
    fn rejects_empty_string() {
        assert!(parse_source("").is_err());
        assert!(parse_source("   ").is_err());
    }

    #[test]
    fn rejects_garbage_with_no_separator() {
        assert!(parse_source("not-a-source").is_err());
    }

    #[test]
    fn canonical_includes_ref_and_subpath() {
        let loc = SourceLocation::GitHub {
            owner: "vercel-labs".to_string(),
            repo: "skills".to_string(),
            subpath: Some("agents/x".to_string()),
            ref_: Some("main".to_string()),
        };
        let c = loc.canonical();
        assert!(c.contains("vercel-labs/skills"));
        assert!(c.contains("#main"));
        assert!(c.contains(":agents/x"));
    }
}
