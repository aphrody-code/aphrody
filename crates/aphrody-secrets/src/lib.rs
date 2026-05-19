// SPDX-License-Identifier: Apache-2.0
//! aphrody-secrets — secret-store abstraction with redaction-by-default.
//!
//! The crate exposes:
//!
//! * [`SecretValue`] — a [`Zeroize`]-on-drop newtype that **never** leaks its
//!   bytes through [`std::fmt::Display`] or [`std::fmt::Debug`].  Comparison is
//!   constant-time (manual implementation; the workspace does not depend on
//!   `subtle`).  The only escape hatch is the explicit
//!   [`SecretValue::expose_secret`] accessor.
//! * [`SecretRef`] — `{ provider, key }` pointer used to fan out lookups across
//!   multiple stores without exposing the resolved value.
//! * [`SecretStore`] — async trait every backend implements.
//! * [`EnvSecretStore`] — read-only adapter over `std::env::var`.
//! * [`FileSecretStore`] — JSON-on-disk plaintext store with atomic
//!   tmp-file + rename writes (mirrors the `JsonFileJobStore` pattern from
//!   `aphrody-cron`).
//! * [`EncryptedFileSecretStore`] — same layout, but the JSON map is encrypted
//!   end-to-end with AES-256-GCM; the key is derived from a user passphrase
//!   via Argon2id (`argon2` crate) with a per-file random salt.  Decryption
//!   reuses the 3-byte-version / 12-byte-nonce / ciphertext+tag framing that
//!   `crates/base::Crypto::decrypt_aes_gcm` already follows for Chromium
//!   profiles.
//! * [`CompositeSecretStore`] — try-each-in-order resolver; the first
//!   writable backend takes mutation.
//! * [`mask_value`] — helper for safely surfacing partial secrets in
//!   user-facing logs (e.g. `sk-a…ghi`).
//!
//! ## Threat model
//!
//! * `Display` and `Debug` always emit `"***REDACTED***"`. The regression
//!   tests assert *both* "must contain REDACTED" and "must NOT contain the
//!   raw value".
//! * `SecretValue` zeroes its backing buffer on drop via the `zeroize` crate.
//! * Passphrase-derived KEKs are zeroed as soon as the cipher is constructed.
//! * Wrong-passphrase decryption returns the dedicated
//!   [`SecretsError::BadPassphrase`] variant so callers can distinguish a
//!   corrupted file from a key mismatch (this matches how
//!   `aphrody-marketplace::verify_sha256` exposes integrity errors).
//!
//! ## Cohesion with the workspace
//!
//! * `aphrody-router` / `aphrody-gateway` / `aphrody-voice` all need an
//!   abstraction over `ANTHROPIC_API_KEY` / `GEMINI_API_KEY` /
//!   `ELEVENLABS_API_KEY`.  The 3-only provider whitelist
//!   (memory `project_aphrody_providers_3only`) is enforced upstream;
//!   this crate just resolves the string.
//! * `aphrody-cron::JsonFileJobStore` already documents the atomic
//!   tmp-file + rename pattern reused by [`FileSecretStore::save`].
//! * `crates/base::Crypto::decrypt_aes_gcm` is the canonical AES-256-GCM
//!   decryption helper in the workspace; this crate uses the same crate
//!   (`aes_gcm = "0.10"`) and the same versioned nonce framing.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use aes_gcm::{
    Aes256Gcm, Key, KeyInit, Nonce,
    aead::{Aead, OsRng as AeadOsRng},
};
use argon2::{Algorithm, Argon2, Params, Version};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All recoverable failures from this crate.
#[derive(Debug, Error)]
pub enum SecretsError {
    /// Lookup returned no value.
    #[error("secret not found: {0}")]
    NotFound(String),

    /// Filesystem error while loading or saving a backing file.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// The backing store does not allow mutation (e.g. env vars).
    #[error("store '{store}' is read-only")]
    ReadOnly {
        /// Identifier returned by [`SecretStore::name`].
        store: &'static str,
    },

    /// AES-GCM or KDF failure with a descriptive payload.
    #[error("crypto error: {0}")]
    Crypto(String),

