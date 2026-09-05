// SPDX-License-Identifier: Apache-2.0
//! `aphrody-model-client` — provider abstraction for the aphrody agent engine.
//!
//! This crate gives the turn loop a provider-agnostic surface:
//!
//! - [`ModelClient`] — an `async_trait` object-safe trait every backend (Gemini REST, gemini-web,
//!   agy) implements.
//! - [`ModelStreamEvent`] — a unified streaming event (text delta, reasoning delta, tool call,
//!   completion with token usage) that hides the wire format of the underlying API.
//! - Neutral request types ([`ChatTurn`], [`ChatMessage`], [`Part`], …) that describe a
//!   conversation turn without leaking any vendor schema.
//! - [`GeminiClient`] — a concrete client that maps Google's `:streamGenerateContent?alt=sse` wire
//!   onto the unified events.
//!
//! The Gemini wire mapping follows the OpenAI/Gemini delta table in
//! `docs/plans/gemini-codex.md` (§3) and the SSE pattern in
//! `var/codex/codex-rs/codex-api/src/sse/`.
//!
//! This crate is **host-only** (it pulls in `reqwest`); it is not part of the
//! wasm matrix.

#![forbid(unsafe_code)]

use std::pin::Pin;

use aphrody_toolcall::sanitize_gemini_json_schema;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::{Value, json};

mod openai_responses;
pub use openai_responses::{LocalResponsesClient, build_responses_request, parse_responses_event};

/// Errors surfaced by a [`ModelClient`] or the pure wire helpers.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// Transport-level failure issuing the request or reading the body.
    #[error("http transport error: {0}")]
    Http(#[from] reqwest::Error),
    /// A JSON payload (SSE frame or response body) failed to deserialize.
    #[error("json decode error: {0}")]
    Decode(#[from] serde_json::Error),
    /// The provider returned a non-success status with an error payload.
    #[error("provider api error (status {status}): {message}")]
    Api {
        /// HTTP status code returned by the provider.
        status: u16,
        /// Human-readable message extracted from the error body.
        message: String,
    },
    /// The byte stream broke or produced a malformed event.
    #[error("stream error: {0}")]
    Stream(String),
}

/// Token accounting reported by the provider at the end of a turn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenUsage {
    /// Tokens consumed by the prompt (input).
    pub input_tokens: u64,
    /// Tokens produced by the model (output / candidates).
    pub output_tokens: u64,
    /// Total tokens billed for the turn.
    pub total_tokens: u64,
}

/// A single unified streaming event emitted while a turn is being generated.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelStreamEvent {
    /// The stream has been opened; emitted once before any content.
    Created,
    /// An incremental chunk of assistant-visible text.
    TextDelta(String),
    /// An incremental chunk of model reasoning / "thinking" text.
    ReasoningDelta(String),
    /// The model requested a tool / function invocation.
    ToolCall {
        /// Stable id for this call (generated locally for Gemini).
        id: String,
        /// Declared name of the function the model wants to invoke.
        name: String,
        /// JSON arguments object for the call.
        arguments: Value,
    },
    /// The turn finished; carries optional usage + finish reason.
    Completed {
        /// Token accounting, when the provider reports it.
        usage: Option<TokenUsage>,
        /// Provider-specific finish reason (e.g. `STOP`, `MAX_TOKENS`).
        finish_reason: Option<String>,
    },
}

/// Role of a message in a [`ChatTurn`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// End-user input.
    User,
    /// Prior model output.
    Model,
    /// Output of a tool / function execution fed back to the model.
    Tool,
}

impl Role {
    /// Gemini `contents[].role` wire string.
    ///
    /// Gemini only knows `user` and `model`; tool results are carried inside a
    /// `user`-role turn via `functionResponse` parts.
    #[must_use]
    fn as_gemini_role(self) -> &'static str {
        match self {
            Role::User | Role::Tool => "user",
            Role::Model => "model",
        }
    }
}

/// A single content part inside a [`ChatMessage`].
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    /// Plain text content.
    Text(String),
    /// A function call previously emitted by the model.
    FunctionCall {
        /// Function name.
        name: String,
        /// JSON arguments object.
        args: Value,
    },
    /// The response of a tool / function execution.
    FunctionResponse {
        /// Function name this response corresponds to.
        name: String,
        /// JSON response payload.
        response: Value,
    },
}

/// One message in the conversation history.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    /// Who authored the message.
    pub role: Role,
    /// Ordered content parts.
    pub parts: Vec<Part>,
}

