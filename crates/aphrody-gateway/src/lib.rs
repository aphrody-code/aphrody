// SPDX-License-Identifier: Apache-2.0
//! `aphrody-gateway` — provider-agnostic OpenAI-compatible AI gateway.
//!
//! Three concrete adapters ship out of the box:
//!
//! | Adapter              | Module              | Endpoint                                               |
//! |----------------------|---------------------|--------------------------------------------------------|
//! | [`CloudflareAdapter`]| [`cloudflare`]      | `gateway.ai.cloudflare.com/v1/<account>/<gw>/openai/…` |
//! | [`VercelAdapter`]    | [`vercel`]          | `ai-gateway.vercel.sh/v1/chat/completions`             |
//! | [`OpenAiProxyAdapter`]| [`openai_proxy`]   | configurable BYOK OpenAI-compatible endpoint           |
//!
//! All adapters implement [`GatewayAdapter`].  Call [`GatewayAdapter::chat`]
//! for a single round-trip or [`GatewayAdapter::chat_stream`] for a
//! server-sent-events (SSE) stream parsed into [`ChatChunk`] items.

pub mod cloudflare;
pub mod gemini_cli;
pub mod google_antigravity;
pub mod openai_proxy;
pub mod vercel;

use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// All errors that a [`GatewayAdapter`] can return.
#[derive(Debug, Error)]
pub enum GatewayError {
    /// Network transport failure (connection refused, timeout, DNS, …).
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// The upstream returned a non-2xx status code.
    #[error("upstream HTTP {status}: {body}")]
    UpstreamStatus { status: u16, body: String },

    /// JSON serialisation or deserialisation failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL construction failure (invalid account_id / gateway_id / base_url).
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// SSE / streaming framing error (malformed `data:` line).
    #[error("stream framing error: {0}")]
    StreamFraming(String),

    /// Configuration is incomplete (e.g. missing API key, blank account_id).
    #[error("configuration error: {0}")]
    Config(String),
}

// ---------------------------------------------------------------------------
// Message model
// ---------------------------------------------------------------------------

/// A single turn in a conversation.
///
/// The `Tool` variant carries `(tool_name, tool_output)` so callers can
/// surface tool-call results back to the model without a custom struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", content = "content", rename_all = "snake_case")]
pub enum Message {
    /// System-level instructions prepended to the conversation.
    System(String),
    /// A turn authored by the human.
    User(String),
    /// A turn authored by the model.
    Assistant(String),
    /// A tool-call result: `(tool_name, tool_output_text)`.
    Tool(String, String),
}

impl Message {
    /// Returns the role string used by the OpenAI Chat Completions wire format.
    pub fn role(&self) -> &'static str {
        match self {
            Self::System(_) => "system",
            Self::User(_) => "user",
            Self::Assistant(_) => "assistant",
            Self::Tool(..) => "tool",
        }
    }

    /// Returns the text content of the message.
    pub fn content(&self) -> &str {
        match self {
            Self::System(s) | Self::User(s) | Self::Assistant(s) => s.as_str(),
            Self::Tool(_, output) => output.as_str(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// A chat completion request in OpenAI-compatible wire format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The model identifier (e.g. `"claude-sonnet-4-6"`,
    /// `"anthropic/claude-opus-4.6"`, `"gpt-5.4"`).
    pub model: String,

    /// Ordered list of conversation turns.
    pub messages: Vec<Message>,

    /// Sampling temperature in `[0.0, 2.0]`.  `None` defers to the upstream
    /// default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens the model may emit in its response.  `None` defers to
    /// the upstream default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// When `true` the adapter returns a stream; the non-streaming path is
    /// used otherwise.  Adapters honour this field when routing internally.
    #[serde(default)]
    pub stream: bool,
}

impl ChatRequest {
    /// Constructs a minimal non-streaming request.
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self { model: model.into(), messages, temperature: None, max_tokens: None, stream: false }
    }
}

/// The content of a single assistant token emitted during streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    /// Incremental text delta (may be empty for metadata frames).
    pub delta: String,

    /// Whether this chunk is the final one in the stream.
    #[serde(default)]
    pub finish: bool,
}

