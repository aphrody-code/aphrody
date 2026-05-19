// SPDX-License-Identifier: Apache-2.0
//! aphrody-cache — two-tier cache primitives for the aphrody runtime.
//!
//! The crate exposes a single async [`Cache`] trait with two concrete
//! implementations:
//!
//! * [`MemoryCache`] — bounded LRU sitting behind a `tokio::sync::Mutex`,
//!   with optional per-entry TTL. Backed by [`LruCache`], a small
//!   `VecDeque + HashMap` LRU that does not pull `linked-hash-map`.
//! * [`DiskCache`] — persistent on-disk store. Each entry lives in two
//!   sibling files (`<sha>.bin` for the payload, `<sha>.meta.json` for the
//!   created-at / TTL metadata). Writes are atomic (tmp + rename).
//!
//! Both implementations key entries by [`CacheKey`], a newtype wrapping
//! an opaque `String`. The [`CacheKey::from_hash`] constructor produces a
//! lowercase hex SHA-256 of the input bytes — perfect for hashing prompts,
//! URLs, or canonicalised request envelopes.
//!
//! ## Cohesion with the wider aphrody workspace
//!
//! * **`aphrody-cron::JsonFileJobStore`** is the canonical pattern for the
//!   atomic tmp + rename writes used by [`DiskCache`].
//! * **`aphrody-marketplace::verify_sha256`** is the inspiration for the
//!   `sha2 + hex` digest pipeline driving [`CacheKey::from_hash`] and the
//!   disk-key derivation.
//! * **`aphrody-context::ContextWindow::trim_to_fit`** seeded the eviction
//!   strategy used by [`LruCache`]: oldest non-pinned entry first, TTL-aware.
//!
//! ## Typical wiring
//!
//! ```no_run
//! use std::time::Duration;
//! use aphrody_cache::{Cache, CacheKey, CachedBytes, MemoryCache};
//!
//! # async fn run() -> Result<(), aphrody_cache::CacheError> {
//! let cache = MemoryCache::new(1024);
//! let key   = CacheKey::from_hash(b"https://api.anthropic.com/v1/models");
//! cache.put(&key, CachedBytes::from(b"<response>".to_vec()), Some(60_000)).await?;
//! if let Some(bytes) = cache.get(&key).await? {
//!     println!("hit: {} bytes", bytes.0.len());
//! }
//! # Ok(()) }
//! ```
//!
//! See `tests/cache_smoke.rs` for the executable contract.

#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All recoverable failures emitted by [`MemoryCache`] / [`DiskCache`].
#[derive(Debug, Error)]
pub enum CacheError {
    /// Filesystem or IO error from the disk-backed tier.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON (de)serialization failure on the on-disk metadata sidecar.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Requested key was not present in the cache (only surfaced by callers
    /// that opt into strict lookups; [`Cache::get`] returns `Ok(None)` on
    /// a regular miss).
    #[error("cache key not found")]
    KeyNotFound,
    /// The key was syntactically rejected (empty / non-ASCII / too long).
    #[error("invalid cache key: {0}")]
    InvalidKey(String),
    /// Entry was present on disk but its TTL had expired at read time.
    #[error("cache entry expired")]
    Expired,
    /// Attempted to build a cache with `capacity == 0`.
    #[error("cache capacity must be > 0")]
    CapacityZero,
}

/// Convenience alias used by every public method.
pub type CacheResult<T> = Result<T, CacheError>;

// ---------------------------------------------------------------------------
// CacheKey + CachedBytes
// ---------------------------------------------------------------------------

/// Opaque cache key. Stored as a [`String`] so it can round-trip through
/// JSON metadata sidecars and be used as a `HashMap` key without further
/// allocations on each lookup.
///
/// Callers can construct a [`CacheKey`] directly from any string-shaped
/// identifier ([`CacheKey::from_string`]), or derive one from arbitrary
/// bytes via SHA-256 ([`CacheKey::from_hash`]). The latter is preferred
/// for prompts / URLs to avoid leaking long inputs into log lines.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey(pub String);

impl CacheKey {
    /// Build a key by lowercase-hex-encoding the SHA-256 of `input`.
    ///
    /// This always produces a 64-character ASCII string suitable for use
    /// as both a memory-cache key and a filename in [`DiskCache`].
    #[must_use]
    pub fn from_hash(input: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(input);
        let digest = hasher.finalize();
        let mut s = String::with_capacity(64);
        for b in digest {
            // Manual hex writer to avoid pulling the `hex` crate just for this.
            s.push(NIBBLE[(b >> 4) as usize] as char);
            s.push(NIBBLE[(b & 0x0F) as usize] as char);
        }
        Self(s)
    }

