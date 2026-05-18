// SPDX-License-Identifier: Apache-2.0
//! Integration tests for `aphrody-terminal-config`.

use std::{collections::BTreeMap, fs};

use aphrody_terminal_config::{
    ClaudeShim, ConfigError, LlmPaneConfig, McpShim, PermissionsConfig, SettingsShim, Shim,
    TerminalConfig, ThemeConfig, VoiceConfig, import_claude_json, import_mcp_json,
    import_settings_json, load_from_path, merge, schema_json, validate,
};
use tempfile::TempDir;

// ── schema_validate_* ─────────────────────────────────────────────────────────

#[test]
fn schema_validate_minimal_valid() {
    let result = validate(r#"{"llm":{"sub_agent":true}}"#);
    assert!(result.is_ok(), "expected ok, got {result:?}");
}

#[test]
fn schema_validate_unknown_field_rejected() {
    let result = validate(r#"{"unknownKey":42}"#);
    match result {
        Err(ConfigError::Validation(msg)) => {
            assert!(
                msg.to_lowercase().contains("unknown"),
                "expected unknown-field error, got: {msg}"
            );
        },
        other => panic!("expected ValidationError, got {other:?}"),
    }
}

#[test]
fn schema_validate_wrong_type() {
    let result = validate(r#"{"llm":{"sub_agent":"yes"}}"#);
    assert!(matches!(result, Err(ConfigError::Validation(_))));
}

#[test]
fn schema_json_well_formed() {
    let schema = schema_json();
    let parsed: serde_json::Value =
        serde_json::from_str(&schema).expect("schema_json must produce valid JSON");
    assert!(parsed.get("$schema").is_some(), "schema must contain $schema");
    assert!(parsed.get("properties").is_some(), "schema must contain properties");
}

// ── import_*_json ─────────────────────────────────────────────────────────────

#[test]
fn import_claude_json_minimal() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("claude.json");
    fs::write(&path, r#"{"theme":"dark"}"#).unwrap();
    let shim: ClaudeShim = import_claude_json(Some(&path)).unwrap();
    assert_eq!(shim.theme.as_deref(), Some("dark"));
}

#[test]
fn import_mcp_json_servers() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".mcp.json");
    fs::write(&path, r#"{"mcpServers":{"foo":{"type":"stdio","command":"echo","args":["hi"]}}}"#)
        .unwrap();
    let shim: McpShim = import_mcp_json(Some(&path)).unwrap();
    assert_eq!(shim.servers.len(), 1);
    let foo = shim.servers.get("foo").expect("foo present");
    assert_eq!(foo.transport_type, "stdio");
    assert_eq!(foo.command.as_deref(), Some("echo"));
    assert_eq!(foo.args, vec!["hi".to_owned()]);
}

#[test]
fn import_settings_json_permissions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"permissions":{"allow":["Bash(ls)"]}}"#).unwrap();
    let shim: SettingsShim = import_settings_json(Some(&path)).unwrap();
    assert_eq!(shim.permissions.allow, vec!["Bash(ls)".to_owned()]);
}

// ── merge_* ───────────────────────────────────────────────────────────────────

#[test]
fn merge_precedence_terminal_wins() {
    let base = TerminalConfig {
        llm: LlmPaneConfig::default(),
        voice: VoiceConfig::default(),
        theme: ThemeConfig { name: "light".to_owned() },
        permissions: PermissionsConfig::default(),
        keymap: BTreeMap::new(),
        mcp_servers: BTreeMap::new(),
    };
    let claude = ClaudeShim { theme: Some("dark".to_owned()), keymap: Default::default() };
    let merged = merge(base, vec![Shim::Claude(claude)]);
    assert_eq!(merged.theme.name, "light", "terminal.json must win");
}

#[test]
fn merge_keymap_union() {
    let mut base_keymap = BTreeMap::new();
    base_keymap.insert("a".to_owned(), "1".to_owned());
    let base = TerminalConfig { keymap: base_keymap, ..TerminalConfig::default() };

    let mut shim_keymap = std::collections::HashMap::new();
    shim_keymap.insert("b".to_owned(), "2".to_owned());
    // Conflict: shim tries to override "a" — base must win.
    shim_keymap.insert("a".to_owned(), "shim_a".to_owned());
    let claude = ClaudeShim { theme: None, keymap: shim_keymap };

    let merged = merge(base, vec![Shim::Claude(claude)]);
    assert_eq!(merged.keymap.get("a"), Some(&"1".to_owned()), "terminal wins on conflict");
    assert_eq!(merged.keymap.get("b"), Some(&"2".to_owned()), "shim fills missing key");
    assert_eq!(merged.keymap.len(), 2);
}

// ── load_from_path ────────────────────────────────────────────────────────────

#[test]
fn load_from_path_missing_falls_back_default() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("does_not_exist.json");
    let cfg = load_from_path(&path).expect("missing file must return default");
    assert_eq!(cfg, TerminalConfig::default());
}

// ── extra: import shims gracefully handle missing files ───────────────────────

#[test]
fn import_shims_missing_file_returns_default() {
    let dir = TempDir::new().unwrap();
    let absent = dir.path().join("nope.json");
    assert!(import_claude_json(Some(&absent)).unwrap().theme.is_none());
    assert!(import_mcp_json(Some(&absent)).unwrap().servers.is_empty());
    assert!(import_settings_json(Some(&absent)).unwrap().permissions.allow.is_empty());
}
