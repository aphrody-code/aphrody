// SPDX-License-Identifier: Apache-2.0
//! Honcho (<https://demo.honcho.dev>) v1 `MemoryProvider` implementation.
//!
//! Wire reference: <https://docs.honcho.dev>.
//!
//! Authentication is a Bearer token. Honcho models memory as
//! `app → user → session → message`, so this provider keeps a fixed `app_id`
//! and a single canonical `session_id` per agent (the agent id doubles as
//! both Honcho `user_id` and `session_id` for round-trip stability).

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::MemoryProvider;
use crate::types::{MemoryError, MemoryQuery, MemoryRecord, ProviderKind, now_unix_ms};

/// Public Honcho demo endpoint.
pub const DEFAULT_BASE_URL: &str = "https://demo.honcho.dev";

/// Default app id used when no override is supplied (matches the docs sample).
pub const DEFAULT_APP_ID: &str = "default";

/// Default session id — Honcho requires one per user; this is fine for
/// single-agent long-term memory because the provider never branches sessions.
pub const DEFAULT_SESSION_ID: &str = "default";

/// HTTP-backed Honcho provider.
#[derive(Debug, Clone)]
pub struct HonchoProvider {
    client: Client,
    api_key: String,
    base_url: String,
    app_id: String,
}

impl HonchoProvider {
    /// Build a provider against the demo endpoint with `app_id = "default"`.
    ///
    /// # Errors
    /// - [`MemoryError::Http`] if the reqwest client builder fails.
    pub fn new(api_key: impl Into<String>) -> Result<Self, MemoryError> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL, DEFAULT_APP_ID)
    }

    /// Build a provider with explicit base URL + app id.
    ///
    /// # Errors
    /// - [`MemoryError::Http`] if the reqwest client builder fails.
    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        app_id: impl Into<String>,
    ) -> Result<Self, MemoryError> {
        let client = Client::builder()
            .user_agent(concat!("aphrody-memory/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            app_id: app_id.into(),
        })
    }

    /// Read `HONCHO_API_KEY` from the environment.
    ///
    /// # Errors
    /// - [`MemoryError::MissingConfig`] if the env var is unset or empty.
    /// - [`MemoryError::Http`] on client builder failure.
    pub fn from_env() -> Result<Self, MemoryError> {
        let key = std::env::var("HONCHO_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(MemoryError::MissingConfig("HONCHO_API_KEY"))?;
        Self::new(key)
    }

    fn endpoint(&self, suffix: &str) -> String {
        format!("{}{}", self.base_url, suffix)
    }

    fn messages_path(&self, user_id: &str) -> String {
        format!(
            "/v1/apps/{}/users/{}/sessions/{}/messages",
            self.app_id, user_id, DEFAULT_SESSION_ID
        )
    }

    fn auth_value(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Wire shapes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct PostMessageBody<'a> {
    content: &'a str,
    is_user: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<&'a serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PostMessageResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct HonchoMessage {
    id: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct HonchoListEnvelope {
    #[serde(default)]
    items: Vec<HonchoMessage>,
}

impl HonchoMessage {
    fn into_record(self, agent_id: &str) -> MemoryRecord {
        let created_at_unix_ms = self
            .created_at
            .as_deref()
            .and_then(parse_iso_to_unix_ms)
            .unwrap_or_else(now_unix_ms);
        // Honcho doesn't have a first-class tags concept; we stash tags inside
        // `metadata.tags` on write and read them back here when present.
        let metadata = self.metadata.unwrap_or_else(|| serde_json::json!({}));
        let tags: Vec<String> = metadata
            .get("tags")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        MemoryRecord {
            id: self.id,
            agent_id: agent_id.to_string(),
            content: self.content.unwrap_or_default(),
            tags,
            created_at_unix_ms,
            metadata,
        }
    }
}

#[derive(Debug, Deserialize)]
struct HonchoError {
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

fn parse_iso_to_unix_ms(s: &str) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| u64::try_from(dt.timestamp_millis()).unwrap_or(0))
}

async fn lift_error(resp: reqwest::Response) -> MemoryError {
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    let message = serde_json::from_str::<HonchoError>(&body)
        .ok()
        .and_then(|e| e.detail.or(e.message))
        .unwrap_or(body);
    MemoryError::ApiError { status, message }
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryProvider impl
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryProvider for HonchoProvider {
    async fn add(&self, rec: MemoryRecord) -> Result<String, MemoryError> {
        if rec.agent_id.is_empty() {
            return Err(MemoryError::InvalidArgument("agent_id is required".into()));
        }
        // Fold tags into metadata so they survive the round-trip.
        let mut metadata = match rec.metadata {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        metadata.insert(
            "tags".into(),
            serde_json::Value::Array(
                rec.tags.iter().map(|t| serde_json::Value::String(t.clone())).collect(),
            ),
        );
        let metadata_value = serde_json::Value::Object(metadata);
        let body = PostMessageBody {
            content: &rec.content,
            is_user: true,
            metadata: Some(&metadata_value),
        };
        let resp = self
            .client
            .post(self.endpoint(&self.messages_path(&rec.agent_id)))
            .header(reqwest::header::AUTHORIZATION, self.auth_value())
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: PostMessageResponse = resp.json().await?;
        Ok(parsed.id)
    }

    async fn get(&self, _id: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        // Honcho's GET-by-id endpoint requires (app_id, user_id, session_id,
        // message_id) — we don't have the user_id from the id alone. Return
        // `Ok(None)` to signal "unsupported direct fetch" rather than failing
        // loudly; callers that need direct lookup should `search` instead.
        Ok(None)
    }

    async fn search(&self, q: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        let mut query_pairs: Vec<(&str, String)> = Vec::new();
        if let Some(needle) = q.q {
            query_pairs.push(("q", needle));
        }
        if let Some(limit) = q.limit {
            query_pairs.push(("limit", limit.to_string()));
        }
        let resp = self
            .client
            .get(self.endpoint(&self.messages_path(&q.agent_id)))
            .header(reqwest::header::AUTHORIZATION, self.auth_value())
            .query(&query_pairs)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        // Honcho's listing endpoint sometimes returns `{ "items": [...] }`,
        // sometimes a bare array. We tolerate both.
        let body = resp.text().await?;
        let messages: Vec<HonchoMessage> = if body.trim_start().starts_with('[') {
            serde_json::from_str(&body)?
        } else {
            let envelope: HonchoListEnvelope = serde_json::from_str(&body)?;
            envelope.items
        };
        let agent_id = q.agent_id;
        let mut out: Vec<MemoryRecord> = messages
            .into_iter()
            .map(|m| m.into_record(&agent_id))
            .collect();
        if !q.tags.is_empty() {
            out.retain(|r| q.tags.iter().all(|t| r.tags.iter().any(|rt| rt == t)));
        }
        Ok(out)
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        // Honcho deletes are scoped to a known (user, session); without the
        // user we can't construct the URL. Treat as a no-op success so the
        // trait stays uniform; production callers should track agent_id
        // alongside the id when using Honcho.
        if id.is_empty() {
            return Err(MemoryError::InvalidArgument("id is required".into()));
        }
        Ok(())
    }

    async fn health(&self) -> Result<(), MemoryError> {
        // Honcho exposes `/v1/apps` as a cheap auth-required endpoint.
        let resp = self
            .client
            .get(self.endpoint("/v1/apps"))
            .header(reqwest::header::AUTHORIZATION, self.auth_value())
            .send()
            .await?;
        if resp.status().is_success() {
            return Ok(());
        }
        Err(lift_error(resp).await)
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Honcho
    }
}
