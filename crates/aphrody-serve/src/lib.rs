// SPDX-License-Identifier: Apache-2.0
//! `aphrody-serve` — an OpenAI-compatible HTTP server for local open-weight models.
//!
//! It exposes the subset of the OpenAI REST surface that real clients (the
//! `openai` SDKs, `curl`, LangChain, LlamaIndex, Open WebUI, Continue, …)
//! depend on:
//!
//! | Route                       | Behaviour                                            |
//! |-----------------------------|------------------------------------------------------|
//! | `GET  /healthz`             | Liveness probe.                                      |
//! | `GET  /v1/models`           | Lists the backend's models (Ollama `/api/tags`,      |
//! |                             | falling back to a pass-through of `/v1/models`).      |
//! | `POST /v1/chat/completions` | Chat completion, streaming (SSE) and non-streaming.  |
//! | `POST /v1/completions`      | Legacy text completion (transparent relay).          |
//! | `POST /v1/embeddings`       | Embeddings (transparent relay).                      |
//!
//! Chat is served through the typed [`aphrody_gateway::GatewayAdapter`] seam —
//! the place where routing, caching and tool-injection will live.  The other
//! `/v1` endpoints are **transparent relays** to the backend, because every
//! supported engine (Ollama, vLLM, llama-server) already implements them in the
//! OpenAI wire format; relaying keeps aphrody-serve engine-agnostic and avoids
//! re-modelling request shapes it would only forward unchanged.
//!
//! The default backend is the [`OpenAiProxyAdapter`], pointed at any local
//! OpenAI-compatible engine — Ollama on `127.0.0.1:11434`, a llama.cpp
//! `llama-server`, or a vLLM server.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Once};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::sse::Event;
use axum::response::{IntoResponse, Response, Sse};
use axum::routing::{get, post};
use futures::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};

use aphrody_gateway::openai_proxy::OpenAiProxyAdapter;
use aphrody_gateway::{ChatRequest, GatewayAdapter, GatewayError, Message};

/// Default backend root: Ollama's OpenAI-compatible endpoint base.
///
/// The adapter appends `/v1/chat/completions`, yielding Ollama's native
/// OpenAI-compatible chat route.
pub const DEFAULT_BACKEND_URL: &str = "http://127.0.0.1:11434";

/// Shared, cheaply-cloneable server state.
///
/// Construct it with [`local_state`]; the inner fields are intentionally
/// private so the adapter wiring can evolve without breaking callers.
#[derive(Clone)]
pub struct AppState {
    adapter: Arc<dyn GatewayAdapter>,
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

/// Installs a process-wide rustls [`CryptoProvider`] exactly once.
///
/// `reqwest` is built with `rustls-no-provider`, so building a `Client`
/// (which both this crate and the proxy adapter do) requires a default
/// provider to be installed first — otherwise rustls 0.23 panics with
/// "no process-level CryptoProvider available" (see CLAUDE.md §7).
///
/// [`CryptoProvider`]: rustls::crypto::CryptoProvider
fn install_crypto_provider() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Builds the [`AppState`] for an OpenAI-compatible backend at `base_url`.
///
/// `api_key` is forwarded as a bearer token; local engines such as Ollama
/// ignore it, but a non-empty value is required by the adapter.
///
/// # Errors
/// Returns an error if `base_url` is not a valid URL or `api_key` is blank.
pub fn local_state(
    base_url: impl Into<String>,
    api_key: impl Into<String>,
) -> Result<AppState, GatewayError> {
    install_crypto_provider();
    let base_url = base_url.into();
    let api_key = api_key.into();
    let adapter = OpenAiProxyAdapter::new(api_key.clone(), base_url.clone())?;
    Ok(AppState { adapter: Arc::new(adapter), http: reqwest::Client::new(), base_url, api_key })
}

/// Builds the Axum [`Router`] for the given state.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/completions", post(completions))
        .route("/v1/embeddings", post(embeddings))
        .with_state(state)
}