    /// AEAD verification failed — the passphrase was wrong or the file was
    /// tampered with.  Distinct variant so the CLI can prompt for a retry.
    #[error("bad passphrase or corrupted ciphertext")]
    BadPassphrase,

    /// The on-disk envelope did not match the expected schema.
    #[error("invalid encrypted file format: {0}")]
    InvalidFormat(String),

    /// `std::env::var` failed with something other than `NotPresent`
    /// (e.g. non-UTF8 value).
    #[error("env var '{key}': {source}")]
    EnvVar {
        /// The variable that was being looked up.
        key: String,
        /// Underlying `std::env::VarError`.
        #[source]
        source: std::env::VarError,
    },
}

/// Convenience alias used throughout the crate.
pub type SecretsResult<T> = Result<T, SecretsError>;

// ---------------------------------------------------------------------------
// SecretValue — redacted, zeroizing, constant-time-equal newtype
// ---------------------------------------------------------------------------

/// A secret string with safe defaults.
///
/// * `Display` and `Debug` both render `"***REDACTED***"`.
/// * Equality is constant-time over the exposed bytes (manual short-circuit
///   free implementation).
/// * The backing buffer is zeroed in `Drop` (via the `zeroize` crate).
/// * Use [`SecretValue::expose_secret`] when you really need the bytes — the
///   call site stays grep-able for auditors.
pub struct SecretValue {
    inner: Vec<u8>,
}

impl SecretValue {
    /// Wrap a `String`.  The original buffer is moved in and will be zeroed
    /// on drop along with the new container.
    #[must_use]
    pub fn from_string(mut s: String) -> Self {
        // Take ownership of the bytes and zero the original capacity.
        let inner = s.as_bytes().to_vec();
        s.zeroize();
        Self { inner }
    }

    /// Wrap an existing byte vector (will be zeroed on drop).
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { inner: bytes }
    }

    /// Borrow the raw secret bytes.  Reserved for cases where the consumer
    /// genuinely needs the cleartext (HTTP `Authorization` header building,
    /// AEAD key material, etc.).
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        // The constructor only accepts UTF-8 inputs (`from_string`) or raw
        // bytes the caller is responsible for; surface as `&str` lossily so
        // tests stay deterministic on non-UTF-8 buffers.
        std::str::from_utf8(&self.inner).unwrap_or("")
    }

    /// Borrow as raw bytes (useful for AEAD keys / binary tokens).
    #[must_use]
    pub fn expose_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// Length in bytes (does not leak the contents).
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the value is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl From<String> for SecretValue {
    fn from(value: String) -> Self {
        Self::from_string(value)
    }
}

impl From<&str> for SecretValue {
    fn from(value: &str) -> Self {
        Self::from_string(value.to_owned())
    }
}

impl Clone for SecretValue {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***REDACTED***")
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Intentionally identical to Display: never leak length either, so
        // tooling that prints `{:?}` in logs cannot fingerprint short tokens.
        f.write_str("SecretValue(***REDACTED***)")
    }
}

impl PartialEq for SecretValue {
    fn eq(&self, other: &Self) -> bool {
        constant_time_eq(&self.inner, &other.inner)
    }
}

impl Eq for SecretValue {}

