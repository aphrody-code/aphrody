// SPDX-License-Identifier: Apache-2.0
//! `WorkspaceGuard` — path-containment sandbox for workspace reads (AH-8).
//!
//! Port of openclaw `openRootFile` boundary semantics + the `contained_in`
//! lexical guard from `aphrody-skills::runtime::plugin_manifest`. Every read
//! of a bootstrap file goes through [`WorkspaceGuard::resolve`], which:
//!
//! 1. lexically normalises the candidate (collapse `.` / `..`) so a hostile
//!    relative component cannot escape;
//! 2. canonicalises the workspace root once (following symlinks) so the
//!    boundary is the real directory;
//! 3. rejects any candidate that does not stay within the root;
//! 4. enforces the `MAX_WORKSPACE_BOOTSTRAP_FILE_BYTES` per-file cap
//!    (openclaw `workspace.ts:40`).
//!
//! The guard is pure (no async, no global state) so it is trivially
//! `Send + Sync` and reusable across the runtime.

use std::path::{Component, Path, PathBuf};

use crate::filenames::MAX_WORKSPACE_BOOTSTRAP_FILE_BYTES;
use crate::HomeError;

/// A canonicalised workspace root that vends boundary-checked child paths.
#[derive(Debug, Clone)]
pub struct WorkspaceGuard {
    /// The canonical (symlink-resolved when it exists) workspace root.
    root: PathBuf,
    /// Per-file byte cap.
    max_bytes: u64,
}

/// Lexical normalisation (no filesystem access): collapse `.` and `..`.
/// Identical to `plugin_manifest::normalize`.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Path-traversal guard: `target` must equal or sit under `base`.
/// Identical to `plugin_manifest::contained_in`.
#[must_use]
pub fn contained_in(target: &Path, base: &Path) -> bool {
    let nb = normalize(base);
    let nt = normalize(target);
    nt == nb || nt.starts_with(&nb)
}

impl WorkspaceGuard {
    /// Build a guard for `root`. The root is canonicalised when it exists on
    /// disk; otherwise the lexically-normalised path is used (so the guard
    /// works for not-yet-created workspaces during onboarding).
    #[must_use]
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| normalize(root));
        Self {
            root: canonical,
            max_bytes: MAX_WORKSPACE_BOOTSTRAP_FILE_BYTES,
        }
    }

    /// Override the per-file byte cap (defaults to 2 MiB).
    #[must_use]
    pub fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    /// The canonical root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The configured per-file byte cap.
    #[must_use]
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    /// Resolve a workspace-relative (or absolute-within-root) path to an
    /// absolute path, rejecting anything that escapes the boundary.
    ///
    /// # Errors
    /// [`HomeError::PathEscape`] when the candidate leaves the workspace.
    pub fn resolve(&self, candidate: impl AsRef<Path>) -> Result<PathBuf, HomeError> {
        let candidate = candidate.as_ref();
        let joined = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.root.join(candidate)
        };
        let normalized = normalize(&joined);
        if contained_in(&normalized, &self.root) {
            Ok(normalized)
        } else {
            Err(HomeError::PathEscape {
                path: normalized,
                root: self.root.clone(),
            })
        }
    }

    /// Boundary-check + cap-check a file, then read it to a `String`.
    ///
    /// Returns `Ok(None)` when the file does not exist (the bootstrap file is
    /// simply absent — not an error). Returns [`HomeError::FileTooLarge`] when
    /// the file exceeds the cap, and [`HomeError::PathEscape`] on traversal.
    ///
    /// # Errors
    /// See above, plus [`HomeError::Io`] for read failures.
    pub fn read_to_string(
        &self,
        candidate: impl AsRef<Path>,
    ) -> Result<Option<String>, HomeError> {
        let path = self.resolve(candidate)?;
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(HomeError::io(path, e)),
        };
        if !meta.is_file() {
            return Ok(None);
        }
        if meta.len() > self.max_bytes {
            return Err(HomeError::FileTooLarge {
                path,
                size: meta.len(),
                cap: self.max_bytes,
            });
        }
        match std::fs::read_to_string(&path) {
            Ok(s) => Ok(Some(s)),
            Err(e) => Err(HomeError::io(path, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolves_child_within_root() {
        let td = tempdir().unwrap();
        let guard = WorkspaceGuard::new(td.path());
        let resolved = guard.resolve("SOUL.md").unwrap();
        assert!(contained_in(&resolved, guard.root()));
    }

    #[test]
    fn rejects_parent_escape() {
        let td = tempdir().unwrap();
        let guard = WorkspaceGuard::new(td.path());
        let err = guard.resolve("../../etc/passwd").unwrap_err();
        assert!(matches!(err, HomeError::PathEscape { .. }));
    }

    #[test]
    fn rejects_absolute_outside_root() {
        let td = tempdir().unwrap();
        let guard = WorkspaceGuard::new(td.path());
        let outside = if cfg!(windows) { "C:\\Windows\\system.ini" } else { "/etc/hosts" };
        let err = guard.resolve(outside).unwrap_err();
        assert!(matches!(err, HomeError::PathEscape { .. }));
    }

    #[test]
    fn missing_file_reads_as_none() {
        let td = tempdir().unwrap();
        let guard = WorkspaceGuard::new(td.path());
        assert_eq!(guard.read_to_string("ABSENT.md").unwrap(), None);
    }

    #[test]
    fn reads_existing_file() {
        let td = tempdir().unwrap();
        std::fs::write(td.path().join("SOUL.md"), "persona").unwrap();
        let guard = WorkspaceGuard::new(td.path());
        assert_eq!(guard.read_to_string("SOUL.md").unwrap().as_deref(), Some("persona"));
    }

    #[test]
    fn enforces_byte_cap() {
        let td = tempdir().unwrap();
        std::fs::write(td.path().join("BIG.md"), "0123456789").unwrap();
        let guard = WorkspaceGuard::new(td.path()).with_max_bytes(4);
        let err = guard.read_to_string("BIG.md").unwrap_err();
        assert!(matches!(err, HomeError::FileTooLarge { size: 10, cap: 4, .. }));
    }
}
