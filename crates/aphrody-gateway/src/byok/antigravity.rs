// SPDX-License-Identifier: Apache-2.0
//! Antigravity BYOK streaming adapter (Google Cloud Code Assist / Vertex
//! preview surface, OpenAI-compatible Chat Completions wire format).
//!
//! Contract derived from openclaw:
//!
//! * `openclaw/extensions/google/provider-models.test.ts` — registers
//!   Antigravity models with `api: "openai-completions"` and a configurable
//!   `baseUrl` such as `https://antigravity.example/v1`, confirming the
//!   surface is OpenAI-compatible.
//! * `openclaw/src/agents/models-config.providers.google-antigravity.test.ts`
//!   — the bare `gemini-3-pro` / `gemini-3.1-pro` model ids must be
//!   suffixed with `-low` before being sent upstream.
//! * `openclaw/extensions/google/model-id.ts` — canonical
//!   `normalizeAntigravityModelId` implementation, ported below.
//!
//! Authentication is Bearer OAuth (Google Cloud OAuth access token forwarded
//! as `Authorization: Bearer …`); the upstream is reached at:
//!
//! ```text
//! POST <base_url>/chat/completions          (when base_url already ends in /v1)
//! POST <base_url>/v1/chat/completions       (otherwise)
//! ```
//!
//! Streaming responses use the canonical OpenAI `data: {…}\n\n` framing with
//! a terminal `data: [DONE]`; the shared
//! [`super::handle_openai_compatible_frame`] decodes each frame into the
//! provider-agnostic [`NormalizedChunk`] enum.
//!
//! When `base_url` is empty the canonical Cloud Code Assist preview endpoint
//! `https://cloudcode-pa.googleapis.com/v1` is used.

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{
    ByokRequest, ByokStream, ChatMessage, FrameOutcome, NormalizedChunk, ChunkStream,
    check_url_ssrf, handle_openai_compatible_frame, ingest_bytes,
};
use crate::GatewayError;

/// Default Antigravity preview endpoint when no base URL is supplied.
pub const DEFAULT_BASE_URL: &str = "https://cloudcode-pa.googleapis.com/v1";

/// Bare gemini-pro identifiers that the Antigravity surface requires to be
/// suffixed with `-low` (mirrors `ANTIGRAVITY_BARE_PRO_IDS` in
/// `openclaw/extensions/google/model-id.ts`).
const BARE_PRO_IDS: &[&str] = &["gemini-3-pro", "gemini-3.1-pro", "gemini-3-1-pro"];

/// Antigravity BYOK adapter.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AntigravityByok {
    #[serde(skip)]
    client: Option<Client>,
}

impl AntigravityByok {
    /// Constructs a new adapter with a fresh HTTP client.
    pub fn new() -> Self {
        Self { client: Some(Client::new()) }
    }

    fn http(&self) -> Client {
        self.client.clone().unwrap_or_default()
    }
}

#[async_trait]
impl ByokStream for AntigravityByok {
    async fn stream(
        &self,
        req: ByokRequest,
    ) -> Result<ChunkStream, GatewayError> {
        let effective_base = if req.base_url.trim().is_empty() {
            DEFAULT_BASE_URL.to_owned()
        } else {
            req.base_url.clone()
        };
        let parsed = Url::parse(&effective_base)?;
        check_url_ssrf(&parsed)?;
        let url = build_antigravity_url(&parsed)?;

        let mut messages: Vec<ChatMessage> = req.messages.clone();
        if let Some(system) = &req.system_prompt {
            messages.insert(0, ChatMessage::system(system));
        }

        let model = normalize_antigravity_model_id(&req.model);
        let payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(8192),
            "stream": true,
        });

        let resp = self
            .http()
            .post(url)
            .bearer_auth(&req.api_key)
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
// Model id normalization (ported from openclaw model-id.ts)
// ---------------------------------------------------------------------------

/// Normalize a model id for the Antigravity provider.
///
/// Mirrors `normalizeAntigravityModelId`: bare `gemini-3-pro` /
/// `gemini-3.1-pro` / `gemini-3-1-pro` get a `-low` suffix; everything else
/// is returned unchanged.  The `google/` prefix (when present) is preserved.
#[must_use]
pub fn normalize_antigravity_model_id(id: &str) -> String {
    const GOOGLE_PROVIDER_PREFIX: &str = "google/";
    if let Some(rest) = id.strip_prefix(GOOGLE_PROVIDER_PREFIX) {
        let normalized = normalize_antigravity_model_id(rest);
        return if normalized == rest {
            id.to_owned()
        } else {
            format!("{GOOGLE_PROVIDER_PREFIX}{normalized}")
        };
    }
    if BARE_PRO_IDS.contains(&id) {
        return format!("{id}-low");
    }
    id.to_owned()
}

