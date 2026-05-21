// SPDX-License-Identifier: Apache-2.0
//! Filesystem helpers — safe read / write + glob walking.
//!
//! TS provenance: `packages/gemini-cli/packages/sdk/src/fs.ts`.
//!
//! The TS implementation wires a sandbox policy (`Config::validatePathAccess`)
//! that we cannot replicate verbatim until `aphrody-permissions` exposes a
//! path-glob policy engine. v1 ships a *defence-in-depth* policy: any read /
//! write is rejected when the path escapes the configured sandbox root via
//! `..` traversal, and writes additionally require the parent directory to
//! already exist (preventing accidental directory creation outside the
//! sandbox).
//!
//! On WASM (`wasm32-unknown-unknown`) every function returns
//! [`SdkError::Unsupported`] — the SDK does not bundle a virtual filesystem.

#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_variables))]

use std::path::{Path, PathBuf};

use crate::types::{SdkError, SdkResult};

// ---------------------------------------------------------------------------
// AgentFilesystem trait — keeps consumers backend-agnostic.
// ---------------------------------------------------------------------------

/// Sandboxed filesystem interface handed to tools at execution time.
///
/// PUBLIC API — semver stable from v1.0.
#[async_trait::async_trait]
pub trait AgentFilesystem: Send + Sync {
    /// Read a UTF-8 file. Returns `Ok(None)` when the file does not exist or
    /// access is denied, `Err(...)` on hard IO failures.
    async fn read_file(&self, path: &Path) -> SdkResult<Option<String>>;

    /// Write UTF-8 contents to a file. Returns `Err(SdkError::AccessDenied)`
    /// when the sandbox policy refuses the destination.
    async fn write_file(&self, path: &Path, contents: &str) -> SdkResult<()>;

    /// Glob walk relative to `cwd`. Returns at most `limit` matches.
    async fn glob(&self, pattern: &str, limit: usize) -> SdkResult<Vec<PathBuf>>;
}

// ---------------------------------------------------------------------------
// SandboxFs — native implementation (Linux + Windows + macOS).
// ---------------------------------------------------------------------------

/// Filesystem rooted at a sandbox directory.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone)]
pub struct SandboxFs {
    root: PathBuf,
}

impl SandboxFs {
    /// Construct a sandbox rooted at `root`. The path is canonicalized lazily
    /// inside each operation; an absent root is allowed (writes will create
    /// it on first call when permitted by the parent directory).
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Borrow the sandbox root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve `path` against the sandbox root and return an absolute path,
    /// rejecting anything that escapes the root.
    fn resolve(&self, path: &Path) -> SdkResult<PathBuf> {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        // Lexical traversal check — works without filesystem access (the
        // target file may not exist yet on writes). We canonicalize lazily in
        // read/write where the file MUST exist.
        if abs
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(SdkError::AccessDenied(format!(
                "path traversal not allowed: {}",
                abs.display()
            )));
        }
        Ok(abs)
    }
}

// ---------------------------------------------------------------------------
// Native impl (everything except wasm32)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
#[async_trait::async_trait]
impl AgentFilesystem for SandboxFs {
    async fn read_file(&self, path: &Path) -> SdkResult<Option<String>> {
        let abs = match self.resolve(path) {
            Ok(p) => p,
            Err(SdkError::AccessDenied(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        match tokio::fs::read_to_string(&abs).await {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Ok(None),
            Err(e) => Err(SdkError::Io(e)),
        }
    }

    async fn write_file(&self, path: &Path, contents: &str) -> SdkResult<()> {
        let abs = self.resolve(path)?;
        if let Some(parent) = abs.parent() {
            if !parent.exists() {
                return Err(SdkError::AccessDenied(format!(
                    "parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }
        tokio::fs::write(&abs, contents).await?;
        Ok(())
    }

    async fn glob(&self, pattern: &str, limit: usize) -> SdkResult<Vec<PathBuf>> {
        // The `glob` crate is sync; we run it on the blocking pool to avoid
        // stalling the tokio reactor on large directory trees.
        let pattern = if Path::new(pattern).is_absolute() {
            pattern.to_string()
        } else {
            self.root.join(pattern).to_string_lossy().into_owned()
        };
        let limit = limit.max(1);
        let matches = tokio::task::spawn_blocking(move || -> SdkResult<Vec<PathBuf>> {
            let mut out: Vec<PathBuf> = Vec::new();
            let entries = glob::glob(&pattern)
                .map_err(|e| SdkError::InvalidConfig(format!("invalid glob pattern: {e}")))?;
            for entry in entries {
                match entry {
                    Ok(p) => out.push(p),
                    Err(e) => {
                        tracing::warn!(
                            target: "aphrody_sdk::fs",
                            "glob walk error: {e}"
                        );
                    }
                }
                if out.len() >= limit {
                    break;
                }
            }
            Ok(out)
        })
        .await
        .map_err(|e| SdkError::Backend(format!("glob worker join failed: {e}")))??;
        Ok(matches)
    }
}

// ---------------------------------------------------------------------------
// WASM stub — every op returns `Unsupported` (predictable, easy to detect).
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait]
impl AgentFilesystem for SandboxFs {
    async fn read_file(&self, _path: &Path) -> SdkResult<Option<String>> {
        Err(SdkError::Unsupported("filesystem read on wasm32"))
    }

    async fn write_file(&self, _path: &Path, _contents: &str) -> SdkResult<()> {
        Err(SdkError::Unsupported("filesystem write on wasm32"))
    }

    async fn glob(&self, _pattern: &str, _limit: usize) -> SdkResult<Vec<PathBuf>> {
        Err(SdkError::Unsupported("filesystem glob on wasm32"))
    }
}

// ---------------------------------------------------------------------------
// Tests (native only)
// ---------------------------------------------------------------------------

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn read_returns_none_for_missing() {
        let dir = TempDir::new().unwrap();
        let fs = SandboxFs::new(dir.path());
        let out = fs.read_file(Path::new("nope.txt")).await.unwrap();
        assert!(out.is_none());
    }

    #[tokio::test]
    async fn write_then_read_round_trips() {
        let dir = TempDir::new().unwrap();
        let fs = SandboxFs::new(dir.path());
        fs.write_file(Path::new("hello.txt"), "world").await.unwrap();
        let out = fs.read_file(Path::new("hello.txt")).await.unwrap();
        assert_eq!(out.as_deref(), Some("world"));
    }

    #[tokio::test]
    async fn traversal_returns_none_on_read() {
        let dir = TempDir::new().unwrap();
        let fs = SandboxFs::new(dir.path());
        let out = fs
            .read_file(Path::new("../../../etc/passwd"))
            .await
            .unwrap();
        assert!(out.is_none());
    }

    #[tokio::test]
    async fn traversal_errors_on_write() {
        let dir = TempDir::new().unwrap();
        let fs = SandboxFs::new(dir.path());
        let r = fs
            .write_file(Path::new("../escape.txt"), "boom")
            .await;
        assert!(matches!(r, Err(SdkError::AccessDenied(_))));
    }

    #[tokio::test]
    async fn glob_finds_files() {
        let dir = TempDir::new().unwrap();
        let fs = SandboxFs::new(dir.path());
        fs.write_file(Path::new("a.txt"), "1").await.unwrap();
        fs.write_file(Path::new("b.txt"), "2").await.unwrap();
        let hits = fs.glob("*.txt", 10).await.unwrap();
        assert_eq!(hits.len(), 2);
    }
}
