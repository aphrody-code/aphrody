// SPDX-License-Identifier: Apache-2.0
//! Google AI Antigravity provider adapter.
//!
//! Wraps the Google Vertex AI "Antigravity" Gemini family (gemini-3-pro-low /
//! gemini-3-pro-high / gemini-3-flash) exposed via the OpenAI-compatible
//! `/v1/chat/completions` surface. The provider is registered under the
//! `google-antigravity` id and honours the bare-pro -> `-low` normalization
//! contract derived from `openclaw/extensions/google/model-id.ts` and
//! `openclaw/src/agents/models-config.providers.google-antigravity.test.ts`.
//!
//! # Environment
//!
//! | Env var                       | Purpose                                  | Default                              |
//! |-------------------------------|------------------------------------------|--------------------------------------|
//! | `GOOGLE_ANTIGRAVITY_API_KEY`  | Bearer token forwarded to the upstream   | *(required)*                         |
//! | `GOOGLE_ANTIGRAVITY_BASE_URL` | Override the base URL                    | `https://aistudio.googleapis.com`    |
//!
//! Two equivalent OpenAI-compat hostnames are recognised by Google:
//! the Vertex form (`https://aistudio.googleapis.com`) and the explicit
//! generative-language form (`https://generativelanguage.googleapis.com`).
//! Either works; we default to the former because it is the canonical surface
//! advertised by the Antigravity preview.

use std::env;

use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    ChatChunk, ChatRequest, ChatResponse, ChunkStream, GatewayAdapter, GatewayError,
    build_openai_body, extract_content, extract_model, extract_total_tokens, parse_sse_line,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default Antigravity base URL.
pub const ANTIGRAVITY_BASE_URL: &str = "https://aistudio.googleapis.com";

/// Path appended to the base URL to reach OpenAI-compat chat completions.
const CHAT_PATH: &str = "/v1/chat/completions";

/// Env var for the API key.
pub const ENV_API_KEY: &str = "GOOGLE_ANTIGRAVITY_API_KEY";

/// Env var for the base URL override.
pub const ENV_BASE_URL: &str = "GOOGLE_ANTIGRAVITY_BASE_URL";

/// Bare gemini-pro identifiers that the Antigravity surface requires to be
/// suffixed with `-low` (mirrors `ANTIGRAVITY_BARE_PRO_IDS` in
/// `openclaw/extensions/google/model-id.ts`).
const ANTIGRAVITY_BARE_PRO_IDS: &[&str] =
    &["gemini-3-pro", "gemini-3.1-pro", "gemini-3-1-pro"];

/// Registered Antigravity model ids advertised by this provider. Mirrors
/// `GEMINI_3_PRO_ANTIGRAVITY_TEMPLATE_IDS` / `GEMINI_3_FLASH_ANTIGRAVITY_TEMPLATE_IDS`
/// in `openclaw/extensions/google/provider-models.ts`.
pub const ANTIGRAVITY_MODEL_IDS: &[&str] =
    &["gemini-3-pro-low", "gemini-3-pro-high", "gemini-3-flash"];

// ---------------------------------------------------------------------------
// Model id normalization (ported from model-id.ts)
// ---------------------------------------------------------------------------

/// Normalize a model id for the `google-antigravity` provider.
///
/// Mirrors `normalizeAntigravityModelId` in `model-id.ts`: bare `gemini-3-pro`
/// / `gemini-3.1-pro` / `gemini-3-1-pro` get a `-low` suffix; everything else
/// is returned unchanged. The `google/` prefix (when present) is preserved.
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
    if ANTIGRAVITY_BARE_PRO_IDS.contains(&id) {
        return format!("{id}-low");
    }
    id.to_owned()
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Google AI Antigravity adapter (Vertex AI Gemini 3 preview family).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAntigravityAdapter {
    /// Bearer token forwarded as `Authorization: Bearer <api_key>`.
    pub api_key: String,

    /// Antigravity base URL. Defaults to [`ANTIGRAVITY_BASE_URL`].
    pub base_url: String,

    /// HTTP client shared across requests.
    #[serde(skip)]
    client: Option<Client>,
}

impl GoogleAntigravityAdapter {
    /// Constructs an adapter with explicit credentials.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError::Config`] when `api_key` or `base_url` is blank,
    /// or [`GatewayError::Url`] when the base URL is not a valid URL.
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, GatewayError> {
        let api_key = api_key.into();
        let base_url = base_url.into();
        if api_key.trim().is_empty() {
            return Err(GatewayError::Config(
                "GoogleAntigravityAdapter: api_key must not be blank".to_owned(),
            ));
        }
        if base_url.trim().is_empty() {
            return Err(GatewayError::Config(
                "GoogleAntigravityAdapter: base_url must not be blank".to_owned(),
            ));
        }
        Url::parse(&base_url)?;
        Ok(Self { api_key, base_url, client: Some(Client::new()) })
    }

