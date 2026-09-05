// SPDX-License-Identifier: Apache-2.0
//! `aphrody mcp …` subcommands — list configured MCP servers / call tools.
//!
//! Backed by `gemini-runtime`'s [`gemini_runtime::McpClient`] and
//! [`gemini_runtime::mcp_config`]. Output is NDJSON on stdout so the command
//! can be piped into `jq` or other downstream tools.

use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;
use gemini_runtime::{
    McpClient, McpConfig, McpServerConfig, McpTransportConfig, load_default_mcp_config,
    load_mcp_config, mcp_default_config_path,
};
use serde_json::json;

use crate::context::{GoogleContext, TerminalCommand};

// ---------------------------------------------------------------------------
// `aphrody mcp list`
// ---------------------------------------------------------------------------

/// List every MCP server declared in the config and emit one NDJSON line per
/// server (with its tool inventory).
///
/// Behaviour:
///
/// - Reads `--config <PATH>` if supplied, otherwise calls [`mcp_default_config_path`] (env / XDG /
///   `%APPDATA%` / `./.mcp.json`).
/// - For each entry: spawns / connects, runs `initialize` + `tools/list`, then emits one NDJSON
///   line of shape `{ "server": "<name>", "transport": "stdio|http|streamable-http", "tools": [...]
///   }`.
/// - Servers that fail to start are still emitted with `{ "error": "..." }` so the caller can
///   detect partial outages.
pub(crate) struct McpListCommand {
    /// Optional path override for the config file.
    pub config: Option<PathBuf>,
    /// When true, skip the live `initialize` + `tools/list` round-trip and
    /// just dump the static config. Useful when offline or for diagnostics.
    pub no_probe: bool,
}

#[async_trait]
impl TerminalCommand for McpListCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let cfg = resolve_config(self.config.as_deref())?;
        if cfg.servers.is_empty() {
            // Emit a single diagnostic NDJSON line so downstream tools can
            // tell "no servers" apart from "command crashed".
            let line = json!({
                "warning": "no MCP servers configured",
                "hint": "create ~/.config/aphrody/mcp.json or set APHRODY_MCP_CONFIG"
            });
            println!("{line}");
            return Ok(());
        }
        for spec in &cfg.servers {
            let line =
                if self.no_probe { describe_static(spec) } else { describe_live(spec).await };
            println!("{line}");
        }
        Ok(())
    }
}

fn describe_static(spec: &McpServerConfig) -> serde_json::Value {
    json!({
        "server": spec.name,
        "transport": transport_tag(&spec.transport),
        "probe": false,
    })
}

async fn describe_live(spec: &McpServerConfig) -> serde_json::Value {
    let transport_tag = transport_tag(&spec.transport).to_owned();
    match connect_client(&spec.transport).await {
        Err(e) => json!({
            "server": spec.name,
            "transport": transport_tag,
            "error": format!("connect failed: {e}"),
        }),
        Ok(mut client) => match client.initialize().await {
            Err(e) => json!({
                "server": spec.name,
                "transport": transport_tag,
                "error": format!("initialize failed: {e}"),
            }),
            Ok(info) => match client.list_tools().await {
                Err(e) => json!({
                    "server": spec.name,
                    "transport": transport_tag,
                    "server_info": info,
                    "error": format!("tools/list failed: {e}"),
                }),
                Ok(tools) => {
                    let tool_descriptors: Vec<serde_json::Value> = tools
                        .into_iter()
                        .map(|t| {
                            json!({
                                "name": t.name,
                                "description": t.description,
                                "input_schema": t.input_schema,
                            })
                        })
                        .collect();
                    json!({
                        "server": spec.name,
                        "transport": transport_tag,
                        "server_info": info,
                        "tools": tool_descriptors,
                    })
                },
            },
        },
    }
}

// ---------------------------------------------------------------------------
// `aphrody mcp call`
// ---------------------------------------------------------------------------

/// Invoke a single tool on a configured MCP server.
///
/// Resolves `--config` → finds `<server>` entry → connects → runs
/// `initialize` + `tools/call` with `--args '<json>'` → emits the raw
/// JSON-RPC result as a single NDJSON line on stdout.
pub(crate) struct McpCallCommand {
    /// Optional path override for the config file.
    pub config: Option<PathBuf>,
    /// Server logical name (as it appears in `mcpServers` keys).
    pub server: String,
    /// Tool name to invoke.
    pub tool: String,
    /// JSON-encoded arguments object (must be a JSON object).
    pub args: String,
}

