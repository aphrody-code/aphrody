// SPDX-License-Identifier: Apache-2.0
//! Anthropic Messages BYOK adapter.
//!
//! Wire shape mirrors the upstream daemon route
//! `POST /api/proxy/anthropic/stream` (`apps/daemon/src/chat-routes.ts:695`):
//!
//! ```text
//! POST <base_url>/v1/messages
//! x-api-key: <api_key>
//! anthropic-version: 2023-06-01
//! Content-Type: application/json
//!
//! { "model": "<model>", "max_tokens": N, "stream": true,
//!   "system": "<optional>", "messages": [{role, content}, ...] }
//! ```
//!
//! Anthropic emits frame events keyed on `event:` lines:
//!
//! * `event: content_block_delta` carries `{delta: {type: "text_delta", text: "…"}}`
//!   → emitted as [`NormalizedChunk::ChatDelta`].
//! * `event: content_block_stop` for a `tool_use` block with the
//!   accumulated `input_json_delta` partials → emitted as
//!   [`NormalizedChunk::ToolUse`].
//! * `event: message_stop` → emitted as [`NormalizedChunk::Done`].
//! * `event: error` or `data.type == "error"` → emitted as
//!   [`NormalizedChunk::Error`] and the stream is closed.
//!
//! The SSE framing helpers and SSRF guard live in the parent module.

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use super::{
    ByokRequest, ByokStream, ChunkStream, NormalizedChunk, SseFrame, check_url_ssrf, ingest_bytes,
};
use crate::GatewayError;

/// Anthropic BYOK adapter.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AnthropicByok {
    #[serde(skip)]
    client: Option<Client>,
}

impl AnthropicByok {
    /// Constructs a new adapter with a fresh HTTP client.
    pub fn new() -> Self {
        Self { client: Some(Client::new()) }
    }

    fn http(&self) -> Client {
        self.client.clone().unwrap_or_default()
    }
}

#[async_trait]
impl ByokStream for AnthropicByok {
    async fn stream(&self, req: ByokRequest) -> Result<ChunkStream, GatewayError> {
        let parsed = Url::parse(&req.base_url)?;
        check_url_ssrf(&parsed)?;
        let url = append_versioned_path(&parsed, "/messages")?;

        let mut payload = serde_json::json!({
            "model": req.model,
            "max_tokens": req.max_tokens.unwrap_or(8192),
            "stream": true,
            "messages": req.messages,
        });
        if let Some(system) = req.system_prompt {
            payload["system"] = Value::String(system);
        }

        let resp = self
            .http()
            .post(url)
            .header("x-api-key", &req.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::UpstreamStatus { status: status.as_u16(), body: text });
        }

