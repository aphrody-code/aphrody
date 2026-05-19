// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for the `aphrody-mcp client …` subcommand.
//!
//! These tests exercise the same code path the CLI dispatches into:
//!   1. Loader resolves a server entry from an in-tree `mcp.json` fixture.
//!   2. Stdio connect handles a missing command gracefully (no panic, typed error).
//!
//! Both rely on the `gemini-runtime` crate that `client_cmd.rs` itself uses,
//! ensuring the contract stays in sync.

use std::collections::HashMap;

use gemini_runtime::{McpClient, McpError, McpTransportConfig, load_mcp_config, parse_mcp_config};

/// The loader must accept the Claude Desktop / `.mcp.json` shape and surface
/// the requested entry by name. This is the exact path `aphrody-mcp client
/// list <server-name>` walks before opening a transport.
#[test]
fn client_loader_resolves_server_from_mcp_json() {
    let fixture = r#"{
        "mcpServers": {
            "google_mcp": { "command": "aphrody-mcp", "args": [] },
            "bxc": {
                "type": "stdio",
                "command": "bxc",
                "args": ["serve", "--stdio"],
                "env": { "BXC_LOG": "info" }
            },
            "supabase": {
                "type": "http",
                "url": "https://mcp.example.org/v1/rpc",
                "headers": { "Authorization": "Bearer demo-token" }
            }
        }
    }"#;
    let cfg = parse_mcp_config(fixture).expect("fixture must parse");
    assert_eq!(cfg.servers.len(), 3, "three servers expected");

    // BTreeMap-driven alphabetical ordering.
    let bxc = cfg.servers.iter().find(|s| s.name == "bxc").expect("bxc present");
    match &bxc.transport {
        McpTransportConfig::Stdio { command, args, env } => {
            assert_eq!(command, "bxc");
            assert_eq!(args, &vec!["serve".to_owned(), "--stdio".to_owned()]);
            assert_eq!(env.get("BXC_LOG").map(String::as_str), Some("info"));
        },
        other => panic!("expected stdio for bxc, got {other:?}"),
    }

    let supa = cfg.servers.iter().find(|s| s.name == "supabase").expect("supabase present");
    match &supa.transport {
        McpTransportConfig::Http { url, headers } => {
            assert_eq!(url, "https://mcp.example.org/v1/rpc");
            assert!(headers.contains_key("Authorization"));
        },
        other => panic!("expected http for supabase, got {other:?}"),
    }

    // Round-trip via the filesystem loader: same fixture, same shape.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("mcp.json");
    std::fs::write(&path, fixture).expect("write fixture");
    let cfg2 = load_mcp_config(&path).expect("load ok");
    assert_eq!(cfg2.servers.len(), 3);
    assert!(cfg2.servers.iter().any(|s| s.name == "google_mcp"));
}

/// Connecting to a non-existent stdio command must surface a typed
/// `McpError::Spawn` instead of panicking. This guarantees `aphrody-mcp
/// client probe --stdio <missing>` returns a clean exit-non-zero rather than
/// crashing the process.
#[tokio::test]
async fn client_probe_handles_missing_command_gracefully() {
    let env: HashMap<String, String> = HashMap::new();
    let bogus = "definitely-not-a-real-binary-xyz-aphrody-smoke";
    let result = McpClient::from_stdio(bogus, &[], &env).await;
    match result {
        Err(McpError::Spawn { command, .. }) => {
            assert_eq!(command, bogus, "command name must be echoed in error");
        },
        Err(other) => panic!("expected Spawn error, got {other:?}"),
        Ok(_) => panic!("phantom binary somehow spawned"),
    }
}

/// The loader must default to an empty config (not an error) when the target
/// file does not exist. Mirrors the behaviour of `aphrody-terminal-config`
/// and what `aphrody-mcp client list` relies on to print a useful diagnostic
/// (rather than crashing on a fresh install).
#[test]
fn client_loader_returns_empty_on_missing_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nope.json");
    let cfg = load_mcp_config(&missing).expect("missing file → empty config");
    assert!(cfg.servers.is_empty());
}