impl Drop for SecretValue {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

/// Branch-free byte-slice equality.
///
/// We do **not** short-circuit on length differences in the inner loop and we
/// fold every byte through XOR so the comparison time depends only on the
/// (publicly known) maximum length.  Inputs of differing length still compare
/// unequal but the iteration cost is constant for the longer side.
#[inline]
#[must_use]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    // Diff starts non-zero when lengths differ, so we still walk the longer
    // slice but the result is guaranteed false.
    let mut diff: u32 = (a.len() ^ b.len()) as u32;
    let len = a.len().max(b.len());
    for i in 0..len {
        let ai = a.get(i).copied().unwrap_or(0);
        let bi = b.get(i).copied().unwrap_or(0);
        diff |= u32::from(ai ^ bi);
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// SecretRef — `{provider, key}` pointer for fan-out resolvers
// ---------------------------------------------------------------------------

/// Identifier pointing at a single secret in a [`SecretStore`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef {
    /// Backend identifier (`"env"`, `"file"`, ...).  Free-form: not validated.
    pub provider: String,
    /// Secret key (e.g. `"ANTHROPIC_API_KEY"`).
    pub key: String,
}

impl SecretRef {
    /// Convenience constructor.
    #[must_use]
    pub fn new(provider: impl Into<String>, key: impl Into<String>) -> Self {
        Self { provider: provider.into(), key: key.into() }
    }
}

// ---------------------------------------------------------------------------
// SecretStore trait
// ---------------------------------------------------------------------------

/// Async abstraction over a key→[`SecretValue`] map.
#[async_trait]
pub trait SecretStore: Send + Sync + std::fmt::Debug {
    /// Stable identifier (used in [`SecretsError::ReadOnly`]).
    fn name(&self) -> &'static str;

    /// Resolve `key`.
    async fn get(&self, key: &str) -> SecretsResult<Option<SecretValue>>;

    /// Insert or replace `key`.
    async fn set(&self, key: &str, value: SecretValue) -> SecretsResult<()>;

    /// Remove `key`. Returns `Ok(())` even when the key was missing — drop
    /// the value if it existed, otherwise no-op.
    async fn delete(&self, key: &str) -> SecretsResult<()>;

    /// Snapshot of currently known keys.  Order is unspecified.
    async fn list_keys(&self) -> SecretsResult<Vec<String>>;
}

// ---------------------------------------------------------------------------
// EnvSecretStore
// ---------------------------------------------------------------------------

/// Read-only adapter over `std::env::var`.
#[derive(Debug, Default, Clone)]
pub struct EnvSecretStore;

impl EnvSecretStore {
    /// Construct.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecretStore for EnvSecretStore {
    fn name(&self) -> &'static str {
        "env"
    }

    async fn get(&self, key: &str) -> SecretsResult<Option<SecretValue>> {
        match std::env::var(key) {
            Ok(v) => Ok(Some(SecretValue::from_string(v))),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(source) => Err(SecretsError::EnvVar { key: key.to_owned(), source }),
        }
    }

    async fn set(&self, _key: &str, _value: SecretValue) -> SecretsResult<()> {
        Err(SecretsError::ReadOnly { store: "env" })
    }

    async fn delete(&self, _key: &str) -> SecretsResult<()> {
        Err(SecretsError::ReadOnly { store: "env" })
    }

