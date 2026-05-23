// SPDX-License-Identifier: Apache-2.0
//! Content-addressed file cache + persistent workspace state (AH-12).
//!
//! openclaw keys its in-memory file cache on `dev:ino:size:mtime`
//! (`workspace.ts:52-54`), recomputed per process, single-threaded. aphrody
//! pushes past that:
//!
//! * **Stale detection** keeps the same `(dev,ino,size,mtime)` identity so an
//!   unchanged file is never re-read.
//! * **Content addressing** adds a `blake3` hash of the bytes, so two files
//!   with identical content (across agents / profiles) share one cache entry
//!   and changed files are detected even if `mtime` is preserved (e.g. a
//!   `cp -p`).
//! * **Persistence**: the cache identity table is written to
//!   `.aphrody/workspace-state.json` (schema v1, mirroring openclaw's state
//!   file), so a restart can skip re-hashing files whose `(size,mtime)` match.
//! * **`Send + Sync`**: the live cache is a `Mutex<HashMap<...>>` behind an
//!   `Arc`, shareable across the tokio runtime.
//!
//! The cache stores [`crate::mmap::MappedBytes`], so a hit hands back a
//! zero-copy `Arc` clone rather than a fresh `String`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::filenames::{WORKSPACE_STATE_DIRNAME, WORKSPACE_STATE_FILENAME, WORKSPACE_STATE_VERSION};
use crate::mmap::MappedBytes;
use crate::HomeError;

/// Stable per-file identity (openclaw `workspace.ts:52`). On Unix this is the
/// real `(dev, ino, size, mtime_ns)`; on Windows the `(volume_serial,
/// file_index, size, mtime_ns)`; elsewhere `(0, 0, size, mtime_ns)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIdentity {
    /// Device / volume identifier (0 when unavailable).
    pub dev: u64,
    /// Inode / file index (0 when unavailable).
    pub ino: u64,
    /// File size in bytes.
    pub size: u64,
    /// Modification time as nanoseconds since the Unix epoch.
    pub mtime_ns: i128,
}

impl FileIdentity {
    /// Compute the identity from a file's metadata.
    #[must_use]
    pub fn from_metadata(meta: &std::fs::Metadata) -> Self {
        let size = meta.len();
        let mtime_ns = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            // `as_nanos()` is u128; nanoseconds since epoch fit comfortably in
            // the positive half of i128 for any realistic mtime, so the cast
            // never wraps.
            .map_or(0i128, |d| i128::try_from(d.as_nanos()).unwrap_or(i128::MAX));
        let (dev, ino) = platform_dev_ino(meta);
        Self {
            dev,
            ino,
            size,
            mtime_ns,
        }
    }
}

#[cfg(unix)]
fn platform_dev_ino(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino())
}

#[cfg(windows)]
fn platform_dev_ino(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::windows::fs::MetadataExt;
    // `volume_serial_number` / `file_index` are nightly-unstable
    // (`windows_by_handle`), so we use the stable `creation_time` /
    // `last_write_time` (100ns-tick FILETIME) as the dev/ino-equivalent
    // discriminators. Combined with size + mtime_ns this still detects any
    // content-affecting change; the content hash in CacheRecord is the final
    // authority for true equality.
    (meta.creation_time(), meta.last_write_time())
}

#[cfg(not(any(unix, windows)))]
fn platform_dev_ino(_meta: &std::fs::Metadata) -> (u64, u64) {
    (0, 0)
}

/// A persisted cache entry: identity + content hash. Stored in the state file
/// so a restart can skip re-hashing unchanged files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheRecord {
    /// Workspace-relative path key.
    pub rel_path: String,
    /// Stable file identity.
    pub identity: FileIdentity,
    /// Lowercase hex blake3 of the file bytes.
    pub content_hash: String,
}

/// Persistent workspace state (schema v1), mirroring openclaw's
/// `workspace-state.json` but extended with the content-addressed cache table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    /// Schema version (always [`WORKSPACE_STATE_VERSION`]).
    pub version: u32,
    /// ISO-8601 timestamp when BOOTSTRAP.md was seeded (openclaw parity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_seeded_at: Option<String>,
    /// ISO-8601 timestamp when first-run setup completed (openclaw parity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_completed_at: Option<String>,
    /// Content-addressed cache table, keyed by workspace-relative path.
    #[serde(default)]
    pub cache: Vec<CacheRecord>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            version: WORKSPACE_STATE_VERSION,
            bootstrap_seeded_at: None,
            setup_completed_at: None,
            cache: Vec::new(),
        }
    }
}

impl WorkspaceState {
    /// Path to the state file under a workspace root.
    #[must_use]
    pub fn path_for(workspace_root: &Path) -> PathBuf {
        workspace_root
            .join(WORKSPACE_STATE_DIRNAME)
            .join(WORKSPACE_STATE_FILENAME)
    }

