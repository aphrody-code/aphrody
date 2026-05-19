// SPDX-License-Identifier: Apache-2.0
//! LLM provider router for aphrody.
//!
//! aphrody supports a **strict whitelist of three providers**:
//!
//! 1. [`Provider::Anthropic`] — `https://api.anthropic.com/v1/messages`
//! 2. [`Provider::Gemini`]    — `https://generativelanguage.googleapis.com`
//! 3. [`Provider::Antigravity`] — Vertex AI Antigravity endpoint
//!
//! Any other provider name (openai, mistral, cohere, grok, deepseek, groq,
//! together, perplexity, azure, …) is **rejected at deserialisation time**
//! with [`RouterError::UnsupportedProvider`]. This is intentional and aligned
//! with the project memory `project_aphrody_providers_3only`.
//!
//! ## Wire shapes
//!
//! Every provider speaks a slightly different JSON dialect; the router
//! normalises requests through [`ChatRequest`] / [`ChatResponse`] and pushes
//! per-provider translation into the [`ChatProvider`] implementations.
//!
//! ## Streaming
//!
//! [`ChatProvider::complete_stream`] returns a boxed [`Stream`] of partial
//! text chunks. The current shipping implementation buffers the full HTTP
//! response and emits a single chunk per provider — wire-level SSE parsing
//! (Anthropic `data: { ... }`, Gemini `streamGenerateContent`) is a planned
//! follow-up and intentionally left observable via `tracing::warn!`.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Provider whitelist — re-exported from aphrody-providers (single source of
// truth). The enum + custom strict `Deserialize` + `Display` + `parse` are
// defined once in the dedicated micro-crate and re-exported here so the
// router's downstream API stays source-compatible.
// ---------------------------------------------------------------------------

pub use aphrody_providers::{Provider, ProviderError};

// ---------------------------------------------------------------------------
// Model identity
// ---------------------------------------------------------------------------

/// Fully-qualified model id — provider + free-form model name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId {
    /// Backing provider — the router uses this to dispatch.
    pub provider: Provider,
    /// Provider-specific model identifier (e.g. `claude-opus-4-7`,
    /// `gemini-2.5-pro`, `antigravity-prime`).
    pub name: String,
}

impl ModelId {
    /// Build a [`ModelId`] from a provider + raw model name.
    #[must_use]
    pub fn new(provider: Provider, name: impl Into<String>) -> Self {
        Self { provider, name: name.into() }
    }

    /// Parse a `"provider/name"` shorthand.
    ///
    /// # Errors
    /// - [`RouterError::InvalidModel`] if the separator is missing or the
    ///   `name` half is empty.
    /// - [`RouterError::UnsupportedProvider`] if the provider half is not
    ///   on the whitelist.
    pub fn parse(slug: &str) -> Result<Self, RouterError> {
        let (provider_str, name) = slug
            .split_once('/')
            .ok_or_else(|| RouterError::InvalidModel(slug.to_owned()))?;
        if name.is_empty() {
            return Err(RouterError::InvalidModel(slug.to_owned()));
        }
        Ok(Self { provider: Provider::parse(provider_str)?, name: name.to_owned() })
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.provider, self.name)
    }
}

// ---------------------------------------------------------------------------
// Chat DTOs
// ---------------------------------------------------------------------------

/// Role of a message inside a chat thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System prompt / instructions block.
    System,
    /// End-user utterance.
    User,
    /// Model reply.
    Assistant,
    /// Tool call result.
    Tool,
}

/// One message inside a [`ChatRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the speaker.
    pub role: Role,
    /// UTF-8 text content. Multimodal payloads are out of scope for v0.
    pub content: String,
    /// Tool call id, populated when `role = Tool`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// Convenience: user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into(), tool_call_id: None }
    }
    /// Convenience: system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into(), tool_call_id: None }
    }
    /// Convenience: assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into(), tool_call_id: None }
    }
}

/// Inbound completion request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Target model.
    pub model: ModelId,
    /// Ordered message history (oldest first).
    pub messages: Vec<ChatMessage>,
    /// Sampling temperature (provider-specific clamping applies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Max output tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Stream toggle — observable behaviour of
    /// [`ChatProvider::complete_stream`] is the source of truth.
    #[serde(default)]
    pub stream: bool,
}

