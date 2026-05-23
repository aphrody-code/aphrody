// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Resolution of the on-disk model cache directory.
//
// fastembed (via hf-hub) downloads model weights on first use. We park them
// under the aphrody state directory (`~/.aphrody/models/embeddings`) so they
// are shared across invocations and never written to the current working
// directory. This module is always compiled (no feature gate) — it is pure
// path logic with a single filesystem `create_dir_all`.

use std::path::{Path, PathBuf};

use crate::error::{EmbedError, Result};

/// Resolve the aphrody root state directory.
///
/// Precedence mirrors `crates/cli/src/oc_cmd.rs::home_dir` so the cache lands
/// inside the same `~/.aphrody` namespace the rest of the CLI uses:
///   1. `$APHRODY_HOME` (explicit override) -> used directly as the root.
///   2. On Windows: `%USERPROFILE%` -> `<userprofile>/.aphrody`.
///   3. `$HOME` -> `<home>/.aphrody`.
///   4. `dirs::home_dir()` -> `<home>/.aphrody` (portable fallback).
fn aphrody_state_dir() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("APHRODY_HOME") {
        if !explicit.is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }
    if cfg!(windows) {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            if !profile.is_empty() {
                return Ok(PathBuf::from(profile).join(".aphrody"));
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Ok(PathBuf::from(home).join(".aphrody"));
        }
    }
    dirs::home_dir()
        .map(|h| h.join(".aphrody"))
        .ok_or(EmbedError::NoHome)
}

/// Compose the embeddings cache path under a given aphrody state root.
///
/// Pure path arithmetic (no I/O, no env), kept separate so it is unit-testable
/// without mutating process environment (the crate is `#![forbid(unsafe_code)]`
/// and Edition-2024 `std::env::set_var` is `unsafe`).
fn embeddings_subdir(state_root: &Path) -> PathBuf {
    state_root.join("models").join("embeddings")
}

/// Resolve (and create) the embeddings model cache directory.
///
/// Returns `~/.aphrody/models/embeddings`, creating the full path if missing.
/// An explicit `override_dir` short-circuits the default and is created the
/// same way — handy for tests (point it at a `tempfile::TempDir`) and for
/// callers that want an isolated cache.
///
/// # Errors
/// [`EmbedError::NoHome`] if no home directory can be resolved, or
/// [`EmbedError::CacheDir`] if the directory cannot be created.
pub fn model_cache_dir(override_dir: Option<PathBuf>) -> Result<PathBuf> {
    let dir = match override_dir {
        Some(d) => d,
        None => embeddings_subdir(&aphrody_state_dir()?),
    };
    std::fs::create_dir_all(&dir).map_err(|source| EmbedError::CacheDir {
        path: dir.clone(),
        source,
    })?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_dir_is_created_and_returned() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("nested").join("cache");
        let resolved = model_cache_dir(Some(target.clone())).expect("resolve");
        assert_eq!(resolved, target);
        assert!(resolved.is_dir(), "cache dir should have been created");
    }

    #[test]
    fn embeddings_subdir_is_under_models_embeddings() {
        let root = Path::new("/some/root/.aphrody");
        let sub = embeddings_subdir(root);
        assert!(sub.starts_with(root));
        assert_eq!(sub.file_name().unwrap(), "embeddings");
        assert_eq!(sub.parent().unwrap().file_name().unwrap(), "models");
    }

    #[test]
    fn aphrody_state_dir_resolves_without_panic() {
        // Read-only: does not mutate env. On any normal dev/CI machine a home
        // is resolvable; assert the happy path produces a `.aphrody` leaf.
        if let Ok(dir) = aphrody_state_dir() {
            assert_eq!(dir.file_name().unwrap(), ".aphrody");
        }
    }
}