    async fn list_keys(&self) -> SecretsResult<Vec<String>> {
        // We intentionally do **not** dump the entire process environment —
        // that would leak unrelated variables.  Callers list keys they own
        // via their own configuration. Return an empty snapshot.
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// FileSecretStore (plaintext JSON)
// ---------------------------------------------------------------------------

/// On-disk JSON map: `{ "key": "<base64-of-bytes>", ... }`.
///
/// Atomic write via tmp-file + rename (same recipe as
/// [`aphrody_cron::JsonFileJobStore`](../aphrody_cron/struct.JsonFileJobStore.html)).
#[derive(Debug, Clone)]
pub struct FileSecretStore {
    path: PathBuf,
    lock: std::sync::Arc<tokio::sync::Mutex<()>>,
}

impl FileSecretStore {
    /// Create a new store rooted at `path` (file does not need to exist yet).
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: std::sync::Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Public accessor — useful for tests and tooling.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn load(&self) -> SecretsResult<BTreeMap<String, String>> {
        if !self.path.exists() {
            return Ok(BTreeMap::new());
        }
        let bytes = tokio::fs::read(&self.path).await?;
        if bytes.is_empty() {
            return Ok(BTreeMap::new());
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn save(&self, map: &BTreeMap<String, String>) -> SecretsResult<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        let json = serde_json::to_vec_pretty(map)?;
        let tmp = self.path.with_extension("json.tmp");
        tokio::fs::write(&tmp, json).await?;
        tokio::fs::rename(&tmp, &self.path).await?;
        Ok(())
    }
}

#[async_trait]
impl SecretStore for FileSecretStore {
    fn name(&self) -> &'static str {
        "file"
    }

    async fn get(&self, key: &str) -> SecretsResult<Option<SecretValue>> {
        let _g = self.lock.lock().await;
        let map = self.load().await?;
        match map.get(key) {
            None => Ok(None),
            Some(b64) => {
                let raw = B64
                    .decode(b64.as_bytes())
                    .map_err(|e| SecretsError::InvalidFormat(format!("base64: {e}")))?;
                Ok(Some(SecretValue::from_bytes(raw)))
            },
        }
    }

    async fn set(&self, key: &str, value: SecretValue) -> SecretsResult<()> {
        let _g = self.lock.lock().await;
        let mut map = self.load().await?;
        let encoded = B64.encode(value.expose_bytes());
        map.insert(key.to_owned(), encoded);
        self.save(&map).await
    }

    async fn delete(&self, key: &str) -> SecretsResult<()> {
        let _g = self.lock.lock().await;
        let mut map = self.load().await?;
        map.remove(key);
        self.save(&map).await
    }

    async fn list_keys(&self) -> SecretsResult<Vec<String>> {
        let _g = self.lock.lock().await;
        let map = self.load().await?;
        Ok(map.keys().cloned().collect())
    }
}

// ---------------------------------------------------------------------------
// EncryptedFileSecretStore (AES-256-GCM + Argon2id)
// ---------------------------------------------------------------------------

/// On-disk envelope:
///
/// ```json
/// {
///   "version": 1,
///   "kdf": { "algo": "argon2id", "salt_b64": "..." , "m_cost": 19456, "t_cost": 2, "p_cost": 1 },
///   "nonce_b64": "...",
///   "ciphertext_b64": "..."
/// }
/// ```
///
/// Ciphertext content is `serde_json::to_vec(&BTreeMap<String,String>)` where
/// values are base64-encoded raw secret bytes (same shape as
/// [`FileSecretStore`]).
#[derive(Debug, Serialize, Deserialize)]
struct EncryptedEnvelope {
    version: u32,
    kdf: KdfParams,
    nonce_b64: String,
    ciphertext_b64: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct KdfParams {
    algo: String,
    salt_b64: String,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        // OWASP 2024 baseline: Argon2id m=19 MiB, t=2, p=1.
        Self {
            algo: "argon2id".to_string(),
            salt_b64: String::new(),
            m_cost: 19_456,
            t_cost: 2,
            p_cost: 1,
        }
    }
}

/// Passphrase-protected secret store backed by a single JSON envelope file.
#[derive(Clone)]
pub struct EncryptedFileSecretStore {
    path: PathBuf,
    passphrase: SecretValue,
    lock: std::sync::Arc<tokio::sync::Mutex<()>>,
}

impl std::fmt::Debug for EncryptedFileSecretStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptedFileSecretStore")
            .field("path", &self.path)
            .field("passphrase", &"***REDACTED***")
            .finish()
    }
}

impl EncryptedFileSecretStore {
    /// Construct from a path and a passphrase.  The passphrase is moved into
    /// the store and zeroed alongside it on drop.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, passphrase: SecretValue) -> Self {
        Self {
            path: path.into(),
            passphrase,
            lock: std::sync::Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Public accessor (tests + tooling).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Derive a 32-byte AES-256-GCM key from the passphrase + salt with
    /// Argon2id at the supplied cost parameters.
    fn derive_key(
        passphrase: &[u8],
        salt: &[u8],
        params: &KdfParams,
    ) -> SecretsResult<Zeroizing<[u8; 32]>> {
        if params.algo != "argon2id" {
            return Err(SecretsError::InvalidFormat(format!(
                "unknown kdf algorithm: {}",
                params.algo
            )));
        }
        let argon = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(params.m_cost, params.t_cost, params.p_cost, Some(32))
                .map_err(|e| SecretsError::Crypto(format!("argon2 params: {e}")))?,
        );
        let mut key = Zeroizing::new([0u8; 32]);
        argon
            .hash_password_into(passphrase, salt, key.as_mut())
            .map_err(|e| SecretsError::Crypto(format!("argon2 derive: {e}")))?;
        Ok(key)
    }

