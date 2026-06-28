// SPDX-License-Identifier: Apache-2.0
//! `LocalOpenAiClient` — a [`ModelClient`] over any **OpenAI-compatible**
//! `/v1/chat/completions` endpoint (Ollama, llama.cpp `llama-server`, vLLM, or
//! aphrody's own `aphrody serve`).
//!
//! This is the **keystone** of the local JARVIS loop (`docs/JARVIS.md` §8): the
//! only other [`ModelClient`] is [`GeminiClient`](crate::GeminiClient), which is
//! cloud-bound. This client lets the entire `aphrody-engine` agentic turn loop
//! (tool dispatch, sessions, autonomy, rollouts) run against a local open-weight
//! model with **no cloud key**.
//!
//! ## Wire mapping
//!
//! Outbound, a neutral [`ChatTurn`] becomes an OpenAI chat request:
//! - [`ChatTurn::system`] → a leading `{"role":"system"}` message;
//! - [`Role::User`] → `{"role":"user","content": <text>}`;
//! - [`Role::Model`] → `{"role":"assistant","content": <text|null>,"tool_calls":[…]}`,
//!   each [`Part::FunctionCall`] becoming a `tool_calls[]` entry with a
//!   synthesized, order-stable `id`;
//! - [`Role::Tool`] → one `{"role":"tool","tool_call_id": <id>,"content": …}`
//!   per [`Part::FunctionResponse`], correlated to the matching assistant call
//!   by function name in FIFO order (the neutral [`Part`] model carries no id,
//!   so we reconstruct the linkage OpenAI requires);
//! - [`ChatTurn::tools`] → `tools:[{"type":"function","function":{…}}]`.
//!
//! Inbound, the streamed `chat.completion.chunk` SSE is folded into unified
//! [`ModelStreamEvent`]s. Unlike Gemini (which delivers a whole `functionCall`
//! object per frame), OpenAI streams tool-call **`arguments` as string
//! fragments across many chunks**, so this client accumulates them by
//! `tool_calls[].index` and emits a single [`ModelStreamEvent::ToolCall`] per
//! call once `finish_reason` arrives.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::{Value, json};

use crate::{
    ChatTurn, ModelClient, ModelError, ModelStream, ModelStreamEvent, Part, Role, TokenUsage,
    split_sse_events, sse_event_data,
};

/// Default local backend: Ollama's native OpenAI-compatible surface.
///
/// Ollama (`ollama serve`) exposes `/v1/chat/completions` on `:11434`. Point a
/// client at `aphrody serve` (`http://127.0.0.1:8080/v1`), `llama-server`
/// (`:8080/v1`), or vLLM (`:8000/v1`) by overriding the base URL. The base URL
/// **includes** the `/v1` prefix; `/chat/completions` is appended.
pub const DEFAULT_LOCAL_BASE_URL: &str = "http://127.0.0.1:11434/v1";

/// A streaming client for any OpenAI-compatible `/v1/chat/completions` endpoint.
#[derive(Debug, Clone)]
pub struct LocalOpenAiClient {
    /// The reqwest client (reused for connection pooling / low latency).
    pub http: reqwest::Client,
    /// API base URL, including the `/v1` segment (e.g. `http://127.0.0.1:11434/v1`).
    pub base_url: String,
    /// Target model id (e.g. `qwen3`, `gemma4:12b`).
    pub model: String,
    /// Bearer token. Local backends ignore it; sent only when non-empty
    /// (Ollama's convention is the literal `ollama`).
    pub api_key: String,
}

