// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// The on-disk model store: layout, registry, eviction.
//
// Layout under the store root (default `~/.aphrody/models`):
//
//   registry.json                     index of every tracked artefact
//   hf/<owner>/<repo>/<rev>/<file>    Hugging Face artefacts
//   url/<url-digest-16>/<basename>    direct-URL artefacts
//   <anything>.part                   in-flight download (never indexed)
//
// The registry is the source of truth for metadata (digest, format, usage
// timestamps); the files on disk are the source of truth for existence.
// `reconcile` walks both and reports the drift, so a manually deleted file or
// a manually dropped-in artefact never leaves the store lying.
//
// Concurrency: the registry is rewritten by writing a sibling temporary file
// and renaming it over the target, which is atomic on both NTFS and every
// POSIX filesystem aphrody targets. Two processes racing a write therefore
// yield one of the two complete registries, never a torn one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::digest;
use crate::error::{ModelError, Result};
use crate::id::ModelRef;
use crate::inspect::{self, ArtifactFormat, Inspection};

/// File name of the registry index inside the store root.
pub const REGISTRY_FILE: &str = "registry.json";

/// Schema version of `registry.json`. Bumped whenever the on-disk shape
/// changes incompatibly; an unknown version is refused rather than guessed.
pub const REGISTRY_VERSION: u32 = 1;

/// Extension used for a download still in flight.
pub(crate) const PART_SUFFIX: &str = ".part";

/// One tracked artefact.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InstalledModel {
    /// Canonical reference this artefact was installed from.
    #[serde(rename = "ref")]
    pub reference: ModelRef,
    /// Absolute path of the artefact on this machine.
    ///
    /// Not serialised: the registry persists a root-relative path so a store
    /// stays valid when the home directory moves. Rebuilt on load.
    #[serde(skip)]
    pub path: PathBuf,
    /// Path as persisted: relative to the store root, or absolute for an
    /// adopted [`ModelRef::Local`] artefact that lives outside the store.
    #[serde(rename = "path")]
    pub stored_path: String,
    /// Artefact size in bytes.
    pub bytes: u64,
    /// Lower-case hex SHA-256 over the whole artefact.
    pub sha256: String,
    /// Container format detected at install time.
    pub format: ArtifactFormat,
    /// Unix seconds when the artefact was first recorded.
    pub installed_at: u64,
    /// Unix seconds of the last [`ModelStore::touch`] call, if any.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_used_at: Option<u64>,
    /// Catalog id this artefact was pulled through, when applicable.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub catalog_id: Option<String>,
    /// Header inspection captured at install time.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub inspection: Option<Inspection>,
}

impl InstalledModel {
    /// RFC 3339 rendering of [`Self::installed_at`].
    #[must_use]
    pub fn installed_at_rfc3339(&self) -> String {
        rfc3339(self.installed_at)
    }

    /// RFC 3339 rendering of [`Self::last_used_at`], when set.
    #[must_use]
    pub fn last_used_at_rfc3339(&self) -> Option<String> {
        self.last_used_at.map(rfc3339)
    }

    /// Effective recency key for eviction: last use, else install time.
    #[must_use]
    pub const fn recency(&self) -> u64 {
        match self.last_used_at {
            Some(t) => t,
            None => self.installed_at,
        }
    }
}

/// Render unix seconds as RFC 3339 UTC.
fn rfc3339(unix_secs: u64) -> String {
    chrono::DateTime::from_timestamp(i64::try_from(unix_secs).unwrap_or(0), 0)
        .map_or_else(|| "1970-01-01T00:00:00Z".to_owned(), |dt| dt.to_rfc3339())
}

/// Current wall clock as unix seconds (0 if the clock predates the epoch).
pub(crate) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The persisted registry document.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RegistryDoc {
    version: u32,
    #[serde(default)]
    entries: Vec<InstalledModel>,
}

/// Drift between the registry and what is actually on disk.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct ReconcileReport {
    /// Registry entries whose file has disappeared (dropped from the index).
    pub missing: Vec<String>,
    /// Artefacts found under the store root that no entry claims.
    pub untracked: Vec<PathBuf>,
    /// Leftover `.part` files from interrupted downloads.
    pub stale_parts: Vec<PathBuf>,
}