/// A complete, non-streaming chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The model's reply text.
    pub content: String,

    /// Model identifier echoed from the upstream response (if present).
    pub model: Option<String>,

    /// Total tokens consumed (prompt + completion), if the upstream reports it.
    pub total_tokens: Option<u32>,
}

// ---------------------------------------------------------------------------
// GatewayAdapter trait
// ---------------------------------------------------------------------------

/// Boxed, pinned stream of streaming chunks.
pub type ChunkStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, GatewayError>> + Send>>;

/// The core trait implemented by every gateway adapter.
///
/// Implementors must be `Send + Sync` so they can be stored behind an `Arc`
/// and shared across async tasks.
#[async_trait::async_trait]
pub trait GatewayAdapter: Send + Sync {
    /// Sends a blocking (non-streaming) chat completion request and returns
    /// the full assembled response.
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, GatewayError>;

    /// Sends a streaming chat completion request and returns a [`Stream`] of
    /// incremental [`ChatChunk`]s.
    ///
    /// The stream terminates with an `Err` on any transport, framing, or
    /// upstream error, and ends naturally when the upstream closes the SSE
    /// stream.
    async fn chat_stream(&self, req: ChatRequest) -> Result<ChunkStream, GatewayError>;
}

// ---------------------------------------------------------------------------
// Wire-format helpers (shared across adapters)
// ---------------------------------------------------------------------------

/// Serialises a [`ChatRequest`] into the OpenAI Chat Completions JSON body.
///
/// The `stream` flag is forwarded as-is so callers that invoke this for the
/// streaming path already have `req.stream = true`.
pub(crate) fn build_openai_body(req: &ChatRequest) -> serde_json::Value {
    let messages: Vec<serde_json::Value> = req
        .messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role(),
                "content": m.content(),
            })
        })
        .collect();

    let mut body = serde_json::json!({
        "model": req.model,
        "messages": messages,
        "stream": req.stream,
    });

    if let Some(t) = req.temperature {
        body["temperature"] = serde_json::json!(t);
    }
    if let Some(mt) = req.max_tokens {
        body["max_tokens"] = serde_json::json!(mt);
    }

    body
}

/// Extracts the assistant content string from a complete OpenAI-compatible
/// JSON response.
pub(crate) fn extract_content(body: &serde_json::Value) -> Option<String> {
    body["choices"][0]["message"]["content"].as_str().map(str::to_owned)
}

/// Extracts the model string echoed by the upstream response.
pub(crate) fn extract_model(body: &serde_json::Value) -> Option<String> {
    body["model"].as_str().map(str::to_owned)
}

/// Extracts the total token count from `usage.total_tokens`.
pub(crate) fn extract_total_tokens(body: &serde_json::Value) -> Option<u32> {
    body["usage"]["total_tokens"].as_u64().map(|v| v as u32)
}