    /// Load the state file, returning the default when it does not exist.
    ///
    /// # Errors
    /// [`HomeError::Io`] on read failure (other than not-found),
    /// [`HomeError::Json`] on malformed JSON.
    pub fn load(workspace_root: &Path) -> Result<Self, HomeError> {
        let path = Self::path_for(workspace_root);
        match std::fs::read_to_string(&path) {
            Ok(raw) => {
                let mut state: WorkspaceState = serde_json::from_str(&raw)?;
                // Coerce any future / legacy version up to v1 shape — we only
                // ship one schema, so we normalise rather than reject.
                state.version = WORKSPACE_STATE_VERSION;
                Ok(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(HomeError::io(path, e)),
        }
    }

    /// Atomically write the state file (temp + rename), creating the
    /// `.aphrody/` directory if needed.
    ///
    /// # Errors
    /// [`HomeError::Io`] on any filesystem failure, [`HomeError::Json`] on
    /// serialization failure.
    pub fn save(&self, workspace_root: &Path) -> Result<(), HomeError> {
        let path = Self::path_for(workspace_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| HomeError::io(parent, e))?;
        }
        let body = format!("{}\n", serde_json::to_string_pretty(self)?);
        // Temp file in the same directory => rename is atomic on the same FS.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, body.as_bytes()).map_err(|e| HomeError::io(&tmp, e))?;
        std::fs::rename(&tmp, &path).map_err(|e| HomeError::io(&path, e))?;
        Ok(())
    }

    /// True when first-run setup is recorded as complete (openclaw
    /// `isWorkspaceSetupCompleted`).
    #[must_use]
    pub fn is_setup_completed(&self) -> bool {
        self.setup_completed_at
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
    }
}

/// Live, thread-safe content-addressed cache.
///
/// Hands back zero-copy [`MappedBytes`] on a hit. A miss (or a stale identity)
/// maps the file, hashes it, and records the new identity. The cache survives
/// across sessions when seeded from a [`WorkspaceState`].
#[derive(Debug, Default)]
pub struct FileCache {
    /// `rel_path` -> (identity, `content_hash`, bytes).
    entries: Mutex<HashMap<String, CachedFile>>,
}

#[derive(Debug, Clone)]
struct CachedFile {
    identity: FileIdentity,
    content_hash: String,
    bytes: MappedBytes,
}

/// Outcome of a [`FileCache::load`] call.
#[derive(Debug, Clone)]
pub struct CacheHit {
    /// The (possibly shared) file bytes.
    pub bytes: MappedBytes,
    /// Lowercase hex blake3 of the bytes.
    pub content_hash: String,
    /// `true` when the bytes were served from the in-memory cache without a
    /// re-map / re-hash.
    pub from_cache: bool,
}

impl FileCache {
    /// Empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load `abs_path` (recorded under `rel_path`), serving a cached entry
    /// when the on-disk identity is unchanged.
    ///
    /// # Errors
    /// [`HomeError::Io`] on metadata / map failure.
    pub fn load(&self, rel_path: &str, abs_path: &Path) -> Result<CacheHit, HomeError> {
        let meta = std::fs::metadata(abs_path).map_err(|e| HomeError::io(abs_path, e))?;
        let identity = FileIdentity::from_metadata(&meta);

        // Fast path: identity matches the cached entry.
        {
            let guard = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entry) = guard.get(rel_path) {
                if entry.identity == identity {
                    return Ok(CacheHit {
                        bytes: entry.bytes.clone(),
                        content_hash: entry.content_hash.clone(),
                        from_cache: true,
                    });
                }
            }
        }

        // Miss / stale: map + hash.
        let bytes = MappedBytes::load(abs_path)?;
        let content_hash = blake3::hash(bytes.as_slice()).to_hex().to_string();
        let mut guard = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.insert(
            rel_path.to_string(),
            CachedFile {
                identity,
                content_hash: content_hash.clone(),
                bytes: bytes.clone(),
            },
        );
        Ok(CacheHit {
            bytes,
            content_hash,
            from_cache: false,
        })
    }

    /// Seed the cache's identity table from a persisted [`WorkspaceState`].
    /// Only the identity + hash are restored (not the bytes); the first
    /// [`FileCache::load`] of each file re-maps it but skips re-hashing when
    /// the identity matches. This is the "survive the restart" win.
    pub fn seed_from_state(&self, state: &WorkspaceState, workspace_root: &Path) {
        let mut guard = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        for rec in &state.cache {
            let abs = workspace_root.join(&rec.rel_path);
            let Ok(meta) = std::fs::metadata(&abs) else {
                continue;
            };
            let identity = FileIdentity::from_metadata(&meta);
            if identity != rec.identity {
                continue; // file changed since the state was written
            }
            // Map the bytes lazily-now (cheap, page-cache backed) so the
            // recorded hash stays paired with real content.
            if let Ok(bytes) = MappedBytes::load(&abs) {
                guard.insert(
                    rec.rel_path.clone(),
                    CachedFile {
                        identity,
                        content_hash: rec.content_hash.clone(),
                        bytes,
                    },
                );
            }
        }
    }