impl LocalOpenAiClient {
    /// Build a client for `model` at `base_url`, authenticating with `api_key`.
    ///
    /// `base_url` should include the `/v1` prefix
    /// ([`DEFAULT_LOCAL_BASE_URL`] is the Ollama default). Pass an empty
    /// `api_key` to omit the `Authorization` header entirely.
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
            api_key: api_key.into(),
        }
    }

    /// Full `/chat/completions` URL for this client's base.
    #[must_use]
    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl ModelClient for LocalOpenAiClient {
    async fn stream(&self, turn: ChatTurn) -> Result<ModelStream, ModelError> {
        let body = build_openai_request_body(&turn, &self.model, true);

        let mut req = self.http.post(self.endpoint()).json(&body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        let resp = req.send().await?;

        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ModelError::Api {
                status: status.as_u16(),
                message: if message.is_empty() {
                    "request failed".to_string()
                } else {
                    message
                },
            });
        }

        let byte_stream = resp.bytes_stream();
        Ok(Box::pin(openai_byte_stream_to_events(byte_stream)))
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

// ---------------------------------------------------------------------------
// Request building (pure, offline-testable).
// ---------------------------------------------------------------------------

/// Build an OpenAI `chat/completions` request body from a neutral [`ChatTurn`].
///
/// See the module docs for the full mapping. The tool-call/tool-response id
/// linkage OpenAI mandates is reconstructed by walking the messages in order and
/// matching each [`Part::FunctionResponse`] to the earliest unconsumed assistant
/// call with the same function name.
#[must_use]
pub fn build_openai_request_body(turn: &ChatTurn, model: &str, stream: bool) -> Value {
    let mut messages: Vec<Value> = Vec::new();

    if let Some(system) = &turn.system {
        messages.push(json!({ "role": "system", "content": system }));
    }

    // FIFO of synthesized call ids per function name, so a later tool response
    // can be correlated back to its assistant `tool_calls[].id`.
    let mut pending: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut seq: u64 = 0;

    for msg in &turn.messages {
        match msg.role {
            Role::User => {
                messages.push(json!({
                    "role": "user",
                    "content": collect_text(&msg.parts),
                }));
            }
            Role::Model => {
                let text = collect_text(&msg.parts);
                let mut tool_calls: Vec<Value> = Vec::new();
                for part in &msg.parts {
                    if let Part::FunctionCall { name, args } = part {
                        let id = format!("call_{name}_{seq}");
                        seq += 1;
                        pending.entry(name.clone()).or_default().push_back(id.clone());
                        tool_calls.push(json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": serde_json::to_string(args)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            },
                        }));
                    }
                }

                let mut m = serde_json::Map::new();
                m.insert("role".to_string(), json!("assistant"));
                if tool_calls.is_empty() {
                    m.insert("content".to_string(), json!(text));
                } else {
                    // OpenAI uses null content for a pure tool-call turn.
                    m.insert(
                        "content".to_string(),
                        if text.is_empty() { Value::Null } else { json!(text) },
                    );
                    m.insert("tool_calls".to_string(), Value::Array(tool_calls));
                }
                messages.push(Value::Object(m));
            }
            Role::Tool => {
                for part in &msg.parts {
                    if let Part::FunctionResponse { name, response } = part {
                        let id = pending
                            .get_mut(name)
                            .and_then(VecDeque::pop_front)
                            .unwrap_or_else(|| format!("call_{name}_0"));
                        let content = match response {
                            Value::String(s) => s.clone(),
                            other => serde_json::to_string(other).unwrap_or_default(),
                        };
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": id,
                            "content": content,
                        }));
                    }
                }
            }
        }
    }

    let mut body = serde_json::Map::new();
    body.insert("model".to_string(), json!(model));
    body.insert("messages".to_string(), Value::Array(messages));
    body.insert("stream".to_string(), json!(stream));
    if stream {
        // Ask compliant servers to emit a final usage frame; lenient servers
        // (Ollama) ignore unknown fields harmlessly.
        body.insert(
            "stream_options".to_string(),
            json!({ "include_usage": true }),
        );
    }

    if !turn.tools.is_empty() {
        let tools: Vec<Value> = turn
            .tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    },
                })
            })
            .collect();
        body.insert("tools".to_string(), Value::Array(tools));
    }

    Value::Object(body)
}

