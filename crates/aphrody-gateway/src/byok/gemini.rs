// SPDX-License-Identifier: Apache-2.0
//! Google Generative AI (Gemini) BYOK adapter.
//!
//! Wire shape mirrors `POST /api/proxy/google/stream`
//! (`apps/daemon/src/chat-routes.ts:1000`):
//!
//! ```text
//! POST <base_url>/v1beta/models/<model>:streamGenerateContent?alt=sse
//! x-goog-api-key: <api_key>
//! Content-Type: application/json
//!
//! { "contents": [{role: "user"|"model", parts: [{text: "…"}]}, ...],
//!   "generationConfig": {"maxOutputTokens": N},
//!   "systemInstruction": {"parts": [{"text": "<optional>"}]} }
//! ```
//!
//! Gemini delivers one SSE `data: {…}\n\n` per chunk; each payload has
//! `candidates[0].content.parts[].text` to concatenate, and a
//! `candidates[0].finishReason` whose terminal values
//! (`STOP`, `MAX_TOKENS`, `FINISH_REASON_UNSPECIFIED`) end the stream.
//! Non-benign finishReasons (e.g. `SAFETY`, `RECITATION`) and any
//! `promptFeedback.blockReason` are surfaced as
//! [`NormalizedChunk::Error`].
//!
//! When `base_url` is empty the default `https://generativelanguage.googleapis.com`
//! is used (same fallback as the upstream daemon).

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

/// Default Gemini endpoint when no base URL is supplied.
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// Google Gemini BYOK adapter (Generative Language API, `x-goog-api-key`).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GeminiByok {
    #[serde(skip)]
    client: Option<Client>,
}

impl GeminiByok {
    /// Constructs a new adapter with a fresh HTTP client.
    pub fn new() -> Self {
        Self { client: Some(Client::new()) }
    }

    fn http(&self) -> Client {
        self.client.clone().unwrap_or_default()
    }
}

#[async_trait]
impl ByokStream for GeminiByok {
    async fn stream(&self, req: ByokRequest) -> Result<ChunkStream, GatewayError> {
        let effective_base = if req.base_url.trim().is_empty() {
            DEFAULT_BASE_URL.to_owned()
        } else {
            req.base_url.clone()
        };
        let parsed = Url::parse(&effective_base)?;
        check_url_ssrf(&parsed)?;
        let url = build_google_url(&parsed, &req.model)?;

        let contents: Vec<Value> = req
            .messages
            .iter()
            .map(|m| {
                let role = if m.role == "assistant" { "model" } else { "user" };
                serde_json::json!({
                    "role": role,
                    "parts": [{ "text": m.content }],
                })
            })
            .collect();

        let mut payload = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": req.max_tokens.unwrap_or(8192),
            },
        });
        if let Some(system) = &req.system_prompt {
            payload["systemInstruction"] = serde_json::json!({ "parts": [{ "text": system }] });
        }

        let resp = self
            .http()
            .post(url)
            .header("x-goog-api-key", &req.api_key)
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
// URL construction
// ---------------------------------------------------------------------------

pub(crate) fn build_google_url(base: &Url, model: &str) -> Result<Url, GatewayError> {
    let trimmed = base.path().trim_end_matches('/').to_owned();
    let new_path = format!("{trimmed}/v1beta/models/{model}:streamGenerateContent");
    let mut out = base.clone();
    out.set_path(&new_path);
    out.query_pairs_mut().clear().append_pair("alt", "sse");
    Ok(out)
}

// ---------------------------------------------------------------------------
// Stream decoding
// ---------------------------------------------------------------------------

