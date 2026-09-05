// SPDX-License-Identifier: Apache-2.0
//
// Disk cache for cloned skill sources.
//
// Fuses:
//   1. `source_spec.rs::SourceLocation` (input variant set)
//   2. `manifest.rs::skills_home` (XDG / env-var resolution pattern)
//   3. Git command invocation pattern from `cli` xtask skills-sync
//      (std::process::Command, no git2 dep — keeps the binary small + AOT-friendly).
//
// Cache layout:
//   <cache_root>/
//     <key>/          # shallow clone of the source
//
// Where <key> = first 16 hex chars of SHA-256(loc.canonical()).

use std::{
    env, fs,
    path::PathBuf,
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};

use crate::runtime::source_spec::SourceLocation;

/// Resolve the cache root directory. Honours `APHRODY_SKILLS_CACHE` first,
/// then falls back to `dirs::data_dir()/aphrody/cache/skills`, then to
/// `$HOME/.aphrody/cache/skills`.
#[must_use]
pub fn cache_dir() -> PathBuf {
    if let Ok(o) = env::var("APHRODY_SKILLS_CACHE") {
        if !o.is_empty() {
            return PathBuf::from(o);
        }
    }
    if let Ok(o) = env::var("XDG_DATA_HOME") {
        if !o.is_empty() {
            return PathBuf::from(o).join("aphrody").join("cache").join("skills");
        }
    }
    if let Some(base) = dirs::data_dir() {
        return base.join("aphrody").join("cache").join("skills");
    }
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".aphrody").join("cache").join("skills")
}

/// Stable 16-hex-char SHA-256 prefix for a source location.
#[must_use]
pub fn cache_key_for(loc: &SourceLocation) -> String {
    let mut hasher = Sha256::new();
    hasher.update(loc.canonical().as_bytes());
    let digest = hasher.finalize();
    let mut s = String::with_capacity(16);
    for b in digest.iter().take(8) {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Materialise a `SourceLocation` to a local directory containing the cloned
/// (or local) tree, plus an optional subpath to descend into.
///
/// Returns `(checkout_root, effective_skills_dir)` where:
/// * `checkout_root` is the actual git clone (or the local path).
/// * `effective_skills_dir` is `checkout_root/<subpath>` when subpath is set,
///   else `checkout_root` itself.
pub fn ensure_cached(loc: &SourceLocation) -> Result<PathBuf> {
    match loc {
        SourceLocation::Local(p) => {
            if !p.exists() {
                return Err(anyhow!("local source `{}` does not exist", p.display()));
            }
            Ok(p.clone())
        }
        SourceLocation::GitUrl(_) => clone_or_update(loc, None, None),
        SourceLocation::GitHub { subpath, ref_, .. } => {
            let root = clone_or_update(loc, ref_.as_deref(), None)?;
            if let Some(sp) = subpath {
                let nested = root.join(sp);
                if !nested.exists() {
                    return Err(anyhow!(
                        "subpath `{sp}` not found under cached clone {}",
                        root.display()
                    ));
                }
                Ok(nested)
            } else {
                Ok(root)
            }
        }
    }
}

fn clone_or_update(
    loc: &SourceLocation,
    git_ref: Option<&str>,
    _depth_override: Option<u32>,
) -> Result<PathBuf> {
    let url = git_url_for(loc)?;
    let root = cache_dir();
    fs::create_dir_all(&root)
        .with_context(|| format!("create cache dir {}", root.display()))?;
    let key = cache_key_for(loc);
    let target = root.join(&key);

    if target.join(".git").exists() {
        // Update existing clone — best-effort `git fetch` + `git reset --hard`.
        let _ = Command::new("git")
            .args(["fetch", "--depth=1", "origin"])
            .current_dir(&target)
            .status();
        if let Some(r) = git_ref {
            let _ = Command::new("git")
                .args(["checkout", r])
                .current_dir(&target)
                .status();
        }
        return Ok(target);
    }

    if target.exists() {
        // Stale, non-git directory — wipe so a fresh clone can land.
        fs::remove_dir_all(&target)
            .with_context(|| format!("remove stale cache dir {}", target.display()))?;
    }

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth=1");
    if let Some(r) = git_ref {
        cmd.arg("--branch").arg(r);
    }
    cmd.arg(&url).arg(&target);

    let status = cmd
        .status()
        .with_context(|| format!("spawn `git clone {url}`"))?;
    if !status.success() {
        return Err(anyhow!(
            "git clone failed for {url} (exit code {:?})",
            status.code()
        ));
    }
    Ok(target)
}

fn git_url_for(loc: &SourceLocation) -> Result<String> {
    Ok(match loc {
        SourceLocation::GitHub { owner, repo, .. } => {
            format!("https://github.com/{owner}/{repo}.git")
        }
        SourceLocation::GitUrl(u) => u.clone(),
        SourceLocation::Local(_) => {
            return Err(anyhow!("local sources do not need cloning"));
        }
    })
}

#[allow(dead_code)]
pub(crate) fn cache_path_for(loc: &SourceLocation) -> PathBuf {
    cache_dir().join(cache_key_for(loc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn restore_env(key: &str, prev: Option<String>) {
        match prev {
            Some(v) => unsafe { env::set_var(key, v) },
            None => unsafe { env::remove_var(key) },
        }
    }

    #[test]
    fn cache_key_is_deterministic() {
        let loc = SourceLocation::GitHub {
            owner: "vercel-labs".to_string(),
            repo: "agent-skills".to_string(),
            subpath: None,
            ref_: None,
        };
        let a = cache_key_for(&loc);
        let b = cache_key_for(&loc);
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn cache_key_differs_per_location() {
        let l1 = SourceLocation::GitHub {
            owner: "a".into(),
            repo: "b".into(),
            subpath: None,
            ref_: None,
        };
        let l2 = SourceLocation::GitHub {
            owner: "a".into(),
            repo: "b".into(),
            subpath: Some("x".into()),
            ref_: None,
        };
        assert_ne!(cache_key_for(&l1), cache_key_for(&l2));
    }

    #[test]
    fn cache_dir_honours_env_override() {
        let prev_cache = env::var("APHRODY_SKILLS_CACHE").ok();
        let prev_xdg = env::var("XDG_DATA_HOME").ok();
        unsafe { env::set_var("APHRODY_SKILLS_CACHE", "/custom/cache/dir") };
        assert_eq!(cache_dir(), PathBuf::from("/custom/cache/dir"));
        unsafe { env::remove_var("APHRODY_SKILLS_CACHE") };
        unsafe { env::set_var("XDG_DATA_HOME", "/xdg/data") };
        assert_eq!(
            cache_dir(),
            PathBuf::from("/xdg/data")
                .join("aphrody")
                .join("cache")
                .join("skills")
        );
        restore_env("APHRODY_SKILLS_CACHE", prev_cache);
        restore_env("XDG_DATA_HOME", prev_xdg);
    }

    #[test]
    fn ensure_cached_local_returns_path_unchanged() {
        let td = tempdir().unwrap();
        let loc = SourceLocation::Local(td.path().to_path_buf());
        let resolved = ensure_cached(&loc).unwrap();
        assert_eq!(resolved, td.path());
    }

    #[test]
    fn ensure_cached_local_errors_on_missing() {
        let loc = SourceLocation::Local(PathBuf::from("/nope/should/not/exist/abc"));
        assert!(ensure_cached(&loc).is_err());
    }
}
