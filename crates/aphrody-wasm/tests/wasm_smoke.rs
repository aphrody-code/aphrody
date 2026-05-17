// SPDX-License-Identifier: Apache-2.0
//
// Smoke tests for the aphrody-wasm browser surface.
//
// Run: cargo test --target wasm32-unknown-unknown -p aphrody-wasm
// Or via wasm-pack: wasm-pack test --headless --firefox crates/aphrody-wasm

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// `version()` must return a string that begins with the current canary prefix.
#[wasm_bindgen_test]
fn version_returns_canary() {
    let v = aphrody_wasm::version();
    assert!(v.starts_with("1.0.0"), "expected 1.0.0-* version, got {v}");
}

/// `platform_short_name()` must identify the wasm32-unknown-unknown target
/// when compiled for the browser.
#[wasm_bindgen_test]
fn platform_short_name_is_wasm32_unknown_unknown() {
    let p = aphrody_wasm::platform_short_name();
    assert_eq!(p, "wasm32-unknown-unknown", "expected wasm32-unknown-unknown, got {p}");
}

/// `decrypt_aes_gcm` must return an error when the ciphertext is empty.
/// The implementation requires at least 15 bytes (3-byte version prefix +
/// 12-byte nonce), so an empty slice must be rejected immediately.
#[wasm_bindgen_test]
fn decrypt_aes_gcm_rejects_empty_ciphertext() {
    let empty: Vec<u8> = vec![];
    let key: Vec<u8> = vec![0u8; 32];
    let result = aphrody_wasm::decrypt_aes_gcm(&empty, &key);
    assert!(result.is_err(), "expected error on empty ciphertext, got Ok");
}

/// `decrypt_aes_gcm` must return an error when the ciphertext is shorter than
/// the mandatory 15-byte minimum (3-byte prefix + 12-byte nonce).
#[wasm_bindgen_test]
fn decrypt_aes_gcm_rejects_short_ciphertext() {
    let short: Vec<u8> = vec![0u8; 10]; // 10 < 15 minimum
    let key: Vec<u8> = vec![0u8; 32];
    let result = aphrody_wasm::decrypt_aes_gcm(&short, &key);
    assert!(result.is_err(), "expected error on short ciphertext, got Ok");
}

/// `decrypt_aes_gcm` must return an error when the ciphertext passes the
/// length check but contains a bad authentication tag (AES-GCM integrity
/// failure).  The payload here is 15+ bytes with a zero nonce and garbage
/// ciphertext bytes — decryption will fail the GCM tag verification.
#[wasm_bindgen_test]
fn decrypt_aes_gcm_rejects_bad_auth_tag() {
    // [3-byte version prefix][12-byte nonce][ciphertext+tag — all zeros, invalid]
    let mut ciphertext: Vec<u8> = vec![0u8; 35];
    ciphertext[0] = b'v';
    ciphertext[1] = b'1';
    ciphertext[2] = b'0';
    let key: Vec<u8> = vec![0u8; 32];
    let result = aphrody_wasm::decrypt_aes_gcm(&ciphertext, &key);
    assert!(result.is_err(), "expected GCM auth failure, got Ok");
}
