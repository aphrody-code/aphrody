// SPDX-License-Identifier: Apache-2.0
//! Browser-facing WASM bridge for aphrody.
//!
//! Exposes base-crate primitives via wasm-bindgen so JS/TS code running in a
//! browser or Node can call into aphrody without a native binary.  The module
//! is designed to be consumed either via `wasm-pack` (producing an npm package)
//! or as a sibling `rlib` dependency within the workspace.

#![forbid(unsafe_code)]

// Pull in the panic hook and console logger only when targeting the browser.
// On native targets these imports do not exist, which is correct: native tests
// use the standard test harness and `tracing`.
#[cfg(target_arch = "wasm32")] use console_error_panic_hook;
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;

/// Installs the browser panic hook and wires `log` macros to `console.*`.
///
/// Must be called once before any other aphrody-wasm function.  Subsequent
/// calls are no-ops (both hooks are idempotent).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    // Level::Debug is verbose but safe for pre-1.0 WASM; callers can filter
    // via the browser console's own log-level controls.
    let _ = console_log::init_with_level(log::Level::Debug);
}

/// Returns the crate version as declared in `Cargo.toml`.
///
/// Useful for JS callers that need to gate behaviour on the aphrody version
/// without fetching a manifest file over the network.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Returns the compile-time target triple short name.
///
/// Distinguishes browser wasm (`wasm32-unknown-unknown`) from WASI
/// (`wasm32-wasip1`) so JS glue can adjust capability detection accordingly.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn platform_short_name() -> String {
    // cfg target_os = "unknown" is set by wasm32-unknown-unknown; all other
    // wasm32 targets (wasip1, emscripten) set a non-empty target_os.
    #[cfg(target_os = "unknown")]
    {
        "wasm32-unknown-unknown".to_owned()
    }
    #[cfg(not(target_os = "unknown"))]
    {
        // Covers wasm32-wasip1, wasm32-wasi, wasm32-emscripten, etc.
        concat!(env!("CARGO_CFG_TARGET_ARCH"), "-", env!("CARGO_CFG_TARGET_OS")).to_owned()
    }
}

/// Decrypts AES-256-GCM ciphertext produced by the base crate.
///
/// `ciphertext` must be at least 15 bytes: 3-byte version prefix, 12-byte
/// nonce, then the actual ciphertext+tag as written by `base::Crypto`.
/// `key` must be exactly 32 bytes.
///
/// Returns the plaintext bytes on success, or throws a JS `Error` on failure.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decrypt_aes_gcm(ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, JsValue> {
    base::Crypto::decrypt_aes_gcm(ciphertext, key).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Encrypts a plaintext buffer using AES-256-GCM with the given key.
///
/// Wire format: `[b'v', b'1', b'0']` (3 bytes) + `[12-byte nonce]` + `[ciphertext + 16-byte GCM
/// tag]`. The output is directly consumable by [`decrypt_aes_gcm`] and by
/// `base::Crypto::decrypt_aes_gcm` on native targets.
///
/// # Arguments
/// * `plaintext` — bytes to encrypt (arbitrary length).
/// * `key`       — exactly 32 bytes (AES-256 key material).
///
/// # Returns
/// `Vec<u8>` in the aphrody binary envelope format on success.
///
/// # Errors
/// Returns a `JsValue` error string if `key.len() != 32` or if the AEAD
/// operation fails (should never happen with a well-formed key and nonce).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn encrypt_aes_gcm(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, JsValue> {
    encrypt_aes_gcm_impl(plaintext, key).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Platform-independent encrypt implementation.
///
/// Separated from the `#[wasm_bindgen]`-decorated export so that native unit
/// tests can call it without a `JsValue` return type.
#[allow(dead_code)]
fn encrypt_aes_gcm_impl(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::{
        Aes256Gcm, Key,
        aead::{Aead, AeadCore, KeyInit, OsRng},
    };

    if key.len() != 32 {
        return Err("key must be 32 bytes (AES-256)".to_owned());
    }

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext =
        cipher.encrypt(&nonce, plaintext).map_err(|e| format!("AES-GCM encrypt failed: {e}"))?;

    // Wire format: 3-byte version prefix identical to what base::Crypto::decrypt_aes_gcm
    // expects at bytes [0..3]; nonce at [3..15]; ciphertext+tag at [15..].
    let mut out = Vec::with_capacity(3 + 12 + ciphertext.len());
    out.extend_from_slice(b"v10");
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Native-only tests: logic that does not require a wasm runtime.
// ---------------------------------------------------------------------------
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::encrypt_aes_gcm_impl;

    /// Verifies the version string is non-empty and matches the expected
    /// semver-ish prefix that every aphrody workspace package carries.
    #[test]
    fn version_string_is_nonempty() {
        let v = env!("CARGO_PKG_VERSION");
        assert!(!v.is_empty(), "CARGO_PKG_VERSION must not be empty");
        // Workspace version starts with "1." for the current canary cycle.
        assert!(v.starts_with("1."), "expected version to start with '1.', got {v:?}");
    }

    /// Verifies that `decrypt_aes_gcm` rejects inputs that are too short
    /// without panicking, returning a proper error.
    #[test]
    fn decrypt_aes_gcm_rejects_short_input() {
        let key = [0u8; 32];
        let short = [0u8; 10]; // < 15 bytes minimum
        let result = base::Crypto::decrypt_aes_gcm(&short, &key);
        assert!(result.is_err(), "expected an error for ciphertext shorter than 15 bytes");
    }

    /// Full encrypt → decrypt round-trip through the aphrody wire format.
    ///
    /// Uses `encrypt_aes_gcm_impl` (the non-WASM shim) for the encrypt half
    /// and `base::Crypto::decrypt_aes_gcm` for the decrypt half — the exact
    /// same paths exercised in production by the WASM exports.
    #[test]
    fn encrypt_decrypt_round_trip() {
        let plaintext = b"hello aphrody world";
        let key = [0u8; 32];

        let ct = encrypt_aes_gcm_impl(plaintext, &key).expect("encrypt must succeed");
        let pt = base::Crypto::decrypt_aes_gcm(&ct, &key).expect("decrypt must succeed");
        assert_eq!(pt.as_slice(), plaintext);
    }

    /// `encrypt_aes_gcm_impl` must reject a 16-byte (AES-128) key.
    #[test]
    fn encrypt_rejects_short_key() {
        let plaintext = b"x";
        let short_key = [0u8; 16];
        assert!(encrypt_aes_gcm_impl(plaintext, &short_key).is_err());
    }
}
