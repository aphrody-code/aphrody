// SPDX-License-Identifier: Apache-2.0
//! Mem0 (<https://api.mem0.ai>) `MemoryProvider` implementation.
//!
//! Wire reference: <https://docs.mem0.ai>.
//!
//! Authentication is a single header — `Authorization: Token {api_key}`.
//! The base URL is overridable via [`Mem0Provider::with_base_url`] so the
//! smoke tests can point at a local axum mock without touching the network.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::MemoryProvider;
use crate::types::{MemoryError, MemoryQuery, MemoryRecord, ProviderKind, now_unix_ms};

/// Public Mem0 cloud endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.mem0.ai";

/// HTTP-backed Mem0 provider.
#[derive(Debug, Clone)]
pub struct Mem0Provider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl Mem0Provider {
    /// Build a provider with the default base URL.
    ///
    /// # Errors
    /// - [`MemoryError::Http`] if the reqwest client builder fails (TLS init).
    pub fn new(api_key: impl Into<String>) -> Result<Self, MemoryError> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    /// Build a provider pointing at an arbitrary base URL (self-hosted, tests).
    ///
    /// # Errors
    /// - [`MemoryError::Http`] if the reqwest client builder fails.
    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, MemoryError> {
        // The CLI installs a CryptoProvider at startup; tests rely on a
        // per-binary `install_crypto_provider` helper (see tests).
        let client = Client::builder()
            .user_agent(concat!("aphrody-memory/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    /// Read `MEM0_API_KEY` from the process environment.
    ///
    /// # Errors
    /// - [`MemoryError::MissingConfig`] if the env var is unset or empty.
    /// - [`MemoryError::Http`] if the client builder fails.
    pub fn from_env() -> Result<Self, MemoryError> {
        let key = std::env::var("MEM0_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(MemoryError::MissingConfig("MEM0_API_KEY"))?;
        Self::new(key)
    }

    fn auth_header(&self) -> String {
        format!("Token {}", self.api_key)
    }

    fn endpoint(&self, suffix: &str) -> String {
        format!("{}{}", self.base_url, suffix)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Wire shapes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AddBody<'a> {
    user_id: &'a str,
    messages: Vec<Mem0Message<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    categories: Vec<&'a str>,
}

#[derive(Debug, Serialize)]
struct Mem0Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct AddResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GetResponse {
    id: String,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    memory: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

impl GetResponse {
    fn into_record(self) -> MemoryRecord {
        // Mem0 stores the memory text in `memory`. Created-at is provider-side
        // ISO 8601; we parse to epoch ms when possible, falling back to now.
        let created_at_unix_ms = self
            .created_at
            .as_deref()
            .and_then(parse_iso_to_unix_ms)
            .unwrap_or_else(now_unix_ms);
        MemoryRecord {
            id: self.id,
            agent_id: self.user_id.unwrap_or_default(),
            content: self.memory.unwrap_or_default(),
            tags: self.categories,
            created_at_unix_ms,
            metadata: self.metadata.unwrap_or_else(|| serde_json::json!({})),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiErrorEnvelope {
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

fn parse_iso_to_unix_ms(s: &str) -> Option<u64> {
    // We accept anything chrono can swallow; tolerate failure quietly because
    // Mem0 occasionally returns naive timestamps without zone offsets.
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| u64::try_from(dt.timestamp_millis()).unwrap_or(0))
}

async fn lift_error(resp: reqwest::Response) -> MemoryError {
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    let message = serde_json::from_str::<ApiErrorEnvelope>(&body)
        .ok()
        .and_then(|env| env.detail.or(env.message).or(env.error))
        .unwrap_or(body);
    MemoryError::ApiError { status, message }
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryProvider impl
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryProvider for Mem0Provider {
    async fn add(&self, rec: MemoryRecord) -> Result<String, MemoryError> {
        if rec.agent_id.is_empty() {
            return Err(MemoryError::InvalidArgument("agent_id is required".into()));
        }
        let tag_refs: Vec<&str> = rec.tags.iter().map(String::as_str).collect();
        let body = AddBody {
            user_id: &rec.agent_id,
            messages: vec![Mem0Message { role: "user", content: &rec.content }],
            categories: tag_refs,
        };
        let resp = self
            .client
            .post(self.endpoint("/v1/memories/"))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: AddResponse = resp.json().await?;
        Ok(parsed.id)
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        let resp = self
            .client
            .get(self.endpoint(&format!("/v1/memories/{id}")))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: GetResponse = resp.json().await?;
        Ok(Some(parsed.into_record()))
    }

    async fn search(&self, q: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        let mut query_pairs: Vec<(&str, String)> = Vec::with_capacity(4);
        query_pairs.push(("user_id", q.agent_id));
        if let Some(needle) = q.q {
            query_pairs.push(("q", needle));
        }
        if let Some(limit) = q.limit {
            query_pairs.push(("limit", limit.to_string()));
        }
        let resp = self
            .client
            .get(self.endpoint("/v1/memories/search"))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .query(&query_pairs)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let list: Vec<GetResponse> = resp.json().await?;
        let mut out: Vec<MemoryRecord> = list.into_iter().map(GetResponse::into_record).collect();
        if !q.tags.is_empty() {
            out.retain(|r| q.tags.iter().all(|t| r.tags.iter().any(|rt| rt == t)));
        }
        Ok(out)
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        let resp = self
            .client
            .delete(self.endpoint(&format!("/v1/memories/{id}")))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .send()
            .await?;
        // 404 → idempotent success.
        if resp.status() == reqwest::StatusCode::NOT_FOUND || resp.status().is_success() {
            return Ok(());
        }
        Err(lift_error(resp).await)
    }

    async fn health(&self) -> Result<(), MemoryError> {
        // Mem0 publishes /v1/memories/ as the canonical listing endpoint; a
        // GET with no body is cheap and verifies auth + reachability.
        let resp = self
            .client
            .get(self.endpoint("/v1/memories/"))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .send()
            .await?;
        if resp.status().is_success()
            || resp.status() == reqwest::StatusCode::NO_CONTENT
        {
            return Ok(());
        }
        Err(lift_error(resp).await)
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Mem0
    }
}
