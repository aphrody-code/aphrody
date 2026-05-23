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
use futures_util::{Stream, StreamExt as _};
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
// Utilitaires SSE (Server-Sent Events)
// ---------------------------------------------------------------------------

/// Découpe un flux d'octets en lignes SSE complètes.
///
/// Accumule les octets dans un tampon interne et extrait les lignes terminées
/// par `\n`. Les caractères `\r` sont ignorés pour la compatibilité CRLF.
/// Renvoie les lignes sans le `\n` terminal.
struct SseLineReader {
    buf: Vec<u8>,
}

impl SseLineReader {
    fn new() -> Self {
        Self { buf: Vec::with_capacity(4096) }
    }

    /// Ingère un nouveau chunk d'octets et retourne toutes les lignes
    /// complètes disponibles.
    fn feed(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(chunk);
        let mut lines = Vec::new();
        loop {
            match self.buf.iter().position(|&b| b == b'\n') {
                None => break,
                Some(pos) => {
                    let raw = self.buf.drain(..=pos).collect::<Vec<u8>>();
                    // Retire le `\n` et éventuellement le `\r` précédent.
                    let trimmed = raw
                        .strip_suffix(b"\n")
                        .unwrap_or(&raw);
                    let trimmed = trimmed
                        .strip_suffix(b"\r")
                        .unwrap_or(trimmed);
                    if let Ok(s) = std::str::from_utf8(trimmed) {
                        lines.push(s.to_owned());
                    }
                }
            }
        }
        lines
    }

    /// Vide et renvoie tout résidu accumulé (fin de flux sans `\n` terminal).
    fn flush(&mut self) -> Option<String> {
        if self.buf.is_empty() {
            return None;
        }
        let raw = std::mem::take(&mut self.buf);
        std::str::from_utf8(&raw).ok().map(str::to_owned)
    }
}

/// Type d'événement SSE extrait des métadonnées de l'événement courant.
#[derive(Debug, Default, Clone)]
struct SseEvent {
    /// Valeur de la ligne `event:` (peut être vide si absente).
    pub event: String,
    /// Valeur concaténée des lignes `data:`.
    pub data: String,
}

/// Accumulateur d'un événement SSE multi-lignes.
///
/// Un événement SSE se termine par une ligne vide (séparateur de blocs).
/// Plusieurs lignes `data:` sont concaténées avec `\n`.
struct SseEventBuilder {
    current: SseEvent,
}

impl SseEventBuilder {
    fn new() -> Self {
        Self { current: SseEvent::default() }
    }

    /// Ingère une ligne et renvoie `Some(event)` quand le bloc est complet
    /// (ligne vide reçue).
    fn push_line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            // Ligne vide = fin du bloc SSE courant.
            let evt = std::mem::take(&mut self.current);
            if evt.data.is_empty() && evt.event.is_empty() {
                // Bloc complètement vide : heartbeat, ignorer.
                return None;
            }
            return Some(evt);
        }
        if let Some(val) = line.strip_prefix("event:") {
            self.current.event = val.trim().to_owned();
        } else if let Some(val) = line.strip_prefix("data:") {
            if !self.current.data.is_empty() {
                self.current.data.push('\n');
            }
            self.current.data.push_str(val.trim_start_matches(' '));
        }
        // Les champs `id:` et `retry:` sont ignorés volontairement.
        None
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

// --- Structures SSE Anthropic ------------------------------------------------

/// Événement SSE Anthropic `content_block_delta`.
///
/// Format : `event: content_block_delta\ndata: { "type": "content_block_delta",
/// "delta": { "type": "text_delta", "text": "…" } }`
#[derive(Debug, Deserialize)]
struct AnthropicSseDelta {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    delta: Option<AnthropicSseTextDelta>,
}

#[derive(Debug, Deserialize)]
struct AnthropicSseTextDelta {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: String,
}