#[async_trait]
impl TerminalCommand for McpCallCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let cfg = resolve_config(self.config.as_deref())?;
        let spec = cfg
            .servers
            .iter()
            .find(|s| s.name == self.server)
            .ok_or_else(|| miette::miette!("MCP server `{}` not in config", self.server))?;

        let parsed_args: serde_json::Value = serde_json::from_str(&self.args)
            .map_err(|e| miette::miette!("--args must be valid JSON: {e}"))?;
        if !parsed_args.is_object() {
            return Err(miette::miette!(
                "--args must be a JSON object (got {})",
                short_type(&parsed_args)
            ));
        }

        let mut client = connect_client(&spec.transport)
            .await
            .map_err(|e| miette::miette!("connect to `{}` failed: {e}", self.server))?;
        client
            .initialize()
            .await
            .map_err(|e| miette::miette!("initialize `{}` failed: {e}", self.server))?;
        let result = client.call_tool(&self.tool, parsed_args).await.map_err(|e| {
            miette::miette!("tools/call `{}` on `{}` failed: {e}", self.tool, self.server)
        })?;

        let envelope = json!({
            "server": self.server,
            "tool": self.tool,
            "result": result,
        });
        println!("{envelope}");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_config(path: Option<&std::path::Path>) -> miette::Result<McpConfig> {
    match path {
        Some(p) => load_mcp_config(p).map_err(|e| miette::miette!("load {}: {e}", p.display())),
        None => {
            // Resolve default path first so we can include it in error
            // diagnostics (rather than the generic "no config found").
            let _resolved =
                mcp_default_config_path().map_err(|e| miette::miette!("resolve default: {e}"))?;
            load_default_mcp_config().map_err(|e| miette::miette!("load default MCP config: {e}"))
        },
    }
}

async fn connect_client(
    transport: &McpTransportConfig,
) -> Result<McpClient, gemini_runtime::McpError> {
    match transport {
        McpTransportConfig::Stdio { command, args, env } => {
            let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            McpClient::from_stdio(command, &args_refs, env).await
        },
        McpTransportConfig::Http { url, headers }
        | McpTransportConfig::StreamableHttp { url, headers } => {
            let bearer = extract_bearer(headers);
            McpClient::from_http(url.clone(), bearer)
        },
    }
}

fn extract_bearer(headers: &HashMap<String, String>) -> Option<String> {
    let raw = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
        .map(|(_, v)| v.clone())?;
    let token = raw.strip_prefix("Bearer ").unwrap_or(&raw).to_owned();
    // Expand `${ENV_VAR}` placeholder for parity with Claude Desktop.
    if let Some(inner) = token.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return std::env::var(inner).ok().filter(|v| !v.is_empty());
    }
    Some(token)
}

fn transport_tag(t: &McpTransportConfig) -> &'static str {
    match t {
        McpTransportConfig::Stdio { .. } => "stdio",
        McpTransportConfig::Http { .. } => "http",
        McpTransportConfig::StreamableHttp { .. } => "streamable-http",
    }
}

fn short_type(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bearer_passthrough() {
        let mut h = HashMap::new();
        h.insert("Authorization".to_owned(), "Bearer abc123".to_owned());
        assert_eq!(extract_bearer(&h).as_deref(), Some("abc123"));
    }

    #[test]
    fn extract_bearer_handles_env_placeholder() {
        let var = "APHRODY_TEST_MCP_BEARER_TOKEN";
        // SAFETY: nightly Rust requires unsafe for env mutation; the test is
        // isolated by using a uniquely-named variable.
        unsafe { std::env::set_var(var, "secret-token") };
        let mut h = HashMap::new();
        h.insert("Authorization".to_owned(), format!("Bearer ${{{var}}}"));
        assert_eq!(extract_bearer(&h).as_deref(), Some("secret-token"));
        unsafe { std::env::remove_var(var) };
    }

    #[test]
    fn extract_bearer_returns_none_on_missing_env() {
        let mut h = HashMap::new();
        h.insert("Authorization".to_owned(), "Bearer ${APHRODY_TEST_DEFINITELY_UNSET}".to_owned());
        assert!(extract_bearer(&h).is_none());
    }

    #[test]
    fn extract_bearer_case_insensitive_header() {
        let mut h = HashMap::new();
        h.insert("authorization".to_owned(), "Bearer xyz".to_owned());
        assert_eq!(extract_bearer(&h).as_deref(), Some("xyz"));
    }

    #[test]
    fn transport_tag_matches_variants() {
        let s =
            McpTransportConfig::Stdio { command: "x".into(), args: vec![], env: HashMap::new() };
        assert_eq!(transport_tag(&s), "stdio");
        let h = McpTransportConfig::Http { url: "https://x".into(), headers: HashMap::new() };
        assert_eq!(transport_tag(&h), "http");
        let sh =
            McpTransportConfig::StreamableHttp { url: "https://x".into(), headers: HashMap::new() };
        assert_eq!(transport_tag(&sh), "streamable-http");
    }
}