impl ReconcileReport {
    /// Whether the store and its registry agree.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.untracked.is_empty() && self.stale_parts.is_empty()
    }
}

/// Outcome of a [`ModelStore::verify`] pass.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VerifyReport {
    /// The reference that was verified.
    pub reference: String,
    /// Digest recorded at install time.
    pub expected_sha256: String,
    /// Digest recomputed from the bytes on disk right now.
    pub actual_sha256: String,
    /// Size recorded at install time.
    pub expected_bytes: u64,
    /// Size on disk right now.
    pub actual_bytes: u64,
}

impl VerifyReport {
    /// Whether the artefact still matches what was installed.
    #[must_use]
    pub fn is_intact(&self) -> bool {
        self.expected_sha256 == self.actual_sha256 && self.expected_bytes == self.actual_bytes
    }
}

/// What a garbage-collection pass removed.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct GcReport {
    /// References evicted, least-recently-used first.
    pub evicted: Vec<String>,
    /// Bytes reclaimed by the eviction.
    pub reclaimed_bytes: u64,
    /// Bytes still held after the pass.
    pub remaining_bytes: u64,
    /// Stale `.part` files deleted during the pass.
    pub removed_parts: Vec<PathBuf>,
}

/// A local model store rooted at a directory.
#[derive(Debug, Clone)]
pub struct ModelStore {
    root: PathBuf,
}

impl ModelStore {
    /// Open the default store, creating the directory tree if needed.
    ///
    /// Root resolution, in order: `$APHRODY_MODELS_DIR`, then
    /// `<aphrody-state-dir>/models` where the state dir is `$APHRODY_HOME`,
    /// `%USERPROFILE%/.aphrody`, `$HOME/.aphrody` or the platform home. This
    /// mirrors `aphrody-embed`'s cache resolution so both crates share one
    /// `~/.aphrody/models` namespace.
    ///
    /// # Errors
    ///
    /// [`ModelError::NoStateDir`] when no home can be resolved, or
    /// [`ModelError::Io`] when the directory cannot be created.
    pub fn open() -> Result<Self> {
        Self::with_root(default_root()?)
    }

