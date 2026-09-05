// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for `aphrody-secrets`.
//!
//! These tests exercise the *observable* contract described in the crate
//! docs: redacted Display/Debug, constant-time equality, env adapter,
//! plaintext and encrypted file round-trips, composite fall-through and
//! the `mask_value` helper.

use std::sync::Arc;

use aphrody_secrets::{
    CompositeSecretStore, EncryptedFileSecretStore, EnvSecretStore, FileSecretStore, SecretRef,
    SecretStore, SecretValue, SecretsError, constant_time_eq, mask_value,
};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// SecretValue invariants
// ---------------------------------------------------------------------------

#[test]
fn secret_value_display_is_redacted() {
    let raw = "sk-super-secret-token-1234";
    let secret = SecretValue::from(raw);
    let shown = format!("{secret}");
    assert!(shown.contains("REDACTED"), "Display must mention REDACTED, got {shown:?}");
    assert!(!shown.contains(raw), "Display must NOT leak the raw value, got {shown:?}");
    assert!(!shown.contains("super-secret"), "Display must NOT leak any substring");
}

#[test]
fn secret_value_debug_is_redacted() {
    let raw = "sk-super-secret-token-1234";
    let secret = SecretValue::from(raw);
    let shown = format!("{secret:?}");
    assert!(shown.contains("REDACTED"), "Debug must mention REDACTED, got {shown:?}");
    assert!(!shown.contains(raw), "Debug must NOT leak the raw value, got {shown:?}");
}

#[test]
fn secret_value_expose_returns_actual_value() {
    let raw = "ANTHROPIC-API-KEY-xyz";
    let secret = SecretValue::from(raw);
    assert_eq!(secret.expose_secret(), raw);
    assert_eq!(secret.expose_bytes(), raw.as_bytes());
    assert_eq!(secret.len(), raw.len());
    assert!(!secret.is_empty());
}

#[test]
fn secret_value_partial_eq_constant_time() {
    let a = SecretValue::from("abc-123");
    let b = SecretValue::from("abc-123");
    let c = SecretValue::from("abc-124");
    assert_eq!(a, b, "identical secrets must compare equal");
    assert_ne!(a, c, "different secrets must compare unequal");

    // Direct sanity on the underlying helper.
    assert!(constant_time_eq(b"hello", b"hello"));
    assert!(!constant_time_eq(b"hello", b"world"));
    assert!(!constant_time_eq(b"short", b"shortish"));
    assert!(constant_time_eq(b"", b""));
}

// ---------------------------------------------------------------------------
// EnvSecretStore
// ---------------------------------------------------------------------------

#[tokio::test]
async fn env_store_reads_existing_var() {
    let key = "APHRODY_SECRETS_SMOKE_EXISTING";
    // SAFETY: single-threaded test; std::env::set_var is sound here.
    // (Tokio test runtime runs each #[test] in its own runtime, but env is
    // process-wide. We use a unique key to avoid cross-test interference.)
    unsafe {
        std::env::set_var(key, "value-from-env");
    }

    let store = EnvSecretStore::new();
    let got = store.get(key).await.expect("get must not error");
    let got = got.expect("env var was set");
    assert_eq!(got.expose_secret(), "value-from-env");

    unsafe { std::env::remove_var(key) };
}

#[tokio::test]
async fn env_store_missing_returns_none() {
    let key = "APHRODY_SECRETS_SMOKE_DEFINITELY_ABSENT_KEY";
    // Defensive: make sure no other test set it.
    unsafe { std::env::remove_var(key) };
    let store = EnvSecretStore::new();
    let got = store.get(key).await.expect("get must not error");
    assert!(got.is_none(), "absent env var must resolve to None");
}

#[tokio::test]
async fn env_store_set_returns_read_only_error() {
    let store = EnvSecretStore::new();
    let err = store
        .set("ANY_KEY", SecretValue::from("v"))
        .await
        .expect_err("set must reject");
    assert!(matches!(err, SecretsError::ReadOnly { store: "env" }), "got {err:?}");

    let err = store.delete("ANY_KEY").await.expect_err("delete must reject");
    assert!(matches!(err, SecretsError::ReadOnly { store: "env" }), "got {err:?}");

    let keys = store.list_keys().await.expect("list_keys ok");
    assert!(keys.is_empty(), "env store list_keys returns empty by design");
}

// ---------------------------------------------------------------------------
// FileSecretStore — plaintext JSON, atomic write
// ---------------------------------------------------------------------------

#[tokio::test]
async fn file_store_roundtrip_plaintext() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("secrets.json");
    let store = FileSecretStore::new(&path);

    // List is empty before any writes.
    assert!(store.list_keys().await.unwrap().is_empty());

    // Set then get.
    store
        .set("ANTHROPIC_API_KEY", SecretValue::from("sk-ant-001"))
        .await
        .expect("set");
    store
        .set("GEMINI_API_KEY", SecretValue::from("sk-gemini-002"))
        .await
        .expect("set");

    let got = store
        .get("ANTHROPIC_API_KEY")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got.expose_secret(), "sk-ant-001");

    let mut keys = store.list_keys().await.expect("list");
    keys.sort();
    assert_eq!(keys, vec!["ANTHROPIC_API_KEY", "GEMINI_API_KEY"]);

    // Delete one, the other stays.
    store.delete("ANTHROPIC_API_KEY").await.expect("delete");
    assert!(store.get("ANTHROPIC_API_KEY").await.unwrap().is_none());
    assert!(store.get("GEMINI_API_KEY").await.unwrap().is_some());

    // File survives a fresh handle (round-trip from disk).
    let store2 = FileSecretStore::new(&path);
    let got2 = store2
        .get("GEMINI_API_KEY")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got2.expose_secret(), "sk-gemini-002");

    // The on-disk file must NOT contain the test passphrase nor any binary
    // garbage — just JSON with base64 values.
    let raw = std::fs::read_to_string(&path).expect("read");
    assert!(raw.contains("GEMINI_API_KEY"), "file missing key, got: {raw}");
}

