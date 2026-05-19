// SPDX-License-Identifier: Apache-2.0
//! `aphrody-mcp client …` subcommand surface.
//!
//! Adds an MCP **client** capability to the `aphrody-mcp` binary, complementing
//! its existing **server** role (the 15 stdio tools exposed via `rmcp`). The
//! same process can now both expose tools and consume tools from arbitrary
//! third-party MCP servers (stdio or HTTP) — including itself, for end-to-end
//! validation.
//!
//! # Subcommands
//!
//! - `aphrody-mcp client list <server-name>` — load the configured `mcp.json`, connect to
//!   `<server-name>`, run the handshake + `tools/list`, print newline-delimited JSON (one object
//!   per tool).
//! - `aphrody-mcp client call <server-name> <tool-name> --args '<json>'` — connect, run
//!   `tools/call`, print the JSON-RPC `result` payload.
//! - `aphrody-mcp client probe --stdio "<cmd args>"` — one-shot connect an arbitrary stdio MCP
//!   server (without touching `mcp.json`) and print a single JSON object: `{ "server": ServerInfo,
//!   "tools": [McpTool, …] }`.
//!
//! # Wire reuse
//!
//! All wire-level work (handshake, JSON-RPC framing, transport selection) is
//! delegated to [`gemini_runtime::McpClient`]. This module is purely a thin
//! CLI surface on top of that client — no protocol logic lives here.
//!
//! # Cross-platform stance
//!
//! Stdio MCP servers require process spawning (`tokio::process`) which is
//! unavailable on `wasm32`. On wasm32 the `list` / `probe` variants degrade to
//! an explicit `Unsupported` error; `call` over HTTP keeps working because
//! `reqwest` supports the wasm-fetch target.

use std::collections::HashMap;

use anyhow::{Context as _, Result, anyhow, bail};
use clap::{Args, Subcommand};
use gemini_runtime::{
    McpClient, McpConfig, McpServerConfig, McpTransportConfig, load_mcp_config,
    mcp_default_config_path,
};
use serde_json::{Value as JsonValue, json};

// ---------------------------------------------------------------------------
// clap-facing types
// ---------------------------------------------------------------------------

/// `aphrody-mcp client <subcmd>` — connect to third-party MCP servers.
#[derive(Debug, Args)]
pub(crate) struct ClientArgs {
    /// Operation to perform against the target server.
    #[command(subcommand)]
    pub(crate) op: ClientOp,
}

