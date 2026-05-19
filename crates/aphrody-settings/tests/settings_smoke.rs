// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke matrix for the hierarchical settings loader.
//!
//! Each test exercises one slice of the contract:
//!   * merge precedence (overlapping keys, deep recursion, array replacement)
//!   * env loader (nested underscore parsing)
//!   * file loader (3-file tempdir, graceful missing-file handling)
//!   * dotted-path navigation
//!   * layer replacement
//!   * minimum-viable JSON Schema validation (happy + sad path)

use std::io::Write;

use serde_json::json;
use tempfile::TempDir;

use aphrody_settings::{
    LayeredSettings, SettingsError, SettingsLayer, SettingsLoader, validate_against_schema,
};

// ---------------------------------------------------------------------------
// Merge contract
// ---------------------------------------------------------------------------

#[test]
fn merge_user_then_project_wins_overlapping_key() {
    let s = LayeredSettings::new()
        .with_user(json!({"llm": {"model": "claude"}}))
        .unwrap()
        .with_project(json!({"llm": {"model": "gemini"}}))
        .unwrap();
    assert_eq!(s.merged(), json!({"llm": {"model": "gemini"}}));
}

#[test]
fn merge_objects_deep_recursively() {
    let s = LayeredSettings::new()
        .with_user(json!({"a": {"b": {"c": 1, "d": 2}}}))
        .unwrap()
        .with_project(json!({"a": {"b": {"d": 99, "e": 3}}}))
        .unwrap();
    assert_eq!(s.merged(), json!({"a": {"b": {"c": 1, "d": 99, "e": 3}}}));
}

#[test]
fn merge_arrays_replace_not_concat() {
    let s = LayeredSettings::new()
        .with_user(json!({"servers": ["a", "b", "c"]}))
        .unwrap()
        .with_project(json!({"servers": ["x"]}))
        .unwrap();
    assert_eq!(s.merged(), json!({"servers": ["x"]}));
}

// ---------------------------------------------------------------------------
// Env loader
// ---------------------------------------------------------------------------

#[tokio::test]
async fn env_prefix_loads_underscore_nested() {
    // Set env vars on this process — clean up afterwards to avoid leaking
    // into sibling tests running on the same tokio executor.
    // SAFETY: tests modify process-global env; we restore unset on drop.
    let key = "APHRODY_TEST_FOO__BAR";
    let key_two = "APHRODY_TEST_FOO__BAZ";
    // SAFETY: write to process env; tests run single-threaded by default in
    // nextest, and we reset below. Other tests use distinct prefixes.
    unsafe {
        std::env::set_var(key, "hello");
        std::env::set_var(key_two, "42");
    }
    let loader = SettingsLoader::new().with_env_prefix("APHRODY_TEST_");
    let settings = loader.load().await.expect("env load");
    unsafe {
        std::env::remove_var(key);
        std::env::remove_var(key_two);
    }
    assert_eq!(
        settings.layer(SettingsLayer::Env).cloned(),
        Some(json!({"foo": {"bar": "hello", "baz": 42}})),
    );
    assert_eq!(
        settings.get("foo.bar"),
        Some(json!("hello")),
    );
}

// ---------------------------------------------------------------------------
// File loader
// ---------------------------------------------------------------------------

fn write_json(dir: &TempDir, name: &str, value: &serde_json::Value) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).expect("create");
    file.write_all(serde_json::to_string(value).unwrap().as_bytes())
        .expect("write");
    path
}

#[tokio::test]
async fn loader_reads_user_project_local_files() {
    let dir = TempDir::new().unwrap();
    let user_path = write_json(&dir, "user.json", &json!({"theme": "light", "depth": 1}));
    let project_path = write_json(&dir, "project.json", &json!({"theme": "dark"}));
    let local_path = write_json(&dir, "local.json", &json!({"depth": 9}));

    let loader = SettingsLoader::new()
        .with_user_path(user_path)
        .with_project_path(project_path)
        .with_local_path(local_path);
    let settings = loader.load().await.expect("load");
    let merged = settings.merged();
    // project beats user on `theme`; local beats both on `depth`.
    assert_eq!(merged, json!({"theme": "dark", "depth": 9}));
}