/// Binds `addr` and serves until the process is terminated.
///
/// # Errors
/// Returns an error if the address cannot be bound or the server loop fails.
pub async fn serve(addr: SocketAddr, state: AppState) -> std::io::Result<()> {
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "aphrody-serve listening (OpenAI-compatible)");
    axum::serve(listener, app).await
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn healthz() -> &'static str {
    "ok"
}

/// `GET /v1/models` — lists the backend's models in OpenAI shape.
///
/// Prefers Ollama's richer native `/api/tags`; for non-Ollama engines (vLLM,
/// llama-server) it transparently passes through their OpenAI `/v1/models`.
async fn list_models(State(st): State<AppState>) -> Response {
    let tags_url = format!("{}/api/tags", st.base_url.trim_end_matches('/'));
    let created = unix_secs();

    if let Ok(resp) = st.http.get(&tags_url).send().await
        && resp.status().is_success()
        && let Ok(body) = resp.json::<Value>().await
        && let Some(models) = body.get("models").and_then(Value::as_array)
    {
        let data: Vec<Value> = models
            .iter()
            .filter_map(|m| m.get("name").and_then(Value::as_str))
            .map(|name| {
                json!({ "id": name, "object": "model", "created": created, "owned_by": "local" })
            })
            .collect();
        return Json(json!({ "object": "list", "data": data })).into_response();
    }

    // Non-Ollama engine: pass through its OpenAI-native model list.
    relay_get(&st, "/v1/models").await
}

/// `POST /v1/chat/completions` — streaming (SSE) and non-streaming.
async fn chat_completions(State(st): State<AppState>, Json(req): Json<OpenAiChatRequest>) -> Response {
    let model = req.model.clone();
    let streaming = req.stream;
    let chat_req = req.into_gateway();

    if streaming {
        match st.adapter.chat_stream(chat_req).await {
            Ok(stream) => sse_response(model, stream).into_response(),
            Err(e) => gateway_error(&e),
        }
    } else {
        match st.adapter.chat(chat_req).await {
            Ok(resp) => Json(json!({
                "id": completion_id(),
                "object": "chat.completion",
                "created": unix_secs(),
                "model": resp.model.unwrap_or(model),
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": resp.content },
                    "finish_reason": "stop",
                }],
                "usage": {
                    "prompt_tokens": Value::Null,
                    "completion_tokens": Value::Null,
                    "total_tokens": resp.total_tokens,
                },
            }))
            .into_response(),
            Err(e) => gateway_error(&e),
        }
    }
}

/// `POST /v1/completions` — transparent relay (legacy text completion).
/// Streaming and non-streaming bodies are forwarded unchanged.
async fn completions(State(st): State<AppState>, body: Bytes) -> Response {
    relay_post(&st, "/v1/completions", body).await
}

/// `POST /v1/embeddings` — transparent relay to the backend.
async fn embeddings(State(st): State<AppState>, body: Bytes) -> Response {
    relay_post(&st, "/v1/embeddings", body).await
}

// ---------------------------------------------------------------------------
// Transparent relay (pass-through to the backend's OpenAI surface)
// ---------------------------------------------------------------------------

/// Relays a JSON `POST` to `{base}{path}`, streaming the response back verbatim.
///
/// Streaming (SSE) and non-streaming responses are handled identically because
/// the upstream bytes are forwarded as-is.
async fn relay_post(st: &AppState, path: &str, body: Bytes) -> Response {
    let url = format!("{}{}", st.base_url.trim_end_matches('/'), path);
    match st
        .http
        .post(&url)
        .bearer_auth(&st.api_key)
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
    {
        Ok(resp) => relay_response(resp),
        Err(e) => gateway_error(&GatewayError::Transport(e)),
    }
}

/// Relays a `GET` to `{base}{path}`, streaming the response back verbatim.
async fn relay_get(st: &AppState, path: &str) -> Response {
    let url = format!("{}{}", st.base_url.trim_end_matches('/'), path);
    match st.http.get(&url).bearer_auth(&st.api_key).send().await {
        Ok(resp) => relay_response(resp),
        Err(e) => gateway_error(&GatewayError::Transport(e)),
    }
}

