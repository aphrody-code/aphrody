// SPDX-License-Identifier: Apache-2.0
//! Brief-mandated smoke tests for the high-level loader surface.
//!
//! Five tests:
//!
//! 1. `schema_validate` — schemars schema generation + roundtrip.
//! 2. `import_claude_json` — flatten `.claude.json` `mcpServers`.
//! 3. `import_mcp_json` — lift `.mcp.json` `mcpServers`.
//! 4. `unknown_provider_rejected` — closed-enum enforcement.
//! 5. `default_profile_is_m3_dark` — baseline palette primary.

use std::fs;

use aphrody_terminal_config::{
    LlmConfig, LlmProvider, M3_BASELINE_DARK_PRIMARY, TerminalConfig, default_profile,
    import_claude_json_as_config, import_mcp_json_as_config, load_terminal_config, schema_json,
};
use tempfile::TempDir;

// 1. schema_validate ─────────────────────────────────────────────────────────

#[test]
fn schema_validate() {
    // Generate schema (proves schemars::JsonSchema derive compiles end-to-end).
    let schema = schema_json();
    let parsed: serde_json::Value =
        serde_json::from_str(&schema).expect("schema must be valid JSON");
    assert!(parsed.get("$schema").is_some(), "schema must carry $schema");
    assert!(parsed.get("properties").is_some(), "schema must list properties");
    assert!(parsed["properties"].get("profiles").is_some(), "profiles field exposed in schema");
    assert!(parsed["properties"].get("default_profile").is_some(), "default_profile exposed");

    // Hand-written sample TerminalConfig must roundtrip without losing data.
    let sample = TerminalConfig {
        profiles: vec![default_profile()],
        default_profile: "default".to_owned(),
        llm_config: Some(LlmConfig {
            provider: LlmProvider::Anthropic,
            model: "claude-opus-4-7".to_owned(),
            api_key_env: "ANTHROPIC_API_KEY".to_owned(),
            default_temperature: 0.7,
        }),
        ..TerminalConfig::default()
    };
    let serialised = serde_json::to_string(&sample).expect("serialise");
    let back: TerminalConfig = serde_json::from_str(&serialised).expect("roundtrip");
    assert_eq!(sample, back);
}

// 2. import_claude_json ──────────────────────────────────────────────────────

#[test]
fn import_claude_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("claude.json");
    fs::write(&path, r#"{"mcpServers":{"foo":{"command":"echo","args":["hi"]}}}"#).unwrap();
    let cfg = import_claude_json_as_config(&path).expect("import");
    assert_eq!(cfg.mcp_servers.len(), 1, "exactly one server lifted");
    let foo = cfg.mcp_servers.get("foo").expect("foo present");
    assert_eq!(foo.command.as_deref(), Some("echo"));
    assert_eq!(foo.args, vec!["hi".to_owned()]);
}

// 3. import_mcp_json ─────────────────────────────────────────────────────────

#[test]
fn import_mcp_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".mcp.json");
    fs::write(&path, r#"{"mcpServers":{"bar":{"type":"stdio","command":"x"}}}"#).unwrap();
    let cfg = import_mcp_json_as_config(&path).expect("import");
    assert_eq!(cfg.mcp_servers.len(), 1);
    let bar = cfg.mcp_servers.get("bar").expect("bar present");
    assert_eq!(bar.transport_type, "stdio");
    assert_eq!(bar.command.as_deref(), Some("x"));
}

// 4. unknown_provider_rejected ───────────────────────────────────────────────

#[test]
fn unknown_provider_rejected() {
    // OpenAI is intentionally absent from the closed `LlmProvider` enum.
    let raw = r#"{
        "llm_config": {
            "provider": "openai",
            "model": "gpt-4",
            "api_key_env": "OPENAI_API_KEY",
            "default_temperature": 0.7
        }
    }"#;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("terminal.json");
    fs::write(&path, raw).unwrap();
    let result = load_terminal_config(&path);
    assert!(result.is_err(), "openai must be rejected, got: {result:?}");
}

// 5. default_profile_is_m3_dark ──────────────────────────────────────────────

#[test]
fn default_profile_is_m3_dark() {
    let p = default_profile();
    assert_eq!(p.name, "default");
    assert_eq!(p.font_family, "Cascadia Code");
    assert_eq!(p.font_size, 14);
    // M3 baseline-dark primary (mirror of m3_tokens::color::BASELINE_DARK.primary).
    assert_eq!(
        p.palette.primary, M3_BASELINE_DARK_PRIMARY,
        "default palette must be M3 baseline-dark (#D0BCFF)",
    );
    assert_eq!(p.palette.primary, 0xFFD0_BCFF);
}