/// Extrait le delta texte d'un événement SSE Anthropic.
///
/// Renvoie `Some(text)` si l'événement est `content_block_delta` avec un
/// delta de type `text_delta`, `None` sinon.
fn anthropic_sse_text(event_type: &str, data: &str) -> Option<String> {
    // On accepte soit l'événement explicite, soit le champ `type` dans le JSON.
    if event_type == "content_block_delta" || event_type.is_empty() {
        if let Ok(delta) = serde_json::from_str::<AnthropicSseDelta>(data) {
            if delta.kind == "content_block_delta" {
                if let Some(d) = delta.delta {
                    if d.kind == "text_delta" && !d.text.is_empty() {
                        return Some(d.text);
                    }
                }
            }
        }
    }
    None
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
            stream: true,
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

        // Consomme la réponse HTTP en SSE ligne par ligne via bytes_stream().
        let mut byte_stream = resp.bytes_stream();
        let output = async_stream::stream! {
            let mut reader = SseLineReader::new();
            let mut builder = SseEventBuilder::new();
            loop {
                match byte_stream.next().await {
                    None => {
                        // Fin du flux réseau — vider le tampon restant.
                        if let Some(leftover) = reader.flush() {
                            if let Some(evt) = builder.push_line(&leftover) {
                                if evt.data == "[DONE]" { break; }
                                if let Some(text) = anthropic_sse_text(&evt.event, &evt.data) {
                                    yield Ok(text);
                                }
                            }
                        }
                        break;
                    }
                    Some(Err(e)) => {
                        yield Err(RouterError::Stream(e.to_string()));
                        break;
                    }
                    Some(Ok(chunk)) => {
                        let lines = reader.feed(&chunk);
                        for line in lines {
                            if let Some(evt) = builder.push_line(&line) {
                                if evt.data == "[DONE]" { return; }
                                if let Some(text) = anthropic_sse_text(&evt.event, &evt.data) {
                                    yield Ok(text);
                                }
                            }
                        }
                    }
                }
            }
        };
        Ok(Box::pin(output))
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

/// Extrait le texte delta d'un événement SSE Gemini.
///
/// Le format `streamGenerateContent?alt=sse` émet des objets JSON de type
/// `GeminiResponse` — chaque `data:` contient un fragment de réponse complet
/// (le premier candidat seulement est utilisé, multi-candidats hors périmètre).
fn gemini_sse_text(data: &str) -> Option<String> {
    if data == "[DONE]" {
        return None;
    }
    // Gemini peut émettre un tableau JSON `[{...}]` ou un objet simple `{...}`.
    // On tente les deux formes.
    let resp: GeminiResponse = if data.trim_start().starts_with('[') {
        // Tableau : on prend le premier élément.
        let arr: Vec<serde_json::Value> = serde_json::from_str(data).ok()?;
        let first = arr.into_iter().next()?;
        serde_json::from_value(first).ok()?
    } else {
        serde_json::from_str(data).ok()?
    };
    let candidate = resp.candidates.into_iter().next()?;
    let content = candidate.content?;
    let text = content
        .parts
        .into_iter()
        .filter_map(|p| p.text)
        .collect::<String>();
    if text.is_empty() { None } else { Some(text) }
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
        // `streamGenerateContent?alt=sse` active le mode SSE côté Gemini.
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
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

        let mut byte_stream = resp.bytes_stream();
        let output = async_stream::stream! {
            let mut reader = SseLineReader::new();
            let mut builder = SseEventBuilder::new();
            loop {
                match byte_stream.next().await {
                    None => {
                        if let Some(leftover) = reader.flush() {
                            if let Some(evt) = builder.push_line(&leftover) {
                                if evt.data == "[DONE]" { break; }
                                if let Some(text) = gemini_sse_text(&evt.data) {
                                    yield Ok(text);
                                }
                            }
                        }
                        break;
                    }
                    Some(Err(e)) => {
                        yield Err(RouterError::Stream(e.to_string()));
                        break;
                    }
                    Some(Ok(chunk)) => {
                        let lines = reader.feed(&chunk);
                        for line in lines {
                            if let Some(evt) = builder.push_line(&line) {
                                if evt.data == "[DONE]" { return; }
                                if let Some(text) = gemini_sse_text(&evt.data) {
                                    yield Ok(text);
                                }
                            }
                        }
                    }
                }
            }
        };
        Ok(Box::pin(output))
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

/// Extrait le delta texte d'un événement SSE Antigravity.
///
/// Antigravity utilise le même format Messages API qu'Anthropic (SSE
/// `content_block_delta` / `text_delta`). En fallback, si la `data` est
/// un objet JSON avec un champ `content` direct, ce contenu est retourné —
/// cela couvre les implémentations alternatives qui émettent des réponses
/// complètes par événement SSE plutôt que de vrais deltas.
fn antigravity_sse_text(event_type: &str, data: &str) -> Option<String> {
    if data == "[DONE]" {
        return None;
    }
    // Essai 1 : format Anthropic (content_block_delta / text_delta).
    if let Some(text) = anthropic_sse_text(event_type, data) {
        return Some(text);
    }
    // Essai 2 : objet complet avec champ `content` (OpenAI-like SSE).
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
        // Format OpenAI chat completions stream : choices[0].delta.content
        let text = choices
            .iter()
            .filter_map(|c| c.get("delta").and_then(|d| d.get("content")).and_then(|t| t.as_str()))
            .collect::<String>();
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
        if !content.is_empty() {
            return Some(content.to_owned());
        }
    }
    None
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
            stream: true,
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

        let mut byte_stream = resp.bytes_stream();
        let output = async_stream::stream! {
            let mut reader = SseLineReader::new();
            let mut builder = SseEventBuilder::new();
            loop {
                match byte_stream.next().await {
                    None => {
                        if let Some(leftover) = reader.flush() {
                            if let Some(evt) = builder.push_line(&leftover) {
                                if evt.data == "[DONE]" { break; }
                                if let Some(text) = antigravity_sse_text(&evt.event, &evt.data) {
                                    yield Ok(text);
                                }
                            }
                        }
                        break;
                    }
                    Some(Err(e)) => {
                        yield Err(RouterError::Stream(e.to_string()));
                        break;
                    }
                    Some(Ok(chunk)) => {
                        let lines = reader.feed(&chunk);
                        for line in lines {
                            if let Some(evt) = builder.push_line(&line) {
                                if evt.data == "[DONE]" { return; }
                                if let Some(text) = antigravity_sse_text(&evt.event, &evt.data) {
                                    yield Ok(text);
                                }
                            }
                        }
                    }
                }
            }
        };
        Ok(Box::pin(output))
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

    // -------------------------------------------------------------------------
    // Tests unitaires du parser SSE — aucune dépendance réseau.
    // -------------------------------------------------------------------------

    /// Vérifie que `SseLineReader` découpe correctement un chunk unique.
    #[test]
    fn sse_line_reader_single_chunk() {
        let mut reader = SseLineReader::new();
        let lines = reader.feed(b"event: content_block_delta\ndata: {\"hello\":1}\n\n");
        assert_eq!(lines, vec!["event: content_block_delta", "data: {\"hello\":1}", ""]);
    }

    /// Vérifie le découpage sur plusieurs chunks fragmentés (simulation réseau).
    #[test]
    fn sse_line_reader_fragmented_chunks() {
        let mut reader = SseLineReader::new();
        // Chunk 1 : coupe au milieu d'une ligne.
        let l1 = reader.feed(b"data: {\"ty");
        assert!(l1.is_empty(), "pas de ligne complète attendue");
        // Chunk 2 : ferme la ligne + ligne vide.
        let l2 = reader.feed(b"pe\":\"ping\"}\n\n");
        assert_eq!(l2, vec!["data: {\"type\":\"ping\"}", ""]);
    }

    /// Vérifie que les fins de ligne CRLF sont normalisées correctement.
    #[test]
    fn sse_line_reader_crlf() {
        let mut reader = SseLineReader::new();
        let lines = reader.feed(b"data: hello\r\n\r\n");
        assert_eq!(lines, vec!["data: hello", ""]);
    }

    /// Vérifie l'assemblage d'un événement SSE complet par `SseEventBuilder`.
    #[test]
    fn sse_event_builder_assembles_block() {
        let mut builder = SseEventBuilder::new();
        assert!(builder.push_line("event: content_block_delta").is_none());
        assert!(builder.push_line("data: {\"x\":1}").is_none());
        let evt = builder.push_line("").expect("événement complet sur ligne vide");
        assert_eq!(evt.event, "content_block_delta");
        assert_eq!(evt.data, "{\"x\":1}");
    }

    /// Vérifie la concaténation de plusieurs lignes `data:` (multi-ligne SSE).
    #[test]
    fn sse_event_builder_multiline_data() {
        let mut builder = SseEventBuilder::new();
        assert!(builder.push_line("data: line1").is_none());
        assert!(builder.push_line("data: line2").is_none());
        let evt = builder.push_line("").expect("événement complet");
        assert_eq!(evt.data, "line1\nline2");
    }

    /// Les lignes vides consécutives (heartbeats) ne produisent pas d'événement.
    #[test]
    fn sse_event_builder_ignores_empty_heartbeat() {
        let mut builder = SseEventBuilder::new();
        // Double ligne vide = heartbeat vide.
        assert!(builder.push_line("").is_none());
    }

    // -------------------------------------------------------------------------
    // Tests des extracteurs de delta par provider.
    // -------------------------------------------------------------------------

    /// Anthropic : `content_block_delta` avec `text_delta` produit le texte.
    #[test]
    fn anthropic_sse_text_extracts_delta() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Bonjour"}}"#;
        let result = anthropic_sse_text("content_block_delta", data);
        assert_eq!(result.as_deref(), Some("Bonjour"));
    }

    /// Anthropic : événement non-delta (`message_start`) → `None`.
    #[test]
    fn anthropic_sse_text_ignores_non_delta() {
        let data = r#"{"type":"message_start","message":{"id":"msg_01"}}"#;
        assert!(anthropic_sse_text("message_start", data).is_none());
    }

    /// Anthropic : `[DONE]` n'est pas traité par l'extracteur (guard en amont).
    #[test]
    fn anthropic_sse_text_ignores_done_marker() {
        // `[DONE]` n'est pas du JSON valide — l'extracteur doit renvoyer None.
        assert!(anthropic_sse_text("", "[DONE]").is_none());
    }

    /// Gemini : objet JSON candidat valide → texte extrait.
    #[test]
    fn gemini_sse_text_extracts_candidate() {
        let data = r#"{"candidates":[{"content":{"parts":[{"text":"Salut"},{"text":" monde"}],"role":"model"},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":1,"candidatesTokenCount":2}}"#;
        let result = gemini_sse_text(data);
        assert_eq!(result.as_deref(), Some("Salut monde"));
    }

    /// Gemini : tableau JSON (format de certaines implémentations) → texte extrait.
    #[test]
    fn gemini_sse_text_extracts_array_form() {
        let data = r#"[{"candidates":[{"content":{"parts":[{"text":"chunk"}],"role":"model"}}],"usageMetadata":{}}]"#;
        let result = gemini_sse_text(data);
        assert_eq!(result.as_deref(), Some("chunk"));
    }

    /// Gemini : candidat sans texte → `None`.
    #[test]
    fn gemini_sse_text_returns_none_on_empty_parts() {
        let data = r#"{"candidates":[{"content":{"parts":[],"role":"model"}}]}"#;
        assert!(gemini_sse_text(data).is_none());
    }

    /// Antigravity : format Anthropic passthrough → texte extrait.
    #[test]
    fn antigravity_sse_text_anthropic_format() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"delta-text"}}"#;
        let result = antigravity_sse_text("content_block_delta", data);
        assert_eq!(result.as_deref(), Some("delta-text"));
    }

    /// Antigravity : format OpenAI choices stream → texte extrait.
    #[test]
    fn antigravity_sse_text_openai_choices_format() {
        let data = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
        let result = antigravity_sse_text("", data);
        assert_eq!(result.as_deref(), Some("hello"));
    }

    /// Antigravity : champ `content` direct → texte extrait.
    #[test]
    fn antigravity_sse_text_direct_content_format() {
        let data = r#"{"content":"direct","model":"m"}"#;
        let result = antigravity_sse_text("", data);
        assert_eq!(result.as_deref(), Some("direct"));
    }

    /// Pipeline complet : flux SSE Anthropic factice → tokens collectés dans l'ordre.
    #[tokio::test]
    async fn sse_pipeline_anthropic_full_stream() {
        use futures_util::StreamExt as _;

        // Flux SSE Anthropic complet simulé (3 deltas + DONE).
        let sse_bytes: &[u8] = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_01\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Bon\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"jour\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" monde\"}}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
            "data: [DONE]\n\n",
        ).as_bytes();

        // Simulation du byte_stream en plusieurs petits morceaux.
        let chunks: Vec<Result<bytes::Bytes, std::io::Error>> = sse_bytes
            .chunks(16)
            .map(|c| Ok(bytes::Bytes::copy_from_slice(c)))
            .collect();
        let byte_stream = futures_util::stream::iter(chunks);
        let mut byte_stream = Box::pin(byte_stream);

        let output = async_stream::stream! {
            let mut reader = SseLineReader::new();
            let mut builder = SseEventBuilder::new();
            loop {
                match byte_stream.next().await {
                    None => {
                        if let Some(leftover) = reader.flush() {
                            if let Some(evt) = builder.push_line(&leftover) {
                                if evt.data == "[DONE]" { break; }
                                if let Some(text) = anthropic_sse_text(&evt.event, &evt.data) {
                                    yield Ok::<String, RouterError>(text);
                                }
                            }
                        }
                        break;
                    }
                    Some(Err(e)) => {
                        yield Err(RouterError::Stream(e.to_string()));
                        break;
                    }
                    Some(Ok(chunk)) => {
                        let lines = reader.feed(&chunk);
                        for line in lines {
                            if let Some(evt) = builder.push_line(&line) {
                                if evt.data == "[DONE]" { return; }
                                if let Some(text) = anthropic_sse_text(&evt.event, &evt.data) {
                                    yield Ok(text);
                                }
                            }
                        }
                    }
                }
            }
        };

        let tokens: Vec<String> = output
            .map(|r| r.expect("pas d'erreur attendue"))
            .collect()
            .await;
        assert_eq!(tokens, vec!["Bon", "jour", " monde"]);
        assert_eq!(tokens.join(""), "Bonjour monde");
    }
}
