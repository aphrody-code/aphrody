// SPDX-License-Identifier: Apache-2.0
//! MCP server status registry, active probing, and probe loop.
//!
//! # Overview
//!
//! This module extends the passive OSC-update registry with an **active probe**
//! path that performs real JSON-RPC `tools/list` calls against registered MCP
//! servers on a configurable interval, then publishes [`LlmEvent::Mcp`] events
//! on the bus whenever a server's state changes.
//!
//! ## Transport types
//!
//! - **Stdio** — spawn the configured command, write `initialize` + `tools/list`
//!   JSON-RPC requests to stdin, read the response from stdout.  Used for
//!   `google_mcp` (the Rust binary) and `bxc` (a Bun TypeScript server).
//! - **Http** — POST `tools/list` to the MCP SSE/HTTP endpoint, optionally
//!   with a pre-obtained Bearer token via the `aphrody-mcp` OAuth 2.1 flow.
//!
//! ## Default servers (feature `mcp-default-servers`)
//!
//! When compiled with `default-features` (i.e. the `mcp-default-servers`
//! feature), [`default_server_specs`] returns two pre-wired specs:
//!
//! 1. **google_mcp** — stdio, resolves the binary from
//!    `target/release/google_mcp[.exe]` relative to the workspace root, or
//!    falls back to `which google_mcp`.
//! 2. **bxc** — stdio via `bun run C:/worktree/bxc/…/server.ts`, matching the
//!    entry in the repo `.mcp.json`.
//!
//! ## Loading from `.mcp.json`
//!
//! [`load_mcp_json`] parses a `.mcp.json` / `mcp.json` file in the format used
//! by Claude Code (the `mcpServers` object with `type`, `command`, `args`,
//! `env`, `url`, and optional `headers` fields).

use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::timeout,
};

use crate::{EventBus, LlmEvent};

// ── OAuth config (available regardless of feature, so callers can build specs) ──

/// Minimal OAuth 2.1 configuration needed to obtain a Bearer token for an HTTP
/// MCP server.  Token acquisition itself is delegated to the `aphrody-mcp`
/// crate (via [`aphrody_mcp::begin_auth`]) but that flow requires a browser
/// redirect and is therefore not exercised during automated probing.
///
/// For probing purposes the token is obtained from the environment variable
/// named by `token_env_var`, if set.  If the variable is not present, the probe
/// attempts an unauthenticated `tools/list` call and records
/// `"oauth_required"` as the state on 401/403.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OAuthConfig {
    /// URL of the MCP server (used for RFC 9728 protected-resource discovery).
    pub server_url: String,
    /// OAuth redirect URI registered with the authorization server.
    pub redirect_uri: String,
    /// Optional override scope string (space-delimited).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Name of the environment variable that holds a pre-obtained Bearer token.
    /// When set and the variable is present, the probe uses it directly instead
    /// of triggering the full browser-redirect flow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_env_var: Option<String>,
}

// ── Transport ─────────────────────────────────────────────────────────────────

/// How to reach an MCP server.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpTransport {
    /// Launch a subprocess, exchange JSON-RPC over stdin/stdout.
    Stdio {
        /// Executable to run (resolved by `which` if not absolute).
        command: String,
        /// Arguments passed to the command.
        #[serde(default)]
        args: Vec<String>,
        /// Extra environment variables for the child process.
        #[serde(default)]
        env: HashMap<String, String>,
    },
    /// POST JSON-RPC to an HTTP/SSE MCP endpoint.
    Http {
        /// Full URL of the MCP server (e.g. `https://mcp.example.com/`).
        url: String,
        /// Optional OAuth 2.1 configuration.  When absent, requests are
        /// unauthenticated.
        #[serde(skip_serializing_if = "Option::is_none")]
        oauth_provider: Option<OAuthConfig>,
    },
}

// ── McpServerSpec ─────────────────────────────────────────────────────────────

/// Description of an MCP server that the probe loop should poll.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct McpServerSpec {
    /// Logical name (matches the key in `.mcp.json` `mcpServers`).
    pub name: String,
    /// How to reach the server.
    pub transport: McpTransport,
}

// ── McpServerStatus ───────────────────────────────────────────────────────────

/// Point-in-time status of one MCP server.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct McpServerStatus {
    /// Server name as declared in `mcp.json`.
    pub server: String,
    /// Current state string (e.g. `"connected"`, `"disconnected"`, `"error"`).
    pub state: String,
    /// Last RPC method name observed, if any.
    pub last_rpc: Option<String>,
}