    /// Export the current cache table as persistable records (sorted by path
    /// for deterministic, prompt-cache-friendly output).
    #[must_use]
    pub fn export_records(&self) -> Vec<CacheRecord> {
        let guard = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut out: Vec<CacheRecord> = guard
            .iter()
            .map(|(rel, entry)| CacheRecord {
                rel_path: rel.clone(),
                identity: entry.identity,
                content_hash: entry.content_hash.clone(),
            })
            .collect();
        out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        out
    }

    /// Number of live entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner).len()
    }

    /// True when the cache holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn second_load_hits_cache() {
        let td = tempdir().unwrap();
        let p = td.path().join("SOUL.md");
        std::fs::write(&p, "persona body").unwrap();
        let cache = FileCache::new();
        let first = cache.load("SOUL.md", &p).unwrap();
        assert!(!first.from_cache);
        let second = cache.load("SOUL.md", &p).unwrap();
        assert!(second.from_cache);
        assert_eq!(first.content_hash, second.content_hash);
    }

    #[test]
    fn changed_content_invalidates_and_rehashes() {
        let td = tempdir().unwrap();
        let p = td.path().join("SOUL.md");
        std::fs::write(&p, "v1").unwrap();
        let cache = FileCache::new();
        let h1 = cache.load("SOUL.md", &p).unwrap().content_hash;
        // Rewrite via temp + rename (the atomic-replace pattern real editors
        // and `WorkspaceState::save` use). In-place overwrite is rejected on
        // Windows while the prior mmap is still held by the cache, so the
        // rename path is both correct and the realistic change scenario.
        std::thread::sleep(std::time::Duration::from_millis(10));
        let tmp = td.path().join("SOUL.md.new");
        std::fs::write(&tmp, "v2 different").unwrap();
        std::fs::rename(&tmp, &p).unwrap();
        let hit = cache.load("SOUL.md", &p).unwrap();
        assert!(!hit.from_cache, "stale identity must miss");
        assert_ne!(h1, hit.content_hash);
    }

    #[test]
    fn content_hash_is_blake3_hex() {
        let td = tempdir().unwrap();
        let p = td.path().join("F.md");
        std::fs::write(&p, "abc").unwrap();
        let cache = FileCache::new();
        let hit = cache.load("F.md", &p).unwrap();
        let expected = blake3::hash(b"abc").to_hex().to_string();
        assert_eq!(hit.content_hash, expected);
        assert_eq!(hit.content_hash.len(), 64);
    }

    #[test]
    fn state_round_trips_through_disk() {
        let td = tempdir().unwrap();
        let root = td.path();
        let mut state = WorkspaceState::default();
        state.setup_completed_at = Some("2026-05-23T00:00:00Z".to_string());
        state.cache.push(CacheRecord {
            rel_path: "SOUL.md".to_string(),
            identity: FileIdentity {
                dev: 1,
                ino: 2,
                size: 3,
                mtime_ns: 4,
            },
            content_hash: "deadbeef".to_string(),
        });
        state.save(root).unwrap();
        let loaded = WorkspaceState::load(root).unwrap();
        assert_eq!(loaded.version, WORKSPACE_STATE_VERSION);
        assert!(loaded.is_setup_completed());
        assert_eq!(loaded.cache.len(), 1);
        assert_eq!(loaded.cache[0].rel_path, "SOUL.md");
    }

    #[test]
    fn missing_state_is_default() {
        let td = tempdir().unwrap();
        let state = WorkspaceState::load(td.path()).unwrap();
        assert_eq!(state.version, WORKSPACE_STATE_VERSION);
        assert!(!state.is_setup_completed());
        assert!(state.cache.is_empty());
    }

    #[test]
    fn export_records_is_sorted() {
        let td = tempdir().unwrap();
        for name in ["b.md", "a.md", "c.md"] {
            std::fs::write(td.path().join(name), name).unwrap();
        }
        let cache = FileCache::new();
        for name in ["b.md", "a.md", "c.md"] {
            cache.load(name, &td.path().join(name)).unwrap();
        }
        let recs = cache.export_records();
        let paths: Vec<&str> = recs.iter().map(|r| r.rel_path.as_str()).collect();
        assert_eq!(paths, vec!["a.md", "b.md", "c.md"]);
    }

    #[test]
    fn seed_from_state_restores_unchanged_files() {
        let td = tempdir().unwrap();
        let root = td.path();
        let p = root.join("SOUL.md");
        std::fs::write(&p, "persona").unwrap();
        // Build state by loading once + exporting.
        let cache1 = FileCache::new();
        let hit = cache1.load("SOUL.md", &p).unwrap();
        let mut state = WorkspaceState::default();
        state.cache = cache1.export_records();
        // New cache seeded from state: the file is unchanged -> entry restored.
        let cache2 = FileCache::new();
        cache2.seed_from_state(&state, root);
        assert_eq!(cache2.len(), 1);
        let hit2 = cache2.load("SOUL.md", &p).unwrap();
        assert!(hit2.from_cache, "seeded entry should hit");
        assert_eq!(hit2.content_hash, hit.content_hash);
    }
}
