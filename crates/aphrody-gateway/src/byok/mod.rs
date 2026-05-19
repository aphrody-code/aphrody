// SPDX-License-Identifier: Apache-2.0
//! BYOK ("Bring Your Own Key") streaming proxy for the three canonical
//! providers shipped today: Anthropic, Google Gemini, and Antigravity.
//!
//! Each adapter normalises its upstream SSE format into the same
//! [`NormalizedChunk`] enum so callers downstream — desktop UI, terminal
//! renderer, automation scripts — get one wire format regardless of which
//! key the user pasted.
//!
//! The module deliberately keeps the public surface tiny:
//!
//! * [`Provider`] — enum naming the three supported providers.
//! * [`ChatMessage`] — provider-agnostic conversation turn.
//! * [`ByokRequest`] — what the user types into Settings ("base URL +
//!   API key + model") plus the actual message list.
//! * [`NormalizedChunk`] — the unified streaming event delivered to callers.
//! * [`ByokStream`] — the async trait every concrete adapter implements.
//! * [`ssrf::check_url_ssrf`] (re-exported at crate root) — the SSRF guard
//!   that **every** adapter calls *before* it issues an upstream `fetch`.
//!
//! The SSE framing primitives below are shared by all three adapters and
//! mirror the upstream daemon helpers in `apps/daemon/src/chat-routes.ts`.

use std::pin::Pin;

use bytes::Bytes;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::GatewayError;

pub mod anthropic;
pub mod antigravity;
pub mod gemini;
pub mod ssrf;

pub use anthropic::AnthropicByok;
pub use antigravity::AntigravityByok;
pub use gemini::GeminiByok;
pub use ssrf::check_url_ssrf;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The three canonical BYOK providers supported by the proxy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Anthropic Messages API (`POST /v1/messages`, SSE `event:`/`data:`).
    Anthropic,
    /// Google Gemini (Generative Language) `streamGenerateContent?alt=sse`.
    Gemini,
    /// Antigravity — Google Cloud Code Assist / Vertex variant that speaks
    /// the OpenAI-compatible Chat Completions wire format with a Bearer
    /// OAuth token.
    Antigravity,
}

impl Provider {
    /// Returns the kebab-case slug used in upstream routes (`/api/proxy/<slug>/stream`).
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Antigravity => "antigravity",
        }
    }
}

/// A single turn in a BYOK conversation.
///
/// The shape is intentionally OpenAI-compatible (`role` / `content`); the
/// per-provider adapters re-map this into the provider-specific payload
/// (Anthropic's `messages`, Gemini's `contents[].parts[]`, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// One of `"system"`, `"user"`, `"assistant"`, or `"tool"`.
    pub role: String,
    /// The raw text content of the turn.
    pub content: String,
}

impl ChatMessage {
    /// Constructs a user-authored turn.
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".to_owned(), content: content.into() }
    }

    /// Constructs an assistant-authored turn.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".to_owned(), content: content.into() }
    }

    /// Constructs a system-level instruction turn.
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".to_owned(), content: content.into() }
    }
}

/// What the user pastes into Settings, plus the conversation history to
/// stream against the upstream.
///
/// The `base_url` is validated by [`ssrf::check_url_ssrf`] **before** the
/// adapter issues any network call.  Adapters are free to additionally
/// append `/v1/<route>` when the URL doesn't already include a versioned
/// segment — see each adapter's docs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByokRequest {
    /// The user-supplied base URL (e.g. `https://api.anthropic.com`,
    /// `https://generativelanguage.googleapis.com`, or the Antigravity
    /// Cloud Code Assist endpoint).
    pub base_url: String,
    /// The user-supplied API key (Anthropic / Gemini) or Bearer OAuth
    /// access token (Antigravity).
    pub api_key: String,
    /// Model / deployment identifier.
    pub model: String,
    /// Whether to stream tokens (always honoured by every adapter — the
    /// field exists so callers can plumb their UI flag through unchanged).
    #[serde(default = "default_stream")]
    pub stream: bool,
    /// Optional system instruction prepended ahead of `messages` per provider rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Ordered conversation history.
    pub messages: Vec<ChatMessage>,
    /// Maximum tokens the upstream may emit; `None` defers to the provider default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

const fn default_stream() -> bool {
    true
}

impl ByokRequest {
    /// Constructs a minimal streaming request with no system prompt and no token cap.
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        messages: Vec<ChatMessage>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            stream: true,
            system_prompt: None,
            messages,
            max_tokens: None,
        }
    }
}

/// A single normalised streaming event delivered to callers regardless of
/// upstream provider.
///
/// Every adapter is expected to emit `ChatDelta` for assistant text, optional
/// `ToolUse` for parsed tool invocations, exactly one terminating `Done` per
/// successful stream, or one `Error` followed by stream closure on upstream
/// failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NormalizedChunk {
    /// Incremental assistant text.
    ChatDelta(String),
    /// A tool invocation with parsed input arguments (JSON serialised back to text).
    ToolUse {
        /// Tool name as reported by the provider.
        name: String,
        /// Tool arguments (raw JSON string — callers parse as they wish).
        args: String,
    },
    /// The stream has reached its natural terminus (`[DONE]`, `message_stop`,
    /// or `finish_reason: stop`).
    Done,
    /// The upstream returned an error after the stream opened.  The string
    /// carries a human-readable, secret-redacted message.
    Error(String),
}