/// A declared tool the model may call.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDecl {
    /// Function name.
    pub name: String,
    /// Human-readable description shown to the model.
    pub description: String,
    /// JSON Schema object describing the function parameters.
    pub parameters: Value,
}

/// A full conversation turn handed to a [`ModelClient`].
#[derive(Debug, Clone, PartialEq)]
pub struct ChatTurn {
    /// Optional system instruction.
    pub system: Option<String>,
    /// Ordered conversation history.
    pub messages: Vec<ChatMessage>,
    /// Tools available to the model this turn.
    pub tools: Vec<ToolDecl>,
}

/// A pinned, boxed stream of unified model events.
pub type ModelStream = Pin<Box<dyn Stream<Item = Result<ModelStreamEvent, ModelError>> + Send>>;

/// Provider-agnostic streaming chat client.
#[async_trait]
pub trait ModelClient: Send + Sync {
    /// Stream a turn, yielding unified [`ModelStreamEvent`]s.
    async fn stream(&self, turn: ChatTurn) -> Result<ModelStream, ModelError>;

    /// The model identifier this client targets.
    fn model_id(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Pure wire helpers (offline-testable, no network).
// ---------------------------------------------------------------------------

/// Build the Gemini `GenerateContentRequest` JSON body from a neutral turn.
///
/// Shape produced (per `docs/plans/gemini-codex.md` §3):
/// - `systemInstruction.parts[].text` when [`ChatTurn::system`] is set;
/// - `contents[{role, parts}]` mapping [`Part`] variants to `text` / `functionCall{name,args}` /
///   `functionResponse{name,response}`;
/// - `tools[{function_declarations:[{name,description,parameters}]}]` when any tool is declared.
#[must_use]
pub fn build_request_body(turn: &ChatTurn) -> Value {
    let mut body = serde_json::Map::new();

    if let Some(system) = &turn.system {
        body.insert("systemInstruction".to_string(), json!({ "parts": [{ "text": system }] }));
    }

    let contents: Vec<Value> = turn
        .messages
        .iter()
        .map(|msg| {
            let parts: Vec<Value> = msg.parts.iter().map(part_to_value).collect();
            json!({ "role": msg.role.as_gemini_role(), "parts": parts })
        })
        .collect();
    body.insert("contents".to_string(), Value::Array(contents));

    if !turn.tools.is_empty() {
        let decls: Vec<Value> = turn
            .tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": sanitize_gemini_json_schema(t.parameters.clone()),
                })
            })
            .collect();
        body.insert("tools".to_string(), json!([{ "function_declarations": decls }]));
    }

    Value::Object(body)
}

/// Map a single neutral [`Part`] onto its Gemini wire `Value`.
fn part_to_value(part: &Part) -> Value {
    match part {
        Part::Text(text) => json!({ "text": text }),
        Part::FunctionCall { name, args } => json!({
            "functionCall": { "name": name, "args": args }
        }),
        Part::FunctionResponse { name, response } => json!({
            "functionResponse": { "name": name, "response": response }
        }),
    }
}

/// Split complete SSE events out of an accumulating `buffer`.
///
/// SSE events are delimited by a blank line (`\n\n`). Any trailing partial
/// event (no terminating blank line yet) is left in `buffer` for the next
/// chunk. Returns the raw event blocks (everything before each `\n\n`),
/// including their `data:`/`event:` field lines.
#[must_use]
pub fn split_sse_events(buffer: &mut String) -> Vec<String> {
    let mut events = Vec::new();
    // Normalize CRLF so the `\n\n` delimiter search is uniform.
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n");
    }
    while let Some(idx) = buffer.find("\n\n") {
        let event: String = buffer.drain(..idx).collect();
        // Drop the delimiter itself.
        buffer.drain(..2);
        if !event.trim().is_empty() {
            events.push(event);
        }
    }
    events
}

/// Extract the concatenated `data:` payload from one raw SSE event block.
///
/// An SSE event may carry multiple `data:` lines which are joined with `\n`
/// per the spec. Lines that are not `data:` fields (e.g. `event:`, comments
/// starting with `:`) are ignored.
#[must_use]
pub fn sse_event_data(event: &str) -> String {
    let mut data_lines = Vec::new();
    for line in event.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            // A single leading space after the colon is part of the syntax.
            data_lines.push(rest.strip_prefix(' ').unwrap_or(rest));
        }
    }
    data_lines.join("\n")
}