fn decode_stream(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> impl Stream<Item = Result<NormalizedChunk, GatewayError>> + Send {
    let mut buf = String::new();
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
                    match handle_google_frame(&frame) {
                        FrameOutcome::Emit(c) => out.push(Ok(c)),
                        FrameOutcome::Terminate(c) => {
                            out.push(Ok(c));
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

enum FrameOutcome {
    Skip,
    Emit(NormalizedChunk),
    Terminate(NormalizedChunk),
}

fn handle_google_frame(frame: &SseFrame) -> FrameOutcome {
    let data = frame.data.trim();
    if data.is_empty() {
        return FrameOutcome::Skip;
    }
    let parsed: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(_) => return FrameOutcome::Skip,
    };

    if let Some(block) = extract_block_message(&parsed) {
        return FrameOutcome::Terminate(NormalizedChunk::Error(block));
    }

    let text = extract_gemini_text(&parsed);
    let finish = parsed
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("finishReason"))
        .and_then(Value::as_str)
        .map(String::from);

    let benign = matches!(finish.as_deref(), Some("") | Some("STOP") | Some("MAX_TOKENS") | Some("FINISH_REASON_UNSPECIFIED") | None);

    if !text.is_empty() {
        // Emit text even if a benign finish reason is attached; the caller
        // will see the terminate on the next frame (or stream EOF).
        return if benign && finish.is_some() {
            // Atomic last frame: text then Done.  We can only emit one per
            // call, so emit the text and rely on stream EOF to close (we
            // can't safely return both Emit and Terminate from one outcome).
            FrameOutcome::Emit(NormalizedChunk::ChatDelta(text))
        } else if benign {
            FrameOutcome::Emit(NormalizedChunk::ChatDelta(text))
        } else {
            // Non-benign finish reason on a frame that also carried text:
            // emit the error after the text would require buffering; the
            // safer path is to surface the safety/block error immediately.
            FrameOutcome::Terminate(NormalizedChunk::Error(format!(
                "Gemini stopped the response ({})",
                finish.unwrap_or_else(|| "OTHER".to_owned())
            )))
        };
    }

    if let Some(reason) = finish {
        if benign {
            return FrameOutcome::Terminate(NormalizedChunk::Done);
        }
        return FrameOutcome::Terminate(NormalizedChunk::Error(format!(
            "Gemini stopped the response ({reason})"
        )));
    }

    FrameOutcome::Skip
}

fn extract_gemini_text(data: &Value) -> String {
    let Some(candidates) = data.get("candidates").and_then(Value::as_array) else {
        return String::new();
    };
    let Some(first) = candidates.first() else {
        return String::new();
    };
    let Some(parts) = first.get("content").and_then(|c| c.get("parts")).and_then(Value::as_array)
    else {
        return String::new();
    };
    parts
        .iter()
        .filter_map(|p| p.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("")
}

fn extract_block_message(data: &Value) -> Option<String> {
    let feedback = data.get("promptFeedback")?;
    let reason = feedback.get("blockReason").and_then(Value::as_str)?;
    if reason.is_empty() {
        return None;
    }
    let tail = feedback
        .get("blockReasonMessage")
        .and_then(Value::as_str)
        .map(|m| format!(" — {m}"))
        .unwrap_or_default();
    Some(format!("Gemini blocked the prompt ({reason}){tail}."))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byok::parse_sse_frames;

    #[test]
    fn google_build_url_default() {
        let base = Url::parse(DEFAULT_BASE_URL).unwrap();
        let url = build_google_url(&base, "gemini-2.0-flash").unwrap();
        assert_eq!(
            url.as_str(),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn parse_google_text_part_emits_chat_delta() {
        let sse = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hi\"}]}}]}\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        match handle_google_frame(&frames[0]) {
            FrameOutcome::Emit(NormalizedChunk::ChatDelta(t)) => assert_eq!(t, "Hi"),
            _ => panic!("expected chat delta"),
        }
    }

    #[test]
    fn parse_google_stop_finish_reason_terminates() {
        let sse = "data: {\"candidates\":[{\"finishReason\":\"STOP\",\"content\":{\"parts\":[]}}]}\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        assert!(matches!(
            handle_google_frame(&frames[0]),
            FrameOutcome::Terminate(NormalizedChunk::Done)
        ));
    }

    #[test]
    fn parse_google_safety_block_surfaces_error() {
        let sse = "data: {\"promptFeedback\":{\"blockReason\":\"SAFETY\",\"blockReasonMessage\":\"unsafe content\"}}\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        match handle_google_frame(&frames[0]) {
            FrameOutcome::Terminate(NormalizedChunk::Error(m)) => {
                assert!(m.contains("SAFETY"));
                assert!(m.contains("unsafe content"));
            },
            _ => panic!("expected error terminate"),
        }
    }

    #[test]
    fn parse_google_non_benign_finish_surfaces_error() {
        let sse = "data: {\"candidates\":[{\"finishReason\":\"RECITATION\",\"content\":{\"parts\":[]}}]}\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        match handle_google_frame(&frames[0]) {
            FrameOutcome::Terminate(NormalizedChunk::Error(m)) => assert!(m.contains("RECITATION")),
            _ => panic!("expected error terminate"),
        }
    }
}