// ── Inner registry state ───────────────────────────────────────────────────────

struct Inner {
    servers: HashMap<String, McpServerStatus>,
}

// ── McpStatusRegistry ─────────────────────────────────────────────────────────

/// Thread-safe registry of MCP server statuses.
///
/// Entries are updated either from OSC events (passive path via
/// [`Self::apply_event`]) or from the active probe loop
/// ([`probe_loop`]).
#[derive(Clone)]
pub struct McpStatusRegistry {
    inner: Arc<Mutex<Inner>>,
}

impl McpStatusRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                servers: HashMap::new(),
            })),
        }
    }

    /// Update (insert or replace) the status entry for `server`.
    pub fn update(
        &self,
        server: impl Into<String>,
        state: impl Into<String>,
        rpc: Option<String>,
    ) {
        let status = McpServerStatus {
            server: server.into(),
            state: state.into(),
            last_rpc: rpc,
        };
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.servers.insert(status.server.clone(), status);
    }

    /// Return a snapshot of all known server statuses.
    pub fn list(&self) -> Vec<McpServerStatus> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.servers.values().cloned().collect()
    }

    /// Convenience method: apply an [`LlmEvent::Mcp`] event from the bus.
    pub fn apply_event(&self, ev: &LlmEvent) {
        if let LlmEvent::Mcp { server, state, rpc } = ev {
            self.update(server, state, rpc.clone());
        }
    }

    /// Retrieve the current status of a single server by name.
    pub fn get(&self, name: &str) -> Option<McpServerStatus> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.servers.get(name).cloned()
    }
}

impl Default for McpStatusRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── JSON-RPC helpers ──────────────────────────────────────────────────────────

/// Build a minimal JSON-RPC 2.0 request frame.
fn jsonrpc_request(id: u64, method: &str, params: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
    .to_string()
}

/// Attempt to extract the `result` field from a JSON-RPC response line.
/// Returns `None` on parse failure or if the response carries an `error`.
fn extract_jsonrpc_result(line: &str) -> Option<serde_json::Value> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("error").is_some() {
        return None;
    }
    v.get("result").cloned()
}

// ── Stdio probe ───────────────────────────────────────────────────────────────

/// Timeout for a single stdio probe exchange (spawn + initialize + tools/list).
const STDIO_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Perform a stdio JSON-RPC probe against an MCP server.
///
/// Spawns `command args`, sends:
///   1. `initialize` — required by the MCP spec before any other call.
///   2. `tools/list`
///
/// Reads newline-delimited JSON responses and looks for the `tools/list`
/// response.  Returns the resulting [`McpServerStatus`].
async fn probe_stdio(
    name: &str,
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> McpServerStatus {
    let result = timeout(STDIO_PROBE_TIMEOUT, async {
        let mut child = Command::new(command)
            .args(args)
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("spawn failed: {e}"))?;

        let mut stdin = child.stdin.take().ok_or("no stdin handle")?;
        let stdout = child.stdout.take().ok_or("no stdout handle")?;
        let mut lines = BufReader::new(stdout).lines();

        // 1. Send `initialize`.
        let init_req = jsonrpc_request(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "aphrody-terminal-llm-probe", "version": "1.0.0" }
            }),
        );
        stdin
            .write_all(format!("{init_req}\n").as_bytes())
            .await
            .map_err(|e| format!("write initialize failed: {e}"))?;

        // 2. Send `tools/list`.
        let list_req = jsonrpc_request(2, "tools/list", serde_json::json!({}));
        stdin
            .write_all(format!("{list_req}\n").as_bytes())
            .await
            .map_err(|e| format!("write tools/list failed: {e}"))?;
        stdin.flush().await.map_err(|e| format!("flush failed: {e}"))?;

        // 3. Read responses until we see a tools/list result (id == 2).
        let mut tools_result: Option<serde_json::Value> = None;
        while let Ok(Some(line)) = lines.next_line().await {
            // Parse id to detect which response this is.
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                let id = v.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
                if id == 2 {
                    tools_result = extract_jsonrpc_result(&line);
                    break;
                }
            }
        }

        Ok::<_, String>(tools_result)
    })
    .await;

    match result {
        Ok(Ok(Some(tools))) => {
            let tool_count = tools
                .get("tools")
                .and_then(|t| t.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            McpServerStatus {
                server: name.to_owned(),
                state: "connected".to_owned(),
                last_rpc: Some(format!("tools/list({tool_count} tools)")),
            }
        }
        Ok(Ok(None)) => McpServerStatus {
            server: name.to_owned(),
            state: "error".to_owned(),
            last_rpc: Some("tools/list(rpc_error)".to_owned()),
        },
        Ok(Err(msg)) => McpServerStatus {
            server: name.to_owned(),
            state: "error".to_owned(),
            last_rpc: Some(format!("spawn_error: {msg}")),
        },
        Err(_) => McpServerStatus {
            server: name.to_owned(),
            state: "timeout".to_owned(),
            last_rpc: Some("tools/list(timeout)".to_owned()),
        },
    }
}