/// Parse one Gemini `GenerateContentResponse` JSON payload into unified events.
///
/// Mapping (per §3 of the codex plan):
/// - `candidates[0].content.parts[].text` -> [`ModelStreamEvent::TextDelta`];
/// - `candidates[0].content.parts[].functionCall{name,args}` -> [`ModelStreamEvent::ToolCall`] (id
///   generated locally);
/// - `candidates[0].finishReason` and/or `usageMetadata` -> [`ModelStreamEvent::Completed`].
///
/// A single frame can produce multiple events (e.g. several text parts, or a
/// text part plus a finish reason).
pub fn parse_sse_frame(json: &str) -> Result<Vec<ModelStreamEvent>, ModelError> {
    let value: Value = serde_json::from_str(json)?;

    // A standalone error object short-circuits.
    if let Some(err) = value.get("error").and_then(Value::as_object) {
        let status = err.get("code").and_then(Value::as_u64).map_or(0, |c| c as u16);
        let message = err
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown provider error")
            .to_string();
        return Err(ModelError::Api { status, message });
    }

    let mut events = Vec::new();

    let candidate = value.get("candidates").and_then(Value::as_array).and_then(|c| c.first());

    if let Some(candidate) = candidate {
        if let Some(parts) =
            candidate.get("content").and_then(|c| c.get("parts")).and_then(Value::as_array)
        {
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        events.push(ModelStreamEvent::TextDelta(text.to_string()));
                    }
                } else if let Some(call) = part.get("functionCall").and_then(Value::as_object) {
                    let name =
                        call.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                    let arguments = call.get("args").cloned().unwrap_or(Value::Null);
                    events.push(ModelStreamEvent::ToolCall {
                        id: generate_call_id(&name),
                        name,
                        arguments,
                    });
                }
            }
        }
    }

    let finish_reason =
        candidate.and_then(|c| c.get("finishReason")).and_then(Value::as_str).map(str::to_string);

    let usage = value.get("usageMetadata").and_then(Value::as_object).map(|u| TokenUsage {
        input_tokens: u.get("promptTokenCount").and_then(Value::as_u64).unwrap_or(0),
        output_tokens: u.get("candidatesTokenCount").and_then(Value::as_u64).unwrap_or(0),
        total_tokens: u.get("totalTokenCount").and_then(Value::as_u64).unwrap_or(0),
    });

    if finish_reason.is_some() || usage.is_some() {
        events.push(ModelStreamEvent::Completed { usage, finish_reason });
    }

    Ok(events)
}

/// Generate a stable-ish call id for a Gemini function call.
///
/// Gemini does not return a call id, but the turn loop needs one to correlate
/// the tool result. We derive a monotonic, name-prefixed id.
fn generate_call_id(name: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("call_{name}_{n}")
}

// ---------------------------------------------------------------------------
// Gemini client.
// ---------------------------------------------------------------------------

/// Default Gemini REST base URL.
pub const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// A streaming Gemini `generateContent` client implementing [`ModelClient`].
#[derive(Debug, Clone)]
pub struct GeminiClient {
    /// The reqwest client (reused for connection pooling / low latency).
    pub http: reqwest::Client,
    /// Gemini API key.
    pub api_key: String,
    /// Target model id (e.g. `gemini-2.5-flash`).
    pub model: String,
    /// API base URL.
    pub base_url: String,
}

impl GeminiClient {
    /// Build a client for `model` authenticating with `api_key`.
    ///
    /// Uses the default public Gemini base URL; override [`GeminiClient::base_url`]
    /// for a proxy or staging endpoint.
    #[must_use]
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
            base_url: DEFAULT_GEMINI_BASE_URL.to_string(),
        }
    }

    /// Full `streamGenerateContent` URL for this client's model.
    #[must_use]
    fn stream_url(&self) -> String {
        build_stream_url(&self.base_url, &self.model, &self.api_key)
    }
}

/// Build the `streamGenerateContent` SSE URL (pure, network-free helper).
#[must_use]
pub fn build_stream_url(base_url: &str, model: &str, api_key: &str) -> String {
    format!(
        "{}/v1beta/models/{model}:streamGenerateContent?alt=sse&key={api_key}",
        base_url.trim_end_matches('/'),
    )
}

#[async_trait]
impl ModelClient for GeminiClient {
    async fn stream(&self, turn: ChatTurn) -> Result<ModelStream, ModelError> {
        let body = build_request_body(&turn);
        let url = self.stream_url();

        let resp = self.http.post(&url).json(&body).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ModelError::Api {
                status: status.as_u16(),
                message: if message.is_empty() { "request failed".to_string() } else { message },
            });
        }

        // Drive the SSE byte stream into a unified event stream.
        let byte_stream = resp.bytes_stream();
        let event_stream = sse_byte_stream_to_events(byte_stream);
        Ok(Box::pin(event_stream))
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