/// Mirrors an upstream `reqwest` response into an axum [`Response`]: identical
/// status, copied `Content-Type`, body streamed chunk-by-chunk.
fn relay_response(resp: reqwest::Response) -> Response {
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = resp.headers().get(CONTENT_TYPE).cloned();

    let mut builder = Response::builder().status(status);
    if let Some(ct) = content_type {
        builder = builder.header(CONTENT_TYPE, ct);
    }
    builder
        .body(Body::from_stream(resp.bytes_stream()))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

// ---------------------------------------------------------------------------
// SSE streaming → OpenAI chat.completion.chunk frames
// ---------------------------------------------------------------------------

/// Wraps a gateway [`ChunkStream`] as OpenAI `chat.completion.chunk` SSE frames,
/// terminated by the `data: [DONE]` sentinel that OpenAI clients expect.
///
/// [`ChunkStream`]: aphrody_gateway::ChunkStream
fn sse_response(
    model: String,
    stream: aphrody_gateway::ChunkStream,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let id = completion_id();
    let created = unix_secs();

    let frames = stream.map(move |item| {
        let event = match item {
            Ok(chunk) => {
                let finish_reason = if chunk.finish { json!("stop") } else { Value::Null };
                let frame = json!({
                    "id": id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model,
                    "choices": [{
                        "index": 0,
                        "delta": { "content": chunk.delta },
                        "finish_reason": finish_reason,
                    }],
                });
                Event::default().data(frame.to_string())
            },
            Err(e) => {
                let frame = json!({ "error": { "message": e.to_string(), "type": "upstream_error" } });
                Event::default().data(frame.to_string())
            },
        };
        Ok(event)
    });

    let done = futures::stream::once(async { Ok(Event::default().data("[DONE]")) });
    Sse::new(frames.chain(done))
}

// ---------------------------------------------------------------------------
// Incoming OpenAI request model
// ---------------------------------------------------------------------------

/// An incoming OpenAI Chat Completions request.  Unknown fields (`top_p`,
/// `n`, `stop`, `tools`, …) are ignored for forward compatibility.
#[derive(Debug, Deserialize)]
struct OpenAiChatRequest {
    model: String,
    #[serde(default)]
    messages: Vec<OpenAiMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    role: String,
    #[serde(default)]
    content: Option<OpenAiContent>,
}

/// Message content: either a plain string or an array of typed parts
/// (the multimodal form).  Only text parts contribute in this revision.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OpenAiContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Deserialize)]
struct ContentPart {
    #[serde(default)]
    text: String,
}

impl OpenAiContent {
    fn into_text(self) -> String {
        match self {
            OpenAiContent::Text(s) => s,
            OpenAiContent::Parts(parts) => {
                parts.into_iter().map(|p| p.text).collect::<Vec<_>>().join("")
            },
        }
    }
}

impl OpenAiChatRequest {
    fn into_gateway(self) -> ChatRequest {
        let messages = self
            .messages
            .into_iter()
            .map(|m| {
                let text = m.content.map(OpenAiContent::into_text).unwrap_or_default();
                match m.role.as_str() {
                    "system" => Message::System(text),
                    "assistant" => Message::Assistant(text),
                    "tool" => Message::Tool("tool".to_owned(), text),
                    _ => Message::User(text),
                }
            })
            .collect();
        ChatRequest {
            model: self.model,
            messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: self.stream,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unix_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn completion_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("chatcmpl-{nanos:x}")
}

/// Maps a [`GatewayError`] to an OpenAI-style error response, preserving the
/// upstream status code when the engine returned a non-2xx.
fn gateway_error(e: &GatewayError) -> Response {
    let (status, msg) = match e {
        GatewayError::UpstreamStatus { status, body } => {
            (StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY), body.clone())
        },
        other => (StatusCode::BAD_GATEWAY, other.to_string()),
    };
    (status, Json(json!({ "error": { "message": msg, "type": "upstream_error" } }))).into_response()
}