// ── HTTP probe ────────────────────────────────────────────────────────────────

/// Timeout for a single HTTP probe exchange.
const HTTP_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Perform an HTTP JSON-RPC probe against an MCP server.
///
/// Posts a `tools/list` JSON-RPC 2.0 request to `url`.  If `oauth_provider`
/// is set and its `token_env_var` resolves to a non-empty value in the current
/// process environment, that token is sent as `Authorization: Bearer <token>`.
///
/// On HTTP 401 / 403 returns state `"oauth_required"` rather than `"error"` so
/// the UI can distinguish "not connected yet, needs login" from real failures.
async fn probe_http(name: &str, url: &str, oauth: Option<&OAuthConfig>) -> McpServerStatus {
    // Attempt to resolve a bearer token from the environment.
    let bearer: Option<String> = oauth
        .and_then(|o| o.token_env_var.as_deref())
        .and_then(|var| env::var(var).ok())
        .filter(|t| !t.is_empty());

    let client = match reqwest::Client::builder()
        .timeout(HTTP_PROBE_TIMEOUT)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return McpServerStatus {
                server: name.to_owned(),
                state: "error".to_owned(),
                last_rpc: Some(format!("client_build_error: {e}")),
            };
        }
    };

    let body = jsonrpc_request(1, "tools/list", serde_json::json!({}));
    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(body);

    if let Some(token) = bearer {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    match req.send().await {
        Err(e) => McpServerStatus {
            server: name.to_owned(),
            state: "disconnected".to_owned(),
            last_rpc: Some(format!("http_error: {e}")),
        },
        Ok(resp) => {
            let status_code = resp.status();
            if status_code == reqwest::StatusCode::UNAUTHORIZED
                || status_code == reqwest::StatusCode::FORBIDDEN
            {
                return McpServerStatus {
                    server: name.to_owned(),
                    state: "oauth_required".to_owned(),
                    last_rpc: Some(format!("tools/list(HTTP {status_code})")),
                };
            }
            if !status_code.is_success() {
                return McpServerStatus {
                    server: name.to_owned(),
                    state: "error".to_owned(),
                    last_rpc: Some(format!("tools/list(HTTP {status_code})")),
                };
            }
            match resp.text().await {
                Err(e) => McpServerStatus {
                    server: name.to_owned(),
                    state: "error".to_owned(),
                    last_rpc: Some(format!("body_read_error: {e}")),
                },
                Ok(text) => match extract_jsonrpc_result(&text) {
                    None => McpServerStatus {
                        server: name.to_owned(),
                        state: "error".to_owned(),
                        last_rpc: Some("tools/list(rpc_error)".to_owned()),
                    },
                    Some(tools) => {
                        let tool_count = tools
                            .get("tools")
                            .and_then(|t| t.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        McpServerStatus {
                            server: name.to_owned(),
                            state: "connected".to_owned(),
                            last_rpc: Some(format!("tools/list({tool_count} tools)")),
                        }
                    }
                },
            }
        }
    }
}

// ── Public probe_server ───────────────────────────────────────────────────────

/// Perform a live `tools/list` JSON-RPC probe against an MCP server.
///
/// - For [`McpTransport::Stdio`]: spawn the command, exchange JSON-RPC over
///   stdin/stdout.
/// - For [`McpTransport::Http`]: POST JSON-RPC to the URL, with optional OAuth
///   Bearer token from the environment.
///
/// Returns a [`McpServerStatus`] describing the outcome.  This function does
/// **not** update the [`McpStatusRegistry`] — it is a pure probe.  Use
/// [`probe_loop`] to continuously poll and update the registry.
pub async fn probe_server(spec: &McpServerSpec) -> McpServerStatus {
    match &spec.transport {
        McpTransport::Stdio { command, args, env } => {
            probe_stdio(&spec.name, command, args, env).await
        }
        McpTransport::Http { url, oauth_provider } => {
            probe_http(&spec.name, url, oauth_provider.as_ref()).await
        }
    }
}

// ── probe_loop ────────────────────────────────────────────────────────────────

/// Background loop that polls each [`McpServerSpec`] at `interval`, updates
/// `registry`, and publishes an [`LlmEvent::Mcp`] on `bus` whenever the
/// server's state or last-rpc changes.
///
/// This function runs indefinitely.  Cancel it by dropping the spawned task
/// handle.
///
/// # Arguments
///
/// - `registry`  — shared status registry to update on each probe.
/// - `specs`     — list of servers to probe.
/// - `interval`  — how long to wait between successive probes of the same
///   server (the loop probes all servers sequentially per tick).
/// - `bus`       — event bus on which `LlmEvent::Mcp` events are published
///   when state changes.
pub async fn probe_loop(
    registry: Arc<McpStatusRegistry>,
    specs: Vec<McpServerSpec>,
    interval: Duration,
    bus: Arc<EventBus>,
) {
    loop {
        for spec in &specs {
            let new_status = probe_server(spec).await;

            // Only publish an event if the state or last_rpc changed.
            let prev = registry.get(&new_status.server);
            let changed = prev
                .as_ref()
                .map(|p| p.state != new_status.state || p.last_rpc != new_status.last_rpc)
                .unwrap_or(true);

            if changed {
                let ev = LlmEvent::Mcp {
                    server: new_status.server.clone(),
                    state: new_status.state.clone(),
                    rpc: new_status.last_rpc.clone(),
                };
                // SendError only occurs when there are no receivers — that is
                // benign; the registry is still updated below.
                let _ = bus.publish(ev);
            }

            registry.update(
                new_status.server.clone(),
                new_status.state.clone(),
                new_status.last_rpc.clone(),
            );
        }

        tokio::time::sleep(interval).await;
    }
}

// ── .mcp.json loading ─────────────────────────────────────────────────────────

/// Raw `mcpServers` entry from a `.mcp.json` / `mcp.json` file.
///
/// Matches the schema used by Claude Code:
/// ```json
/// {
///   "type": "stdio",
///   "command": "bun",
///   "args": ["run", "/path/to/server.ts"],
///   "env": { "FOO": "bar" }
/// }
/// ```
/// or for HTTP/SSE:
/// ```json
/// {
///   "type": "streamable-http",
///   "url": "https://api.example.com/mcp/",
///   "headers": { "Authorization": "Bearer ${TOKEN}" }
/// }
/// ```
#[derive(Debug, serde::Deserialize)]
struct McpJsonEntry {
    #[serde(rename = "type", default)]
    transport_type: String,
    // Stdio fields.
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    // HTTP fields.
    url: Option<String>,
    // headers are accepted but not used for probing (they may contain secrets).
    #[serde(default)]
    headers: HashMap<String, String>,
}

/// Top-level structure of a `.mcp.json` file.
#[derive(Debug, serde::Deserialize)]
struct McpJsonFile {
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, McpJsonEntry>,
}

/// Parse a `.mcp.json` / `mcp.json` file and convert its entries to
/// [`McpServerSpec`]s.
///
/// Entries with `type = "streamable-http"` or `type = "http"` are mapped to
/// [`McpTransport::Http`].  All other types (including `"stdio"` and the empty
/// string default) are mapped to [`McpTransport::Stdio`].
///
/// # Errors
///
/// Returns `Err` if the file cannot be read or if the JSON structure is
/// invalid.
pub fn load_mcp_json(path: &std::path::Path) -> anyhow::Result<Vec<McpServerSpec>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
    let file: McpJsonFile = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("invalid JSON in {}: {e}", path.display()))?;

    let specs = file
        .mcp_servers
        .into_iter()
        .map(|(name, entry)| {
            let transport = match entry.transport_type.as_str() {
                "streamable-http" | "http" => {
                    let url = entry.url.unwrap_or_default();
                    // If a header named "Authorization" is present and its value
                    // starts with "Bearer ${", try to extract the env-var name
                    // so probing can use a live token.
                    let token_env_var = entry
                        .headers
                        .get("Authorization")
                        .and_then(|v| {
                            v.strip_prefix("Bearer ${")
                                .and_then(|s| s.strip_suffix('}'))
                                .map(str::to_owned)
                        });
                    let oauth_provider = if token_env_var.is_some() {
                        Some(OAuthConfig {
                            server_url: url.clone(),
                            redirect_uri: String::new(),
                            scope: None,
                            token_env_var,
                        })
                    } else {
                        None
                    };
                    McpTransport::Http { url, oauth_provider }
                }
                _ => {
                    let command = entry.command.unwrap_or_default();
                    McpTransport::Stdio { command, args: entry.args, env: entry.env }
                }
            };
            McpServerSpec { name, transport }
        })
        .collect();

    Ok(specs)
}

