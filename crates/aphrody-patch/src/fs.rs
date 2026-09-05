// SPDX-License-Identifier: Apache-2.0
//! Pluggable filesystem abstraction used when applying patches.

use std::io;
use std::path::Path;

/// Filesystem operations needed to apply a patch.
///
/// Implementing this trait lets callers apply patches against in-memory
/// snapshots, sandboxes, or virtual filesystems instead of real disk. The
/// [`StdFileSystem`] implementation operates on [`std::fs`].
pub trait PatchFileSystem {
    /// Read a file's full contents as UTF-8 text.
    ///
    /// # Errors
    /// Returns an [`io::Error`] when the file cannot be read or is not UTF-8.
    fn read(&self, path: &Path) -> io::Result<String>;

    /// Write `contents` to `path`, creating it (and parent directories) if
    /// necessary and truncating any existing file.
    ///
    /// # Errors
    /// Returns an [`io::Error`] when the file (or a parent directory) cannot be
    /// created or written.
    fn write(&self, path: &Path, contents: &str) -> io::Result<()>;

    /// Remove a file.
    ///
    /// # Errors
    /// Returns an [`io::Error`] when the file cannot be removed.
    fn remove(&self, path: &Path) -> io::Result<()>;

    /// Whether `path` currently exists.
    fn exists(&self, path: &Path) -> bool;
}

/// [`PatchFileSystem`] backed by [`std::fs`].
///
/// `write` creates parent directories as needed. This type is zero-sized and
/// available on every target that has a filesystem (i.e. not the bare
/// `wasm32-unknown-unknown` target, where it still compiles but `std::fs`
/// operations return errors at runtime).
#[derive(Debug, Default, Clone, Copy)]
pub struct StdFileSystem;

impl PatchFileSystem for StdFileSystem {
    fn read(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, contents)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}