        Ok(Box::pin(decode_stream(resp.bytes_stream())))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn append_versioned_path(base: &Url, suffix: &str) -> Result<Url, GatewayError> {
    let trimmed = base.path().trim_end_matches('/');
    let needs_version =
        !trimmed.split('/').any(|seg| seg.starts_with('v') && seg[1..].chars().all(|c| c.is_ascii_digit()) && seg.len() > 1);
    let new_path = if needs_version {
        format!("{trimmed}/v1{suffix}")
    } else {
        format!("{trimmed}{suffix}")
    };
    let mut out = base.clone();
    out.set_path(&new_path);
    Ok(out)
}

fn decode_stream(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> impl Stream<Item = Result<NormalizedChunk, GatewayError>> + Send {
    let mut buf = String::new();
    // Tool-use input accumulator keyed by content-block `index` (Anthropic streams
    // `input_json_delta` partials that we concatenate then emit at content_block_stop).
    let mut tool_inputs: std::collections::HashMap<u64, ToolBlockState> =
        std::collections::HashMap::new();
    let mut done = false;

    stream.flat_map(move |result| {
        if done {
            return futures::stream::iter(Vec::new());
        }
        let mut out: Vec<Result<NormalizedChunk, GatewayError>> = Vec::new();
        match result {
            Err(e) => {
                out.push(Err(GatewayError::Transport(e)));
                done = true;
            },
            Ok(bytes) => {
                let frames = ingest_bytes(&mut buf, &bytes);
                for frame in frames {
                    match handle_frame(&frame, &mut tool_inputs) {
                        FrameOutcome::Emit(chunks) => {
                            for c in chunks {
                                out.push(Ok(c));
                            }
                        },
                        FrameOutcome::Terminate(chunk) => {
                            out.push(Ok(chunk));
                            done = true;
                            break;
                        },
                        FrameOutcome::Skip => {},
                    }
                }
            },
        }
        futures::stream::iter(out)
    })
}

#[derive(Default, Debug)]
struct ToolBlockState {
    name: String,
    id: String,
    input: String,
}

enum FrameOutcome {
    Skip,
    Emit(Vec<NormalizedChunk>),
    Terminate(NormalizedChunk),
}

fn handle_frame(
    frame: &SseFrame,
    tools: &mut std::collections::HashMap<u64, ToolBlockState>,
) -> FrameOutcome {
    if frame.data.is_empty() {
        return FrameOutcome::Skip;
    }
    let parsed: Value = match serde_json::from_str(&frame.data) {
        Ok(v) => v,
        Err(_) => return FrameOutcome::Skip,
    };

    let event = frame.event.as_str();
    let event_type = parsed.get("type").and_then(Value::as_str).unwrap_or("");

    if event == "error" || event_type == "error" {
        let msg = parsed
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .or_else(|| parsed.get("message").and_then(Value::as_str))
            .unwrap_or("Anthropic upstream error");
        return FrameOutcome::Terminate(NormalizedChunk::Error(msg.to_owned()));
    }

    if event == "content_block_start" {
        if let (Some(idx), Some(block)) =
            (parsed.get("index").and_then(Value::as_u64), parsed.get("content_block"))
        {
            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                tools.insert(
                    idx,
                    ToolBlockState {
                        name: block
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned(),
                        id: block.get("id").and_then(Value::as_str).unwrap_or("").to_owned(),
                        input: String::new(),
                    },
                );
            }
        }
        return FrameOutcome::Skip;
    }

    if event == "content_block_delta" {
        if let Some(delta) = parsed.get("delta") {
            let dt = delta.get("type").and_then(Value::as_str).unwrap_or("");
            if dt == "text_delta" {
                if let Some(text) = delta.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        return FrameOutcome::Emit(vec![NormalizedChunk::ChatDelta(
                            text.to_owned(),
                        )]);
                    }
                }
            } else if dt == "input_json_delta" {
                if let (Some(idx), Some(partial)) = (
                    parsed.get("index").and_then(Value::as_u64),
                    delta.get("partial_json").and_then(Value::as_str),
                ) {
                    if let Some(state) = tools.get_mut(&idx) {
                        state.input.push_str(partial);
                    }
                }
            }
        }
        return FrameOutcome::Skip;
    }

    if event == "content_block_stop" {
        if let Some(idx) = parsed.get("index").and_then(Value::as_u64) {
            if let Some(state) = tools.remove(&idx) {
                if !state.input.trim().is_empty() {
                    return FrameOutcome::Emit(vec![NormalizedChunk::ToolUse {
                        name: state.name,
                        args: state.input,
                    }]);
                }
                // Tool with no streamed input — still surface the name so the
                // caller knows a tool was invoked, with empty args object.
                let _ = state.id;
                return FrameOutcome::Emit(vec![NormalizedChunk::ToolUse {
                    name: state.name,
                    args: "{}".to_owned(),
                }]);
            }
        }
        return FrameOutcome::Skip;
    }

    if event == "message_stop" {
        return FrameOutcome::Terminate(NormalizedChunk::Done);
    }

    FrameOutcome::Skip
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byok::parse_sse_frames;

    #[test]
    fn append_versioned_path_adds_v1_when_missing() {
        let base = Url::parse("https://api.anthropic.com").unwrap();
        let out = append_versioned_path(&base, "/messages").unwrap();
        assert_eq!(out.as_str(), "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn append_versioned_path_preserves_existing_v1() {
        let base = Url::parse("https://api.anthropic.com/v1").unwrap();
        let out = append_versioned_path(&base, "/messages").unwrap();
        assert_eq!(out.as_str(), "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn parse_anthropic_content_block_delta_emits_text() {
        let frame = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n";
        let mut buf = String::from(frame);
        let frames = parse_sse_frames(&mut buf);
        let mut tools = std::collections::HashMap::new();
        let outcome = handle_frame(&frames[0], &mut tools);
        let chunks = match outcome {
            FrameOutcome::Emit(c) => c,
            _ => panic!("expected emit"),
        };
        assert_eq!(chunks, vec![NormalizedChunk::ChatDelta("hello".to_owned())]);
    }

    #[test]
    fn parse_anthropic_message_stop_terminates() {
        let frame = "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let mut buf = String::from(frame);
        let frames = parse_sse_frames(&mut buf);
        let mut tools = std::collections::HashMap::new();
        let outcome = handle_frame(&frames[0], &mut tools);
        assert!(matches!(outcome, FrameOutcome::Terminate(NormalizedChunk::Done)));
    }

    #[test]
    fn parse_anthropic_error_event_terminates() {
        let frame =
            "event: error\ndata: {\"type\":\"error\",\"error\":{\"message\":\"bad key\"}}\n\n";
        let mut buf = String::from(frame);
        let frames = parse_sse_frames(&mut buf);
        let mut tools = std::collections::HashMap::new();
        let outcome = handle_frame(&frames[0], &mut tools);
        match outcome {
            FrameOutcome::Terminate(NormalizedChunk::Error(m)) => assert_eq!(m, "bad key"),
            _ => panic!("expected error terminate"),
        }
    }

    #[test]
    fn parse_anthropic_tool_use_accumulates_and_emits_on_stop() {
        let sse = "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"tool_1\",\"name\":\"calculator\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"x\\\":\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"42}\"}}\n\n\
event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        let mut tools = std::collections::HashMap::new();
        let mut emitted: Vec<NormalizedChunk> = Vec::new();
        for f in &frames {
            match handle_frame(f, &mut tools) {
                FrameOutcome::Emit(chunks) => emitted.extend(chunks),
                FrameOutcome::Terminate(c) => {
                    emitted.push(c);
                    break;
                },
                FrameOutcome::Skip => {},
            }
        }
        assert_eq!(
            emitted,
            vec![NormalizedChunk::ToolUse {
                name: "calculator".to_owned(),
                args: "{\"x\":42}".to_owned()
            }]
        );
    }
}