/// Boxed, pinned stream of normalised chunks.
pub type ChunkStream =
    Pin<Box<dyn Stream<Item = Result<NormalizedChunk, GatewayError>> + Send>>;

/// The contract every BYOK adapter implements.
#[async_trait::async_trait]
pub trait ByokStream: Send + Sync {
    /// Validates [`ByokRequest::base_url`] with [`check_url_ssrf`], issues the
    /// upstream POST, and returns a stream of normalised chunks.
    ///
    /// Adapters that fail the SSRF check return `Err(GatewayError::Config)`
    /// *before* opening the connection.
    async fn stream(&self, req: ByokRequest) -> Result<ChunkStream, GatewayError>;
}

// ---------------------------------------------------------------------------
// Shared SSE framing helpers
// ---------------------------------------------------------------------------

/// One parsed SSE frame: an event name (defaults to `"message"`) plus the raw
/// data payload (concatenated `data:` lines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SseFrame {
    pub event: String,
    pub data: String,
}

/// Splits an SSE byte stream into complete frames.  Frames are separated by
/// a blank line (`\r?\n\r?\n`); within a frame each line may be `event: …`,
/// `data: …`, or a comment / id line we ignore.
pub(crate) fn parse_sse_frames(buf: &mut String) -> Vec<SseFrame> {
    let mut frames = Vec::new();
    loop {
        let Some(idx) = find_blank_line(buf) else { break };
        let (head, tail) = buf.split_at(idx);
        let frame_text = head.to_owned();
        let new_buf = tail[blank_line_len(tail)..].to_owned();
        let _ = std::mem::replace(buf, new_buf);

        frames.push(parse_single_frame(&frame_text));
    }
    frames
}

fn find_blank_line(s: &str) -> Option<usize> {
    // Detect the first occurrence of "\n\n" or "\r\n\r\n".
    let bytes = s.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
            return Some(i);
        }
        if i + 3 < bytes.len()
            && bytes[i] == b'\r'
            && bytes[i + 1] == b'\n'
            && bytes[i + 2] == b'\r'
            && bytes[i + 3] == b'\n'
        {
            return Some(i);
        }
    }
    None
}

fn blank_line_len(s: &str) -> usize {
    let b = s.as_bytes();
    if b.len() >= 4 && &b[..4] == b"\r\n\r\n" {
        4
    } else if b.len() >= 2 && &b[..2] == b"\n\n" {
        2
    } else {
        0
    }
}

fn parse_single_frame(frame: &str) -> SseFrame {
    let mut event = String::from("message");
    let mut data_lines: Vec<&str> = Vec::new();
    for line in frame.split('\n') {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("event:") {
            event = rest.trim().to_owned();
        } else if let Some(rest) = line.strip_prefix("data:") {
            let v = rest.strip_prefix(' ').unwrap_or(rest);
            data_lines.push(v);
        }
        // ignore `id:`, `retry:`, comments, etc.
    }
    SseFrame { event, data: data_lines.join("\n") }
}

/// Convenience wrapper that ingests a `Bytes` chunk into the rolling buffer
/// and returns the freshly available frames.
pub(crate) fn ingest_bytes(buf: &mut String, chunk: &Bytes) -> Vec<SseFrame> {
    buf.push_str(&String::from_utf8_lossy(chunk));
    parse_sse_frames(buf)
}

// ---------------------------------------------------------------------------
// Shared OpenAI-compatible SSE frame handler
// ---------------------------------------------------------------------------
//
// Antigravity ships an OpenAI-compatible chat-completions surface, so the
// per-frame decode logic lives here in the parent module rather than being
// duplicated.  Gemini and Anthropic have their own dedicated handlers
// inside their respective adapter modules.

/// The three outcomes a per-frame handler can produce.
pub(crate) enum FrameOutcome {
    Skip,
    Emit(NormalizedChunk),
    Terminate(NormalizedChunk),
}