    /// Wrap a caller-controlled identifier verbatim. Validates that the
    /// string is non-empty and ASCII (disk tier requirement).
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::InvalidKey`] if the input is empty.
    pub fn from_string(s: impl Into<String>) -> CacheResult<Self> {
        let s = s.into();
        if s.is_empty() {
            return Err(CacheError::InvalidKey("empty".into()));
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Derive a filesystem-safe identifier for the disk tier. The key is
    /// re-hashed (lowercase hex SHA-256) so arbitrary input (URLs, paths,
    /// arbitrary bytes) cannot smuggle directory separators.
    #[must_use]
    pub fn disk_id(&self) -> String {
        Self::from_hash(self.0.as_bytes()).0
    }
}

const NIBBLE: &[u8; 16] = b"0123456789abcdef";

/// Opaque byte payload stored in a cache entry. The wrapper exists to
/// keep the public API explicit (cache values are *bytes*, not strings)
/// and to leave room for cheap conversions in the future without breaking
/// the [`Cache`] trait surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedBytes(pub Vec<u8>);

impl From<Vec<u8>> for CachedBytes {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl AsRef<[u8]> for CachedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// CacheEntry
// ---------------------------------------------------------------------------

/// One value stored inside [`LruCache`]. Tracks creation/access time so
/// the eviction loop can implement both LRU and TTL semantics off the
/// same record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry<V> {
    /// The stored value.
    pub value: V,
    /// UTC timestamp at insertion time.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the most recent successful `get`.
    pub last_accessed_at: DateTime<Utc>,
    /// Optional time-to-live in milliseconds (relative to `created_at`).
    pub ttl_ms: Option<u64>,
    /// Number of successful `get`s seen by this entry.
    pub hits: u64,
}

impl<V> CacheEntry<V> {
    /// Build a fresh entry stamped at `now`.
    pub fn new(value: V, ttl_ms: Option<u64>, now: DateTime<Utc>) -> Self {
        Self {
            value,
            created_at: now,
            last_accessed_at: now,
            ttl_ms,
            hits: 0,
        }
    }

    /// Returns `true` when the entry has a TTL configured and `now` is past
    /// `created_at + ttl_ms`.
    #[must_use]
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        let Some(ttl) = self.ttl_ms else {
            return false;
        };
        let age_ms = now
            .signed_duration_since(self.created_at)
            .num_milliseconds();
        age_ms >= 0 && (age_ms as u64) > ttl
    }
}

// ---------------------------------------------------------------------------
// LruCache (sync core, used by MemoryCache)
// ---------------------------------------------------------------------------

/// In-memory bounded LRU cache. Independent of any runtime concern — the
/// async [`MemoryCache`] wraps this struct behind a `tokio::sync::Mutex`.
///
/// Recency is tracked with a [`VecDeque<CacheKey>`] (oldest at the front,
/// newest at the back). Lookups are amortised O(n) because `VecDeque`
/// reordering is linear; the trade-off is intentional: zero unsafe code,
/// no extra crate dep, and good cache locality for the small caches that
/// dominate aphrody workloads (typically ≤ 4 k entries).
#[derive(Debug)]
pub struct LruCache<V> {
    capacity: usize,
    entries: HashMap<CacheKey, CacheEntry<V>>,
    order: VecDeque<CacheKey>,
    evictions: u64,
}

impl<V> LruCache<V> {
    /// Build an LRU cache that holds at most `capacity` entries.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::CapacityZero`] when `capacity == 0`.
    pub fn new(capacity: usize) -> CacheResult<Self> {
        if capacity == 0 {
            return Err(CacheError::CapacityZero);
        }
        Ok(Self {
            capacity,
            entries: HashMap::with_capacity(capacity.min(1024)),
            order: VecDeque::with_capacity(capacity.min(1024)),
            evictions: 0,
        })
    }

    /// Read the configured capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of live entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when the cache holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total number of evictions ever observed (LRU rotation + TTL purge).
    #[must_use]
    pub fn evictions(&self) -> u64 {
        self.evictions
    }

    /// Fetch a value, promoting the entry to most-recently-used and
    /// bumping the hit counter / access timestamp.
    pub fn get(&mut self, key: &CacheKey, now: DateTime<Utc>) -> Option<&V> {
        let entry = self.entries.get_mut(key)?;
        if entry.is_expired(now) {
            // Drop the expired entry transparently.
            self.entries.remove(key);
            self.order.retain(|k| k != key);
            self.evictions = self.evictions.saturating_add(1);
            return None;
        }
        entry.hits = entry.hits.saturating_add(1);
        entry.last_accessed_at = now;
        // Promote: remove from order, push to the back.
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key.clone());
        // Re-borrow immutably for the return.
        self.entries.get(key).map(|e| &e.value)
    }

    /// Insert (or overwrite) an entry. Evicts the oldest live entry when
    /// the cache is at capacity. Returns `true` if a previous value for
    /// `key` was overwritten.
    pub fn put(&mut self, key: CacheKey, value: V, ttl_ms: Option<u64>, now: DateTime<Utc>) -> bool {
        let overwrite = self.entries.contains_key(&key);
        if overwrite {
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
        } else if self.entries.len() >= self.capacity {
            if let Some(victim) = self.order.pop_front() {
                self.entries.remove(&victim);
                self.evictions = self.evictions.saturating_add(1);
            }
        }
        self.entries
            .insert(key.clone(), CacheEntry::new(value, ttl_ms, now));
        self.order.push_back(key);
        overwrite
    }

    /// Remove an entry by key. Returns `true` when something was removed.
    pub fn remove(&mut self, key: &CacheKey) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed {
            self.order.retain(|k| k != key);
        }
        removed
    }

    /// Drop every entry. Eviction counter is left untouched (it tracks the
    /// lifetime of the cache, not a single epoch).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// Drop every entry whose TTL has expired at `now`. Returns the number
    /// of entries evicted.
    pub fn evict_expired(&mut self, now: DateTime<Utc>) -> usize {
        let stale: Vec<CacheKey> = self
            .entries
            .iter()
            .filter_map(|(k, v)| if v.is_expired(now) { Some(k.clone()) } else { None })
            .collect();
        let n = stale.len();
        for key in stale {
            self.entries.remove(&key);
            self.order.retain(|k| k != &key);
            self.evictions = self.evictions.saturating_add(1);
        }
        n
    }

    /// Peek at an entry without touching recency — useful for stats /
    /// debugging surfaces.
    #[must_use]
    pub fn peek(&self, key: &CacheKey) -> Option<&CacheEntry<V>> {
        self.entries.get(key)
    }
}