// ── Default server specs (feature mcp-default-servers) ───────────────────────

/// Resolve the `google_mcp` binary path.
///
/// Strategy (in order):
/// 1. `GOOGLE_MCP_BIN` environment variable.
/// 2. `<workspace_root>/target/release/google_mcp[.exe]` where workspace root
///    is the directory three levels above the running binary's directory
///    (i.e. `target/release` → `target` → workspace root).
/// 3. `which google_mcp` / `which google_mcp.exe`.
///
/// Returns `None` if none of the above succeeds.
#[cfg(feature = "mcp-default-servers")]
fn resolve_google_mcp_binary() -> Option<String> {
    // 1. Explicit env override.
    if let Ok(v) = env::var("GOOGLE_MCP_BIN") {
        if !v.is_empty() {
            return Some(v);
        }
    }

    // 2. Relative to `cargo`'s output directory: climb from the running binary
    //    path to find `target/release/google_mcp[.exe]`.
    //    When running as `cargo run`, `std::env::current_exe` is inside the
    //    `target/<profile>/` directory.  We walk up until we find a directory
    //    whose parent contains a `Cargo.toml` (workspace root heuristic), then
    //    look for `target/release/google_mcp`.
    let exe_bin = std::env::current_exe().ok()?;
    // target/release/<binary>  ->  parent = target/release  ->  parent = target  ->  parent = workspace root
    let candidate_root: Option<PathBuf> = exe_bin
        .parent() // target/release or target/debug
        .and_then(|p| p.parent()) // target/
        .and_then(|p| p.parent()) // workspace root
        .map(PathBuf::from);

    if let Some(root) = candidate_root {
        let binary_name = if cfg!(target_os = "windows") {
            "google_mcp.exe"
        } else {
            "google_mcp"
        };
        let candidate = root.join("target").join("release").join(binary_name);
        if candidate.exists() {
            return candidate.to_str().map(str::to_owned);
        }
    }

    // 3. PATH lookup via `which`.
    let binary_name = if cfg!(target_os = "windows") { "google_mcp.exe" } else { "google_mcp" };
    which::which(binary_name).ok().and_then(|p| p.to_str().map(str::to_owned))
}

