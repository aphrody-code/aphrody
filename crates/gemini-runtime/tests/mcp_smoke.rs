// SPDX-License-Identifier: Apache-2.0
//! Integration smoke tests for the MCP client / config / tools surfaces.
//!
//! These tests don't require any network access by default — the live web
//! search test is gated behind `GEMINI_API_KEY`, so CI / sandbox runs will
//! skip it automatically.

use std::sync::Arc;

use gemini_runtime::{
    McpConfig, McpTransportConfig, Tool, ToolError, ToolRegistry, WebSearchConfig, WebSearchTool,
    parse_mcp_config,
};

/// Verify [`parse_mcp_config`] handles the Claude Desktop format end to end —
/// including stdio entries with no `type` field and HTTP entries with custom
/// headers.  Exercises every shape the CLI subcommands rely on.
#[test]
fn mcp_loader_parses_claude_format() {
    // Fixtures inline — mirrors the format actually shipped by Claude Desktop
    // (`~/Library/Application Support/Claude/claude_desktop_config.json`) and
    // by the repo's own `~/.config/aphrody/mcp.json`.
    let raw = r#"{
        "mcpServers": {
            "google_mcp": {
                "command": "google_mcp",
                "args": [],
                "env": { "RUST_LOG": "info" }
            },
            "bxc": {
                "type": "stdio",
                "command": "bxc",
                "args": ["serve", "--stdio"]
            },
            "supabase": {
                "type": "http",
                "url": "https://mcp.supabase.com/v1/rpc",
                "headers": {
                    "Authorization": "Bearer ${SUPABASE_TOKEN}",
                    "X-Org": "aphrody-code"
                }
            },
            "vault": {
                "type": "streamable-http",
                "url": "https://vault.example.com/mcp"
            }
        }
    }"#;

    let cfg: McpConfig = parse_mcp_config(raw).expect("config must parse");
    assert_eq!(cfg.servers.len(), 4, "all four servers must be parsed");

    // Servers are alphabetically ordered (BTreeMap iteration).
    let names: Vec<&str> = cfg.servers.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["bxc", "google_mcp", "supabase", "vault"]);

    // bxc → explicit stdio entry.
    match &cfg.servers[0].transport {
        McpTransportConfig::Stdio { command, args, env } => {
            assert_eq!(command, "bxc");
            assert_eq!(args, &vec!["serve".to_owned(), "--stdio".to_owned()]);
            assert!(env.is_empty(), "bxc entry sets no env");
        },
        other => panic!("bxc must be stdio, got {other:?}"),
    }

    // google_mcp → implicit stdio (no `type` field).
    match &cfg.servers[1].transport {
        McpTransportConfig::Stdio { command, env, .. } => {
            assert_eq!(command, "google_mcp");
            assert_eq!(env.get("RUST_LOG").map(String::as_str), Some("info"));
        },
        other => panic!("google_mcp must default to stdio, got {other:?}"),
    }

    // supabase → http with two headers.
    match &cfg.servers[2].transport {
        McpTransportConfig::Http { url, headers } => {
            assert_eq!(url, "https://mcp.supabase.com/v1/rpc");
            assert_eq!(headers.len(), 2);
            assert_eq!(
                headers.get("Authorization").map(String::as_str),
                Some("Bearer ${SUPABASE_TOKEN}")
            );
            assert_eq!(headers.get("X-Org").map(String::as_str), Some("aphrody-code"));
        },
        other => panic!("supabase must be http, got {other:?}"),
    }

    // vault → streamable-http variant preserved (callers may subscribe to SSE).
    match &cfg.servers[3].transport {
        McpTransportConfig::StreamableHttp { url, .. } => {
            assert_eq!(url, "https://vault.example.com/mcp");
        },
        other => panic!("vault must be streamable-http, got {other:?}"),
    }
}

/// Verify the [`Tool`] trait dispatch works against the built-in
/// [`WebSearchTool`].  We do *not* hit the network here — the test is
/// considered passed when:
///
///   * The tool advertises the documented descriptor shape.
///   * Invoking it with a missing `query` field returns
///     [`ToolError::BadArgs`].
///   * If `GEMINI_API_KEY` is set, we additionally fire a real `web_search`
///     call and assert the result deserialises to a non-empty content string.
#[tokio::test]
async fn tool_trait_web_search_invokes() {
    // Required to construct a reqwest::Client in this workspace — the global
    // CryptoProvider must be installed before any TLS handshake.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let tool = WebSearchTool::new(
        reqwest::Client::new(),
        WebSearchConfig::with_api_key(
            std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| "AIza-fake".to_owned()),
        ),
    );

    // ── 1. Descriptor surface
    let descriptor = tool.descriptor();
    assert_eq!(descriptor.name, "web_search");
    assert_eq!(descriptor.input_schema["type"], "object");
    assert_eq!(descriptor.input_schema["properties"]["query"]["type"], "string");
    assert_eq!(descriptor.input_schema["required"][0], "query");

    // ── 2. Bad args (no query)
    let err = tool.invoke(serde_json::json!({})).await.expect_err("must reject missing query");
    match err {
        ToolError::BadArgs { name, reason } => {
            assert_eq!(name, "web_search");
            assert!(reason.contains("query"), "diagnostic mentions field name");
        },
        other => panic!("expected BadArgs, got {other:?}"),
    }

    // ── 3. Optional live call (skipped if no API key)
    if std::env::var("GEMINI_API_KEY").is_ok() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(WebSearchTool::new(
            reqwest::Client::new(),
            WebSearchConfig::with_api_key(std::env::var("GEMINI_API_KEY").unwrap()),
        )));
        let res = reg
            .invoke("web_search", serde_json::json!({"query": "rust async traits 2026"}))
            .await
            .expect("live call must succeed");
        let content = res
            .get("content")
            .and_then(serde_json::Value::as_str)
            .expect("result has content string");
        assert!(!content.is_empty(), "non-empty live response");
    }
}

/// Verify [`ToolRegistry`] is a uniform dispatcher and surfaces NotFound for
/// unregistered tools (mirrors the contract used by `aphrody mcp call`).
#[tokio::test]
async fn registry_dispatch_surfaces_not_found() {
    let reg = ToolRegistry::new();
    let err = reg.invoke("does-not-exist", serde_json::Value::Null).await.unwrap_err();
    match err {
        ToolError::NotFound(name) => assert_eq!(name, "does-not-exist"),
        other => panic!("expected NotFound, got {other:?}"),
    }
}