    /// Open a store at an explicit root, creating it if needed.
    ///
    /// # Errors
    ///
    /// [`ModelError::Io`] when the directory cannot be created.
    pub fn with_root(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|e| ModelError::io(root.clone(), e))?;
        Ok(Self { root })
    }

    /// The store root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Absolute path where `reference` lives (or would live once pulled).
    ///
    /// For [`ModelRef::Local`] this is the referenced path itself.
    #[must_use]
    pub fn path_for(&self, reference: &ModelRef) -> PathBuf {
        match reference.relative_path() {
            Some(rel) => self.root.join(rel),
            None => match reference {
                ModelRef::Local(p) => p.clone(),
                // `relative_path` only returns None for Local refs.
                _ => self.root.clone(),
            },
        }
    }

    /// Path of the in-flight download file for `reference`.
    #[must_use]
    pub fn part_path_for(&self, reference: &ModelRef) -> PathBuf {
        let final_path = self.path_for(reference);
        let mut name = final_path.file_name().unwrap_or_default().to_os_string();
        name.push(PART_SUFFIX);
        final_path.with_file_name(name)
    }

    /// Whether the artefact bytes are already present on disk.
    #[must_use]
    pub fn is_present(&self, reference: &ModelRef) -> bool {
        self.path_for(reference).is_file()
    }

    // -- registry --------------------------------------------------------

    /// Absolute path of `registry.json`.
    #[must_use]
    pub fn registry_path(&self) -> PathBuf {
        self.root.join(REGISTRY_FILE)
    }

    /// Every tracked artefact, sorted by reference.
    ///
    /// # Errors
    ///
    /// [`ModelError::Registry`] if the index is corrupt, [`ModelError::Io`]
    /// if it cannot be read.
    pub fn list(&self) -> Result<Vec<InstalledModel>> {
        Ok(self.load_registry()?.into_values().collect())
    }

    /// Look up one artefact by reference.
    ///
    /// # Errors
    ///
    /// Propagates registry read failures.
    pub fn get(&self, reference: &ModelRef) -> Result<Option<InstalledModel>> {
        Ok(self.load_registry()?.remove(&reference.to_string()))
    }

    /// Sum of every tracked artefact's size.
    ///
    /// # Errors
    ///
    /// Propagates registry read failures.
    pub fn total_bytes(&self) -> Result<u64> {
        Ok(self.load_registry()?.values().map(|m| m.bytes).sum())
    }

    /// Insert or replace a registry entry.
    ///
    /// # Errors
    ///
    /// Propagates registry read/write failures.
    pub fn record(&self, entry: InstalledModel) -> Result<()> {
        let mut registry = self.load_registry()?;
        registry.insert(entry.reference.to_string(), entry);
        self.save_registry(&registry)
    }

    /// Stamp `last_used_at` on an entry, so eviction can order by recency.
    ///
    /// A reference that is not installed is a no-op returning `false`, which
    /// keeps callers from having to pre-check before every inference run.
    ///
    /// # Errors
    ///
    /// Propagates registry read/write failures.
    pub fn touch(&self, reference: &ModelRef) -> Result<bool> {
        let mut registry = self.load_registry()?;
        let Some(entry) = registry.get_mut(&reference.to_string()) else {
            return Ok(false);
        };
        entry.last_used_at = Some(now_unix());
        self.save_registry(&registry)?;
        Ok(true)
    }

    /// Build a registry entry for an artefact already sitting at `path`.
    ///
    /// Hashes the file, reads its header prefix and detects the format. This
    /// is what the downloader calls once a transfer lands, and what
    /// [`Self::adopt_local`] calls for a file the user points at.
    ///
    /// # Errors
    ///
    /// [`ModelError::Io`] when the file cannot be read or stat'ed.
    pub fn describe_file(
        &self,
        reference: &ModelRef,
        path: &Path,
        catalog_id: Option<String>,
    ) -> Result<InstalledModel> {
        let meta = std::fs::metadata(path).map_err(|e| ModelError::io(path.to_path_buf(), e))?;
        let sha256 = digest::sha256_file(path)?;
        let prefix = read_prefix(path, inspect::PREFIX_BYTES)?;
        let inspection = inspect::inspect_prefix(&prefix, &reference.basename());

        let stored_path = path
            .strip_prefix(&self.root)
            .map_or_else(|_| path.display().to_string(), |rel| to_slash(rel));

        Ok(InstalledModel {
            reference: reference.clone(),
            path: path.to_path_buf(),
            stored_path,
            bytes: meta.len(),
            sha256,
            format: inspection.format,
            installed_at: now_unix(),
            last_used_at: None,
            catalog_id,
            inspection: Some(inspection),
        })
    }

    /// Track an artefact that already exists elsewhere on disk, without
    /// copying it. Useful for weights shipped by a system package or shared
    /// with another tool.
    ///
    /// # Errors
    ///
    /// [`ModelError::Io`] when the path is unreadable.
    pub fn adopt_local(&self, path: impl AsRef<Path>) -> Result<InstalledModel> {
        let path = path.as_ref();
        let absolute = path.canonicalize().map_or_else(|_| path.to_path_buf(), |p| strip_unc(&p));
        let reference = ModelRef::Local(absolute.clone());
        let entry = self.describe_file(&reference, &absolute, None)?;
        self.record(entry.clone())?;
        Ok(entry)
    }

    /// Drop an artefact: delete its bytes (unless adopted from outside the
    /// store) and remove its registry entry.
    ///
    /// # Errors
    ///
    /// [`ModelError::NotInstalled`] when nothing matches, or
    /// [`ModelError::Io`] when the file cannot be deleted.
    pub fn remove(&self, reference: &ModelRef) -> Result<InstalledModel> {
        let mut registry = self.load_registry()?;
        let key = reference.to_string();
        let entry = registry.remove(&key).ok_or_else(|| ModelError::NotInstalled(key))?;

        // An adopted `file:` artefact lives outside the store and belongs to
        // whoever put it there: forget it, never delete it.
        let inside_store = entry.path.starts_with(&self.root);
        if inside_store && entry.path.exists() {
            std::fs::remove_file(&entry.path)
                .map_err(|e| ModelError::io(entry.path.clone(), e))?;
            prune_empty_dirs(&entry.path, &self.root);
        }
        self.save_registry(&registry)?;
        Ok(entry)
    }

    /// Re-hash an artefact and compare it against what was recorded.
    ///
    /// # Errors
    ///
    /// [`ModelError::NotInstalled`] when the reference is unknown, or
    /// [`ModelError::Io`] when the bytes cannot be read.
    pub fn verify(&self, reference: &ModelRef) -> Result<VerifyReport> {
        let entry = self
            .get(reference)?
            .ok_or_else(|| ModelError::NotInstalled(reference.to_string()))?;
        let meta = std::fs::metadata(&entry.path)
            .map_err(|e| ModelError::io(entry.path.clone(), e))?;
        let actual = digest::sha256_file(&entry.path)?;
        Ok(VerifyReport {
            reference: entry.reference.to_string(),
            expected_sha256: entry.sha256,
            actual_sha256: actual,
            expected_bytes: entry.bytes,
            actual_bytes: meta.len(),
        })
    }

    /// Compare the registry against the filesystem.
    ///
    /// Registry entries whose file vanished are dropped from the index (the
    /// bytes are already gone, so keeping the row would only mislead). Files
    /// present but unclaimed, and leftover `.part` downloads, are reported so
    /// a caller can decide whether to adopt or delete them.
    ///
    /// # Errors
    ///
    /// Propagates registry and directory-walk failures.
    pub fn reconcile(&self) -> Result<ReconcileReport> {
        let mut registry = self.load_registry()?;
        let mut report = ReconcileReport::default();

        let mut tracked = std::collections::BTreeSet::new();
        registry.retain(|key, entry| {
            if entry.path.is_file() {
                tracked.insert(entry.path.clone());
                true
            } else {
                report.missing.push(key.clone());
                false
            }
        });

        for path in walk_files(&self.root)? {
            if path.file_name().is_some_and(|n| n == REGISTRY_FILE) {
                continue;
            }
            if path.to_string_lossy().ends_with(PART_SUFFIX) {
                report.stale_parts.push(path);
                continue;
            }
            if !tracked.contains(&path) {
                report.untracked.push(path);
            }
        }

        if !report.missing.is_empty() {
            self.save_registry(&registry)?;
        }
        report.untracked.sort();
        report.stale_parts.sort();
        Ok(report)
    }

    /// Evict least-recently-used artefacts until the store fits in
    /// `budget_bytes`, and delete any leftover `.part` files.
    ///
    /// Adopted `file:` artefacts are un-tracked but never deleted, and they
    /// do not count toward the budget: the store does not own those bytes.
    ///
    /// # Errors
    ///
    /// Propagates registry and filesystem failures.
    pub fn gc(&self, budget_bytes: u64) -> Result<GcReport> {
        let mut report = GcReport::default();

        // A `.part` is by definition an interrupted transfer; a resumed pull
        // rewrites it from scratch, so reclaiming it here is always safe.
        for path in walk_files(&self.root)? {
            if path.to_string_lossy().ends_with(PART_SUFFIX) {
                std::fs::remove_file(&path).map_err(|e| ModelError::io(path.clone(), e))?;
                report.removed_parts.push(path);
            }
        }
        report.removed_parts.sort();

        let mut registry = self.load_registry()?;
        let mut owned: Vec<InstalledModel> = registry
            .values()
            .filter(|m| m.path.starts_with(&self.root))
            .cloned()
            .collect();
        let mut held: u64 = owned.iter().map(|m| m.bytes).sum();

        // Oldest use first: that is the eviction order.
        owned.sort_by_key(|m| (m.recency(), m.reference.to_string()));

        for victim in owned {
            if held <= budget_bytes {
                break;
            }
            let key = victim.reference.to_string();
            if victim.path.exists() {
                std::fs::remove_file(&victim.path)
                    .map_err(|e| ModelError::io(victim.path.clone(), e))?;
                prune_empty_dirs(&victim.path, &self.root);
            }
            registry.remove(&key);
            held = held.saturating_sub(victim.bytes);
            report.reclaimed_bytes = report.reclaimed_bytes.saturating_add(victim.bytes);
            report.evicted.push(key);
        }

        report.remaining_bytes = held;
        self.save_registry(&registry)?;
        Ok(report)
    }

    // -- persistence -----------------------------------------------------

    fn load_registry(&self) -> Result<BTreeMap<String, InstalledModel>> {
        let path = self.registry_path();
        let raw = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
            Err(e) => return Err(ModelError::io(path, e)),
        };
        // An empty file is what a crash between create and write leaves
        // behind; treat it as an empty store rather than a corrupt one.
        if raw.iter().all(u8::is_ascii_whitespace) {
            return Ok(BTreeMap::new());
        }

        let doc: RegistryDoc = serde_json::from_slice(&raw)
            .map_err(|source| ModelError::Registry { path: path.clone(), source })?;
        if doc.version != REGISTRY_VERSION {
            return Err(ModelError::Inspect {
                path,
                reason: format!(
                    "registry schema version {} is not supported (this build expects {REGISTRY_VERSION})",
                    doc.version
                ),
            });
        }

        let mut map = BTreeMap::new();
        for mut entry in doc.entries {
            // `stored_path` is root-relative for owned artefacts and absolute
            // for adopted ones; rebuilding through `join` handles both, since
            // joining an absolute path replaces the base.
            entry.path = self.root.join(PathBuf::from(entry.stored_path.replace('/', MAIN_SEP_STR)));
            map.insert(entry.reference.to_string(), entry);
        }
        Ok(map)
    }

    fn save_registry(&self, registry: &BTreeMap<String, InstalledModel>) -> Result<()> {
        let doc = RegistryDoc {
            version: REGISTRY_VERSION,
            entries: registry.values().cloned().collect(),
        };
        let mut json = serde_json::to_vec_pretty(&doc).map_err(|source| ModelError::Registry {
            path: self.registry_path(),
            source,
        })?;
        json.push(b'\n');
        atomic_write(&self.registry_path(), &json)
    }
}