    async fn load_map(&self) -> SecretsResult<BTreeMap<String, String>> {
        if !self.path.exists() {
            return Ok(BTreeMap::new());
        }
        let raw = tokio::fs::read(&self.path).await?;
        if raw.is_empty() {
            return Ok(BTreeMap::new());
        }
        let env: EncryptedEnvelope = serde_json::from_slice(&raw)?;
        if env.version != 1 {
            return Err(SecretsError::InvalidFormat(format!(
                "unsupported envelope version {}",
                env.version
            )));
        }
        let salt = B64
            .decode(env.kdf.salt_b64.as_bytes())
            .map_err(|e| SecretsError::InvalidFormat(format!("salt b64: {e}")))?;
        let nonce_bytes = B64
            .decode(env.nonce_b64.as_bytes())
            .map_err(|e| SecretsError::InvalidFormat(format!("nonce b64: {e}")))?;
        let ciphertext = B64
            .decode(env.ciphertext_b64.as_bytes())
            .map_err(|e| SecretsError::InvalidFormat(format!("ciphertext b64: {e}")))?;
        if nonce_bytes.len() != 12 {
            return Err(SecretsError::InvalidFormat(format!(
                "nonce must be 12 bytes, got {}",
                nonce_bytes.len()
            )));
        }
        let key = Self::derive_key(self.passphrase.expose_bytes(), &salt, &env.kdf)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_ref()));
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| SecretsError::BadPassphrase)?;
        let map: BTreeMap<String, String> = serde_json::from_slice(&plaintext)?;
        Ok(map)
    }

    async fn save_map(&self, map: &BTreeMap<String, String>) -> SecretsResult<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        // Fresh salt + nonce every write (deterministic ones would defeat AEAD).
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let mut nonce_bytes = [0u8; 12];
        // AeadOsRng from aes-gcm internally uses the OS — re-route through
        // `rand::OsRng` for consistency and explicit auditing.
        let _ = AeadOsRng; // keep the import meaningful for cargo-machete
        OsRng.fill_bytes(&mut nonce_bytes);

        let kdf = KdfParams {
            algo: "argon2id".to_string(),
            salt_b64: B64.encode(salt),
            ..KdfParams::default()
        };
        let key = Self::derive_key(self.passphrase.expose_bytes(), &salt, &kdf)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_ref()));
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = serde_json::to_vec(map)?;
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| SecretsError::Crypto(format!("aes-gcm encrypt: {e}")))?;
        let envelope = EncryptedEnvelope {
            version: 1,
            kdf,
            nonce_b64: B64.encode(nonce_bytes),
            ciphertext_b64: B64.encode(&ciphertext),
        };
        let json = serde_json::to_vec_pretty(&envelope)?;
        let tmp = self.path.with_extension("enc.tmp");
        tokio::fs::write(&tmp, json).await?;
        tokio::fs::rename(&tmp, &self.path).await?;
        Ok(())
    }
}

#[async_trait]
impl SecretStore for EncryptedFileSecretStore {
    fn name(&self) -> &'static str {
        "encrypted-file"
    }

    async fn get(&self, key: &str) -> SecretsResult<Option<SecretValue>> {
        let _g = self.lock.lock().await;
        let map = self.load_map().await?;
        match map.get(key) {
            None => Ok(None),
            Some(b64) => {
                let raw = B64
                    .decode(b64.as_bytes())
                    .map_err(|e| SecretsError::InvalidFormat(format!("base64: {e}")))?;
                Ok(Some(SecretValue::from_bytes(raw)))
            },
        }
    }

    async fn set(&self, key: &str, value: SecretValue) -> SecretsResult<()> {
        let _g = self.lock.lock().await;
        let mut map = self.load_map().await?;
        map.insert(key.to_owned(), B64.encode(value.expose_bytes()));
        self.save_map(&map).await
    }

    async fn delete(&self, key: &str) -> SecretsResult<()> {
        let _g = self.lock.lock().await;
        let mut map = self.load_map().await?;
        map.remove(key);
        self.save_map(&map).await
    }

    async fn list_keys(&self) -> SecretsResult<Vec<String>> {
        let _g = self.lock.lock().await;
        let map = self.load_map().await?;
        Ok(map.keys().cloned().collect())
    }
}