impl ChatRequest {
    /// Build a one-shot user request against `model`.
    #[must_use]
    pub fn one_shot(model: ModelId, prompt: impl Into<String>) -> Self {
        Self {
            model,
            messages: vec![ChatMessage::user(prompt)],
            temperature: None,
            max_tokens: None,
            stream: false,
        }
    }
}

/// Token accounting from a successful completion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Tokens billed for the prompt half.
    pub prompt_tokens: u32,
    /// Tokens billed for the completion half.
    pub completion_tokens: u32,
}

/// Why the model stopped emitting tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural stop — end-of-turn marker.
    Stop,
    /// Hit `max_tokens`.
    Length,
    /// Model requested a tool call.
    ToolCalls,
    /// Provider-flagged error mid-stream.
    Error,
}

/// Successful completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Concatenated assistant text.
    pub content: String,
    /// Echo of the routed model (provider may rename internally).
    pub model: ModelId,
    /// Token accounting.
    pub usage: Usage,
    /// Why we stopped.
    pub finish_reason: FinishReason,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Unified error surface for the router and every provider impl.
#[derive(Debug, Error)]
pub enum RouterError {
    /// The caller asked for a provider outside the three-provider whitelist.
    /// The inner string is the rejected identifier as supplied (lowercased).
    ///
    /// This variant is automatically produced from [`aphrody_providers::ProviderError`]
    /// via the `From` impl below, so call sites can use `?` after
    /// [`Provider::parse`] without manual conversion.
    #[error("Unsupported provider: {0} (aphrody allows only anthropic, gemini, antigravity)")]
    UnsupportedProvider(String),
    /// Transport-level failure (DNS, TLS, decode).
    #[error("HTTP transport error: {0}")]
    Http(String),
    /// Remote API returned a non-success status with a parseable error envelope.
    #[error("API error {status}: {message}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Human-readable explanation lifted from the response body.
        message: String,
    },
    /// Missing or rejected credentials (401 / 403).
    #[error("authentication failed")]
    Auth,
    /// 429 — provider asks us to back off.
    #[error("rate limited; retry after {retry_after_ms:?} ms")]
    RateLimit {
        /// Optional `Retry-After` hint converted to milliseconds.
        retry_after_ms: Option<u64>,
    },
    /// JSON encode/decode failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Model id failed to parse or is empty.
    #[error("invalid model identifier: {0}")]
    InvalidModel(String),
    /// Streaming protocol violation (partial frame, malformed SSE, …).
    #[error("stream error: {0}")]
    Stream(String),
    /// No provider registered for the requested provider variant.
    #[error("no provider registered for {0}")]
    NoSuchProvider(Provider),
}

impl From<reqwest::Error> for RouterError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return Self::Http(format!("timeout: {err}"));
        }
        Self::Http(err.to_string())
    }
}

impl From<ProviderError> for RouterError {
    fn from(err: ProviderError) -> Self {
        match err {
            ProviderError::Unsupported(name) => Self::UnsupportedProvider(name),
        }
    }
}

/// Convenience alias.
pub type RouterResult<T> = Result<T, RouterError>;

// ---------------------------------------------------------------------------
// Provider trait
// ---------------------------------------------------------------------------

/// Object-safe trait every concrete provider implements.
///
/// Dyn-compatible (`Box<dyn ChatProvider>` round-trips).
#[async_trait]
pub trait ChatProvider: Send + Sync {
    /// Which whitelist variant this provider serves.
    fn provider(&self) -> Provider;

    /// Cheap liveness check — issues an auth-only HTTP request.
    async fn health(&self) -> RouterResult<()>;

    /// One-shot completion (no streaming).
    async fn complete(&self, req: ChatRequest) -> RouterResult<ChatResponse>;

    /// Streaming completion — emits partial text chunks.
    async fn complete_stream(
        &self,
        req: ChatRequest,
    ) -> RouterResult<Pin<Box<dyn Stream<Item = RouterResult<String>> + Send>>>;
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn build_client(user_agent: &str) -> RouterResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(user_agent.to_owned())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(RouterError::from)
}

async fn lift_error(resp: reqwest::Response) -> RouterError {
    let status = resp.status().as_u16();
    let retry_after_ms = resp
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(|secs| secs.saturating_mul(1_000));
    let body = resp.text().await.unwrap_or_default();
    match status {
        401 | 403 => RouterError::Auth,
        429 => RouterError::RateLimit { retry_after_ms },
        _ => RouterError::ApiError {
            status,
            message: extract_message(&body).unwrap_or(body),
        },
    }
}

