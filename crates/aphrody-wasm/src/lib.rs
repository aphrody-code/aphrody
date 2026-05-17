// SPDX-License-Identifier: Apache-2.0
//! Browser-facing WASM bridge for aphrody.
//!
//! Exposes base-crate primitives via wasm-bindgen so JS/TS code running in a
//! browser or Node can call into aphrody without a native binary.  The module
//! is designed to be consumed either via `wasm-pack` (producing an npm package)
//! or as a sibling `rlib` dependency within the workspace.

#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Pull in the panic hook and console logger only when targeting the browser.
// On native targets these imports do not exist, which is correct: native tests
// use the standard test harness and `tracing`.
#[cfg(target_arch = "wasm32")]
use console_error_panic_hook;

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
    base::Crypto::decrypt_aes_gcm(ciphertext, key)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

// ---------------------------------------------------------------------------
// Native-only tests: logic that does not require a wasm runtime.
// ---------------------------------------------------------------------------
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

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
}
