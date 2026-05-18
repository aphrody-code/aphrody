// SPDX-License-Identifier: Apache-2.0
//! `agent-browser` backend (vercel-labs) — full Chromium via CDP, driven over
//! stdin/stdout JSON-RPC 2.0, identical wire protocol to the `bxc` backend.
//!
//! ## Binary
//!
//! Resolved via `which("agent-browser")` at spawn time.  Install from
//! `vercel-labs/agent-browser` or place the binary on `PATH`.
//!
//! ## Capability profile
//!
//! - Navigation, JS eval, DOM queries, screenshots: full support.
//! - Request interception: full support (CDP `Fetch.enable` + `requestPaused`).
//! - Structured extraction: supported via CDP `Runtime.evaluate` + schema walk.
//! - Session recording: supported (CDP `Page.startScreencast` style chunking).

use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
    sync::Mutex,
};
use tracing::{debug, instrument, warn};

use crate::{
    BrowserBackend, BrowserError,
    proto::{BrowserResponse, RecordState, ScreenshotArea},
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

struct Io {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// Backend that delegates to a running `agent-browser` subprocess.
pub struct AgentBrowserBackend {
    _child: Child,
    io: Mutex<Io>,
    /// Kept for diagnostics / tracing; prefixed `_` to silence dead_code.
    _binary: PathBuf,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl AgentBrowserBackend {
    /// Spawn a new `agent-browser` subprocess.
    pub async fn spawn() -> Result<Self, BrowserError> {
        let binary = which::which("agent-browser").map_err(|e| BrowserError::SpawnFailed {
            backend: "agent-browser".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()),
        })?;

        let mut cmd = tokio::process::Command::new(&binary);
        cmd.arg("--stdio")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| BrowserError::SpawnFailed {
            backend: "agent-browser".into(),
            source: e,
        })?;

        let stdin = child.stdin.take().ok_or_else(|| BrowserError::SpawnFailed {
            backend: "agent-browser".into(),
            source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdin pipe missing"),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| BrowserError::SpawnFailed {
            backend: "agent-browser".into(),
            source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdout pipe missing"),
        })?;

        Ok(Self {
            _child: child,
            io: Mutex::new(Io { stdin, stdout: BufReader::new(stdout) }),
            _binary: binary,
        })
    }
}

// ---------------------------------------------------------------------------
// RPC helper
// ---------------------------------------------------------------------------

async fn rpc_call(
    io: &mut Io,
    method: &str,
    params: Value,
    backend: &str,
) -> Result<Value, BrowserError> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    let mut line = serde_json::to_string(&req)?;
    line.push('\n');

    debug!(backend, method, id, "agent-browser rpc →");
    io.stdin.write_all(line.as_bytes()).await.map_err(BrowserError::Io)?;
    io.stdin.flush().await.map_err(BrowserError::Io)?;

    let mut resp_line = String::new();
    io.stdout.read_line(&mut resp_line).await.map_err(BrowserError::Io)?;

    debug!(backend, method, id, resp = %resp_line.trim(), "agent-browser rpc ←");

    let resp: Value = serde_json::from_str(resp_line.trim())?;

    if let Some(err) = resp.get("error") {
        let msg =
            err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown RPC error").to_owned();
        warn!(backend, method, id, error = %msg, "agent-browser rpc error");
        return Err(BrowserError::ProtocolError { backend: backend.into(), message: msg });
    }

    resp.get("result").cloned().ok_or_else(|| BrowserError::ProtocolError {
        backend: backend.into(),
        message: "JSON-RPC response missing `result` field".into(),
    })
}

// ---------------------------------------------------------------------------
// BrowserBackend implementation
// ---------------------------------------------------------------------------

impl BrowserBackend for AgentBrowserBackend {
    fn name(&self) -> &'static str {
        "agent-browser"
    }

    #[instrument(skip(self))]
    async fn navigate(&mut self, url: &str) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result = rpc_call(&mut io, "navigate", json!({ "url": url }), "agent-browser").await?;
        let final_url = result.get("url").and_then(|u| u.as_str()).unwrap_or(url).to_owned();
        Ok(BrowserResponse::Navigated { url: final_url })
    }

    #[instrument(skip(self, src))]
    async fn eval_js(&mut self, src: &str) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result = rpc_call(&mut io, "eval", json!({ "src": src }), "agent-browser").await?;
        let value = result.get("value").cloned().unwrap_or(Value::Null);
        Ok(BrowserResponse::EvalResult { value })
    }

    #[instrument(skip(self))]
    async fn query_selector(&mut self, sel: &str) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result =
            rpc_call(&mut io, "querySelector", json!({ "selector": sel }), "agent-browser").await?;
        let nodes = result.get("nodes").cloned().unwrap_or(Value::Array(vec![]));
        Ok(BrowserResponse::DomResult { nodes })
    }

    #[instrument(skip(self))]
    async fn screenshot(&mut self, area: ScreenshotArea) -> Result<BrowserResponse, BrowserError> {
        let params = match &area {
            ScreenshotArea::Viewport => json!({ "area": "viewport" }),
            ScreenshotArea::Fullpage => json!({ "area": "fullpage" }),
            ScreenshotArea::Element { selector } => {
                json!({ "area": "element", "selector": selector })
            },
        };
        let mut io = self.io.lock().await;
        let result = rpc_call(&mut io, "screenshot", params, "agent-browser").await?;
        let png_b64 = result
            .get("png_b64")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BrowserError::ProtocolError {
                backend: "agent-browser".into(),
                message: "screenshot result missing `png_b64`".into(),
            })?
            .to_owned();
        Ok(BrowserResponse::Screenshot { area, png_b64 })
    }

    #[instrument(skip(self, schema))]
    async fn extract(
        &mut self,
        schema: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result =
            rpc_call(&mut io, "extract", json!({ "schema": schema }), "agent-browser").await?;
        let data = result.get("data").cloned().unwrap_or(Value::Null);
        Ok(BrowserResponse::Extracted { data })
    }

    #[instrument(skip(self, rule))]
    async fn intercept(
        &mut self,
        rule: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result =
            rpc_call(&mut io, "intercept", json!({ "rule": rule }), "agent-browser").await?;
        let rule_id = result.get("rule_id").and_then(|v| v.as_str()).unwrap_or("0").to_owned();
        Ok(BrowserResponse::InterceptInstalled { rule_id })
    }

    #[instrument(skip(self))]
    async fn record(
        &mut self,
        id: &str,
        state: RecordState,
    ) -> Result<BrowserResponse, BrowserError> {
        let state_str = match state {
            RecordState::Start => "start",
            RecordState::Stop => "stop",
        };
        let mut io = self.io.lock().await;
        rpc_call(&mut io, "record", json!({ "id": id, "state": state_str }), "agent-browser")
            .await?;
        Ok(BrowserResponse::RecordAck { id: id.to_owned(), state })
    }
}