/// Convert a `reqwest` byte stream of SSE into a unified event stream.
///
/// `Created` is emitted first, then each decoded frame's events, then any
/// trailing buffered frame is flushed on stream end.
fn sse_byte_stream_to_events<S>(
    byte_stream: S,
) -> impl Stream<Item = Result<ModelStreamEvent, ModelError>> + Send
where
    S: Stream<Item = reqwest::Result<bytes::Bytes>> + Send + 'static,
{
    futures::stream::once(async { Ok(ModelStreamEvent::Created) }).chain(
        futures::stream::unfold(
            (Box::pin(byte_stream), String::new(), false),
            move |(mut stream, mut buffer, mut started)| async move {
                loop {
                    // Drain any complete events already buffered.
                    let frames = split_sse_events(&mut buffer);
                    if let Some(items) = frames_to_items(&frames) {
                        started = true;
                        return Some((items, (stream, buffer, started)));
                    }

                    match stream.next().await {
                        Some(Ok(chunk)) => match std::str::from_utf8(&chunk) {
                            Ok(text) => buffer.push_str(text),
                            Err(e) => {
                                return Some((
                                    vec![Err(ModelError::Stream(format!("invalid utf-8: {e}")))],
                                    (stream, buffer, started),
                                ));
                            }
                        },
                        Some(Err(e)) => {
                            return Some((
                                vec![Err(ModelError::Http(e))],
                                (stream, buffer, started),
                            ));
                        }
                        None => {
                            // Flush any final partial frame (Gemini may not send
                            // a trailing blank line on the last event).
                            let tail = std::mem::take(&mut buffer);
                            let trimmed = tail.trim();
                            if trimmed.is_empty() {
                                return None;
                            }
                            let final_frames = vec![trimmed.to_string()];
                            return frames_to_items(&final_frames)
                                .map(|items| (items, (stream, String::new(), started)));
                        }
                    }
                }
            },
        )
        // Flatten the per-poll batches into individual events.
        .map(futures::stream::iter)
        .flatten(),
    )
}