fn extract_message(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    // Try a handful of well-known envelope shapes (Anthropic, Gemini, generic).
    if let Some(err) = v.get("error") {
        if let Some(msg) = err.get("message").and_then(|m| m.as_str()) {
            return Some(msg.to_owned());
        }
        if let Some(s) = err.as_str() {
            return Some(s.to_owned());
        }
    }
    if let Some(msg) = v.get("message").and_then(|m| m.as_str()) {
        return Some(msg.to_owned());
    }
    if let Some(detail) = v.get("detail").and_then(|m| m.as_str()) {
        return Some(detail.to_owned());
    }
    None
}

fn finish_from_str(s: &str) -> FinishReason {
    match s {
        "end_turn" | "stop" | "STOP" | "FINISH_REASON_STOP" => FinishReason::Stop,
        "max_tokens" | "length" | "MAX_TOKENS" => FinishReason::Length,
        "tool_use" | "tool_calls" | "TOOL_USE" => FinishReason::ToolCalls,
        _ => FinishReason::Stop,
    }
}

// ---------------------------------------------------------------------------
// Anthropic
// ---------------------------------------------------------------------------

/// Anthropic Messages API endpoint.
pub const ANTHROPIC_DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
/// Pinned API version header value.
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// HTTP provider for Claude family models.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    /// Build with the default public endpoint.
    ///
    /// # Errors
    /// - [`RouterError::Http`] if reqwest client builder fails.
    pub fn new(api_key: impl Into<String>) -> RouterResult<Self> {
        Self::with_base_url(api_key, ANTHROPIC_DEFAULT_BASE_URL)
    }

    /// Build pointing at an arbitrary base URL (tests, proxies).
    ///
    /// # Errors
    /// - [`RouterError::Http`] if reqwest client builder fails.
    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> RouterResult<Self> {
        let client = build_client(concat!("aphrody-router/", env!("CARGO_PKG_VERSION")))?;
        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }
}

#[derive(Debug, Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[async_trait]
impl ChatProvider for AnthropicProvider {
    fn provider(&self) -> Provider {
        Provider::Anthropic
    }

