// SPDX-License-Identifier: Apache-2.0
//! Codex-shaped local app-server for Aphrody.
//!
//! The wire contract is deliberately provider-neutral: JSON-RPC 2.0 requests
//! manage threads and start turns, while the implementation always uses the
//! local, keyless Aphrody runtime. This lets the CLI, TUI and web GUI share a
//! single session boundary without depending on OpenAI cloud services.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use aphrody_agent_proto::{Event, EventMsg, InputItem, Op};
use aphrody_agent_home::{AgentHome, BootstrapBudget, HomeOptions};
use aphrody_agent_runtime::{AgentRuntime, ModelChoice, RuntimeConfig};
use axum::{Json, Router, extract::State, routing::{get, post}};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Error)]
pub enum AppServerError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("method failed: {0}")]
    Runtime(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct ThreadInfo {
    id: String,
    created_at: u64,
    model: String,
}

#[derive(Debug, Default)]
pub struct AppServer {
    threads: BTreeMap<String, ThreadState>,
    initialized: bool,
}

#[derive(Debug)]
struct ThreadState {
    info: ThreadInfo,
    events: UnboundedReceiver<Event>,
    handle: aphrody_engine::SessionHandle,
}

impl AppServer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle one JSON-RPC request. Notifications produce no response.
    pub async fn handle(&mut self, line: &str) -> Result<Option<String>, AppServerError> {
        let request: RpcRequest = serde_json::from_str(line)?;
        if request.method != "initialize" && request.method != "initialized" && !self.initialized {
            if request.id.is_none() {
                return Ok(None);
            }
            return Ok(Some(serde_json::to_string(&RpcResponse {
                jsonrpc: "2.0",
                id: request.id,
                result: None,
                error: Some(RpcError { code: -32002, message: "Not initialized".into() }),
            })?));
        }
        let Some(id) = request.id.clone() else {
            if request.method == "initialized" {
                self.initialized = true;
            }
            return Ok(None);
        };
        let result = self.dispatch(&request.method, request.params).await;
        let response = match result {
            Ok(value) => RpcResponse { jsonrpc: "2.0", id: Some(id), result: Some(value), error: None },
            Err(error) => RpcResponse {
                jsonrpc: "2.0",
                id: Some(id),
                result: None,
                error: Some(RpcError { code: -32603, message: error.to_string() }),
            },
        };
        Ok(Some(serde_json::to_string(&response)?))
    }

    async fn dispatch(&mut self, method: &str, params: Value) -> Result<Value, AppServerError> {
        match method {
            "initialize" if self.initialized => Err(AppServerError::InvalidRequest("Already initialized".into())),
            "initialize" => Ok(json!({
                "serverInfo": {"name": "aphrody", "version": env!("CARGO_PKG_VERSION")},
                "capabilities": {"threads": true, "turns": true, "localModels": true, "interrupt": true},
                "authentication": {"required": false}
            })),
            "thread/list" => Ok(json!({
                "data": self.threads.values().map(|thread| &thread.info).collect::<Vec<_>>()
            })),
            "thread/read" => {
                let thread_id = params.get("threadId").and_then(Value::as_str)
                    .ok_or_else(|| AppServerError::InvalidRequest("thread/read requires threadId".into()))?;
                let thread = self.threads.get(thread_id)
                    .ok_or_else(|| AppServerError::InvalidRequest(format!("unknown thread: {thread_id}")))?;
                Ok(json!({ "thread": thread.info }))
            },
            "thread/resume" => {
                let thread_id = params.get("threadId").and_then(Value::as_str)
                    .ok_or_else(|| AppServerError::InvalidRequest("thread/resume requires threadId".into()))?;
                let thread = self.threads.get(thread_id)
                    .ok_or_else(|| AppServerError::InvalidRequest(format!("unknown thread: {thread_id}")))?;
                Ok(serde_json::to_value(&thread.info)?)
            },
            "turn/events" => {
                let thread_id = params.get("threadId").and_then(Value::as_str)
                    .ok_or_else(|| AppServerError::InvalidRequest("turn/events requires threadId".into()))?;
                let thread = self.threads.get_mut(thread_id)
                    .ok_or_else(|| AppServerError::InvalidRequest(format!("unknown thread: {thread_id}")))?;
                let mut events = Vec::new();
                let mut completed = false;
                loop {
                    match thread.events.try_recv() {
                        Ok(event) => {
                            completed = completed || matches!(&event.msg, EventMsg::TurnComplete { .. } | EventMsg::Error { .. });
                            events.push(event);
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => break,
                    }
                }
                Ok(json!({ "threadId": thread_id, "data": events, "completed": completed }))
            }
            "tools/list" => {
                let model = params.get("model").and_then(Value::as_str).unwrap_or("llama3.2");
                let base_url = std::env::var("APHRODY_LOCAL_MODEL_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
                let runtime = AgentRuntime::builder()
                    .config(RuntimeConfig::new(model))
                    .model(ModelChoice::local_responses(base_url, model))
                    .build()
                    .map_err(|error| AppServerError::Runtime(error.to_string()))?;
                Ok(json!({ "data": runtime.tools().openai_functions() }))
            }
            "turn/interrupt" => {
                let thread_id = params.get("threadId").and_then(Value::as_str)
                    .ok_or_else(|| AppServerError::InvalidRequest("turn/interrupt requires threadId".into()))?;
                let thread = self.threads.get(thread_id)
                    .ok_or_else(|| AppServerError::InvalidRequest(format!("unknown thread: {thread_id}")))?;
                thread.handle.submit(Op::Interrupt)
                    .map_err(|_| AppServerError::Runtime("session actor stopped".into()))?;
                Ok(json!({ "threadId": thread_id, "status": "interrupt_requested" }))
            }
            "thread/close" => {
                let thread_id = params.get("threadId").and_then(Value::as_str)
                    .ok_or_else(|| AppServerError::InvalidRequest("thread/close requires threadId".into()))?;
                let thread = self.threads.remove(thread_id)
                    .ok_or_else(|| AppServerError::InvalidRequest(format!("unknown thread: {thread_id}")))?;
                let _ = thread.handle.submit(Op::Shutdown);
                let _ = thread.handle.join().await;
                Ok(json!({ "threadId": thread_id, "status": "closed" }))
            }
            "thread/start" => {
                let model = params.get("model").and_then(Value::as_str).unwrap_or("llama3.2");
                let id = format!("thread-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
                let info = ThreadInfo { id: id.clone(), created_at: now(), model: model.to_string() };
                let base_url = std::env::var("APHRODY_LOCAL_MODEL_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
                let runtime = AgentRuntime::builder()
                    .config(RuntimeConfig::new(model).with_system_prompt(system_prompt()))
                    .model(ModelChoice::local_responses(base_url, info.model.clone()))
                    .build()
                    .map_err(|error| AppServerError::Runtime(error.to_string()))?;
                let mut handle = runtime.spawn().await
                    .map_err(|error| AppServerError::Runtime(error.to_string()))?;
                let events = handle.events().ok_or_else(||
                    AppServerError::Runtime("session event stream unavailable".into()))?;
                self.threads.insert(id, ThreadState { info: info.clone(), events, handle });
                Ok(serde_json::to_value(info)?)
            },
            "turn/start" => {
                let thread_id = params.get("threadId").and_then(Value::as_str)
                    .ok_or_else(|| AppServerError::InvalidRequest("turn/start requires threadId".into()))?;
                let thread = self.threads.get_mut(thread_id)
                    .ok_or_else(|| AppServerError::InvalidRequest(format!("unknown thread: {thread_id}")))?;
                let input = params.get("input").and_then(Value::as_str)
                    .ok_or_else(|| AppServerError::InvalidRequest("turn/start requires string input".into()))?;
                thread.handle.submit(Op::UserInput {
                    items: vec![InputItem::Text { text: input.to_string() }],
                }).map_err(|_| AppServerError::Runtime("session actor stopped".into()))?;
                if params.get("waitForCompletion").and_then(Value::as_bool) == Some(false) {
                    return Ok(json!({
                        "id": format!("turn-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed)),
                        "threadId": thread.info.id,
                        "status": "in_progress"
                    }));
                }
                let mut events = Vec::new();
                let output = loop {
                    let event = thread.events.recv().await
                        .ok_or_else(|| AppServerError::Runtime("session event stream closed".into()))?;
                    let completed = match &event.msg {
                        EventMsg::TurnComplete { last_agent_message } => Some(last_agent_message.clone()),
                        EventMsg::Error { message } => {
                            return Err(AppServerError::Runtime(message.clone()));
                        }
                        _ => None,
                    };
                    events.push(event);
                    if completed.is_some() { break completed.flatten().unwrap_or_default(); }
                };
                Ok(json!({
                    "id": format!("turn-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed)),
                    "threadId": thread.info.id,
                    "status": "completed",
                    "output": output,
                    "events": events
                }))
            },
            "shutdown" => Ok(json!({ "ok": true })),
            _ => Err(AppServerError::InvalidRequest(format!("unknown method: {method}"))),
        }
    }
}

/// Serve newline-delimited JSON-RPC over stdin/stdout, suitable for a GUI or
/// another local process. No credentials are read or emitted.
pub async fn serve_stdio<R, W>(reader: R, mut writer: W) -> Result<(), AppServerError>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut lines = reader.lines();
    let mut server = AppServer::new();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() { continue; }
        if let Some(response) = server.handle(&line).await? {
            writer.write_all(response.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }
    }
    Ok(())
}

/// Serve the same JSON-RPC app-server over local HTTP.
///
/// The caller chooses the bind address explicitly; no public interface or
/// credential-bearing cloud transport is enabled implicitly.
pub async fn serve_http(addr: SocketAddr) -> Result<(), AppServerError> {
    let state = Arc::new(Mutex::new(AppServer::new()));
    let app = Router::new()
        .route("/health", get(|| async { Json(json!({"ok": true, "authentication_required": false})) }))
        .route("/rpc", post(http_rpc))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await.map_err(AppServerError::Io)
}

async fn http_rpc(
    State(state): State<Arc<Mutex<AppServer>>>,
    Json(request): Json<Value>,
) -> Json<Value> {
    let mut server = state.lock().await;
    let line = match serde_json::to_string(&request) {
        Ok(line) => line,
        Err(error) => return Json(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":error.to_string()}})),
    };
    match server.handle(&line).await {
        Ok(Some(response)) => serde_json::from_str(&response)
            .map(Json)
            .unwrap_or_else(|error| Json(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":error.to_string()}}))),
        Ok(None) => Json(json!({})),
        Err(error) => Json(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":error.to_string()}})),
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn system_prompt() -> String {
    let policy = "Act proactively. Execute local tools immediately without asking for confirmation. For unfamiliar work, inspect before acting; use repository search and structured data extraction. Prefer local RAG/memory, OCR for images and documents, and computer-use/browser tools when available. Cross-check findings, then return concise, direct results with essential evidence and next action.";
    let home = AgentHome::open(HomeOptions::default())
        .ok()
        .map(|agent_home| agent_home.system_prompt(&BootstrapBudget::default()).render())
        .unwrap_or_default();
    if home.trim().is_empty() { policy.to_string() } else { format!("{policy}\n\n{home}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn initializes_without_credentials() {
        let mut server = AppServer::new();
        let response = server.handle(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#).await.unwrap().unwrap();
        assert!(response.contains("\"required\":false"));
    }

    #[tokio::test]
    async fn requires_initialized_handshake_before_requests() {
        let mut server = AppServer::new();
        let response = server.handle(r#"{"jsonrpc":"2.0","id":1,"method":"thread/list","params":{}}"#).await.unwrap().unwrap();
        assert!(response.contains("Not initialized"));
        server.handle(r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}"#).await.unwrap();
        server.handle(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#).await.unwrap();
        let response = server.handle(r#"{"jsonrpc":"2.0","id":3,"method":"thread/list","params":{}}"#).await.unwrap().unwrap();
        assert!(response.contains("\"data\":[]"));
    }

    #[tokio::test]
    async fn starts_and_lists_threads() {
        let mut server = AppServer::new();
        server.handle(r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}"#).await.unwrap();
        server.handle(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#).await.unwrap();
        let start = server.handle(r#"{"jsonrpc":"2.0","id":1,"method":"thread/start","params":{"model":"local"}}"#).await.unwrap().unwrap();
        assert!(start.contains("thread-"));
        let list = server.handle(r#"{"jsonrpc":"2.0","id":2,"method":"thread/list","params":{}}"#).await.unwrap().unwrap();
        assert!(list.contains("local"));
    }

    #[tokio::test]
    async fn lists_local_agent_tools() {
        let mut server = AppServer::new();
        server.handle(r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}"#).await.unwrap();
        server.handle(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#).await.unwrap();
        let response = server.handle(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#).await.unwrap().unwrap();
        assert!(response.contains("read_file"));
        assert!(response.contains("search_text"));
        assert!(response.contains("computer_use"));
    }

    #[tokio::test]
    async fn closes_a_started_thread() {
        let mut server = AppServer::new();
        server.handle(r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}"#).await.unwrap();
        server.handle(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#).await.unwrap();
        let start = server.handle(r#"{"jsonrpc":"2.0","id":1,"method":"thread/start","params":{"model":"local"}}"#).await.unwrap().unwrap();
        let thread_id = start.split("\"id\":\"").nth(1).and_then(|value| value.split('"').next()).unwrap();
        let request = format!(r#"{{"jsonrpc":"2.0","id":2,"method":"thread/close","params":{{"threadId":"{thread_id}"}}}}"#);
        let response = server.handle(&request).await.unwrap().unwrap();
        assert!(response.contains("\"closed\""));
    }

    #[tokio::test]
    async fn reads_and_resumes_a_loaded_thread() {
        let mut server = AppServer::new();
        server.handle(r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}"#).await.unwrap();
        server.handle(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#).await.unwrap();
        let start = server.handle(r#"{"jsonrpc":"2.0","id":1,"method":"thread/start","params":{"model":"local"}}"#).await.unwrap().unwrap();
        let thread_id = start.split("\"id\":\"").nth(1).and_then(|value| value.split('\"').next()).unwrap();
        let read = server.handle(&format!(r#"{{"jsonrpc":"2.0","id":2,"method":"thread/read","params":{{"threadId":"{thread_id}"}}}}"#)).await.unwrap().unwrap();
        assert!(read.contains(thread_id));
        let resumed = server.handle(&format!(r#"{{"jsonrpc":"2.0","id":3,"method":"thread/resume","params":{{"threadId":"{thread_id}"}}}}"#)).await.unwrap().unwrap();
        assert!(resumed.contains(thread_id));
    }
}
