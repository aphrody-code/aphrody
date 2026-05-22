// SPDX-License-Identifier: Apache-2.0
//! Honcho **v3** (<https://api.honcho.dev>) `MemoryProvider` implementation.
//!
//! Wire reference: the v3 OpenAPI (`title: "Honcho API"`, `servers:
//! https://api.honcho.dev`). v3 replaced the v1 `app → user → session →
//! message` hierarchy with **workspace → (peers | sessions)**: peers are the
//! first-class agents/humans, messages live under a session and carry a
//! `peer_id`, and a peer's accumulated knowledge is queried through the
//! *dialectic* endpoint rather than by listing rows.
//!
//! This is additive — [`crate::honcho::HonchoProvider`] keeps speaking v1.
//! Both report [`ProviderKind::Honcho`]; v3 is a wire upgrade, not a new
//! logical backend.
//!
//! ## Trait mapping
//!
//! | `MemoryProvider` | Honcho v3 wire |
//! |------------------|----------------|
//! | `add`    | `POST /v3/workspaces/{ws}/sessions/{sess}/messages/` (one message, `peer_id = agent_id`) |
//! | `search` | `POST /v3/workspaces/{ws}/peers/{agent_id}/chat` (dialectic representation query) |
//! | `get`    | unsupported by id alone → `Ok(None)` |
//! | `delete` | not exposed for a bare message id → no-op success |
//! | `health` | `GET /v3/workspaces/{ws}` (cheap authed reachability probe) |
//!
//! `search` is intentionally *not* a row dump: Honcho v3's idiom is to ask the
//! peer's representation a natural-language question and receive a synthesised
//! answer, which we return as a single [`MemoryRecord`].

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::MemoryProvider;
use crate::types::{MemoryError, MemoryQuery, MemoryRecord, ProviderKind, now_unix_ms};

/// Production Honcho v3 SaaS endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.honcho.dev";

/// Default workspace id used when no override is supplied.
pub const DEFAULT_WORKSPACE_ID: &str = "default";

/// Canonical session id — v3 messages live under a session; a single
/// long-lived session per workspace is sufficient for agent long-term memory.
pub const DEFAULT_SESSION_ID: &str = "default";

/// Prompt used by `search` when the caller supplies no keyword — asks the
/// peer's representation for everything it knows. Satisfies the dialectic
/// `query` `minLength: 1` constraint.
const DEFAULT_DIALECTIC_QUERY: &str =
    "Summarise everything you currently know about this peer.";

/// HTTP-backed Honcho **v3** provider.
#[derive(Debug, Clone)]
pub struct HonchoV3Provider {
    client: Client,
    api_key: String,
    base_url: String,
    workspace_id: String,
    session_id: String,
}

impl HonchoV3Provider {
    /// Build a provider against the production endpoint with the default
    /// workspace + session ids.
    ///
    /// # Errors
    /// - [`MemoryError::Http`] if the reqwest client builder fails (TLS init).
    pub fn new(api_key: impl Into<String>) -> Result<Self, MemoryError> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL, DEFAULT_WORKSPACE_ID)
    }

    /// Build a provider with an explicit base URL + workspace id (self-hosted
    /// `localhost:8000`, or an axum mock in tests). The session id defaults to
    /// [`DEFAULT_SESSION_ID`]; override it with [`Self::with_session`].
    ///
    /// # Errors
    /// - [`MemoryError::Http`] if the reqwest client builder fails.
    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Result<Self, MemoryError> {
        let client = Client::builder()
            .user_agent(concat!("aphrody-memory/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            workspace_id: workspace_id.into(),
            session_id: DEFAULT_SESSION_ID.to_string(),
        })
    }

    /// Override the canonical session id (builder style).
    #[must_use]
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
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

    fn messages_path(&self) -> String {
        format!(
            "/v3/workspaces/{}/sessions/{}/messages/",
            self.workspace_id, self.session_id
        )
    }

    fn chat_path(&self, peer_id: &str) -> String {
        format!(
            "/v3/workspaces/{}/peers/{}/chat",
            self.workspace_id, peer_id
        )
    }

    fn workspace_path(&self) -> String {
        format!("/v3/workspaces/{}", self.workspace_id)
    }

    fn auth_value(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Wire shapes
// ─────────────────────────────────────────────────────────────────────────────

/// `POST .../messages/` body: a batch of one message tagged with its peer.
#[derive(Debug, Serialize)]
struct CreateMessagesBody<'a> {
    messages: Vec<NewMessage<'a>>,
}

#[derive(Debug, Serialize)]
struct NewMessage<'a> {
    content: &'a str,
    peer_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<&'a serde_json::Value>,
}