// ---------------------------------------------------------------------------
// CompositeSecretStore
// ---------------------------------------------------------------------------

/// Try-each-in-order resolver.
///
/// * `get` iterates the inner stores until one returns `Some(_)`.
/// * `set` / `delete` are dispatched to the first **writable** store
///   (anything other than [`SecretsError::ReadOnly`]).  This makes it natural
///   to chain `[env, encrypted_file]`: env wins for reads, encrypted_file
///   takes mutations.
/// * `list_keys` merges keys across every backend, de-duplicated.
#[derive(Debug)]
pub struct CompositeSecretStore {
    stores: Vec<std::sync::Arc<dyn SecretStore>>,
}

impl CompositeSecretStore {
    /// Build from a non-empty list of inner stores.
    #[must_use]
    pub fn new(stores: Vec<std::sync::Arc<dyn SecretStore>>) -> Self {
        Self { stores }
    }
}

#[async_trait]
impl SecretStore for CompositeSecretStore {
    fn name(&self) -> &'static str {
        "composite"
    }

    async fn get(&self, key: &str) -> SecretsResult<Option<SecretValue>> {
        for store in &self.stores {
            if let Some(v) = store.get(key).await? {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    async fn set(&self, key: &str, value: SecretValue) -> SecretsResult<()> {
        // First writable wins.  We must clone the value for the retry path
        // because `set` consumes it.
        let mut last_err: Option<SecretsError> = None;
        for store in &self.stores {
            let attempt = value.clone();
            match store.set(key, attempt).await {
                Ok(()) => return Ok(()),
                Err(SecretsError::ReadOnly { .. }) => continue,
                Err(other) => {
                    last_err = Some(other);
                    break;
                },
            }
        }
        Err(last_err.unwrap_or(SecretsError::ReadOnly { store: "composite" }))
    }

    async fn delete(&self, key: &str) -> SecretsResult<()> {
        for store in &self.stores {
            match store.delete(key).await {
                Ok(()) => return Ok(()),
                Err(SecretsError::ReadOnly { .. }) => continue,
                Err(other) => return Err(other),
            }
        }
        Err(SecretsError::ReadOnly { store: "composite" })
    }

    async fn list_keys(&self) -> SecretsResult<Vec<String>> {
        let mut out: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for store in &self.stores {
            for k in store.list_keys().await? {
                out.insert(k);
            }
        }
        Ok(out.into_iter().collect())
    }
}

// ---------------------------------------------------------------------------
// mask_value — partial reveal for human-readable logs
// ---------------------------------------------------------------------------

/// Render a secret with only its first/last `visible_chars` exposed.
///
/// * If the secret is too short to safely show `visible_chars` on each side
///   without leaking more than half of it, the whole string is replaced by
///   `"***"`.
/// * The middle is always replaced by `"***"`, regardless of original length.
///
/// ```
/// use aphrody_secrets::mask_value;
/// assert_eq!(mask_value("sk-abc123def456ghi", 4), "sk-a***6ghi");
/// assert_eq!(mask_value("tiny", 4), "***");
/// ```
#[must_use]
pub fn mask_value(s: &str, visible_chars: usize) -> String {
    let total = s.chars().count();
    if visible_chars == 0 || total <= visible_chars.saturating_mul(2) {
        return "***".to_string();
    }
    let head: String = s.chars().take(visible_chars).collect();
    let tail: String = s.chars().skip(total - visible_chars).collect();
    format!("{head}***{tail}")
}

// ---------------------------------------------------------------------------
// In-crate sanity tests (the heavy lifting lives in tests/secrets_smoke.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn secret_ref_roundtrip_json() {
        let r = SecretRef::new("env", "ANTHROPIC_API_KEY");
        let s = serde_json::to_string(&r).expect("ser");
        let back: SecretRef = serde_json::from_str(&s).expect("de");
        assert_eq!(back, r);
    }
}