// ---------------------------------------------------------------------------
// Cache trait
// ---------------------------------------------------------------------------

/// Stats snapshot returned by [`Cache::stats`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cumulative successful reads.
    pub hits: u64,
    /// Cumulative reads that missed (or hit an expired entry).
    pub misses: u64,
    /// Cumulative inserts (overwrites included).
    pub puts: u64,
    /// Cumulative evictions (LRU rotation + TTL purge).
    pub evictions: u64,
    /// Current live entry count.
    pub entries: usize,
}

/// Async cache surface implemented by [`MemoryCache`] and [`DiskCache`].
///
/// The trait is intentionally narrow — callers compose tiers in user code
/// (try memory first, fall through to disk on miss) rather than relying on
/// a magic façade.
#[async_trait]
pub trait Cache: Send + Sync {
    /// Fetch the value bound to `key`. Returns `Ok(None)` on a cold miss
    /// or when the entry's TTL has elapsed (silently evicted).
    async fn get(&self, key: &CacheKey) -> CacheResult<Option<CachedBytes>>;

    /// Insert or overwrite an entry. `ttl_ms == None` disables expiry.
    async fn put(&self, key: &CacheKey, value: CachedBytes, ttl_ms: Option<u64>) -> CacheResult<()>;

    /// Remove a single entry. Missing keys are a silent no-op (returns Ok).
    async fn delete(&self, key: &CacheKey) -> CacheResult<()>;

    /// Drop every entry held by this cache.
    async fn clear(&self) -> CacheResult<()>;

    /// Snapshot the in-process stats counters.
    async fn stats(&self) -> CacheStats;
}

// ---------------------------------------------------------------------------
// MemoryCache
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct StatsCore {
    hits: u64,
    misses: u64,
    puts: u64,
}

/// In-memory tier — bounded LRU + TTL, safe to share across tasks behind
/// an `Arc`.
#[derive(Debug)]
pub struct MemoryCache {
    inner: Mutex<LruCache<CachedBytes>>,
    counters: Mutex<StatsCore>,
}