    /// Construct from `GOOGLE_ANTIGRAVITY_API_KEY` (+ optional
    /// `GOOGLE_ANTIGRAVITY_BASE_URL`).
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError::Config`] when the API key env var is unset.
    pub fn from_env() -> Result<Self, GatewayError> {
        let api_key = env::var(ENV_API_KEY).map_err(|_| {
            GatewayError::Config(format!(
                "GoogleAntigravityAdapter: env var {ENV_API_KEY} not set"
            ))
        })?;
        let base_url =
            env::var(ENV_BASE_URL).unwrap_or_else(|_| ANTIGRAVITY_BASE_URL.to_owned());
        Self::new(api_key, base_url)
    }

    /// Returns the fully-qualified chat completions URL.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError::Url`] when the configured base URL cannot be
    /// joined with the chat completions path.
    pub fn endpoint_url(&self) -> Result<Url, GatewayError> {
        let base = Url::parse(&self.base_url)?;
        Ok(base.join(CHAT_PATH)?)
    }

    fn http_client(&self) -> Client {
        self.client.clone().unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl GatewayAdapter for GoogleAntigravityAdapter {
    async fn chat(&self, mut req: ChatRequest) -> Result<ChatResponse, GatewayError> {
        req.stream = false;
        req.model = normalize_antigravity_model_id(&req.model);
        let url = self.endpoint_url()?;
        let body = build_openai_body(&req);

        let resp = self
            .http_client()
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::UpstreamStatus { status, body: text });
        }

        let v: serde_json::Value = resp.json().await?;
        Ok(ChatResponse {
            content: extract_content(&v).unwrap_or_default(),
            model: extract_model(&v),
            total_tokens: extract_total_tokens(&v),
        })
    }

    async fn chat_stream(&self, mut req: ChatRequest) -> Result<ChunkStream, GatewayError> {
        req.stream = true;
        req.model = normalize_antigravity_model_id(&req.model);
        let url = self.endpoint_url()?;
        let body = build_openai_body(&req);

        let resp = self
            .http_client()
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::UpstreamStatus { status, body: text });
        }

        Ok(Box::pin(sse_bytes_to_chunks(resp.bytes_stream())))
    }
}

// ---------------------------------------------------------------------------
// SSE framing (copy of the openai_proxy helper — kept local to avoid widening
// the crate's pub(crate) surface).
// ---------------------------------------------------------------------------

fn sse_bytes_to_chunks(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> impl Stream<Item = Result<ChatChunk, GatewayError>> + Send {
    let mut buf = String::new();
    stream.flat_map(move |result| {
        let items: Vec<Result<ChatChunk, GatewayError>> = match result {
            Err(e) => vec![Err(GatewayError::Transport(e))],
            Ok(bytes) => {
                buf.push_str(&String::from_utf8_lossy(&bytes));
                let mut chunks = Vec::new();
                while let Some(nl) = buf.find('\n') {
                    let line = buf[..nl].trim_end_matches('\r').to_owned();
                    buf.drain(..=nl);
                    match parse_sse_line(&line) {
                        Ok(Some(c)) => {
                            let done = c.finish;
                            chunks.push(Ok(c));
                            if done {
                                break;
                            }
                        },
                        Ok(None) => {},
                        Err(e) => {
                            chunks.push(Err(e));
                            break;
                        },
                    }
                }
                chunks
            },
        };
        futures::stream::iter(items)
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_url_default_base() {
        let adapter = GoogleAntigravityAdapter::new("ag-test", ANTIGRAVITY_BASE_URL)
            .expect("construct");
        let url = adapter.endpoint_url().expect("url");
        assert_eq!(url.as_str(), "https://aistudio.googleapis.com/v1/chat/completions");
    }

    #[test]
    fn endpoint_url_alt_origin() {
        let adapter = GoogleAntigravityAdapter::new(
            "ag-test",
            "https://generativelanguage.googleapis.com",
        )
        .expect("construct");
        let url = adapter.endpoint_url().expect("url");
        assert_eq!(
            url.as_str(),
            "https://generativelanguage.googleapis.com/v1/chat/completions"
        );
    }

    #[test]
    fn blank_api_key_rejected() {
        let err = GoogleAntigravityAdapter::new("", ANTIGRAVITY_BASE_URL).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn blank_base_url_rejected() {
        let err = GoogleAntigravityAdapter::new("k", "").unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    // ---- Model id normalization (mirrors models-config.providers.google-antigravity.test.ts) ----

    #[test]
    fn normalize_bare_pro_ids_get_low_suffix() {
        assert_eq!(normalize_antigravity_model_id("gemini-3-pro"), "gemini-3-pro-low");
        assert_eq!(normalize_antigravity_model_id("gemini-3.1-pro"), "gemini-3.1-pro-low");
        assert_eq!(normalize_antigravity_model_id("gemini-3-1-pro"), "gemini-3-1-pro-low");
    }

    #[test]
    fn normalize_already_suffixed_passthrough() {
        assert_eq!(normalize_antigravity_model_id("gemini-3-pro-low"), "gemini-3-pro-low");
        assert_eq!(normalize_antigravity_model_id("gemini-3-pro-high"), "gemini-3-pro-high");
        assert_eq!(normalize_antigravity_model_id("gemini-3-flash"), "gemini-3-flash");
        assert_eq!(
            normalize_antigravity_model_id("claude-opus-4-6-thinking"),
            "claude-opus-4-6-thinking"
        );
    }

    #[test]
    fn normalize_google_prefix_preserved() {
        assert_eq!(
            normalize_antigravity_model_id("google/gemini-3-pro"),
            "google/gemini-3-pro-low"
        );
        assert_eq!(
            normalize_antigravity_model_id("google/gemini-3-pro-low"),
            "google/gemini-3-pro-low"
        );
    }

    #[test]
    fn registered_models_include_low_high_flash() {
        let ids: &[&str] = ANTIGRAVITY_MODEL_IDS;
        assert!(ids.contains(&"gemini-3-pro-low"));
        assert!(ids.contains(&"gemini-3-pro-high"));
        assert!(ids.contains(&"gemini-3-flash"));
    }
}