/// Concatenate the [`Part::Text`] fragments of a message into a single string.
fn collect_text(parts: &[Part]) -> String {
    let mut out = String::new();
    for part in parts {
        if let Part::Text(text) = part {
            out.push_str(text);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Streaming decode (stateful: accumulates fragmented tool-call arguments).
// ---------------------------------------------------------------------------

/// Per-call accumulator for a streamed OpenAI tool call.
#[derive(Debug, Default, Clone)]
struct ToolAccum {
    id: String,
    name: String,
    args: String,
}

/// Running state while decoding one OpenAI chat stream.
#[derive(Debug, Default)]
struct OpenAiStreamState {
    /// Tool calls keyed by their `tool_calls[].index`, accumulated until flush.
    tools: BTreeMap<u64, ToolAccum>,
    /// Latest token usage seen (OpenAI sends it in a trailing frame).
    usage: Option<TokenUsage>,
    /// Finish reason once the model stops.
    finish_reason: Option<String>,
    /// Whether the accumulated tool calls have already been emitted.
    tools_flushed: bool,
}

/// Generate a stable-ish call id when the server omits one (some do in streaming).
fn generate_call_id(name: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("call_{name}_{n}")
}

/// Parse a tool-call argument buffer into a JSON object (lenient).
fn parse_tool_args(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return json!({});
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| json!({}))
}

/// Drain the accumulated tool calls into [`ModelStreamEvent::ToolCall`]s.
fn flush_tools(state: &mut OpenAiStreamState) -> Vec<ModelStreamEvent> {
    let mut out = Vec::new();
    for (_, tool) in std::mem::take(&mut state.tools) {
        let arguments = parse_tool_args(&tool.args);
        let id = if tool.id.is_empty() {
            generate_call_id(&tool.name)
        } else {
            tool.id
        };
        out.push(ModelStreamEvent::ToolCall {
            id,
            name: tool.name,
            arguments,
        });
    }
    state.tools_flushed = true;
    out
}

/// Emit the terminal events for a finished stream: any not-yet-flushed tool
/// calls, then a single [`ModelStreamEvent::Completed`].
fn finalize(state: &mut OpenAiStreamState) -> Vec<Result<ModelStreamEvent, ModelError>> {
    let mut out: Vec<Result<ModelStreamEvent, ModelError>> = Vec::new();
    if !state.tools_flushed && !state.tools.is_empty() {
        out.extend(flush_tools(state).into_iter().map(Ok));
    }
    out.push(Ok(ModelStreamEvent::Completed {
        usage: state.usage.clone(),
        finish_reason: state.finish_reason.clone(),
    }));
    out
}

/// Fold one `chat.completion.chunk` JSON payload into unified events, mutating
/// `state`. Text and reasoning deltas are emitted immediately; tool calls are
/// accumulated and flushed when `finish_reason` arrives.
fn ingest_openai_chunk(
    state: &mut OpenAiStreamState,
    json: &str,
) -> Result<Vec<ModelStreamEvent>, ModelError> {
    let value: Value = serde_json::from_str(json)?;

    // A standalone error object/string short-circuits.
    if let Some(err) = value.get("error") {
        let (status, message) = match err {
            Value::Object(o) => (
                o.get("code").and_then(Value::as_u64).map_or(0, |c| c as u16),
                o.get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown provider error")
                    .to_string(),
            ),
            Value::String(s) => (0, s.clone()),
            _ => (0, "unknown provider error".to_string()),
        };
        return Err(ModelError::Api { status, message });
    }

    let mut events = Vec::new();

    if let Some(usage) = value.get("usage").and_then(Value::as_object) {
        state.usage = Some(TokenUsage {
            input_tokens: usage.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(0),
            output_tokens: usage
                .get("completion_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            total_tokens: usage.get("total_tokens").and_then(Value::as_u64).unwrap_or(0),
        });
    }

    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|c| c.first());

    if let Some(choice) = choice {
        if let Some(delta) = choice.get("delta") {
            if let Some(text) = delta.get("content").and_then(Value::as_str) {
                if !text.is_empty() {
                    events.push(ModelStreamEvent::TextDelta(text.to_string()));
                }
            }
            // Reasoning channel: deepseek-r1 (`reasoning_content`) and others.
            for key in ["reasoning_content", "reasoning"] {
                if let Some(r) = delta.get(key).and_then(Value::as_str) {
                    if !r.is_empty() {
                        events.push(ModelStreamEvent::ReasoningDelta(r.to_string()));
                    }
                }
            }
            if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
                for tc in tool_calls {
                    let idx = tc.get("index").and_then(Value::as_u64).unwrap_or(0);
                    let entry = state.tools.entry(idx).or_default();
                    if let Some(id) = tc.get("id").and_then(Value::as_str) {
                        if !id.is_empty() {
                            entry.id = id.to_string();
                        }
                    }
                    if let Some(func) = tc.get("function") {
                        if let Some(name) = func.get("name").and_then(Value::as_str) {
                            entry.name.push_str(name);
                        }
                        if let Some(args) = func.get("arguments").and_then(Value::as_str) {
                            entry.args.push_str(args);
                        }
                    }
                }
            }
        }

        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            if !reason.is_empty() {
                state.finish_reason = Some(reason.to_string());
                if !state.tools_flushed && !state.tools.is_empty() {
                    events.extend(flush_tools(state));
                }
            }
        }
    }

    Ok(events)
}

