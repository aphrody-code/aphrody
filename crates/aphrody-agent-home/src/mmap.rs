// SPDX-License-Identifier: Apache-2.0
//! Zero-copy mapped bytes for workspace bootstrap files (AH-11).
//!
//! This is the ONLY module in the crate that uses `unsafe`. The crate is
//! `#![forbid(unsafe_code)]` everywhere else; here we scope a local
//! `#![allow(unsafe_code)]` to the single `memmap2::Mmap::map` call, which is
//! `unsafe` because the OS mapping aliases a file whose bytes another process
//! could mutate. We accept that contract for read-only bootstrap files: the
//! content-addressed cache (cache.rs) re-validates `(dev,ino,size,mtime)` on
//! every open, so a changed file is detected and re-mapped rather than read
//! stale.
//!
//! On `wasm32` (and any target without a real filesystem mapping) the whole
//! mmap path is cfg-gated out and [`MappedBytes::load`] falls back to a plain
//! in-memory read returning an `Arc<[u8]>`. Both variants `Deref` to `&[u8]`,
//! so callers are oblivious to which path produced the bytes.

#![allow(unsafe_code)]

use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use crate::HomeError;

/// Backing storage for a mapped (or read) file.
#[derive(Clone)]
enum Backing {
    /// Host path: a shared, page-cache-backed memory map.
    #[cfg(not(target_arch = "wasm32"))]
    Mmap(Arc<memmap2::Mmap>),
    /// Fallback / wasm path, and host-side empty-file path: heap bytes.
    /// (mmap of a zero-length file is invalid on several platforms, so an
    /// empty file always uses this variant even on host.)
    Heap(Arc<[u8]>),
}

/// A read-only view over a workspace file's bytes, cheap to clone (`Arc`).
///
/// Shared across every session and thread that needs the file — exactly the
/// "map once, share everywhere" win called out in the plan §3 over openclaw's
/// per-session `String` copy.
#[derive(Clone)]
pub struct MappedBytes {
    backing: Backing,
}

impl std::fmt::Debug for MappedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedBytes")
            .field("len", &self.len())
            .field("backed_by", &self.backing_kind())
            .finish()
    }
}

impl MappedBytes {
    /// Load `path` into a [`MappedBytes`].
    ///
    /// On host targets this memory-maps the file (zero-copy, shared via the OS
    /// page cache). On wasm, or for empty files, it reads into heap bytes.
    ///
    /// # Errors
    /// [`HomeError::Io`] when the file cannot be opened / mapped / read.
    pub fn load(path: &Path) -> Result<Self, HomeError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::load_mmap(path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::load_heap(path)
        }
    }

    /// Construct directly from owned bytes (used by tests and the cache when
    /// it already holds the content).
    #[must_use]
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            backing: Backing::Heap(bytes.into()),
        }
    }

    /// Host-only memory-mapping path.
    #[cfg(not(target_arch = "wasm32"))]
    fn load_mmap(path: &Path) -> Result<Self, HomeError> {
        let file = std::fs::File::open(path).map_err(|e| HomeError::io(path, e))?;
        let len = file
            .metadata()
            .map_err(|e| HomeError::io(path, e))?
            .len();
        // Mapping a zero-length file is UB / errors on multiple platforms.
        // Use a heap empty slice instead — same observable behaviour.
        if len == 0 {
            return Ok(Self {
                backing: Backing::Heap(Arc::from(&[][..])),
            });
        }
        // SAFETY: We map a file we just opened read-only. memmap2 requires the
        // caller to ensure the underlying file is not concurrently truncated
        // or mutated in a way that invalidates the mapping for the lifetime of
        // the `Mmap`. For bootstrap files this holds in practice (they are
        // owned by the agent home), and the content-addressed cache detects
        // any change via `(dev,ino,size,mtime)` and drops the stale mapping
        // before the bytes are re-read. The mapping is read-only and never
        // written through.
        let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(|e| HomeError::io(path, e))?;
        Ok(Self {
            backing: Backing::Mmap(Arc::new(mmap)),
        })
    }

    /// Heap read path (wasm fallback; also reusable on host).
    #[cfg(target_arch = "wasm32")]
    fn load_heap(path: &Path) -> Result<Self, HomeError> {
        let bytes = std::fs::read(path).map_err(|e| HomeError::io(path, e))?;
        Ok(Self {
            backing: Backing::Heap(Arc::from(bytes.into_boxed_slice())),
        })
    }

    /// Byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// True if zero-length.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    /// The bytes as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        match &self.backing {
            #[cfg(not(target_arch = "wasm32"))]
            Backing::Mmap(m) => &m[..],
            Backing::Heap(h) => &h[..],
        }
    }

    /// Interpret the bytes as UTF-8, returning a borrowed `&str`.
    ///
    /// # Errors
    /// Propagates [`std::str::Utf8Error`] when the bytes are not valid UTF-8.
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.as_slice())
    }

    /// Lossy UTF-8 view, never failing (used for display / hashing of files
    /// that may contain stray bytes).
    #[must_use]
    pub fn to_string_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.as_slice())
    }

    /// Whether the backing is a real memory map (`false` on wasm / empty).
    #[must_use]
    pub fn backing_kind(&self) -> &'static str {
        match &self.backing {
            #[cfg(not(target_arch = "wasm32"))]
            Backing::Mmap(_) => "mmap",
            Backing::Heap(_) => "heap",
        }
    }
}

impl Deref for MappedBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loads_file_zero_copy_view() {
        let td = tempdir().unwrap();
        let p = td.path().join("SOUL.md");
        std::fs::write(&p, "hello persona").unwrap();
        let m = MappedBytes::load(&p).unwrap();
        assert_eq!(m.as_str().unwrap(), "hello persona");
        assert_eq!(m.len(), 13);
        assert!(!m.is_empty());
    }

    #[test]
    fn empty_file_uses_heap_backing() {
        let td = tempdir().unwrap();
        let p = td.path().join("EMPTY.md");
        std::fs::write(&p, "").unwrap();
        let m = MappedBytes::load(&p).unwrap();
        assert!(m.is_empty());
        assert_eq!(m.backing_kind(), "heap");
    }

    #[test]
    fn from_bytes_round_trips() {
        let m = MappedBytes::from_bytes(b"abc".to_vec().into_boxed_slice() as Box<[u8]>);
        assert_eq!(&*m, b"abc");
        assert_eq!(m.backing_kind(), "heap");
    }

    #[test]
    fn clone_shares_backing_cheaply() {
        let td = tempdir().unwrap();
        let p = td.path().join("F.md");
        std::fs::write(&p, "shared").unwrap();
        let a = MappedBytes::load(&p).unwrap();
        let b = a.clone();
        assert_eq!(a.as_slice(), b.as_slice());
    }

    #[test]
    fn missing_file_is_io_error() {
        let td = tempdir().unwrap();
        let err = MappedBytes::load(&td.path().join("nope.md")).unwrap_err();
        assert!(matches!(err, HomeError::Io { .. }));
    }
}