/// Platform path separator as a string, for rebuilding stored paths.
const MAIN_SEP_STR: &str = if cfg!(windows) { "\\" } else { "/" };

/// Render a relative path with forward slashes, so a registry written on
/// Windows still resolves on Linux.
fn to_slash(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Drop the Windows `\\?\` verbatim prefix that `canonicalize` prepends.
///
/// A verbatim path is correct but leaks into every reference the store prints
/// and into the registry, where it is both ugly and non-portable. Only the
/// plain `\\?\C:\…` disk form is unwrapped; `\\?\UNC\server\share` is left as
/// it is, because there the prefix carries meaning.
pub(crate) fn strip_unc(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    match raw.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Write `bytes` to `path` atomically: temp sibling then rename.
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ModelError::io(parent.to_path_buf(), e))?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| ModelError::io(tmp.clone(), e))?;
    // `rename` replaces the destination atomically on POSIX; on Windows the
    // std implementation maps to MoveFileEx with MOVEFILE_REPLACE_EXISTING.
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(ModelError::io(path.to_path_buf(), e))
        }
    }
}

/// Read at most `limit` leading bytes of a file.
pub(crate) fn read_prefix(path: &Path, limit: usize) -> Result<Vec<u8>> {
    use std::io::Read as _;

    let file = std::fs::File::open(path).map_err(|e| ModelError::io(path.to_path_buf(), e))?;
    let mut buf = Vec::new();
    file.take(limit as u64)
        .read_to_end(&mut buf)
        .map_err(|e| ModelError::io(path.to_path_buf(), e))?;
    Ok(buf)
}

/// Every regular file under `root`, recursively, sorted for determinism.
fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(ModelError::io(dir, e)),
        };
        for entry in entries {
            let entry = entry.map_err(|e| ModelError::io(dir.clone(), e))?;
            let path = entry.path();
            let file_type =
                entry.file_type().map_err(|e| ModelError::io(path.clone(), e))?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                out.push(path);
            }
            // Symlinks are deliberately skipped: following them could walk
            // out of the store root entirely.
        }
    }
    out.sort();
    Ok(out)
}

/// After deleting an artefact, remove the directories it left empty, stopping
/// at the store root. Failures are ignored: empty directories are cosmetic.
fn prune_empty_dirs(deleted: &Path, root: &Path) {
    let mut cursor = deleted.parent();
    while let Some(dir) = cursor {
        if dir == root || !dir.starts_with(root) {
            return;
        }
        if std::fs::remove_dir(dir).is_err() {
            return;
        }
        cursor = dir.parent();
    }
}

/// Resolve the default store root.
fn default_root() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("APHRODY_MODELS_DIR") {
        if !explicit.is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }
    Ok(state_dir()?.join("models"))
}

/// Resolve the aphrody state directory, mirroring `aphrody-embed::cache`.
fn state_dir() -> Result<PathBuf> {
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
    dirs::home_dir().map(|h| h.join(".aphrody")).ok_or(ModelError::NoStateDir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, ModelStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = ModelStore::with_root(dir.path().join("models")).unwrap();
        (dir, store)
    }

    /// Install a synthetic artefact and return its reference.
    fn install(store: &ModelStore, reference: &str, bytes: &[u8]) -> ModelRef {
        let r = ModelRef::parse(reference).unwrap();
        let path = store.path_for(&r);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        let entry = store.describe_file(&r, &path, None).unwrap();
        store.record(entry).unwrap();
        r
    }

    #[test]
    fn hf_ref_lands_under_a_deterministic_path() {
        let (_g, store) = store();
        let r = ModelRef::parse("hf:BAAI/bge/onnx/model.onnx@v1").unwrap();
        let path = store.path_for(&r);
        assert!(path.starts_with(store.root()));
        assert!(path.ends_with(Path::new("hf/BAAI/bge/v1/onnx/model.onnx")), "{path:?}");
        assert_eq!(store.part_path_for(&r).file_name().unwrap(), "model.onnx.part");
    }

    #[test]
    fn record_then_get_round_trips_through_disk() {
        let (_g, store) = store();
        let r = install(&store, "hf:a/b/m.gguf", b"GGUF-not-really");
        // A fresh handle proves the data survived serialisation.
        let reopened = ModelStore::with_root(store.root()).unwrap();
        let got = reopened.get(&r).unwrap().expect("entry should be indexed");
        assert_eq!(got.bytes, 15);
        assert_eq!(got.sha256, digest::sha256_hex(b"GGUF-not-really"));
        assert_eq!(got.path, store.path_for(&r));
        assert!(got.installed_at_rfc3339().starts_with("19") || got.installed_at_rfc3339().starts_with("20"));
    }

    #[test]
    fn registry_stores_relative_paths_so_the_store_can_move() {
        let (_g, store) = store();
        install(&store, "hf:a/b/m.gguf", b"x");
        let raw = std::fs::read_to_string(store.registry_path()).unwrap();
        assert!(raw.contains("\"path\": \"hf/a/b/main/m.gguf\""), "{raw}");
        assert!(!raw.contains(&store.root().display().to_string()));
    }

    #[test]
    fn total_bytes_sums_every_entry() {
        let (_g, store) = store();
        install(&store, "hf:a/b/one.gguf", &[0_u8; 100]);
        install(&store, "hf:a/b/two.gguf", &[0_u8; 250]);
        assert_eq!(store.total_bytes().unwrap(), 350);
        assert_eq!(store.list().unwrap().len(), 2);
    }

    #[test]
    fn remove_deletes_bytes_and_prunes_empty_dirs() {
        let (_g, store) = store();
        let r = install(&store, "hf:a/b/m.gguf", b"payload");
        let path = store.path_for(&r);
        assert!(path.is_file());

        let removed = store.remove(&r).unwrap();
        assert_eq!(removed.bytes, 7);
        assert!(!path.exists());
        assert!(!path.parent().unwrap().exists(), "empty dirs should be pruned");
        assert!(store.root().is_dir(), "the root itself must survive");
        assert!(store.get(&r).unwrap().is_none());
    }

    #[test]
    fn removing_an_unknown_ref_is_an_error() {
        let (_g, store) = store();
        let r = ModelRef::parse("hf:a/b/absent.gguf").unwrap();
        assert!(matches!(store.remove(&r), Err(ModelError::NotInstalled(_))));
    }

    #[test]
    fn adopted_local_files_are_tracked_but_never_deleted() {
        let (guard, store) = store();
        let outside = guard.path().join("external.gguf");
        std::fs::write(&outside, b"external-weights").unwrap();

        let entry = store.adopt_local(&outside).unwrap();
        assert_eq!(entry.bytes, 16);
        assert!(matches!(entry.reference, ModelRef::Local(_)));

        store.remove(&entry.reference).unwrap();
        assert!(outside.is_file(), "adopted files belong to the user, not the store");
    }

    #[test]
    fn verify_detects_tampering() {
        let (_g, store) = store();
        let r = install(&store, "hf:a/b/m.gguf", b"original");
        assert!(store.verify(&r).unwrap().is_intact());

        std::fs::write(store.path_for(&r), b"tampered!").unwrap();
        let report = store.verify(&r).unwrap();
        assert!(!report.is_intact());
        assert_ne!(report.expected_sha256, report.actual_sha256);
        assert_eq!(report.expected_bytes, 8);
        assert_eq!(report.actual_bytes, 9);
    }

    #[test]
    fn reconcile_drops_vanished_entries_and_reports_strays() {
        let (_g, store) = store();
        let gone = install(&store, "hf:a/b/gone.gguf", b"bytes");
        install(&store, "hf:a/b/kept.gguf", b"bytes");
        std::fs::remove_file(store.path_for(&gone)).unwrap();

        let stray = store.root().join("hf/a/b/main/stray.gguf");
        std::fs::write(&stray, b"unclaimed").unwrap();
        let part = store.root().join("hf/a/b/main/half.gguf.part");
        std::fs::write(&part, b"partial").unwrap();

        let report = store.reconcile().unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.missing, vec!["hf:a/b/gone.gguf".to_owned()]);
        assert_eq!(report.untracked, vec![stray]);
        assert_eq!(report.stale_parts, vec![part]);
        // The vanished entry is gone from the index; the survivor stays.
        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn reconcile_is_clean_on_a_healthy_store() {
        let (_g, store) = store();
        install(&store, "hf:a/b/m.gguf", b"bytes");
        assert!(store.reconcile().unwrap().is_clean());
    }

    #[test]
    fn gc_evicts_least_recently_used_until_the_budget_is_met() {
        let (_g, store) = store();
        let old = install(&store, "hf:a/b/old.gguf", &[1_u8; 400]);
        let mid = install(&store, "hf:a/b/mid.gguf", &[2_u8; 400]);
        let new = install(&store, "hf:a/b/new.gguf", &[3_u8; 400]);

        // `install` stamps all three with the same second, so order them by
        // hand through the public recency field.
        for (reference, when) in [(&old, 1_000_u64), (&mid, 2_000), (&new, 3_000)] {
            let mut entry = store.get(reference).unwrap().unwrap();
            entry.last_used_at = Some(when);
            store.record(entry).unwrap();
        }

        let report = store.gc(800).unwrap();
        assert_eq!(report.evicted, vec!["hf:a/b/old.gguf".to_owned()]);
        assert_eq!(report.reclaimed_bytes, 400);
        assert_eq!(report.remaining_bytes, 800);
        assert!(!store.path_for(&old).exists());
        assert!(store.path_for(&mid).is_file());
        assert!(store.path_for(&new).is_file());
    }

    #[test]
    fn gc_sweeps_stale_part_files_even_when_under_budget() {
        let (_g, store) = store();
        install(&store, "hf:a/b/m.gguf", &[0_u8; 10]);
        let part = store.root().join("hf/a/b/main/m.gguf.part");
        std::fs::write(&part, b"interrupted").unwrap();

        let report = store.gc(u64::MAX).unwrap();
        assert!(report.evicted.is_empty());
        assert_eq!(report.removed_parts, vec![part.clone()]);
        assert!(!part.exists());
    }

    #[test]
    fn gc_never_deletes_adopted_files() {
        let (guard, store) = store();
        let outside = guard.path().join("big.gguf");
        std::fs::write(&outside, &[7_u8; 5_000]).unwrap();
        let entry = store.adopt_local(&outside).unwrap();

        let report = store.gc(0).unwrap();
        assert!(report.evicted.is_empty(), "adopted bytes are not the store's to reclaim");
        assert!(outside.is_file());
        assert!(store.get(&entry.reference).unwrap().is_some());
    }

    #[test]
    fn touch_updates_recency_and_reports_unknown_refs() {
        let (_g, store) = store();
        let r = install(&store, "hf:a/b/m.gguf", b"bytes");
        assert!(store.get(&r).unwrap().unwrap().last_used_at.is_none());
        assert!(store.touch(&r).unwrap());
        let entry = store.get(&r).unwrap().unwrap();
        assert!(entry.last_used_at.is_some());
        assert_eq!(entry.recency(), entry.last_used_at.unwrap());

        let absent = ModelRef::parse("hf:a/b/absent.gguf").unwrap();
        assert!(!store.touch(&absent).unwrap(), "touching an absent model is a no-op");
    }

    #[test]
    fn format_is_detected_at_install_time() {
        let (_g, store) = store();
        let mut gguf = b"GGUF".to_vec();
        gguf.extend_from_slice(&3_u32.to_le_bytes());
        gguf.extend_from_slice(&0_u64.to_le_bytes());
        gguf.extend_from_slice(&0_u64.to_le_bytes());
        let r = install(&store, "hf:a/b/m.gguf", &gguf);
        let entry = store.get(&r).unwrap().unwrap();
        assert_eq!(entry.format, ArtifactFormat::Gguf);
        assert!(entry.format.is_loadable());
        assert!(entry.inspection.is_some());
    }

    #[test]
    fn missing_registry_reads_as_an_empty_store() {
        let (_g, store) = store();
        assert!(store.list().unwrap().is_empty());
        assert_eq!(store.total_bytes().unwrap(), 0);
    }

    #[test]
    fn empty_registry_file_is_tolerated() {
        let (_g, store) = store();
        std::fs::write(store.registry_path(), b"   \n").unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn corrupt_registry_is_reported_not_silently_reset() {
        let (_g, store) = store();
        std::fs::write(store.registry_path(), b"{not json").unwrap();
        assert!(matches!(store.list(), Err(ModelError::Registry { .. })));
    }

    #[test]
    fn future_schema_version_is_refused() {
        let (_g, store) = store();
        std::fs::write(store.registry_path(), br#"{"version":999,"entries":[]}"#).unwrap();
        let err = store.list().unwrap_err();
        assert!(err.to_string().contains("999"), "{err}");
    }

    #[test]
    fn atomic_write_leaves_no_temp_file_behind() {
        let (guard, _store) = store();
        let target = guard.path().join("nested/deep/file.json");
        atomic_write(&target, b"payload").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"payload");
        let siblings: Vec<_> = std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(siblings, vec![std::ffi::OsString::from("file.json")]);
    }

    #[test]
    fn rfc3339_rendering_is_utc() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn verbatim_prefixes_are_stripped_but_unc_shares_are_kept() {
        assert_eq!(strip_unc(Path::new(r"\\?\C:\models\m.gguf")), PathBuf::from(r"C:\models\m.gguf"));
        // A real UNC share needs the prefix to stay resolvable.
        assert_eq!(
            strip_unc(Path::new(r"\\?\UNC\server\share\m.gguf")),
            PathBuf::from(r"\\?\UNC\server\share\m.gguf")
        );
        // Anything already plain is returned unchanged.
        assert_eq!(strip_unc(Path::new("/home/u/m.gguf")), PathBuf::from("/home/u/m.gguf"));
    }

    #[test]
    fn adopting_records_a_readable_path() {
        let (guard, store) = store();
        let outside = guard.path().join("weights.gguf");
        std::fs::write(&outside, b"bytes").unwrap();
        let entry = store.adopt_local(&outside).unwrap();
        assert!(
            !entry.path.to_string_lossy().starts_with(r"\\?\"),
            "verbatim prefix leaked into the registry: {}",
            entry.path.display()
        );
        assert!(!entry.reference.to_string().contains(r"\\?\"));
    }
}