/// Tolerant parse of the create-messages response. v3's exact body is not
/// pinned in the public docs, so we accept the three shapes seen in the wild:
/// a bare `{ "id": … }`, an envelope `{ "messages": [{ "id": … }] }`, or a
/// bare array `[{ "id": … }]`.
fn extract_message_id(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    match &value {
        serde_json::Value::Object(map) => {
            if let Some(id) = map.get("id").and_then(serde_json::Value::as_str) {
                return Some(id.to_string());
            }
            map.get("messages")
                .and_then(serde_json::Value::as_array)
                .and_then(|arr| arr.first())
                .and_then(|m| m.get("id"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        }
        serde_json::Value::Array(arr) => arr
            .first()
            .and_then(|m| m.get("id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

/// `POST .../peers/{id}/chat` body — the `DialecticOptions` schema.
#[derive(Debug, Serialize)]
struct DialecticBody {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    stream: bool,
}

/// `DialecticResponse` — `{ "content": string | null }`.
#[derive(Debug, Deserialize)]
struct DialecticResponse {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HonchoValidationDetail {
    #[serde(default)]
    msg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HonchoError {
    // FastAPI/`HTTPValidationError`: `{ "detail": [ { "msg": … } ] }` or a
    // bare `{ "detail": "…" }`. We accept both via `serde_json::Value`.
    #[serde(default)]
    detail: Option<serde_json::Value>,
    #[serde(default)]
    message: Option<String>,
}

fn flatten_detail(detail: &serde_json::Value) -> Option<String> {
    match detail {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(arr) => {
            let msgs: Vec<String> = arr
                .iter()
                .filter_map(|v| {
                    serde_json::from_value::<HonchoValidationDetail>(v.clone())
                        .ok()
                        .and_then(|d| d.msg)
                })
                .collect();
            (!msgs.is_empty()).then(|| msgs.join("; "))
        }
        _ => None,
    }
}

async fn lift_error(resp: reqwest::Response) -> MemoryError {
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    let message = serde_json::from_str::<HonchoError>(&body)
        .ok()
        .and_then(|e| e.detail.as_ref().and_then(flatten_detail).or(e.message))
        .unwrap_or(body);
    MemoryError::ApiError { status, message }
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryProvider impl
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryProvider for HonchoV3Provider {
    async fn add(&self, rec: MemoryRecord) -> Result<String, MemoryError> {
        if rec.agent_id.is_empty() {
            return Err(MemoryError::InvalidArgument("agent_id is required".into()));
        }
        // Fold tags into message metadata so they survive the round-trip; the
        // field is omitted entirely when there are no tags, keeping the body
        // identical to the documented minimal `{content, peer_id}` shape.
        let metadata_value = (!rec.tags.is_empty()).then(|| {
            serde_json::json!({
                "tags": rec.tags,
            })
        });
        let body = CreateMessagesBody {
            messages: vec![NewMessage {
                content: &rec.content,
                peer_id: &rec.agent_id,
                metadata: metadata_value.as_ref(),
            }],
        };
        let resp = self
            .client
            .post(self.endpoint(&self.messages_path()))
            .header(reqwest::header::AUTHORIZATION, self.auth_value())
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let text = resp.text().await?;
        // The server assigns ids; if the response shape hides them, fall back
        // to an empty string rather than failing the write that already
        // succeeded server-side.
        Ok(extract_message_id(&text).unwrap_or_default())
    }

    async fn get(&self, _id: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        // v3 has no get-message-by-bare-id route (ids are scoped to a session);
        // signal "unsupported direct fetch" rather than erroring. Callers that
        // need recall should `search` (dialectic).
        Ok(None)
    }

    async fn search(&self, q: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        if q.agent_id.is_empty() {
            return Err(MemoryError::InvalidArgument("agent_id is required".into()));
        }
        let query = q.q.unwrap_or_else(|| DEFAULT_DIALECTIC_QUERY.to_string());
        let body = DialecticBody {
            query,
            session_id: Some(self.session_id.clone()),
            stream: false,
        };
        let resp = self
            .client
            .post(self.endpoint(&self.chat_path(&q.agent_id)))
            .header(reqwest::header::AUTHORIZATION, self.auth_value())
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: DialecticResponse = resp.json().await?;
        // A dialectic answer is one synthesised representation, returned as a
        // single record. An empty/None answer means "nothing known" → no rows.
        let content = parsed.content.unwrap_or_default();
        if content.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![MemoryRecord {
            id: String::new(),
            agent_id: q.agent_id,
            content,
            tags: Vec::new(),
            created_at_unix_ms: now_unix_ms(),
            metadata: serde_json::json!({ "source": "honcho_v3_dialectic" }),
        }])
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        // No documented delete-by-bare-message-id route; treat as an idempotent
        // no-op so the trait stays uniform.
        if id.is_empty() {
            return Err(MemoryError::InvalidArgument("id is required".into()));
        }
        Ok(())
    }

    async fn health(&self) -> Result<(), MemoryError> {
        // GET the workspace root: a 2xx (exists) or 404 (not yet created) both
        // prove the bearer cleared auth and the service is reachable. 401/403
        // and 5xx surface as errors.
        let resp = self
            .client
            .get(self.endpoint(&self.workspace_path()))
            .header(reqwest::header::AUTHORIZATION, self.auth_value())
            .send()
            .await?;
        if resp.status().is_success() || resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        Err(lift_error(resp).await)
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Honcho
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_id_from_bare_object() {
        assert_eq!(extract_message_id(r#"{"id":"m1"}"#).as_deref(), Some("m1"));
    }

    #[test]
    fn extract_id_from_messages_envelope() {
        assert_eq!(
            extract_message_id(r#"{"messages":[{"id":"m2"},{"id":"m3"}]}"#).as_deref(),
            Some("m2")
        );
    }

    #[test]
    fn extract_id_from_bare_array() {
        assert_eq!(extract_message_id(r#"[{"id":"m4"}]"#).as_deref(), Some("m4"));
    }

    #[test]
    fn extract_id_absent_is_none() {
        assert!(extract_message_id(r#"{"ok":true}"#).is_none());
    }

    #[test]
    fn flatten_validation_detail_array() {
        let v = serde_json::json!([{ "msg": "field required" }, { "msg": "too long" }]);
        assert_eq!(flatten_detail(&v).as_deref(), Some("field required; too long"));
    }

    #[test]
    fn flatten_string_detail() {
        let v = serde_json::json!("not found");
        assert_eq!(flatten_detail(&v).as_deref(), Some("not found"));
    }
}