/// Decode a batch of raw SSE event blocks into result items.
///
/// Returns `None` when the batch produced nothing (so the caller keeps
/// polling), otherwise a non-empty vec of results.
fn frames_to_items(frames: &[String]) -> Option<Vec<Result<ModelStreamEvent, ModelError>>> {
    if frames.is_empty() {
        return None;
    }
    let mut items = Vec::new();
    for frame in frames {
        let data = sse_event_data(frame);
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        match parse_sse_frame(data) {
            Ok(events) => items.extend(events.into_iter().map(Ok)),
            Err(e) => items.push(Err(e)),
        }
    }
    if items.is_empty() { None } else { Some(items) }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn sample_turn() -> ChatTurn {
        ChatTurn {
            system: Some("You are a helpful coding agent.".to_string()),
            messages: vec![
                ChatMessage { role: Role::User, parts: vec![Part::Text("List files".to_string())] },
                ChatMessage {
                    role: Role::Model,
                    parts: vec![Part::FunctionCall {
                        name: "ls".to_string(),
                        args: json!({ "path": "." }),
                    }],
                },
                ChatMessage {
                    role: Role::Tool,
                    parts: vec![Part::FunctionResponse {
                        name: "ls".to_string(),
                        response: json!({ "files": ["a.rs", "b.rs"] }),
                    }],
                },
            ],
            tools: vec![ToolDecl {
                name: "ls".to_string(),
                description: "List directory entries".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }),
            }],
        }
    }

    #[test]
    fn build_request_body_matches_golden() {
        let got = build_request_body(&sample_turn());
        let want = json!({
            "systemInstruction": {
                "parts": [{ "text": "You are a helpful coding agent." }]
            },
            "contents": [
                { "role": "user", "parts": [{ "text": "List files" }] },
                {
                    "role": "model",
                    "parts": [{ "functionCall": { "name": "ls", "args": { "path": "." } } }]
                },
                {
                    "role": "user",
                    "parts": [{
                        "functionResponse": {
                            "name": "ls",
                            "response": { "files": ["a.rs", "b.rs"] }
                        }
                    }]
                }
            ],
            "tools": [{
                "function_declarations": [{
                    "name": "ls",
                    "description": "List directory entries",
                    "parameters": {
                        "type": "object",
                        "properties": { "path": { "type": "string" } },
                        "required": ["path"]
                    }
                }]
            }]
        });
        assert_eq!(got, want);
    }

    #[test]
    fn build_request_body_omits_optional_sections() {
        let turn = ChatTurn {
            system: None,
            messages: vec![ChatMessage {
                role: Role::User,
                parts: vec![Part::Text("hi".to_string())],
            }],
            tools: vec![],
        };
        let got = build_request_body(&turn);
        assert!(got.get("systemInstruction").is_none());
        assert!(got.get("tools").is_none());
        assert_eq!(got["contents"], json!([{ "role": "user", "parts": [{ "text": "hi" }] }]));
    }

    #[test]
    fn parse_text_frame_yields_text_delta() {
        let frame = r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"}}]}"#;
        let events = parse_sse_frame(frame).unwrap();
        assert_eq!(events, vec![ModelStreamEvent::TextDelta("Hello".to_string())]);
    }

    #[test]
    fn parse_function_call_frame_yields_tool_call() {
        let frame = r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"ls","args":{"path":"."}}}],"role":"model"}}]}"#;
        let events = parse_sse_frame(frame).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ModelStreamEvent::ToolCall { id, name, arguments } => {
                assert!(id.starts_with("call_ls_"));
                assert_eq!(name, "ls");
                assert_eq!(*arguments, json!({ "path": "." }));
            },
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn parse_final_frame_yields_completed_with_usage() {
        let frame = r#"{
            "candidates":[{"content":{"parts":[{"text":"done"}],"role":"model"},"finishReason":"STOP"}],
            "usageMetadata":{"promptTokenCount":12,"candidatesTokenCount":7,"totalTokenCount":19}
        }"#;
        let events = parse_sse_frame(frame).unwrap();
        assert_eq!(events, vec![
            ModelStreamEvent::TextDelta("done".to_string()),
            ModelStreamEvent::Completed {
                usage: Some(TokenUsage { input_tokens: 12, output_tokens: 7, total_tokens: 19 }),
                finish_reason: Some("STOP".to_string()),
            },
        ]);
    }

    #[test]
    fn parse_error_frame_yields_api_error() {
        let frame = r#"{"error":{"code":429,"message":"rate limited"}}"#;
        let err = parse_sse_frame(frame).unwrap_err();
        match err {
            ModelError::Api { status, message } => {
                assert_eq!(status, 429);
                assert_eq!(message, "rate limited");
            },
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn split_sse_events_keeps_partial_in_buffer() {
        let mut buffer = String::from("data: {\"a\":1}\n\ndata: {\"b\":2");
        let events = split_sse_events(&mut buffer);
        assert_eq!(events, vec!["data: {\"a\":1}".to_string()]);
        // The incomplete second event stays buffered.
        assert_eq!(buffer, "data: {\"b\":2");

        // Completing it now emits it.
        buffer.push_str("}\n\n");
        let events = split_sse_events(&mut buffer);
        assert_eq!(events, vec!["data: {\"b\":2}".to_string()]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn split_sse_events_handles_crlf_and_multiple() {
        let mut buffer = String::from("data: a\r\n\r\ndata: b\r\n\r\n");
        let events = split_sse_events(&mut buffer);
        assert_eq!(events, vec!["data: a".to_string(), "data: b".to_string()]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn sse_event_data_extracts_and_joins_data_lines() {
        let event = "event: message\ndata: line1\ndata: line2";
        assert_eq!(sse_event_data(event), "line1\nline2");
    }

    #[test]
    fn sse_event_data_strips_single_leading_space() {
        assert_eq!(sse_event_data("data: {\"x\":1}"), "{\"x\":1}");
        assert_eq!(sse_event_data("data:{\"x\":1}"), "{\"x\":1}");
    }

    #[test]
    fn empty_candidates_frame_yields_nothing() {
        let frame = r#"{"candidates":[]}"#;
        assert!(parse_sse_frame(frame).unwrap().is_empty());
    }

    #[test]
    fn frames_to_items_skips_done_sentinel() {
        let frames = vec!["data: [DONE]".to_string()];
        assert!(frames_to_items(&frames).is_none());
    }

    // Note: we test the pure URL builder directly rather than constructing a
    // `GeminiClient`, because `reqwest::Client::new()` panics with "No provider
    // set" under rustls 0.23 in a bare test process (CLAUDE.md §7). Tests stay
    // network-free and provider-free.
    #[test]
    fn build_stream_url_matches_gemini_endpoint() {
        let url = build_stream_url(DEFAULT_GEMINI_BASE_URL, "gemini-2.5-flash", "KEY123");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse&key=KEY123"
        );
    }

    #[test]
    fn build_stream_url_trims_trailing_slash() {
        let url = build_stream_url("https://proxy.local/", "m", "k");
        assert_eq!(url, "https://proxy.local/v1beta/models/m:streamGenerateContent?alt=sse&key=k");
    }
}