#[tokio::test]
async fn loader_missing_file_skipped_gracefully() {
    let dir = TempDir::new().unwrap();
    let project_path = write_json(&dir, "project.json", &json!({"present": true}));
    let absent_local = dir.path().join("nope.json");

    let loader = SettingsLoader::new()
        .with_project_path(project_path)
        .with_local_path(absent_local);
    let settings = loader.load().await.expect("missing local should be silent");
    assert_eq!(settings.merged(), json!({"present": true}));
}

#[tokio::test]
async fn loader_parse_failure_surfaces_path() {
    let dir = TempDir::new().unwrap();
    let bad_path = dir.path().join("bad.json");
    std::fs::write(&bad_path, b"{ not json").unwrap();

    let err = SettingsLoader::new()
        .with_project_path(bad_path.clone())
        .load()
        .await
        .expect_err("expected parse failure");
    let SettingsError::Json { path, .. } = err else {
        panic!("expected Json error, got {err:?}");
    };
    assert!(path.contains("bad.json"), "path should reference the offending file: {path}");
}

// ---------------------------------------------------------------------------
// Path navigation + layer mutation
// ---------------------------------------------------------------------------

#[test]
fn get_path_navigates_nested_keys() {
    let s = LayeredSettings::new()
        .with_project(json!({
            "llm": {"providers": ["anthropic", "gemini"], "model": "claude"}
        }))
        .unwrap();
    assert_eq!(s.get("llm.model"), Some(json!("claude")));
    assert_eq!(s.get("llm.providers.1"), Some(json!("gemini")));
    assert_eq!(s.get("nope.missing"), None);
}

#[test]
fn set_layer_replaces_completely() {
    let mut s = LayeredSettings::new();
    s.set_layer(SettingsLayer::User, json!({"a": 1, "b": 2})).unwrap();
    s.set_layer(SettingsLayer::User, json!({"only_this": true})).unwrap();
    assert_eq!(s.layer(SettingsLayer::User), Some(&json!({"only_this": true})));
}

#[test]
fn set_layer_rejects_non_object_root() {
    let mut s = LayeredSettings::new();
    let err = s.set_layer(SettingsLayer::User, json!([1, 2, 3])).unwrap_err();
    assert!(matches!(err, SettingsError::InvalidLayer(_)));
}

#[test]
fn clear_layer_drops_value() {
    let mut s = LayeredSettings::new()
        .with_user(json!({"a": 1}))
        .unwrap()
        .with_project(json!({"a": 2}))
        .unwrap();
    s.clear_layer(SettingsLayer::Project);
    assert_eq!(s.merged(), json!({"a": 1}));
}

// ---------------------------------------------------------------------------
// Schema validation
// ---------------------------------------------------------------------------

#[test]
fn validate_against_schema_passes_valid() {
    let schema = json!({
        "type": "object",
        "required": ["model"],
        "properties": {
            "model": {"type": "string"},
            "temperature": {"type": "number"}
        }
    });
    let value = json!({"model": "claude", "temperature": 0.7});
    validate_against_schema(&value, &schema).expect("should pass");
}

#[test]
fn validate_against_schema_rejects_missing_required() {
    let schema = json!({
        "type": "object",
        "required": ["model"],
        "properties": {"model": {"type": "string"}}
    });
    let value = json!({"temperature": 0.7});
    let err = validate_against_schema(&value, &schema).expect_err("should fail");
    let SettingsError::SchemaViolation { path, message } = err else {
        panic!("expected SchemaViolation, got {err:?}");
    };
    assert_eq!(path, "model");
    assert!(message.contains("required"), "{message}");
}

#[test]
fn validate_against_schema_rejects_wrong_type() {
    let schema = json!({"type": "object", "properties": {"port": {"type": "integer"}}});
    let value = json!({"port": "not-a-number"});
    let err = validate_against_schema(&value, &schema).expect_err("should fail");
    let SettingsError::SchemaViolation { path, .. } = err else {
        panic!("expected SchemaViolation, got {err:?}");
    };
    assert_eq!(path, "port");
}

#[test]
fn validate_against_schema_rejects_unknown_additional_property() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {"known": {"type": "string"}}
    });
    let value = json!({"known": "ok", "surprise": 1});
    let err = validate_against_schema(&value, &schema).expect_err("should fail");
    let SettingsError::SchemaViolation { path, .. } = err else {
        panic!("expected SchemaViolation, got {err:?}");
    };
    assert_eq!(path, "surprise");
}