/// Return the default [`McpServerSpec`]s registered by the `aphrody` terminal.
///
/// Only available when the `mcp-default-servers` feature is enabled (the
/// default).  Returns two specs:
///
/// 1. **google_mcp** — stdio via the resolved binary path (see
///    [`resolve_google_mcp_binary`]).  If the binary cannot be found the spec
///    uses the string `"google_mcp"` as the command (relying on `$PATH`).
/// 2. **bxc** — stdio via `bun run C:/worktree/bxc/packages/bxc-extension/server.ts`,
///    matching the entry in the repository `.mcp.json`.
#[cfg(feature = "mcp-default-servers")]
pub fn default_server_specs() -> Vec<McpServerSpec> {
    let google_mcp_cmd = resolve_google_mcp_binary().unwrap_or_else(|| "google_mcp".to_owned());

    let bxc_env: HashMap<String, String> = [(
        "BXC_MEMORY_DB".to_owned(),
        // Default path: prefer APHRODY_ROOT env var, fall back to the hard-coded
        // Windows worktree path from .mcp.json.
        env::var("APHRODY_ROOT")
            .map(|r| format!("{r}/var/data/bxc-memory.sqlite"))
            .unwrap_or_else(|_| "C:/src/aphrody/var/data/bxc-memory.sqlite".to_owned()),
    )]
    .into_iter()
    .collect();

    vec![
        McpServerSpec {
            name: "google_mcp".to_owned(),
            transport: McpTransport::Stdio {
                command: google_mcp_cmd,
                args: vec![],
                env: HashMap::new(),
            },
        },
        McpServerSpec {
            name: "bxc".to_owned(),
            transport: McpTransport::Stdio {
                command: "bun".to_owned(),
                args: vec![
                    "run".to_owned(),
                    "C:/worktree/bxc/packages/bxc-extension/server.ts".to_owned(),
                ],
                env: bxc_env,
            },
        },
    ]
}