/// Decodes one OpenAI-style SSE frame (used by the Antigravity adapter).
///
/// * `data: [DONE]` → `Terminate(Done)`.
/// * `data: {"choices": [{"delta": {"content": "..."}}]}` → `Emit(ChatDelta(...))`.
/// * `data: {"choices": [{"finish_reason": "stop"|"length"}]}` → `Terminate(Done)`.
/// * Inline `error.message` → `Terminate(Error(msg))`.
/// * Anything else (keepalive, unknown shape) → `Skip`.
pub(crate) fn handle_openai_compatible_frame(frame: &SseFrame) -> FrameOutcome {
    let data = frame.data.trim();
    if data.is_empty() {
        return FrameOutcome::Skip;
    }
    if data == "[DONE]" {
        return FrameOutcome::Terminate(NormalizedChunk::Done);
    }
    let parsed: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(_) => return FrameOutcome::Skip,
    };

    if let Some(err) = parsed.get("error") {
        let msg = err
            .as_str()
            .map(String::from)
            .or_else(|| err.get("message").and_then(Value::as_str).map(String::from));
        return FrameOutcome::Terminate(NormalizedChunk::Error(
            msg.unwrap_or_else(|| "upstream error".to_owned()),
        ));
    }

    let Some(first) = parsed.get("choices").and_then(|c| c.get(0)) else {
        return FrameOutcome::Skip;
    };
    let finish = first
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(|r| r == "stop" || r == "length")
        .unwrap_or(false);
    let delta = first
        .get("delta")
        .and_then(|d| d.get("content"))
        .and_then(Value::as_str)
        .map(String::from)
        .or_else(|| first.get("text").and_then(Value::as_str).map(String::from))
        .unwrap_or_default();

    if !delta.is_empty() {
        return FrameOutcome::Emit(NormalizedChunk::ChatDelta(delta));
    }
    if finish {
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

    #[test]
    fn provider_slug_matches_routes() {
        assert_eq!(Provider::Anthropic.slug(), "anthropic");
        assert_eq!(Provider::Gemini.slug(), "gemini");
        assert_eq!(Provider::Antigravity.slug(), "antigravity");
    }

    #[test]
    fn byok_request_roundtrip_json() {
        let req = ByokRequest {
            base_url: "https://api.anthropic.com".to_owned(),
            api_key: "sk-test".to_owned(),
            model: "claude-sonnet-4-6".to_owned(),
            stream: true,
            system_prompt: Some("You are concise.".to_owned()),
            messages: vec![ChatMessage::user("ping")],
            max_tokens: Some(256),
        };
        let json = serde_json::to_string(&req).expect("ser");
        let back: ByokRequest = serde_json::from_str(&json).expect("de");
        assert_eq!(back.base_url, req.base_url);
        assert_eq!(back.model, req.model);
        assert_eq!(back.messages.len(), 1);
        assert_eq!(back.system_prompt.as_deref(), Some("You are concise."));
        assert_eq!(back.max_tokens, Some(256));
        assert!(back.stream);
    }

    #[test]
    fn normalized_chunk_serde_roundtrip() {
        let delta = NormalizedChunk::ChatDelta("hello".to_owned());
        let json = serde_json::to_string(&delta).expect("ser");
        assert!(json.contains("chat_delta"));
        let back: NormalizedChunk = serde_json::from_str(&json).expect("de");
        assert_eq!(back, delta);

        let done = NormalizedChunk::Done;
        let json = serde_json::to_string(&done).expect("ser");
        assert!(json.contains("done"));

        let tu = NormalizedChunk::ToolUse {
            name: "search".to_owned(),
            args: "{\"q\":\"rust\"}".to_owned(),
        };
        let json = serde_json::to_string(&tu).expect("ser");
        assert!(json.contains("tool_use"));
        let back: NormalizedChunk = serde_json::from_str(&json).expect("de");
        assert_eq!(back, tu);
    }

    #[test]
    fn parse_sse_frames_splits_on_blank_line_lf() {
        let mut buf =
            String::from("event: message_start\ndata: {\"hi\":1}\n\nevent: ping\ndata: 2\n\n");
        let frames = parse_sse_frames(&mut buf);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].event, "message_start");
        assert_eq!(frames[0].data, "{\"hi\":1}");
        assert_eq!(frames[1].event, "ping");
        assert_eq!(frames[1].data, "2");
        assert!(buf.is_empty(), "buf should drain to empty, got: {buf:?}");
    }

    #[test]
    fn parse_sse_frames_splits_on_blank_line_crlf() {
        let mut buf = String::from("event: a\r\ndata: x\r\n\r\nevent: b\r\ndata: y\r\n\r\n");
        let frames = parse_sse_frames(&mut buf);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].data, "x");
        assert_eq!(frames[1].data, "y");
    }

    #[test]
    fn parse_sse_frames_handles_multi_line_data() {
        let mut buf = String::from("data: line1\ndata: line2\n\n");
        let frames = parse_sse_frames(&mut buf);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "line1\nline2");
    }

    #[test]
    fn parse_sse_frames_leaves_partial_in_buffer() {
        let mut buf = String::from("data: complete\n\ndata: partial\n");
        let frames = parse_sse_frames(&mut buf);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "complete");
        assert!(buf.contains("partial"));
    }

    #[test]
    fn openai_compatible_frame_emits_chat_delta() {
        let frame = SseFrame {
            event: "message".to_owned(),
            data: "{\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}"
                .to_owned(),
        };
        match handle_openai_compatible_frame(&frame) {
            FrameOutcome::Emit(NormalizedChunk::ChatDelta(t)) => assert_eq!(t, "Hi"),
            _ => panic!("expected chat_delta emit"),
        }
    }

    #[test]
    fn openai_compatible_frame_done_marker_terminates() {
        let frame = SseFrame { event: "message".to_owned(), data: "[DONE]".to_owned() };
        assert!(matches!(
            handle_openai_compatible_frame(&frame),
            FrameOutcome::Terminate(NormalizedChunk::Done)
        ));
    }
}