/// Parses a single `data: <json>` line from an OpenAI SSE stream into a
/// [`ChatChunk`].  Returns `None` for keepalive frames (`data: [DONE]` or
/// blank).
pub(crate) fn parse_sse_line(line: &str) -> Result<Option<ChatChunk>, GatewayError> {
    let data = match line.strip_prefix("data: ") {
        Some(d) => d.trim(),
        None => return Ok(None),
    };

    if data == "[DONE]" {
        return Ok(Some(ChatChunk { delta: String::new(), finish: true }));
    }

    if data.is_empty() {
        return Ok(None);
    }

    let v: serde_json::Value = serde_json::from_str(data)?;
    let delta = v["choices"][0]["delta"]["content"].as_str().unwrap_or("").to_owned();
    let finish = v["choices"][0]["finish_reason"].as_str().map(|r| r == "stop").unwrap_or(false);

    Ok(Some(ChatChunk { delta, finish }))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- ChatRequest JSON round-trip ----------------------------------------

    #[test]
    fn chat_request_round_trip() {
        let req = ChatRequest {
            model: "claude-sonnet-4-6".to_owned(),
            messages: vec![
                Message::System("You are a test harness.".to_owned()),
                Message::User("ping".to_owned()),
            ],
            temperature: Some(0.7),
            max_tokens: Some(256),
            stream: false,
        };

        let json = serde_json::to_string(&req).expect("serialise");
        let roundtrip: ChatRequest = serde_json::from_str(&json).expect("deserialise");

        assert_eq!(roundtrip.model, req.model);
        assert_eq!(roundtrip.messages.len(), req.messages.len());
        assert_eq!(roundtrip.temperature, req.temperature);
        assert_eq!(roundtrip.max_tokens, req.max_tokens);
        assert!(!roundtrip.stream);
    }

    #[test]
    fn chat_request_defaults_omitted_in_json() {
        let req = ChatRequest::new("gpt-5.4", vec![Message::User("hello".to_owned())]);
        let json = serde_json::to_string(&req).expect("serialise");
        // temperature and max_tokens omitted when None
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    // --- Message role mapping -----------------------------------------------

    #[test]
    fn message_roles_map_correctly() {
        assert_eq!(Message::System(String::new()).role(), "system");
        assert_eq!(Message::User(String::new()).role(), "user");
        assert_eq!(Message::Assistant(String::new()).role(), "assistant");
        assert_eq!(Message::Tool("fn".to_owned(), "out".to_owned()).role(), "tool");
    }

    // --- build_openai_body --------------------------------------------------

    #[test]
    fn build_body_includes_all_fields() {
        let req = ChatRequest {
            model: "gpt-5.4".to_owned(),
            messages: vec![Message::User("hi".to_owned())],
            temperature: Some(1.0),
            max_tokens: Some(512),
            stream: true,
        };
        let body = build_openai_body(&req);
        assert_eq!(body["model"].as_str(), Some("gpt-5.4"));
        assert!(body["messages"].is_array());
        assert_eq!(body["stream"].as_bool(), Some(true));
        assert_eq!(body["temperature"].as_f64(), Some(1.0));
        assert_eq!(body["max_tokens"].as_u64(), Some(512));
    }

    // --- SSE line parser ----------------------------------------------------

    #[test]
    fn parse_sse_done_returns_finish_chunk() {
        let chunk = parse_sse_line("data: [DONE]").expect("parse").expect("some chunk");
        assert!(chunk.finish);
        assert!(chunk.delta.is_empty());
    }

    #[test]
    fn parse_sse_blank_returns_none() {
        assert!(parse_sse_line("").expect("parse").is_none());
        assert!(parse_sse_line("event: ping").expect("parse").is_none());
    }

    #[test]
    fn parse_sse_delta_extracted() {
        let line = r#"data: {"choices":[{"delta":{"content":"hello"},"finish_reason":null}]}"#;
        let chunk = parse_sse_line(line).expect("parse").expect("some");
        assert_eq!(chunk.delta, "hello");
        assert!(!chunk.finish);
    }

    #[test]
    fn parse_sse_stop_sets_finish() {
        let line = r#"data: {"choices":[{"delta":{"content":""},"finish_reason":"stop"}]}"#;
        let chunk = parse_sse_line(line).expect("parse").expect("some");
        assert!(chunk.finish);
    }

    // --- extract_content / model / total_tokens ----------------------------

    #[test]
    fn extract_helpers_work() {
        let body = serde_json::json!({
            "model": "gpt-5.4",
            "choices": [{"message": {"content": "world"}}],
            "usage": {"total_tokens": 42}
        });
        assert_eq!(extract_content(&body).as_deref(), Some("world"));
        assert_eq!(extract_model(&body).as_deref(), Some("gpt-5.4"));
        assert_eq!(extract_total_tokens(&body), Some(42));
    }

    // --- ChatChunk round-trip -----------------------------------------------

    #[test]
    fn chunk_round_trip() {
        let chunk = ChatChunk { delta: "tok".to_owned(), finish: false };
        let json = serde_json::to_string(&chunk).expect("ser");
        let back: ChatChunk = serde_json::from_str(&json).expect("de");
        assert_eq!(back.delta, "tok");
        assert!(!back.finish);
    }
}
