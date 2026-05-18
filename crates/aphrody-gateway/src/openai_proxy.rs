// SPDX-License-Identifier: Apache-2.0
//! BYOK ("Bring Your Own Key") OpenAI-compatible proxy adapter.
//!
//! This is the minimal-env fallback: it posts to any configurable
//! OpenAI-compatible endpoint (OpenAI itself, Azure OpenAI, Together AI,
//! Groq, Ollama, LM Studio, …) without routing through a third-party gateway.
//!
//! Configuration is env-keyed for 12-factor compatibility:
//!
//! | Env var              | Purpose                                        | Default                              |
//! |----------------------|------------------------------------------------|--------------------------------------|
//! | `OPENAI_API_KEY`     | Bearer token                                   | *(required)*                         |
//! | `OPENAI_BASE_URL`    | Base URL of the OpenAI-compatible API          | `https://api.openai.com`             |
//!
//! From the open-design README: "OpenAI-compatible BYOK proxy is the same loop
//! minus the spawn."  This adapter is intentionally the simplest of the three.

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

/// Default base URL when `OPENAI_BASE_URL` is not set.
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com";

/// Chat completions path.
const CHAT_PATH: &str = "/v1/chat/completions";

/// Env var for the API key.
pub const ENV_API_KEY: &str = "OPENAI_API_KEY";

/// Env var for the base URL override.
pub const ENV_BASE_URL: &str = "OPENAI_BASE_URL";

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// BYOK OpenAI-compatible proxy.
///
/// Construct via [`OpenAiProxyAdapter::new`] or [`OpenAiProxyAdapter::from_env`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiProxyAdapter {
    /// API key sent as `Authorization: Bearer <api_key>`.
    pub api_key: String,

    /// Base URL of the OpenAI-compatible service.  Defaults to
    /// `https://api.openai.com`.
    pub base_url: String,

    /// HTTP client shared across requests.
    #[serde(skip)]
    client: Option<Client>,
}

impl OpenAiProxyAdapter {
    /// Constructs an adapter with an explicit key and base URL.
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, GatewayError> {
        let api_key = api_key.into();
        let base_url = base_url.into();
        if api_key.trim().is_empty() {
            return Err(GatewayError::Config(
                "OpenAiProxyAdapter: api_key must not be blank".to_owned(),
            ));
        }
        if base_url.trim().is_empty() {
            return Err(GatewayError::Config(
                "OpenAiProxyAdapter: base_url must not be blank".to_owned(),
            ));
        }
        // Validate immediately so callers get a clear error at construction
        // time rather than at the first network call.
        Url::parse(&base_url)?;
        Ok(Self { api_key, base_url, client: Some(Client::new()) })
    }

    /// Constructs an adapter from environment variables.
    ///
    /// - `OPENAI_API_KEY` — required.
    /// - `OPENAI_BASE_URL` — optional, defaults to `https://api.openai.com`.
    pub fn from_env() -> Result<Self, GatewayError> {
        let api_key = env::var(ENV_API_KEY).map_err(|_| {
            GatewayError::Config(format!("OpenAiProxyAdapter: env var {ENV_API_KEY} not set"))
        })?;
        let base_url = env::var(ENV_BASE_URL).unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
        Self::new(api_key, base_url)
    }

    /// Returns the full chat completions URL.
    pub fn endpoint_url(&self) -> Result<Url, GatewayError> {
        let base = Url::parse(&self.base_url)?;
        Ok(base.join(CHAT_PATH)?)
    }

    fn http_client(&self) -> Client {
        self.client.clone().unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl GatewayAdapter for OpenAiProxyAdapter {
    async fn chat(&self, mut req: ChatRequest) -> Result<ChatResponse, GatewayError> {
        req.stream = false;
        let url = self.endpoint_url()?;
        let body = build_openai_body(&req);

        let resp =
            self.http_client().post(url).bearer_auth(&self.api_key).json(&body).send().await?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::UpstreamStatus { status, body: text });
        }

        let v: serde_json::Value = resp.json().await?;
        let content = extract_content(&v).unwrap_or_default();
        Ok(ChatResponse {
            content,
            model: extract_model(&v),
            total_tokens: extract_total_tokens(&v),
        })
    }

    async fn chat_stream(&self, mut req: ChatRequest) -> Result<ChunkStream, GatewayError> {
        req.stream = true;
        let url = self.endpoint_url()?;
        let body = build_openai_body(&req);

        let resp =
            self.http_client().post(url).bearer_auth(&self.api_key).json(&body).send().await?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::UpstreamStatus { status, body: text });
        }

        let byte_stream = resp.bytes_stream();
        let chunk_stream = sse_bytes_to_chunks(byte_stream);
        Ok(Box::pin(chunk_stream))
    }
}

// ---------------------------------------------------------------------------
// SSE framing
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
                loop {
                    if let Some(nl) = buf.find('\n') {
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
                    } else {
                        break;
                    }
                }
                chunks
            },
        };
        futures::stream::iter(items)
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_url_default_base() {
        let adapter = OpenAiProxyAdapter::new("sk-test", DEFAULT_BASE_URL).expect("construct");
        let url = adapter.endpoint_url().expect("url");
        assert_eq!(url.as_str(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn endpoint_url_custom_base() {
        let adapter =
            OpenAiProxyAdapter::new("sk-test", "http://localhost:11434").expect("construct");
        let url = adapter.endpoint_url().expect("url");
        assert_eq!(url.as_str(), "http://localhost:11434/v1/chat/completions");
    }

    #[test]
    fn blank_api_key_rejected() {
        let err = OpenAiProxyAdapter::new("", DEFAULT_BASE_URL).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn blank_base_url_rejected() {
        let err = OpenAiProxyAdapter::new("key", "").unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn invalid_base_url_rejected() {
        let err = OpenAiProxyAdapter::new("key", "://invalid").unwrap_err();
        // Either a URL parse error or a config error; both are acceptable.
        assert!(matches!(err, GatewayError::Url(_) | GatewayError::Config(_)));
    }
}