// ---------------------------------------------------------------------------
// URL construction
// ---------------------------------------------------------------------------

/// Appends `/chat/completions` to the supplied base URL, inserting `/v1` when
/// the base URL does not already include a versioned segment.
pub(crate) fn build_antigravity_url(base: &Url) -> Result<Url, GatewayError> {
    let trimmed = base.path().trim_end_matches('/');
    let needs_version = !trimmed.split('/').any(|seg| {
        seg.starts_with('v')
            && seg.len() > 1
            && seg[1..].chars().all(|c| c.is_ascii_digit())
    });
    let new_path = if needs_version {
        format!("{trimmed}/v1/chat/completions")
    } else {
        format!("{trimmed}/chat/completions")
    };
    let mut out = base.clone();
    out.set_path(&new_path);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Stream decoding (delegates to shared OpenAI-compatible frame handler)
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
                    match handle_openai_compatible_frame(&frame) {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byok::parse_sse_frames;

    #[test]
    fn antigravity_build_url_adds_v1_when_missing() {
        let base = Url::parse("https://cloudcode-pa.googleapis.com").unwrap();
        let out = build_antigravity_url(&base).unwrap();
        assert_eq!(
            out.as_str(),
            "https://cloudcode-pa.googleapis.com/v1/chat/completions"
        );
    }

    #[test]
    fn antigravity_build_url_preserves_existing_version() {
        let base = Url::parse("https://cloudcode-pa.googleapis.com/v1").unwrap();
        let out = build_antigravity_url(&base).unwrap();
        assert_eq!(
            out.as_str(),
            "https://cloudcode-pa.googleapis.com/v1/chat/completions"
        );
    }

    #[test]
    fn antigravity_normalize_bare_pro_appends_low() {
        assert_eq!(normalize_antigravity_model_id("gemini-3-pro"), "gemini-3-pro-low");
        assert_eq!(normalize_antigravity_model_id("gemini-3.1-pro"), "gemini-3.1-pro-low");
        assert_eq!(normalize_antigravity_model_id("gemini-3-1-pro"), "gemini-3-1-pro-low");
    }

    #[test]
    fn antigravity_normalize_preserves_google_prefix() {
        assert_eq!(
            normalize_antigravity_model_id("google/gemini-3-pro"),
            "google/gemini-3-pro-low"
        );
        assert_eq!(
            normalize_antigravity_model_id("google/gemini-3-pro-high"),
            "google/gemini-3-pro-high"
        );
    }

    #[test]
    fn antigravity_normalize_passes_through_other_ids() {
        assert_eq!(normalize_antigravity_model_id("gemini-3-pro-high"), "gemini-3-pro-high");
        assert_eq!(normalize_antigravity_model_id("gemini-3-flash"), "gemini-3-flash");
        assert_eq!(
            normalize_antigravity_model_id("claude-opus-4-6-thinking"),
            "claude-opus-4-6-thinking"
        );
    }

    #[test]
    fn parse_antigravity_delta_emits_chat_delta() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"AG\"},\"finish_reason\":null}]}\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        match handle_openai_compatible_frame(&frames[0]) {
            FrameOutcome::Emit(NormalizedChunk::ChatDelta(t)) => assert_eq!(t, "AG"),
            _ => panic!("expected chat_delta emit"),
        }
    }

    #[test]
    fn parse_antigravity_done_terminates() {
        let sse = "data: [DONE]\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        assert!(matches!(
            handle_openai_compatible_frame(&frames[0]),
            FrameOutcome::Terminate(NormalizedChunk::Done)
        ));
    }

    #[test]
    fn parse_antigravity_stop_finish_reason_terminates() {
        let sse = "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        assert!(matches!(
            handle_openai_compatible_frame(&frames[0]),
            FrameOutcome::Terminate(NormalizedChunk::Done)
        ));
    }

    #[test]
    fn parse_antigravity_inline_error_terminates() {
        let sse = "data: {\"error\":{\"message\":\"quota exceeded\"}}\n\n";
        let mut buf = String::from(sse);
        let frames = parse_sse_frames(&mut buf);
        match handle_openai_compatible_frame(&frames[0]) {
            FrameOutcome::Terminate(NormalizedChunk::Error(m)) => {
                assert_eq!(m, "quota exceeded");
            },
            _ => panic!("expected error terminate"),
        }
    }
}
