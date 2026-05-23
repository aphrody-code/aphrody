// SPDX-License-Identifier: Apache-2.0
//! Git-backed workspace backup via pure-Rust `gix` (AH-9, feature `git`).
//!
//! openclaw shells out to the system `git` binary (`workspace.ts:433` runs
//! `git --version` / `git init`). aphrody uses `gix` so there is no shell-out,
//! no dependency on a system git, and the path is identical on Linux and
//! Windows. The module is host-only (gix does not build on wasm) and gated
//! behind the `git` feature; with the feature off, the crate still compiles
//! everywhere and [`AgentHome::git_backup`](crate::AgentHome::git_backup) is
//! simply absent.
//!
//! [`backup`] initialises the repository if needed, snapshots every file under
//! the workspace (excluding `.git`) into a tree, and writes a commit on `HEAD`
//! with the snapshot. It builds the tree from the worktree directly via gix's
//! object database — no index round-trip required for an append-only backup.

use std::collections::BTreeMap;
use std::path::Path;

use crate::HomeError;

/// Author/committer identity stamped on backup commits. Neutral, non-personal
/// (CLAUDE.md anonymisation rule).
const BACKUP_AUTHOR_NAME: &str = "aphrody-agent-home";
const BACKUP_AUTHOR_EMAIL: &str = "agent-home@aphrody.local";

/// Outcome of a [`backup`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupOutcome {
    /// Hex object id of the commit just written.
    pub commit_id: String,
    /// Whether the repository was freshly initialised by this call.
    pub initialized: bool,
    /// Number of files captured in the snapshot tree.
    pub files: usize,
}

fn git_err(e: impl std::fmt::Display) -> HomeError {
    HomeError::Git(e.to_string())
}

/// Initialise (if needed) the git repository at `workspace`, snapshot the
/// worktree, and write a commit on `HEAD`.
///
/// # Errors
/// [`HomeError::Git`] on any gix failure, [`HomeError::Io`] on filesystem
/// failure walking the worktree.
pub fn backup(workspace: &Path, message: &str) -> Result<BackupOutcome, HomeError> {
    std::fs::create_dir_all(workspace).map_err(|e| HomeError::io(workspace, e))?;

    let git_dir = workspace.join(".git");
    let (repo, initialized) = if git_dir.exists() {
        (gix::open(workspace).map_err(git_err)?, false)
    } else {
        (gix::init(workspace).map_err(git_err)?, true)
    };

    // Build the snapshot tree from the worktree.
    let (tree_id, files) = write_worktree_tree(&repo, workspace)?;

    // Determine the parent commit (HEAD), if the repo already has one.
    let parents: Vec<gix::ObjectId> = match repo.head_commit() {
        Ok(commit) => vec![commit.id],
        Err(_) => Vec::new(),
    };

    // Commit with a deterministic, non-personal identity. We set author +
    // committer via the configured signature override so the call does not
    // depend on a global git config being present.
    let commit_id = commit_tree(&repo, tree_id, &parents, message)?;

    Ok(BackupOutcome {
        commit_id: commit_id.to_hex().to_string(),
        initialized,
        files,
    })
}

/// Recursively snapshot the worktree into a gix tree, returning the tree id and
/// the file count. `.git` is skipped. Subdirectories become nested trees.
fn write_worktree_tree(
    repo: &gix::Repository,
    dir: &Path,
) -> Result<(gix::ObjectId, usize), HomeError> {
    let mut file_count = 0usize;
    let id = write_dir_tree(repo, dir, &mut file_count)?;
    Ok((id, file_count))
}

