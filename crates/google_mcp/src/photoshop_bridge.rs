// SPDX-License-Identifier: Apache-2.0
//! Live Photoshop bridge — a local WebSocket server that the in-app **UXP
//! plugin** (`apps/photoshop-uxp`) connects to. It lets the `photoshop_live_*`
//! MCP tools drive the *running* desktop Photoshop **from the inside** via
//! `batchPlay` (the universal action-descriptor executor) and the UXP DOM.
//!
//! This is the in-app counterpart to the headless cloud Photoshop API: where
//! the cloud API is limited to a handful of REST operations, `batchPlay` plus
//! an `eval` escape hatch expose the *entire* Photoshop surface — every menu,
//! filter, action and DOM object — the same mechanism ScriptListener records.
//!
//! Protocol (newline-free JSON text frames):
//! - aphrody → plugin: `{ "id": <u64>, "op": "<info|batchPlay|eval>", "args": {} }`
//! - plugin → aphrody: `{ "id": <u64>, "ok": <bool>, "result"?: <any>, "error"?: <str> }`
//!
//! The server binds lazily on the first `photoshop_live_*` call. Only one
//! Photoshop panel is expected; the most recent connection wins.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

/// Loopback bind address for the bridge WebSocket server.
pub(crate) const BRIDGE_ADDR: &str = "127.0.0.1:8765";

/// Shared bridge state: the outbound channel to the connected plugin and the
/// table of in-flight request ids awaiting a correlated response.
struct Bridge {
    started: AtomicBool,
    next_id: AtomicU64,
    sender: Mutex<Option<mpsc::UnboundedSender<String>>>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
}

static BRIDGE: OnceLock<Bridge> = OnceLock::new();

fn bridge() -> &'static Bridge {
    BRIDGE.get_or_init(|| Bridge {
        started: AtomicBool::new(false),
        next_id: AtomicU64::new(1),
        sender: Mutex::new(None),
        pending: Mutex::new(HashMap::new()),
    })
}

/// Start the WS server once. A bind failure (port already held) is non-fatal:
/// another aphrody-mcp instance may already own the bridge.
async fn ensure_started() {
    let b = bridge();
    if b.started.swap(true, Ordering::SeqCst) {
        return;
    }
    match TcpListener::bind(BRIDGE_ADDR).await {
        Ok(listener) => {
            tokio::spawn(async move {
                loop {
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            tokio::spawn(handle_conn(stream));
                        }
                        Err(e) => tracing::warn!("ps-bridge accept failed: {e}"),
                    }
                }
            });
        }
        Err(e) => tracing::warn!("ps-bridge bind {BRIDGE_ADDR} failed: {e}"),
    }
}

/// One UXP-plugin connection: pump outbound commands, resolve inbound responses.
async fn handle_conn(stream: TcpStream) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::warn!("ps-bridge handshake failed: {e}");
            return;
        }
    };
    tracing::info!("ps-bridge: Photoshop UXP plugin connected");
    let (mut sink, mut read) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    *bridge().sender.lock().await = Some(tx);

    // Outbound: forward queued commands to the plugin.
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Inbound: match each response to its pending request id.
    while let Some(Ok(msg)) = read.next().await {
        if let Message::Text(t) = msg {
            if let Ok(v) = serde_json::from_str::<Value>(&t.to_string()) {
                if let Some(id) = v.get("id").and_then(Value::as_u64) {
                    if let Some(resp) = bridge().pending.lock().await.remove(&id) {
                        let _ = resp.send(v);
                    }
                }
            }
        }
    }

    *bridge().sender.lock().await = None;
    writer.abort();
    tracing::info!("ps-bridge: Photoshop UXP plugin disconnected");
}

/// Send `op`/`args` to the live plugin and await its correlated response.
///
/// # Errors
///
/// Returns a human-readable error string when the plugin is not connected, the
/// channel drops, the call times out, or the plugin reports `ok: false`.
pub(crate) async fn call(op: &str, args: Value, timeout: Duration) -> Result<Value, String> {
    ensure_started().await;
    let b = bridge();
    let id = b.next_id.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = oneshot::channel();
    b.pending.lock().await.insert(id, tx);

    let payload = json!({ "id": id, "op": op, "args": args }).to_string();
    {
        let guard = b.sender.lock().await;
        match guard.as_ref() {
            Some(s) if s.send(payload).is_ok() => {}
            _ => {
                b.pending.lock().await.remove(&id);
                return Err("Photoshop UXP plugin not connected — load the aphrody panel \
                    in Photoshop (Plugins ▸ aphrody) and confirm it shows Connected"
                    .to_string());
            }
        }
    }

    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(v)) => {
            if v.get("ok").and_then(Value::as_bool) == Some(false) {
                Err(v
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("plugin reported an error")
                    .to_string())
            } else {
                Ok(v.get("result").cloned().unwrap_or(Value::Null))
            }
        }
        Ok(Err(_)) => Err("bridge response channel dropped".to_string()),
        Err(_) => {
            b.pending.lock().await.remove(&id);
            Err("timeout waiting for Photoshop response".to_string())
        }
    }
}