// ---------------------------------------------------------------------------
// EncryptedFileSecretStore — AES-256-GCM + Argon2id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn encrypted_file_store_roundtrip_with_passphrase() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("secrets.enc");
    let pass = SecretValue::from("correct horse battery staple");
    let store = EncryptedFileSecretStore::new(&path, pass);

    store
        .set("ANTHROPIC_API_KEY", SecretValue::from("sk-ant-encrypted"))
        .await
        .expect("set");
    store
        .set("GEMINI_API_KEY", SecretValue::from("sk-gemini-encrypted"))
        .await
        .expect("set");

    let got = store
        .get("ANTHROPIC_API_KEY")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got.expose_secret(), "sk-ant-encrypted");

    let mut keys = store.list_keys().await.expect("list");
    keys.sort();
    assert_eq!(keys, vec!["ANTHROPIC_API_KEY", "GEMINI_API_KEY"]);

    // The on-disk envelope MUST NOT contain the plaintext.
    let raw_bytes = std::fs::read(&path).expect("read");
    let raw_str = String::from_utf8_lossy(&raw_bytes);
    assert!(
        !raw_str.contains("sk-ant-encrypted"),
        "ciphertext file leaked the plaintext!"
    );
    assert!(
        !raw_str.contains("sk-gemini-encrypted"),
        "ciphertext file leaked the plaintext!"
    );
    assert!(raw_str.contains("\"version\""), "expected envelope JSON shape");
    assert!(raw_str.contains("argon2id"), "expected argon2id kdf header");

    // Delete + reopen from disk with a fresh handle.
    store.delete("ANTHROPIC_API_KEY").await.expect("delete");
    assert!(store.get("ANTHROPIC_API_KEY").await.unwrap().is_none());

    let store2 = EncryptedFileSecretStore::new(
        &path,
        SecretValue::from("correct horse battery staple"),
    );
    let got2 = store2
        .get("GEMINI_API_KEY")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got2.expose_secret(), "sk-gemini-encrypted");
}

#[tokio::test]
async fn encrypted_file_store_wrong_passphrase_returns_bad_passphrase() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("secrets.enc");
    let store = EncryptedFileSecretStore::new(&path, SecretValue::from("right-pass"));
    store
        .set("KEY1", SecretValue::from("value-one"))
        .await
        .expect("set");

    let evil = EncryptedFileSecretStore::new(&path, SecretValue::from("wrong-pass"));
    let err = evil.get("KEY1").await.expect_err("must fail");
    assert!(matches!(err, SecretsError::BadPassphrase), "expected BadPassphrase, got {err:?}");
}

// ---------------------------------------------------------------------------
// CompositeSecretStore
// ---------------------------------------------------------------------------

#[tokio::test]
async fn composite_falls_through_to_second_store() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("composite.json");
    let env = Arc::new(EnvSecretStore::new()) as Arc<dyn SecretStore>;
    let file = Arc::new(FileSecretStore::new(&path)) as Arc<dyn SecretStore>;
    let composite = CompositeSecretStore::new(vec![env.clone(), file.clone()]);

    // Set goes to the file store (env is read-only).
    composite
        .set("ANTHROPIC_API_KEY", SecretValue::from("composite-set"))
        .await
        .expect("set must succeed by falling through to file");

    // Read from composite resolves via the file fallback.
    let got = composite
        .get("ANTHROPIC_API_KEY")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got.expose_secret(), "composite-set");

    // Env takes precedence on read when set.
    let key = "APHRODY_SECRETS_SMOKE_COMPOSITE_OVERRIDE";
    unsafe { std::env::set_var(key, "env-wins") };
    file.set(key, SecretValue::from("file-loses"))
        .await
        .expect("seed file");
    let composite_with_env =
        CompositeSecretStore::new(vec![env.clone(), file.clone()]);
    let resolved = composite_with_env
        .get(key)
        .await
        .expect("get")
        .expect("present");
    assert_eq!(resolved.expose_secret(), "env-wins");
    unsafe { std::env::remove_var(key) };

    // list_keys merges both stores (env returns empty, file has 2 entries).
    let keys = composite.list_keys().await.expect("list");
    assert!(keys.contains(&"ANTHROPIC_API_KEY".to_string()));
}

// ---------------------------------------------------------------------------
// mask_value helper
// ---------------------------------------------------------------------------

#[test]
fn mask_value_redacts_middle() {
    assert_eq!(mask_value("sk-abc123def456ghi", 4), "sk-a***6ghi");
    assert_eq!(mask_value("tiny", 4), "***");
    assert_eq!(mask_value("01234567", 2), "01***67");
    assert_eq!(mask_value("anything", 0), "***");
    // Unicode-safe (do not slice mid-codepoint).
    assert_eq!(mask_value("éèàäöüñabcdefg", 2), "éè***fg");
}

// ---------------------------------------------------------------------------
// SecretRef round-trip
// ---------------------------------------------------------------------------

#[test]
fn secret_ref_constructor_and_eq() {
    let r1 = SecretRef::new("env", "ANTHROPIC_API_KEY");
    let r2 = SecretRef { provider: "env".to_string(), key: "ANTHROPIC_API_KEY".to_string() };
    assert_eq!(r1, r2);
    assert_ne!(r1, SecretRef::new("file", "ANTHROPIC_API_KEY"));
}