fn write_dir_tree(
    repo: &gix::Repository,
    dir: &Path,
    file_count: &mut usize,
) -> Result<gix::ObjectId, HomeError> {
    use gix::objs::tree::{Entry, EntryKind};

    // Collect entries sorted by filename for a deterministic tree (git requires
    // a specific entry ordering; gix sorts on write, but we sort for stability).
    let mut entries: BTreeMap<Vec<u8>, Entry> = BTreeMap::new();

    let read = std::fs::read_dir(dir).map_err(|e| HomeError::io(dir, e))?;
    for entry in read {
        let entry = entry.map_err(|e| HomeError::io(dir, e))?;
        let name = entry.file_name();
        let name_bytes = name.to_string_lossy().into_owned().into_bytes();
        // Skip the git directory itself.
        if name_bytes == b".git" {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| HomeError::io(&path, e))?;

        if file_type.is_dir() {
            let sub_id = write_dir_tree(repo, &path, file_count)?;
            // Skip empty subtrees (git has no concept of an empty directory).
            if is_empty_tree(repo, sub_id) {
                continue;
            }
            entries.insert(
                name_bytes.clone(),
                Entry {
                    mode: EntryKind::Tree.into(),
                    filename: name_bytes.into(),
                    oid: sub_id,
                },
            );
        } else if file_type.is_file() {
            let bytes = std::fs::read(&path).map_err(|e| HomeError::io(&path, e))?;
            let blob_id = repo.write_blob(&bytes).map_err(git_err)?.into();
            *file_count += 1;
            // Preserve the executable bit on Unix; default to a regular blob.
            let mode = file_mode(&path);
            entries.insert(
                name_bytes.clone(),
                Entry {
                    mode,
                    filename: name_bytes.into(),
                    oid: blob_id,
                },
            );
        }
        // Symlinks and other types are skipped from the backup snapshot.
    }

    let tree = gix::objs::Tree {
        entries: entries.into_values().collect(),
    };
    let id = repo.write_object(&tree).map_err(git_err)?.into();
    Ok(id)
}

/// The empty-tree object id (git's well-known empty tree).
fn is_empty_tree(repo: &gix::Repository, id: gix::ObjectId) -> bool {
    repo.find_object(id)
        .ok()
        .and_then(|o| o.try_into_tree().ok())
        .is_some_and(|t| t.iter().count() == 0)
}

#[cfg(unix)]
fn file_mode(path: &Path) -> gix::objs::tree::EntryMode {
    use std::os::unix::fs::PermissionsExt;
    let exec = std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false);
    if exec {
        gix::objs::tree::EntryKind::BlobExecutable.into()
    } else {
        gix::objs::tree::EntryKind::Blob.into()
    }
}

#[cfg(not(unix))]
fn file_mode(_path: &Path) -> gix::objs::tree::EntryMode {
    gix::objs::tree::EntryKind::Blob.into()
}

/// Write a commit object on `HEAD` pointing at `tree_id`. Uses a fixed
/// non-personal signature so the commit does not require a global git config.
fn commit_tree(
    repo: &gix::Repository,
    tree_id: gix::ObjectId,
    parents: &[gix::ObjectId],
    message: &str,
) -> Result<gix::ObjectId, HomeError> {
    use gix::actor::SignatureRef;

    // gix `SignatureRef::time` is the *raw* git time field (`<unix> <+HHMM>`),
    // borrowed from a backing string. Build it (UTC) and keep it alive for the
    // duration of the borrow.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let time_raw = format!("{secs} +0000");

    let sig = SignatureRef {
        name: BACKUP_AUTHOR_NAME.into(),
        email: BACKUP_AUTHOR_EMAIL.into(),
        time: time_raw.as_str(),
    };

    // gix's `commit_as` writes the commit object AND updates the HEAD ref +
    // reflog in one call, with explicit author/committer (no global config).
    let commit = repo
        .commit_as(sig, sig, "HEAD", message, tree_id, parents.iter().copied())
        .map_err(git_err)?;
    Ok(commit.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn backup_initializes_and_commits() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("SOUL.md"), "persona").unwrap();
        std::fs::write(ws.join("AGENTS.md"), "rules").unwrap();

        let outcome = backup(&ws, "initial backup").unwrap();
        assert!(outcome.initialized);
        assert_eq!(outcome.files, 2);
        assert_eq!(outcome.commit_id.len(), 40); // sha-1 hex
        assert!(ws.join(".git").exists());
    }

    #[test]
    fn second_backup_chains_a_parent() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("SOUL.md"), "v1").unwrap();
        let first = backup(&ws, "first").unwrap();
        assert!(first.initialized);

        std::fs::write(ws.join("SOUL.md"), "v2").unwrap();
        let second = backup(&ws, "second").unwrap();
        assert!(!second.initialized);
        assert_ne!(first.commit_id, second.commit_id);

        // The second commit must have the first as a parent.
        let repo = gix::open(&ws).unwrap();
        let head = repo.head_commit().unwrap();
        let parent_ids: Vec<_> = head.parent_ids().map(|id| id.to_hex().to_string()).collect();
        assert_eq!(parent_ids, vec![first.commit_id]);
    }

    #[test]
    fn backup_snapshots_nested_dirs() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        std::fs::create_dir_all(ws.join("memory")).unwrap();
        std::fs::write(ws.join("memory").join("2026-05-23.md"), "log").unwrap();
        std::fs::write(ws.join("SOUL.md"), "persona").unwrap();
        let outcome = backup(&ws, "nested").unwrap();
        assert_eq!(outcome.files, 2);
    }
}
