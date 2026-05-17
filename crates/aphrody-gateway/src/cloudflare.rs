// SPDX-License-Identifier: Apache-2.0
//! Cloudflare AI Gateway adapter.
//!
//! Proxies OpenAI-compatible chat completions through the Cloudflare AI
//! Gateway endpoint:
//!
//! ```text
//! POST https://gateway.ai.cloudflare.com/v1/<account_id>/<gateway_id>/openai/chat/completions
//! Authorization: Bearer <api_token>
//! Content-Type: application/json
//! ```
//!
//! Reference (upstream TS): `openclaw/extensions/cloudflare-ai-gateway/`.

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

/// Base URL template before appending `/openai/chat/completions`.
const GATEWAY_BASE: &str = "https://gateway.ai.cloudflare.com/v1";

/// Environment variable name for the API token (mirrors the TS plugin).
pub const ENV_API_TOKEN: &str = "CLOUDFLARE_AI_GATEWAY_API_KEY";

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Cloudflare AI Gateway adapter.
///
/// Construct via [`CloudflareAdapter::new`] or [`CloudflareAdapter::from_env`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareAdapter {
    /// Cloudflare account ID (shown in the dashboard → "Account ID").
    pub account_id: String,

    /// Cloudflare AI Gateway ID (the slug you chose when creating the gateway).
    pub gateway_id: String,

    /// Cloudflare API token with AI Gateway permissions.
    pub api_token: String,

    /// HTTP client shared across requests.
    #[serde(skip)]
    client: Option<Client>,
}

impl CloudflareAdapter {
    /// Constructs an adapter with an explicit token.
    ///
    /// `account_id` and `gateway_id` must both be non-empty.
    pub fn new(
        account_id: impl Into<String>,
        gateway_id: impl Into<String>,
        api_token: impl Into<String>,
    ) -> Result<Self, GatewayError> {
        let account_id = account_id.into();
        let gateway_id = gateway_id.into();
        let api_token = api_token.into();

        if account_id.trim().is_empty() {
            return Err(GatewayError::Config(
                "CloudflareAdapter: account_id must not be blank".to_owned(),
            ));
        }
        if gateway_id.trim().is_empty() {
            return Err(GatewayError::Config(
                "CloudflareAdapter: gateway_id must not be blank".to_owned(),
            ));
        }
        if api_token.trim().is_empty() {
            return Err(GatewayError::Config(
                "CloudflareAdapter: api_token must not be blank".to_owned(),
            ));
        }

        Ok(Self { account_id, gateway_id, api_token, client: Some(Client::new()) })
    }

    /// Constructs an adapter by reading credentials from environment variables.
    ///
    /// Required env vars:
    /// - `CLOUDFLARE_AI_GATEWAY_API_KEY` — Cloudflare API token.
    /// - `CLOUDFLARE_AI_GATEWAY_ACCOUNT_ID` — Cloudflare account ID.
    /// - `CLOUDFLARE_AI_GATEWAY_GATEWAY_ID` — AI Gateway ID.
    pub fn from_env() -> Result<Self, GatewayError> {
        let api_token = env::var(ENV_API_TOKEN).map_err(|_| {
            GatewayError::Config(format!("CloudflareAdapter: env var {ENV_API_TOKEN} not set"))
        })?;
        let account_id = env::var("CLOUDFLARE_AI_GATEWAY_ACCOUNT_ID").map_err(|_| {
            GatewayError::Config(
                "CloudflareAdapter: env var CLOUDFLARE_AI_GATEWAY_ACCOUNT_ID not set".to_owned(),
            )
        })?;
        let gateway_id = env::var("CLOUDFLARE_AI_GATEWAY_GATEWAY_ID").map_err(|_| {
            GatewayError::Config(
                "CloudflareAdapter: env var CLOUDFLARE_AI_GATEWAY_GATEWAY_ID not set".to_owned(),
            )
        })?;
        Self::new(account_id, gateway_id, api_token)
    }

    /// Returns the full chat completions URL for this adapter.
    pub fn endpoint_url(&self) -> Result<Url, GatewayError> {
        let raw = format!(
            "{}/{}/{}/openai/chat/completions",
            GATEWAY_BASE,
            self.account_id.trim(),
            self.gateway_id.trim(),
        );
        Ok(Url::parse(&raw)?)
    }

    fn http_client(&self) -> Client {
        self.client.clone().unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl GatewayAdapter for CloudflareAdapter {
    async fn chat(&self, mut req: ChatRequest) -> Result<ChatResponse, GatewayError> {
        req.stream = false;
        let url = self.endpoint_url()?;
        let body = build_openai_body(&req);

        let resp =
            self.http_client().post(url).bearer_auth(&self.api_token).json(&body).send().await?;

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
            self.http_client().post(url).bearer_auth(&self.api_token).json(&body).send().await?;

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

/// Converts a raw byte stream from an SSE response into parsed [`ChatChunk`]s.
///
/// Handles partial line buffering across `Bytes` frames, which is the normal
/// case for streaming HTTP/2 responses.
fn sse_bytes_to_chunks(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> impl Stream<Item = Result<ChatChunk, GatewayError>> + Send {
    let mut buf = String::new();
    stream.flat_map(move |result| {
        let items: Vec<Result<ChatChunk, GatewayError>> = match result {
            Err(e) => vec![Err(GatewayError::Transport(e))],
            Ok(bytes) => {
                // Append incoming bytes to buffer, parse complete lines.
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
    fn endpoint_url_format() {
        let adapter = CloudflareAdapter::new("acc123", "gw456", "tok").expect("construct");
        let url = adapter.endpoint_url().expect("url");
        assert_eq!(
            url.as_str(),
            "https://gateway.ai.cloudflare.com/v1/acc123/gw456/openai/chat/completions"
        );
    }

    #[test]
    fn blank_account_id_rejected() {
        let err = CloudflareAdapter::new("", "gw", "tok").unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn blank_gateway_id_rejected() {
        let err = CloudflareAdapter::new("acc", "", "tok").unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn blank_api_token_rejected() {
        let err = CloudflareAdapter::new("acc", "gw", "").unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }
}