/// Convert a `reqwest` SSE byte stream of `chat.completion.chunk`s into a
/// unified [`ModelStreamEvent`] stream.
///
/// `Created` is emitted first; text/reasoning deltas and tool calls follow as
/// they decode; a single `Completed` (carrying usage + finish reason) is emitted
/// on `[DONE]` or stream close.
fn openai_byte_stream_to_events<S>(
    byte_stream: S,
) -> impl Stream<Item = Result<ModelStreamEvent, ModelError>> + Send
where
    S: Stream<Item = reqwest::Result<bytes::Bytes>> + Send + 'static,
{
    futures::stream::once(async { Ok(ModelStreamEvent::Created) }).chain(
        futures::stream::unfold(
            (
                Box::pin(byte_stream),
                String::new(),
                OpenAiStreamState::default(),
                false,
            ),
            move |(mut stream, mut buffer, mut state, finished)| async move {
                if finished {
                    return None;
                }
                loop {
                    // Decode any complete SSE frames already buffered.
                    let frames = split_sse_events(&mut buffer);
                    if !frames.is_empty() {
                        let mut out: Vec<Result<ModelStreamEvent, ModelError>> = Vec::new();
                        let mut saw_done = false;
                        for frame in &frames {
                            let data = sse_event_data(frame);
                            let data = data.trim();
                            if data.is_empty() {
                                continue;
                            }
                            if data == "[DONE]" {
                                saw_done = true;
                                break;
                            }
                            match ingest_openai_chunk(&mut state, data) {
                                Ok(evs) => out.extend(evs.into_iter().map(Ok)),
                                Err(e) => out.push(Err(e)),
                            }
                        }
                        if saw_done {
                            out.extend(finalize(&mut state));
                            return Some((out, (stream, buffer, state, true)));
                        }
                        if !out.is_empty() {
                            return Some((out, (stream, buffer, state, false)));
                        }
                        // Frames decoded to nothing visible; read more bytes.
                    }

                    match stream.next().await {
                        Some(Ok(chunk)) => match std::str::from_utf8(&chunk) {
                            Ok(text) => buffer.push_str(text),
                            Err(e) => {
                                return Some((
                                    vec![Err(ModelError::Stream(format!("invalid utf-8: {e}")))],
                                    (stream, buffer, state, false),
                                ));
                            }
                        },
                        Some(Err(e)) => {
                            return Some((
                                vec![Err(ModelError::Http(e))],
                                (stream, buffer, state, false),
                            ));
                        }
                        None => {
                            // Stream closed without an explicit `[DONE]`.
                            let final_events = finalize(&mut state);
                            return Some((final_events, (stream, buffer, state, true)));
                        }
                    }
                }
            },
        )
        .map(futures::stream::iter)
        .flatten(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatMessage, ToolDecl};
    use pretty_assertions::assert_eq;

    fn sample_turn() -> ChatTurn {
        ChatTurn {
            system: Some("You are JARVIS.".to_string()),
            messages: vec![
                ChatMessage {
                    role: Role::User,
                    parts: vec![Part::Text("List files".to_string())],
                },
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
                        response: json!({ "files": ["a.rs"] }),
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
    fn build_request_body_maps_roles_tools_and_links_ids() {
        let body = build_openai_request_body(&sample_turn(), "qwen3", true);

        assert_eq!(body["model"], json!("qwen3"));
        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["stream_options"], json!({ "include_usage": true }));

        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0], json!({ "role": "system", "content": "You are JARVIS." }));
        assert_eq!(messages[1], json!({ "role": "user", "content": "List files" }));

        // Assistant turn: null content + one tool call with a synthesized id.
        let assistant = &messages[2];
        assert_eq!(assistant["role"], json!("assistant"));
        assert_eq!(assistant["content"], Value::Null);
        let call = &assistant["tool_calls"][0];
        assert_eq!(call["type"], json!("function"));
        assert_eq!(call["function"]["name"], json!("ls"));
        // Arguments are a JSON *string*, per the OpenAI schema.
        assert_eq!(call["function"]["arguments"], json!("{\"path\":\".\"}"));
        let call_id = call["id"].as_str().unwrap().to_string();

        // Tool response is linked back to the same id.
        let tool = &messages[3];
        assert_eq!(tool["role"], json!("tool"));
        assert_eq!(tool["tool_call_id"], json!(call_id));
        assert_eq!(tool["content"], json!("{\"files\":[\"a.rs\"]}"));

        // Tools declared in OpenAI function shape (no Gemini sanitization).
        let decl = &body["tools"][0];
        assert_eq!(decl["type"], json!("function"));
        assert_eq!(decl["function"]["name"], json!("ls"));
        assert_eq!(decl["function"]["parameters"]["required"], json!(["path"]));
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
        let body = build_openai_request_body(&turn, "m", false);
        assert!(body.get("tools").is_none());
        assert!(body.get("stream_options").is_none());
        assert_eq!(body["stream"], json!(false));
        assert_eq!(
            body["messages"],
            json!([{ "role": "user", "content": "hi" }])
        );
    }

    #[test]
    fn ingest_text_delta() {
        let mut state = OpenAiStreamState::default();
        let chunk = r#"{"choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let events = ingest_openai_chunk(&mut state, chunk).unwrap();
        assert_eq!(events, vec![ModelStreamEvent::TextDelta("Hello".to_string())]);
    }

    #[test]
    fn ingest_reasoning_delta() {
        let mut state = OpenAiStreamState::default();
        let chunk = r#"{"choices":[{"index":0,"delta":{"reasoning_content":"hmm"}}]}"#;
        let events = ingest_openai_chunk(&mut state, chunk).unwrap();
        assert_eq!(events, vec![ModelStreamEvent::ReasoningDelta("hmm".to_string())]);
    }

    #[test]
    fn ingest_accumulates_fragmented_tool_call_then_flushes_on_finish() {
        let mut state = OpenAiStreamState::default();
        // Fragment 1: id + name + start of arguments.
        let c1 = r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"ls","arguments":"{\"path\":"}}]}}]}"#;
        assert!(ingest_openai_chunk(&mut state, c1).unwrap().is_empty());
        // Fragment 2: rest of arguments.
        let c2 = r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\".\"}"}}]}}]}"#;
        assert!(ingest_openai_chunk(&mut state, c2).unwrap().is_empty());
        // Finish: flush the assembled call.
        let c3 = r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#;
        let events = ingest_openai_chunk(&mut state, c3).unwrap();
        assert_eq!(
            events,
            vec![ModelStreamEvent::ToolCall {
                id: "call_abc".to_string(),
                name: "ls".to_string(),
                arguments: json!({ "path": "." }),
            }]
        );
        assert!(state.tools_flushed);
    }

    #[test]
    fn ingest_captures_usage_frame() {
        let mut state = OpenAiStreamState::default();
        let chunk = r#"{"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":7,"total_tokens":19}}"#;
        assert!(ingest_openai_chunk(&mut state, chunk).unwrap().is_empty());
        assert_eq!(
            state.usage,
            Some(TokenUsage { input_tokens: 12, output_tokens: 7, total_tokens: 19 })
        );
    }

    #[test]
    fn ingest_error_chunk_yields_api_error() {
        let mut state = OpenAiStreamState::default();
        let chunk = r#"{"error":{"code":404,"message":"model not found"}}"#;
        let err = ingest_openai_chunk(&mut state, chunk).unwrap_err();
        match err {
            ModelError::Api { status, message } => {
                assert_eq!(status, 404);
                assert_eq!(message, "model not found");
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    /// End-to-end decode of a realistic streamed completion built from raw SSE
    /// bytes — proves frame splitting, delta order, and the terminal `Completed`.
    #[tokio::test]
    async fn full_stream_decodes_text_then_completed() {
        let sse = concat!(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"}}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"}}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" there\"}}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":2,\"total_tokens\":5}}\n\n",
            "data: [DONE]\n\n",
        );
        let byte_stream = futures::stream::iter(vec![Ok::<_, reqwest::Error>(
            bytes::Bytes::from(sse),
        )]);
        let events: Vec<_> = openai_byte_stream_to_events(byte_stream)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(Result::unwrap)
            .collect();

        assert_eq!(
            events,
            vec![
                ModelStreamEvent::Created,
                ModelStreamEvent::TextDelta("Hi".to_string()),
                ModelStreamEvent::TextDelta(" there".to_string()),
                ModelStreamEvent::Completed {
                    usage: Some(TokenUsage {
                        input_tokens: 3,
                        output_tokens: 2,
                        total_tokens: 5,
                    }),
                    finish_reason: Some("stop".to_string()),
                },
            ]
        );
    }
}