impl MemoryCache {
    /// Build a memory cache holding at most `capacity` entries.
    ///
    /// # Panics
    ///
    /// Panics when `capacity == 0`. Use [`MemoryCache::try_new`] for the
    /// fallible variant.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self::try_new(capacity).expect("capacity must be > 0")
    }

    /// Fallible constructor — returns [`CacheError::CapacityZero`] on
    /// `capacity == 0` instead of panicking.
    pub fn try_new(capacity: usize) -> CacheResult<Self> {
        Ok(Self {
            inner: Mutex::new(LruCache::new(capacity)?),
            counters: Mutex::new(StatsCore::default()),
        })
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: &CacheKey) -> CacheResult<Option<CachedBytes>> {
        let now = Utc::now();
        let mut guard = self.inner.lock().await;
        let cloned = guard.get(key, now).cloned();
        drop(guard);
        let mut counters = self.counters.lock().await;
        if cloned.is_some() {
            counters.hits = counters.hits.saturating_add(1);
        } else {
            counters.misses = counters.misses.saturating_add(1);
        }
        Ok(cloned)
    }

    async fn put(&self, key: &CacheKey, value: CachedBytes, ttl_ms: Option<u64>) -> CacheResult<()> {
        let now = Utc::now();
        let mut guard = self.inner.lock().await;
        guard.put(key.clone(), value, ttl_ms, now);
        drop(guard);
        let mut counters = self.counters.lock().await;
        counters.puts = counters.puts.saturating_add(1);
        Ok(())
    }

    async fn delete(&self, key: &CacheKey) -> CacheResult<()> {
        let mut guard = self.inner.lock().await;
        guard.remove(key);
        Ok(())
    }

    async fn clear(&self) -> CacheResult<()> {
        let mut guard = self.inner.lock().await;
        guard.clear();
        Ok(())
    }

    async fn stats(&self) -> CacheStats {
        let guard = self.inner.lock().await;
        let entries = guard.len();
        let evictions = guard.evictions();
        drop(guard);
        let counters = self.counters.lock().await;
        CacheStats {
            hits: counters.hits,
            misses: counters.misses,
            puts: counters.puts,
            evictions,
            entries,
        }
    }
}

// ---------------------------------------------------------------------------
// DiskCache
// ---------------------------------------------------------------------------

/// On-disk metadata sidecar — kept next to the `*.bin` payload file.
/// Persisted as JSON so it can be inspected and migrated by hand.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiskMeta {
    key: String,
    created_at: DateTime<Utc>,
    ttl_ms: Option<u64>,
}

/// Persistent on-disk cache tier. Entry filenames are derived from the
/// SHA-256 of the key so callers never have to think about path safety.
///
/// Writes are atomic: the payload is written to a `*.bin.tmp` sibling
/// then renamed into place, mirroring the pattern from
/// `aphrody-cron::JsonFileJobStore`.
#[derive(Debug)]
pub struct DiskCache {
    base_dir: PathBuf,
    counters: Mutex<StatsCore>,
    evictions: Mutex<u64>,
}

impl DiskCache {
    /// Open (or initialise) a disk cache rooted at `base_dir`. Creates the
    /// directory on demand.
    pub async fn open(base_dir: impl Into<PathBuf>) -> CacheResult<Self> {
        let base_dir = base_dir.into();
        tokio::fs::create_dir_all(&base_dir).await?;
        Ok(Self {
            base_dir,
            counters: Mutex::new(StatsCore::default()),
            evictions: Mutex::new(0),
        })
    }

    /// Read the configured base directory.
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    fn payload_path(&self, key: &CacheKey) -> PathBuf {
        self.base_dir.join(format!("{}.bin", key.disk_id()))
    }

    fn meta_path(&self, key: &CacheKey) -> PathBuf {
        self.base_dir.join(format!("{}.meta.json", key.disk_id()))
    }

