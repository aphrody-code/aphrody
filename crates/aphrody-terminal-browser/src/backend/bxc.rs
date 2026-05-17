// SPDX-License-Identifier: Apache-2.0
//! `bxc` backend — spawns the `bxc` CLI and drives it over stdin/stdout
//! using newline-delimited JSON-RPC 2.0.
//!
//! ## Protocol
//!
//! Each request is a JSON object sent on a single line to the child's stdin:
//!
//! ```json
//! {"jsonrpc":"2.0","id":1,"method":"navigate","params":{"url":"https://…"}}
//! ```
//!
//! The child responds with one JSON line per request:
//!
//! ```json
//! {"jsonrpc":"2.0","id":1,"result":{…}}
//! ```
//!
//! Error responses follow the JSON-RPC 2.0 error envelope:
//!
//! ```json
//! {"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"…"}}
//! ```
//!
//! ## Binary location
//!
//! The backend resolves `bxc` via `which` at spawn time.  The worktree at
//! `C:/worktree/bxc/` (or any directory on `PATH`) is sufficient.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

use crate::proto::{BrowserResponse, RecordState, ScreenshotArea};
use crate::{BrowserBackend, BrowserError};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Counter shared across all method calls; produces monotonically increasing
/// JSON-RPC request IDs without locking.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

struct Io {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// Backend that delegates all operations to a running `bxc` subprocess.
pub struct BxcBackend {
    _child: Child,
    io: Mutex<Io>,
    binary: PathBuf,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl BxcBackend {
    /// Spawn a new `bxc` subprocess.  Returns `BrowserError::SpawnFailed` if
    /// the binary cannot be found or if the OS refuses to start it.
    pub async fn spawn() -> Result<Self, BrowserError> {
        let binary = which::which("bxc").map_err(|e| BrowserError::SpawnFailed {
            backend: "bxc".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()),
        })?;

        let mut cmd = tokio::process::Command::new(&binary);
        cmd.arg("--rpc")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| BrowserError::SpawnFailed {
            backend: "bxc".into(),
            source: e,
        })?;

        let stdin = child.stdin.take().ok_or_else(|| BrowserError::SpawnFailed {
            backend: "bxc".into(),
            source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdin pipe missing"),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| BrowserError::SpawnFailed {
            backend: "bxc".into(),
            source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdout pipe missing"),
        })?;

        Ok(Self {
            _child: child,
            io: Mutex::new(Io {
                stdin,
                stdout: BufReader::new(stdout),
            }),
            binary,
        })
    }
}

// ---------------------------------------------------------------------------
// RPC helper
// ---------------------------------------------------------------------------

/// Send one JSON-RPC request and read one response line.
///
/// The lock on `io` must be held by the caller.
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

    debug!(backend, method, id, "bxc rpc →");
    io.stdin
        .write_all(line.as_bytes())
        .await
        .map_err(BrowserError::Io)?;
    io.stdin.flush().await.map_err(BrowserError::Io)?;

    let mut resp_line = String::new();
    io.stdout
        .read_line(&mut resp_line)
        .await
        .map_err(BrowserError::Io)?;

    debug!(backend, method, id, resp = %resp_line.trim(), "bxc rpc ←");

    let resp: Value = serde_json::from_str(resp_line.trim())?;

    if let Some(err) = resp.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown RPC error")
            .to_owned();
        warn!(backend, method, id, error = %msg, "bxc rpc error");
        return Err(BrowserError::ProtocolError {
            backend: backend.into(),
            message: msg,
        });
    }

    resp.get("result")
        .cloned()
        .ok_or_else(|| BrowserError::ProtocolError {
            backend: backend.into(),
            message: "JSON-RPC response missing `result` field".into(),
        })
}

// ---------------------------------------------------------------------------
// BrowserBackend implementation
// ---------------------------------------------------------------------------

impl BrowserBackend for BxcBackend {
    fn name(&self) -> &'static str {
        "bxc"
    }

    #[instrument(skip(self))]
    async fn navigate(&mut self, url: &str) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result = rpc_call(
            &mut io,
            "navigate",
            json!({ "url": url }),
            self.binary.to_str().unwrap_or("bxc"),
        )
        .await?;
        let final_url = result
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or(url)
            .to_owned();
        Ok(BrowserResponse::Navigated { url: final_url })
    }

    #[instrument(skip(self, src))]
    async fn eval_js(&mut self, src: &str) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result = rpc_call(
            &mut io,
            "eval",
            json!({ "src": src }),
            "bxc",
        )
        .await?;
        let value = result.get("value").cloned().unwrap_or(Value::Null);
        Ok(BrowserResponse::EvalResult { value })
    }

    #[instrument(skip(self))]
    async fn query_selector(&mut self, sel: &str) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result = rpc_call(
            &mut io,
            "querySelector",
            json!({ "selector": sel }),
            "bxc",
        )
        .await?;
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
            }
        };
        let mut io = self.io.lock().await;
        let result = rpc_call(&mut io, "screenshot", params, "bxc").await?;
        let png_b64 = result
            .get("png_b64")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BrowserError::ProtocolError {
                backend: "bxc".into(),
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
        let result = rpc_call(
            &mut io,
            "extract",
            json!({ "schema": schema }),
            "bxc",
        )
        .await?;
        let data = result.get("data").cloned().unwrap_or(Value::Null);
        Ok(BrowserResponse::Extracted { data })
    }

    #[instrument(skip(self, rule))]
    async fn intercept(
        &mut self,
        rule: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError> {
        let mut io = self.io.lock().await;
        let result = rpc_call(
            &mut io,
            "intercept",
            json!({ "rule": rule }),
            "bxc",
        )
        .await?;
        let rule_id = result
            .get("rule_id")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_owned();
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
        rpc_call(
            &mut io,
            "record",
            json!({ "id": id, "state": state_str }),
            "bxc",
        )
        .await?;
        Ok(BrowserResponse::RecordAck {
            id: id.to_owned(),
            state,
        })
    }
}