/// Operations exposed by the `client` subcommand.
#[derive(Debug, Subcommand)]
pub(crate) enum ClientOp {
    /// List the tools advertised by the named MCP server (resolved via
    /// `mcp.json`). Each tool is printed as one NDJSON object.
    List {
        /// Logical server name (the key in `mcpServers` of `mcp.json`).
        server_name: String,
        /// Override the `mcp.json` path. Defaults to
        /// `$APHRODY_MCP_CONFIG` → `<config>/aphrody/mcp.json` →
        /// `<home>/.config/aphrody/mcp.json` → `<repo>/.mcp.json`.
        #[arg(long)]
        config: Option<std::path::PathBuf>,
    },
    /// Invoke a tool on the named MCP server and print its `result` payload.
    Call {
        /// Logical server name (the key in `mcpServers` of `mcp.json`).
        server_name: String,
        /// Tool name as advertised by the server's `tools/list`.
        tool_name: String,
        /// JSON object of arguments to pass to the tool. Defaults to `{}`.
        #[arg(long, default_value = "{}")]
        args: String,
        /// Override the `mcp.json` path (see `list --config`).
        #[arg(long)]
        config: Option<std::path::PathBuf>,
    },
    /// One-shot probe of an arbitrary stdio MCP server, bypassing `mcp.json`.
    /// Prints a single JSON object describing the server + its tool list.
    Probe {
        /// Full command line for the stdio MCP server to probe, e.g.
        /// `"/path/to/aphrody-mcp"` or `"bxc serve --stdio"`. The first token
        /// is the executable; remaining tokens are passed as args.
        #[arg(long)]
        stdio: String,
    },
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Entry point for `aphrody-mcp client <subcmd>`. Returns process exit code 0
/// on success or surfaces an `anyhow::Error` (mapped by `main` to a non-zero
/// exit and a structured error JSON line on stderr).
pub(crate) async fn run(args: ClientArgs) -> Result<()> {
    match args.op {
        ClientOp::List { server_name, config } => run_list(&server_name, config.as_deref()).await,
        ClientOp::Call { server_name, tool_name, args, config } => {
            run_call(&server_name, &tool_name, &args, config.as_deref()).await
        },
        ClientOp::Probe { stdio } => run_probe(&stdio).await,
    }
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

async fn run_list(server_name: &str, override_path: Option<&std::path::Path>) -> Result<()> {
    let cfg = load_config(override_path)?;
    let entry = find_server(&cfg, server_name)?;
    let mut client = connect_from_config(entry).await?;
    let info = client
        .initialize()
        .await
        .with_context(|| format!("MCP initialize handshake against `{server_name}`"))?;
    let tools = client
        .list_tools()
        .await
        .with_context(|| format!("MCP tools/list against `{server_name}`"))?;

    // Header line: server info as NDJSON.
    let header = json!({
        "kind": "server_info",
        "server_name": server_name,
        "name": info.name,
        "version": info.version,
        "protocol_version": info.protocol_version,
        "tool_count": tools.len(),
    });
    println!("{}", serde_json::to_string(&header)?);

    // One NDJSON line per tool.
    for tool in tools {
        let line = json!({
            "kind": "tool",
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.input_schema,
        });
        println!("{}", serde_json::to_string(&line)?);
    }

    #[cfg(not(target_arch = "wasm32"))]
    client.shutdown().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// call
// ---------------------------------------------------------------------------

async fn run_call(
    server_name: &str,
    tool_name: &str,
    args_json: &str,
    override_path: Option<&std::path::Path>,
) -> Result<()> {
    let parsed_args: JsonValue = serde_json::from_str(args_json)
        .with_context(|| format!("--args must be a JSON value; got `{args_json}`"))?;
    if !parsed_args.is_object() {
        bail!("--args must be a JSON object (got: {})", parsed_args);
    }

    let cfg = load_config(override_path)?;
    let entry = find_server(&cfg, server_name)?;
    let mut client = connect_from_config(entry).await?;
    client
        .initialize()
        .await
        .with_context(|| format!("MCP initialize handshake against `{server_name}`"))?;

    let result = client
        .call_tool(tool_name, parsed_args)
        .await
        .with_context(|| format!("MCP tools/call `{tool_name}` on `{server_name}`"))?;

    let envelope = json!({
        "kind": "tool_call_result",
        "server_name": server_name,
        "tool_name": tool_name,
        "result": result,
    });
    println!("{}", serde_json::to_string(&envelope)?);

    #[cfg(not(target_arch = "wasm32"))]
    client.shutdown().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// probe
// ---------------------------------------------------------------------------

async fn run_probe(stdio_cmdline: &str) -> Result<()> {
    let (command, args) = split_stdio_command(stdio_cmdline)?;

    let env: HashMap<String, String> = HashMap::new();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let mut client = McpClient::from_stdio(&command, &arg_refs, &env)
        .await
        .with_context(|| format!("failed to spawn MCP server `{command}`"))?;
    let info = client
        .initialize()
        .await
        .with_context(|| format!("MCP initialize handshake against `{command}`"))?;
    let tools =
        client.list_tools().await.with_context(|| format!("MCP tools/list against `{command}`"))?;

    let tool_entries: Vec<JsonValue> = tools
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect();

    let envelope = json!({
        "kind": "probe",
        "command": command,
        "args": args,
        "server": {
            "name": info.name,
            "version": info.version,
            "protocol_version": info.protocol_version,
            "capabilities": info.capabilities,
        },
        "tool_count": tool_entries.len(),
        "tools": tool_entries,
    });
    println!("{}", serde_json::to_string(&envelope)?);

    #[cfg(not(target_arch = "wasm32"))]
    client.shutdown().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers (public so the integration test can exercise them)
// ---------------------------------------------------------------------------

/// Load the MCP config from `override_path` if provided, otherwise from the
/// platform-default path (`APHRODY_MCP_CONFIG` / XDG / APPDATA / `.mcp.json`).
pub(crate) fn load_config(override_path: Option<&std::path::Path>) -> Result<McpConfig> {
    let path = match override_path {
        Some(p) => p.to_path_buf(),
        None => mcp_default_config_path()
            .map_err(|e| anyhow!("cannot resolve default mcp.json path: {e}"))?,
    };
    load_mcp_config(&path).map_err(|e| anyhow!("loading mcp.json from {}: {e}", path.display()))
}

/// Locate `server_name` in `cfg.servers`, returning a descriptive error when
/// the entry is missing.
pub(crate) fn find_server<'a>(
    cfg: &'a McpConfig,
    server_name: &str,
) -> Result<&'a McpServerConfig> {
    cfg.servers.iter().find(|s| s.name == server_name).ok_or_else(|| {
        let known: Vec<&str> = cfg.servers.iter().map(|s| s.name.as_str()).collect();
        anyhow!(
            "no MCP server named `{server_name}` in config (known: {})",
            if known.is_empty() { "<none>".to_owned() } else { known.join(", ") }
        )
    })
}

/// Connect to a server described by an `McpServerConfig` entry. Picks the
/// transport according to the entry variant.
pub(crate) async fn connect_from_config(entry: &McpServerConfig) -> Result<McpClient> {
    match &entry.transport {
        McpTransportConfig::Stdio { command, args, env } => {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            McpClient::from_stdio(command, &arg_refs, env)
                .await
                .map_err(|e| anyhow!("stdio connect to `{command}`: {e}"))
        },
        McpTransportConfig::Http { url, headers }
        | McpTransportConfig::StreamableHttp { url, headers } => {
            let bearer = headers
                .get("Authorization")
                .and_then(|v| v.strip_prefix("Bearer ").map(str::to_owned));
            McpClient::from_http(url.clone(), bearer)
                .map_err(|e| anyhow!("http client for `{url}`: {e}"))
        },
    }
}

/// Split a stdio command line into (command, args). Whitespace separated; no
/// shell-style quoting is interpreted (callers needing quoting should pass
/// the command via a wrapper script). Returns an error for empty input.
pub(crate) fn split_stdio_command(cmdline: &str) -> Result<(String, Vec<String>)> {
    let mut iter = cmdline.split_whitespace().map(str::to_owned);
    let command = iter.next().ok_or_else(|| anyhow!("--stdio must not be empty"))?;
    let args: Vec<String> = iter.collect();
    Ok((command, args))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use gemini_runtime::parse_mcp_config;

    use super::*;

    #[test]
    fn split_stdio_command_parses_program_and_args() {
        let (cmd, args) = split_stdio_command("/usr/bin/aphrody-mcp serve --stdio").expect("ok");
        assert_eq!(cmd, "/usr/bin/aphrody-mcp");
        assert_eq!(args, vec!["serve".to_owned(), "--stdio".to_owned()]);
    }

    #[test]
    fn split_stdio_command_rejects_empty() {
        let err = split_stdio_command("   ").expect_err("must err");
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn find_server_returns_existing_entry() {
        let cfg = parse_mcp_config(
            r#"{ "mcpServers": { "alpha": { "command": "a" }, "beta": { "command": "b" } } }"#,
        )
        .expect("parse");
        let entry = find_server(&cfg, "beta").expect("found");
        assert_eq!(entry.name, "beta");
    }

    #[test]
    fn find_server_error_lists_known_names() {
        let cfg = parse_mcp_config(
            r#"{ "mcpServers": { "alpha": { "command": "a" }, "beta": { "command": "b" } } }"#,
        )
        .expect("parse");
        let err = find_server(&cfg, "missing").expect_err("must err");
        let msg = err.to_string();
        assert!(msg.contains("missing"));
        assert!(msg.contains("alpha"));
        assert!(msg.contains("beta"));
    }

    #[test]
    fn find_server_error_when_no_servers_configured() {
        let cfg = parse_mcp_config(r#"{}"#).expect("parse");
        let err = find_server(&cfg, "anything").expect_err("must err");
        assert!(err.to_string().contains("<none>"));
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn connect_to_missing_command_returns_spawn_error() {
        let entry = McpServerConfig {
            name: "phantom".to_owned(),
            transport: McpTransportConfig::Stdio {
                command: "this-binary-definitely-does-not-exist-zzz-aphrody".to_owned(),
                args: vec![],
                env: HashMap::new(),
            },
        };
        // `McpClient` does not implement `Debug`, so we cannot use `expect_err`
        // (which requires `T: Debug`). Match on the `Result` manually.
        match connect_from_config(&entry).await {
            Ok(_) => panic!("phantom binary somehow spawned"),
            Err(err) => {
                let msg = err.to_string();
                assert!(msg.contains("stdio connect to"), "unexpected error: {msg}");
            },
        }
    }
}