    async fn write_atomic(path: &Path, bytes: &[u8]) -> CacheResult<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        let mut tmp = path.to_path_buf();
        let mut ext = tmp
            .extension()
            .map(|e| e.to_os_string())
            .unwrap_or_default();
        ext.push(".tmp");
        tmp.set_extension(ext);
        let mut file = tokio::fs::File::create(&tmp).await?;
        file.write_all(bytes).await?;
        file.flush().await?;
        // Ensure data is durable before rename for crash safety.
        file.sync_all().await?;
        drop(file);
        tokio::fs::rename(&tmp, path).await?;
        Ok(())
    }

    async fn remove_pair(&self, key: &CacheKey) -> CacheResult<()> {
        let payload = self.payload_path(key);
        let meta = self.meta_path(key);
        if tokio::fs::try_exists(&payload).await? {
            tokio::fs::remove_file(&payload).await?;
        }
        if tokio::fs::try_exists(&meta).await? {
            tokio::fs::remove_file(&meta).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Cache for DiskCache {
    async fn get(&self, key: &CacheKey) -> CacheResult<Option<CachedBytes>> {
        let meta_path = self.meta_path(key);
        let payload_path = self.payload_path(key);
        if !tokio::fs::try_exists(&meta_path).await? || !tokio::fs::try_exists(&payload_path).await?
        {
            let mut counters = self.counters.lock().await;
            counters.misses = counters.misses.saturating_add(1);
            return Ok(None);
        }
        let meta_bytes = tokio::fs::read(&meta_path).await?;
        let meta: DiskMeta = serde_json::from_slice(&meta_bytes)?;
        let now = Utc::now();
        let expired = match meta.ttl_ms {
            Some(ttl) => {
                let age_ms = now
                    .signed_duration_since(meta.created_at)
                    .num_milliseconds();
                age_ms >= 0 && (age_ms as u64) > ttl
            }
            None => false,
        };
        if expired {
            // Treat as miss + evict.
            self.remove_pair(key).await?;
            let mut counters = self.counters.lock().await;
            counters.misses = counters.misses.saturating_add(1);
            drop(counters);
            let mut evictions = self.evictions.lock().await;
            *evictions = evictions.saturating_add(1);
            return Ok(None);
        }
        let bytes = tokio::fs::read(&payload_path).await?;
        let mut counters = self.counters.lock().await;
        counters.hits = counters.hits.saturating_add(1);
        Ok(Some(CachedBytes(bytes)))
    }

    async fn put(&self, key: &CacheKey, value: CachedBytes, ttl_ms: Option<u64>) -> CacheResult<()> {
        let meta = DiskMeta {
            key: key.0.clone(),
            created_at: Utc::now(),
            ttl_ms,
        };
        let meta_bytes = serde_json::to_vec_pretty(&meta)?;
        Self::write_atomic(&self.payload_path(key), &value.0).await?;
        Self::write_atomic(&self.meta_path(key), &meta_bytes).await?;
        let mut counters = self.counters.lock().await;
        counters.puts = counters.puts.saturating_add(1);
        Ok(())
    }

    async fn delete(&self, key: &CacheKey) -> CacheResult<()> {
        self.remove_pair(key).await
    }

    async fn clear(&self) -> CacheResult<()> {
        if !tokio::fs::try_exists(&self.base_dir).await? {
            return Ok(());
        }
        let mut rd = tokio::fs::read_dir(&self.base_dir).await?;
        while let Some(ent) = rd.next_entry().await? {
            let path = ent.path();
            if path.is_file() {
                let drop_it = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".bin") || n.ends_with(".meta.json"))
                    .unwrap_or(false);
                if drop_it {
                    tokio::fs::remove_file(&path).await?;
                }
            }
        }
        Ok(())
    }

    async fn stats(&self) -> CacheStats {
        // Count live entries by scanning the directory for `.bin` files.
        let mut entries = 0usize;
        if tokio::fs::try_exists(&self.base_dir).await.unwrap_or(false) {
            if let Ok(mut rd) = tokio::fs::read_dir(&self.base_dir).await {
                while let Ok(Some(ent)) = rd.next_entry().await {
                    if ent
                        .file_name()
                        .to_str()
                        .map(|n| n.ends_with(".bin"))
                        .unwrap_or(false)
                    {
                        entries = entries.saturating_add(1);
                    }
                }
            }
        }
        let counters = self.counters.lock().await;
        let evictions = *self.evictions.lock().await;
        CacheStats {
            hits: counters.hits,
            misses: counters.misses,
            puts: counters.puts,
            evictions,
            entries,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests (in-crate, light)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_from_hash_is_64_hex() {
        let k = CacheKey::from_hash(b"abc");
        assert_eq!(k.0.len(), 64);
        assert!(k.0.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn lru_capacity_zero_is_rejected() {
        let err = LruCache::<u32>::new(0).unwrap_err();
        assert!(matches!(err, CacheError::CapacityZero));
    }

    #[test]
    fn entry_no_ttl_never_expires() {
        let now = Utc::now();
        let entry = CacheEntry::new(1u32, None, now);
        let later = now + chrono::Duration::days(365);
        assert!(!entry.is_expired(later));
    }
}