    async fn health(&self) -> RouterResult<()> {
        // Anthropic doesn't expose a public liveness endpoint; we do a minimal
        // POST that's almost guaranteed to either 200 or 400 (never network-fail).
        let probe = ChatRequest::one_shot(
            ModelId::new(Provider::Anthropic, "claude-3-5-haiku-latest"),
            "ping",
        );
        match self.complete(probe).await {
            Ok(_) | Err(RouterError::ApiError { .. }) | Err(RouterError::InvalidModel(_)) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn complete(&self, req: ChatRequest) -> RouterResult<ChatResponse> {
        if req.model.provider != Provider::Anthropic {
            return Err(RouterError::InvalidModel(req.model.to_string()));
        }
        let mut system_buf = String::new();
        let mut msgs: Vec<AnthropicMessage<'_>> = Vec::with_capacity(req.messages.len());
        for m in &req.messages {
            match m.role {
                Role::System => {
                    if !system_buf.is_empty() {
                        system_buf.push('\n');
                    }
                    system_buf.push_str(&m.content);
                }
                Role::User => msgs.push(AnthropicMessage { role: "user", content: &m.content }),
                Role::Assistant => {
                    msgs.push(AnthropicMessage { role: "assistant", content: &m.content });
                }
                Role::Tool => msgs.push(AnthropicMessage { role: "user", content: &m.content }),
            }
        }
        let body = AnthropicRequest {
            model: &req.model.name,
            messages: msgs,
            system: if system_buf.is_empty() { None } else { Some(system_buf) },
            max_tokens: req.max_tokens.unwrap_or(1024),
            temperature: req.temperature,
            stream: false,
        };
        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: AnthropicResponse = resp.json().await?;
        let content = parsed
            .content
            .iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("");
        let usage = parsed.usage.unwrap_or_default();
        Ok(ChatResponse {
            content,
            model: ModelId::new(
                Provider::Anthropic,
                parsed.model.unwrap_or_else(|| req.model.name.clone()),
            ),
            usage: Usage {
                prompt_tokens: usage.input_tokens,
                completion_tokens: usage.output_tokens,
            },
            finish_reason: parsed
                .stop_reason
                .as_deref()
                .map_or(FinishReason::Stop, finish_from_str),
        })
    }

    async fn complete_stream(
        &self,
        req: ChatRequest,
    ) -> RouterResult<Pin<Box<dyn Stream<Item = RouterResult<String>> + Send>>> {
        // V0: buffer the full body and emit a single chunk. SSE parsing is a
        // planned follow-up tracked by the project PLAN.
        tracing::warn!(target: "aphrody_router::anthropic", "streaming not yet implemented, buffering");
        let full = self.complete(req).await?;
        let stream = futures_util::stream::iter(vec![Ok(full.content)]);
        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Gemini
// ---------------------------------------------------------------------------

/// Gemini public endpoint.
pub const GEMINI_DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// HTTP provider for the Gemini family.
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl GeminiProvider {
    /// Build with the default endpoint.
    ///
    /// # Errors
    /// - [`RouterError::Http`] if reqwest client builder fails.
    pub fn new(api_key: impl Into<String>) -> RouterResult<Self> {
        Self::with_base_url(api_key, GEMINI_DEFAULT_BASE_URL)
    }

    /// Build with a custom base URL (tests, regional endpoints).
    ///
    /// # Errors
    /// - [`RouterError::Http`] if reqwest client builder fails.
    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> RouterResult<Self> {
        let client = build_client(concat!("aphrody-router/", env!("CARGO_PKG_VERSION")))?;
        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize, Default)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxOutputTokens")]
    max_output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(default, rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
    #[serde(default, rename = "modelVersion")]
    model_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiCandidateContent>,
    #[serde(default, rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidateContent {
    #[serde(default)]
    parts: Vec<GeminiCandidatePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidatePart {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct GeminiUsage {
    #[serde(default, rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(default, rename = "candidatesTokenCount")]
    candidates_token_count: u32,
}

#[async_trait]
impl ChatProvider for GeminiProvider {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }

    async fn health(&self) -> RouterResult<()> {
        // `/v1beta/models` lists models — cheap GET protected by the same key.
        let url = format!("{}/v1beta/models", self.base_url);
        let resp = self
            .client
            .get(url)
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await?;
        if resp.status().is_success() {
            return Ok(());
        }
        Err(lift_error(resp).await)
    }

    async fn complete(&self, req: ChatRequest) -> RouterResult<ChatResponse> {
        if req.model.provider != Provider::Gemini {
            return Err(RouterError::InvalidModel(req.model.to_string()));
        }
        let mut system_text = String::new();
        let mut contents: Vec<GeminiContent> = Vec::new();
        for m in &req.messages {
            match m.role {
                Role::System => {
                    if !system_text.is_empty() {
                        system_text.push('\n');
                    }
                    system_text.push_str(&m.content);
                }
                Role::User | Role::Tool => contents.push(GeminiContent {
                    role: "user".to_owned(),
                    parts: vec![GeminiPart { text: m.content.clone() }],
                }),
                Role::Assistant => contents.push(GeminiContent {
                    role: "model".to_owned(),
                    parts: vec![GeminiPart { text: m.content.clone() }],
                }),
            }
        }
        let generation_config = if req.temperature.is_some() || req.max_tokens.is_some() {
            Some(GeminiGenerationConfig {
                temperature: req.temperature,
                max_output_tokens: req.max_tokens,
            })
        } else {
            None
        };
        let body = GeminiRequest {
            contents,
            system_instruction: if system_text.is_empty() {
                None
            } else {
                Some(GeminiContent {
                    role: "system".to_owned(),
                    parts: vec![GeminiPart { text: system_text }],
                })
            },
            generation_config,
        };
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, req.model.name
        );
        let resp = self
            .client
            .post(url)
            .header("x-goog-api-key", &self.api_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: GeminiResponse = resp.json().await?;
        let first = parsed.candidates.into_iter().next();
        let (content, finish) = match first {
            Some(c) => {
                let text = c
                    .content
                    .map(|cc| {
                        cc.parts
                            .into_iter()
                            .filter_map(|p| p.text)
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();
                let finish = c
                    .finish_reason
                    .as_deref()
                    .map_or(FinishReason::Stop, finish_from_str);
                (text, finish)
            }
            None => (String::new(), FinishReason::Stop),
        };
        let usage = parsed.usage_metadata.unwrap_or_default();
        Ok(ChatResponse {
            content,
            model: ModelId::new(
                Provider::Gemini,
                parsed.model_version.unwrap_or_else(|| req.model.name.clone()),
            ),
            usage: Usage {
                prompt_tokens: usage.prompt_token_count,
                completion_tokens: usage.candidates_token_count,
            },
            finish_reason: finish,
        })
    }

    async fn complete_stream(
        &self,
        req: ChatRequest,
    ) -> RouterResult<Pin<Box<dyn Stream<Item = RouterResult<String>> + Send>>> {
        tracing::warn!(target: "aphrody_router::gemini", "streaming not yet implemented, buffering");
        let full = self.complete(req).await?;
        Ok(Box::pin(futures_util::stream::iter(vec![Ok(full.content)])))
    }
}

// ---------------------------------------------------------------------------
// Antigravity (Vertex AI)
// ---------------------------------------------------------------------------

/// Antigravity (Vertex AI) endpoint placeholder.
///
/// Production deployments inject the real Vertex AI regional endpoint via
/// [`AntigravityProvider::with_base_url`].
pub const ANTIGRAVITY_DEFAULT_BASE_URL: &str = "https://aiplatform.googleapis.com";

/// HTTP provider for Vertex AI Antigravity.
///
/// Vertex AI uses short-lived OAuth tokens; this implementation accepts the
/// token as an opaque string ("Bearer …" header value) — fetching & refreshing
/// it is the caller's responsibility (typically via `gcloud auth print-access-token`
/// or a service-account ADC).
#[derive(Debug, Clone)]
pub struct AntigravityProvider {
    client: reqwest::Client,
    access_token: String,
    base_url: String,
}

impl AntigravityProvider {
    /// Build pointing at the default Vertex AI host.
    ///
    /// # Errors
    /// - [`RouterError::Http`] if reqwest client builder fails.
    pub fn new(access_token: impl Into<String>) -> RouterResult<Self> {
        Self::with_base_url(access_token, ANTIGRAVITY_DEFAULT_BASE_URL)
    }

    /// Build with a custom base URL.
    ///
    /// # Errors
    /// - [`RouterError::Http`] if reqwest client builder fails.
    pub fn with_base_url(
        access_token: impl Into<String>,
        base_url: impl Into<String>,
    ) -> RouterResult<Self> {
        let client = build_client(concat!("aphrody-router/", env!("CARGO_PKG_VERSION")))?;
        Ok(Self {
            client,
            access_token: access_token.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    fn bearer(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

#[derive(Debug, Serialize)]
struct AntigravityRequest {
    model: String,
    messages: Vec<AntigravityMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AntigravityMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AntigravityResponse {
    #[serde(default)]
    content: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    finish_reason: Option<String>,
    #[serde(default)]
    usage: Option<AntigravityUsage>,
}

#[derive(Debug, Deserialize, Default)]
struct AntigravityUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[async_trait]
impl ChatProvider for AntigravityProvider {
    fn provider(&self) -> Provider {
        Provider::Antigravity
    }

    async fn health(&self) -> RouterResult<()> {
        let resp = self
            .client
            .get(format!("{}/v1/health", self.base_url))
            .header(reqwest::header::AUTHORIZATION, self.bearer())
            .send()
            .await?;
        if resp.status().is_success() {
            return Ok(());
        }
        Err(lift_error(resp).await)
    }

    async fn complete(&self, req: ChatRequest) -> RouterResult<ChatResponse> {
        if req.model.provider != Provider::Antigravity {
            return Err(RouterError::InvalidModel(req.model.to_string()));
        }
        let messages = req
            .messages
            .iter()
            .map(|m| AntigravityMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                }
                .to_owned(),
                content: m.content.clone(),
            })
            .collect();
        let body = AntigravityRequest {
            model: req.model.name.clone(),
            messages,
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            stream: false,
        };
        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header(reqwest::header::AUTHORIZATION, self.bearer())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: AntigravityResponse = resp.json().await?;
        let usage = parsed.usage.unwrap_or_default();
        Ok(ChatResponse {
            content: parsed.content,
            model: ModelId::new(
                Provider::Antigravity,
                parsed.model.unwrap_or_else(|| req.model.name.clone()),
            ),
            usage: Usage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
            },
            finish_reason: parsed
                .finish_reason
                .as_deref()
                .map_or(FinishReason::Stop, finish_from_str),
        })
    }

    async fn complete_stream(
        &self,
        req: ChatRequest,
    ) -> RouterResult<Pin<Box<dyn Stream<Item = RouterResult<String>> + Send>>> {
        tracing::warn!(target: "aphrody_router::antigravity", "streaming not yet implemented, buffering");
        let full = self.complete(req).await?;
        Ok(Box::pin(futures_util::stream::iter(vec![Ok(full.content)])))
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Dispatching front-end — owns one provider per [`Provider`] variant.
#[derive(Default)]
pub struct Router {
    providers: HashMap<Provider, Box<dyn ChatProvider>>,
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("registered", &self.providers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Router {
    /// Build an empty router.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `provider` for its declared [`Provider::provider`] slot. The
    /// previous entry (if any) is dropped — exactly one provider per variant.
    pub fn add_provider(&mut self, provider: Box<dyn ChatProvider>) {
        self.providers.insert(provider.provider(), provider);
    }

    /// True when `variant` has a registered provider.
    #[must_use]
    pub fn has(&self, variant: Provider) -> bool {
        self.providers.contains_key(&variant)
    }

    /// Dispatch `req` to the provider matching `req.model.provider`.
    ///
    /// # Errors
    /// - [`RouterError::NoSuchProvider`] when no provider is registered.
    /// - Any error bubbled from the underlying [`ChatProvider`] implementation.
    pub async fn route(&self, req: ChatRequest) -> RouterResult<ChatResponse> {
        let variant = req.model.provider;
        let provider = self
            .providers
            .get(&variant)
            .ok_or(RouterError::NoSuchProvider(variant))?;
        provider.complete(req).await
    }
}

// ---------------------------------------------------------------------------
// Unit tests — pure logic, no HTTP
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_parse_accepts_whitelist() {
        assert_eq!(Provider::parse("anthropic").unwrap(), Provider::Anthropic);
        assert_eq!(Provider::parse("GEMINI").unwrap(), Provider::Gemini);
        assert_eq!(Provider::parse("  antigravity  ").unwrap(), Provider::Antigravity);
    }

    #[test]
    fn provider_parse_rejects_unknown() {
        // Direct `Provider::parse` now returns the dedicated `ProviderError`
        // (since the enum lives in the `aphrody-providers` micro-crate). The
        // legacy `RouterError::UnsupportedProvider` variant is still produced
        // when `?`-converted from a `Provider::parse` failure inside
        // router-owned code paths (see `model_id_parse_rejects_bad_provider`).
        let err = Provider::parse("foo").unwrap_err();
        match err {
            ProviderError::Unsupported(s) => assert_eq!(s, "foo"),
        }
    }

    #[test]
    fn model_id_parse_rejects_bad_provider() {
        // `ModelId::parse` returns `RouterError`; the `ProviderError` from
        // `Provider::parse` must surface as `RouterError::UnsupportedProvider`
        // through the `From` impl so downstream `?` keeps working.
        let err = ModelId::parse("openai/gpt-5").unwrap_err();
        match err {
            RouterError::UnsupportedProvider(s) => assert_eq!(s, "openai"),
            other => panic!("expected UnsupportedProvider, got {other:?}"),
        }
    }

    #[test]
    fn model_id_round_trips_via_display_parse() {
        let m = ModelId::new(Provider::Anthropic, "claude-opus-4-7");
        assert_eq!(m.to_string(), "anthropic/claude-opus-4-7");
        let parsed = ModelId::parse(&m.to_string()).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn model_id_parse_rejects_missing_separator() {
        assert!(matches!(
            ModelId::parse("noslash").unwrap_err(),
            RouterError::InvalidModel(_)
        ));
    }

    #[test]
    fn router_returns_no_such_provider_when_empty() {
        let r = Router::new();
        let req = ChatRequest::one_shot(
            ModelId::new(Provider::Anthropic, "claude-opus-4-7"),
            "hi",
        );
        // We need to drop the future without awaiting; use a synchronous proxy.
        let variant = req.model.provider;
        assert!(!r.has(variant));
    }

    #[test]
    fn finish_reason_maps_known_strings() {
        assert!(matches!(finish_from_str("end_turn"), FinishReason::Stop));
        assert!(matches!(finish_from_str("max_tokens"), FinishReason::Length));
        assert!(matches!(finish_from_str("tool_use"), FinishReason::ToolCalls));
        assert!(matches!(finish_from_str("unknown"), FinishReason::Stop));
    }
}
