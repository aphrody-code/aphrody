// SPDX-License-Identifier: Apache-2.0
//! Mem0 **v3** (<https://api.mem0.ai>) `MemoryProvider` implementation.
//!
//! Wire reference: <https://docs.mem0.ai>.
//!
//! v3 turns memory creation into a **queued, asynchronous** operation:
//! `POST /v3/memories/add/` returns `202 { status: "PENDING", event_id }`
//! immediately, and the extraction runs in the background. Completion is
//! observed by polling `GET /v1/event/{event_id}/`. Reads moved to a
//! POST-with-filters model (`POST /v2/memories/search/`, `POST /v3/memories/`),
//! while get/delete stay on the v1 id routes.
//!
//! This is additive — [`crate::mem0::Mem0Provider`] keeps speaking v1 sync.
//! Both report [`ProviderKind::Mem0`]; v3 is a wire upgrade, not a new logical
//! backend.
//!
//! Authentication is a single header — `Authorization: Token {api_key}`. The
//! base URL is overridable via [`Mem0V3Provider::with_base_url`] so the smoke
//! tests can point at a local axum mock without touching the network.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::MemoryProvider;
use crate::types::{MemoryError, MemoryQuery, MemoryRecord, ProviderKind, now_unix_ms};

/// Public Mem0 cloud endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.mem0.ai";

/// Max number of times [`Mem0V3Provider::poll_event`] checks an async event
/// before giving up (the write is still queued server-side; we just stop
/// waiting). Bounds the worst-case `add` latency to
/// `MAX_POLL_ATTEMPTS * POLL_INTERVAL_MS`.
pub const MAX_POLL_ATTEMPTS: u32 = 10;

/// Delay between async-event poll attempts, in milliseconds.
pub const POLL_INTERVAL_MS: u64 = 300;

/// HTTP-backed Mem0 **v3** provider.
#[derive(Debug, Clone)]
pub struct Mem0V3Provider {
    client: Client,
    api_key: String,
    base_url: String,
}

/// Terminal-or-not classification of an async event poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventOutcome {
    /// Processing finished; carries the resolved memory id when the event
    /// surfaced one (else empty — the write succeeded but no id was returned).
    Done {
        /// First memory id reported by the completed event, if any.
        memory_id: String,
    },
    /// The event failed server-side.
    Failed {
        /// Server-reported failure reason, best-effort.
        reason: String,
    },
    /// Still pending after [`MAX_POLL_ATTEMPTS`] — caller may poll again later.
    Pending,
}

impl Mem0V3Provider {
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

    /// Fire the async add and return the server's `event_id` without waiting
    /// for background processing. Use [`Self::poll_event`] to await completion.
    ///
    /// # Errors
    /// - [`MemoryError::InvalidArgument`] if `rec.agent_id` is empty.
    /// - [`MemoryError::ApiError`] / [`MemoryError::Http`] on transport/HTTP.
    pub async fn add_async(&self, rec: &MemoryRecord) -> Result<String, MemoryError> {
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
            .post(self.endpoint("/v3/memories/add/"))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: AddAsyncResponse = resp.json().await?;
        if parsed.event_id.is_empty() {
            return Err(MemoryError::Unexpected(
                "async add returned no event_id".into(),
            ));
        }
        Ok(parsed.event_id)
    }

    /// Poll `GET /v1/event/{event_id}/` until the event reaches a terminal
    /// state or [`MAX_POLL_ATTEMPTS`] is exhausted.
    ///
    /// # Errors
    /// - [`MemoryError::ApiError`] / [`MemoryError::Http`] on transport/HTTP.
    pub async fn poll_event(&self, event_id: &str) -> Result<EventOutcome, MemoryError> {
        for attempt in 0..MAX_POLL_ATTEMPTS {
            let resp = self
                .client
                .get(self.endpoint(&format!("/v1/event/{event_id}/")))
                .header(reqwest::header::AUTHORIZATION, self.auth_header())
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(lift_error(resp).await);
            }
            let event: EventStatus = resp.json().await?;
            match event.classify() {
                EventClass::Done => {
                    return Ok(EventOutcome::Done { memory_id: event.first_memory_id() });
                }
                EventClass::Failed => {
                    return Ok(EventOutcome::Failed {
                        reason: event.status.unwrap_or_else(|| "unknown".into()),
                    });
                }
                EventClass::Pending => {
                    // Sleep before the next attempt, but never after the last.
                    if attempt + 1 < MAX_POLL_ATTEMPTS {
                        tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS))
                            .await;
                    }
                }
            }
        }
        Ok(EventOutcome::Pending)
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

/// `202` response from `POST /v3/memories/add/`.
#[derive(Debug, Deserialize)]
struct AddAsyncResponse {
    #[serde(default)]
    event_id: String,
}

/// Poll response from `GET /v1/event/{event_id}/`. The exact body is not
/// pinned in the public docs, so every result-bearing field is optional and
/// parsed defensively.
#[derive(Debug, Deserialize)]
struct EventStatus {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    memory_id: Option<String>,
    #[serde(default)]
    memory_ids: Vec<String>,
    #[serde(default)]
    results: Vec<IdOnly>,
}

#[derive(Debug, Deserialize)]
struct IdOnly {
    #[serde(default)]
    id: Option<String>,
}

enum EventClass {
    Done,
    Failed,
    Pending,
}

impl EventStatus {
    fn classify(&self) -> EventClass {
        match self.status.as_deref().map(str::to_ascii_uppercase).as_deref() {
            Some("DONE" | "COMPLETED" | "SUCCESS" | "SUCCEEDED" | "FINISHED") => EventClass::Done,
            Some("FAILED" | "ERROR" | "FAILURE") => EventClass::Failed,
            // Missing / PENDING / PROCESSING / QUEUED / anything else → keep waiting.
            _ => EventClass::Pending,
        }
    }

    fn first_memory_id(&self) -> String {
        if let Some(id) = &self.memory_id {
            return id.clone();
        }
        if let Some(id) = self.memory_ids.first() {
            return id.clone();
        }
        self.results
            .iter()
            .find_map(|r| r.id.clone())
            .unwrap_or_default()
    }
}

/// `POST /v2/memories/search/` body.
#[derive(Debug, Serialize)]
struct SearchBody<'a> {
    query: &'a str,
    user_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
}

/// `POST /v3/memories/` (list) body — filter by owner, one page.
#[derive(Debug, Serialize)]
struct ListBody {
    filters: serde_json::Value,
    page: u32,
    page_size: u32,
}

/// `{ "results": [ … ], "count": N }` — shared by v2 search and v3 list.
#[derive(Debug, Deserialize)]
struct ResultsEnvelope {
    #[serde(default)]
    results: Vec<Mem0Memory>,
}

/// A memory row as returned by search / list / get. Tolerates `content` vs
/// `memory` for the body text and a missing `user_id`.
#[derive(Debug, Deserialize)]
struct Mem0Memory {
    #[serde(default)]
    id: String,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default, alias = "memory")]
    content: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

impl Mem0Memory {
    fn into_record(self, fallback_agent: &str) -> MemoryRecord {
        let created_at_unix_ms = self
            .created_at
            .as_deref()
            .and_then(parse_iso_to_unix_ms)
            .unwrap_or_else(now_unix_ms);
        MemoryRecord {
            id: self.id,
            agent_id: self.user_id.unwrap_or_else(|| fallback_agent.to_string()),
            content: self.content.unwrap_or_default(),
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
impl MemoryProvider for Mem0V3Provider {
    async fn add(&self, rec: MemoryRecord) -> Result<String, MemoryError> {
        // Async add → poll to completion. Returns the resolved memory id when
        // the event surfaces one, otherwise the event_id as a stable handle so
        // callers always receive a non-empty identifier for a queued write.
        let event_id = self.add_async(&rec).await?;
        match self.poll_event(&event_id).await? {
            EventOutcome::Done { memory_id } if !memory_id.is_empty() => Ok(memory_id),
            EventOutcome::Done { .. } | EventOutcome::Pending => Ok(event_id),
            EventOutcome::Failed { reason } => Err(MemoryError::ApiError {
                status: 0,
                message: format!("async memory add failed: {reason}"),
            }),
        }
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        let resp = self
            .client
            .get(self.endpoint(&format!("/v1/memories/{id}/")))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let parsed: Mem0Memory = resp.json().await?;
        Ok(Some(parsed.into_record("")))
    }

    async fn search(&self, q: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        if q.agent_id.is_empty() {
            return Err(MemoryError::InvalidArgument("agent_id is required".into()));
        }
        // Keyword present → v2 semantic search; absent → v3 owner-scoped list.
        let resp = if let Some(needle) = q.q.as_deref() {
            let body = SearchBody { query: needle, user_id: &q.agent_id, top_k: q.limit };
            self.client
                .post(self.endpoint("/v2/memories/search/"))
                .header(reqwest::header::AUTHORIZATION, self.auth_header())
                .json(&body)
                .send()
                .await?
        } else {
            let body = ListBody {
                filters: serde_json::json!({ "user_id": q.agent_id }),
                page: 1,
                page_size: q.limit.unwrap_or(100),
            };
            self.client
                .post(self.endpoint("/v3/memories/"))
                .header(reqwest::header::AUTHORIZATION, self.auth_header())
                .json(&body)
                .send()
                .await?
        };
        if !resp.status().is_success() {
            return Err(lift_error(resp).await);
        }
        let envelope: ResultsEnvelope = resp.json().await?;
        let agent_id = q.agent_id;
        let mut out: Vec<MemoryRecord> = envelope
            .results
            .into_iter()
            .map(|m| m.into_record(&agent_id))
            .collect();
        if !q.tags.is_empty() {
            out.retain(|r| q.tags.iter().all(|t| r.tags.iter().any(|rt| rt == t)));
        }
        Ok(out)
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        let resp = self
            .client
            .delete(self.endpoint(&format!("/v1/memories/{id}/")))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND || resp.status().is_success() {
            return Ok(());
        }
        Err(lift_error(resp).await)
    }

    async fn health(&self) -> Result<(), MemoryError> {
        // Cheapest authed reachability probe: a single-row owner-scoped list.
        let body = ListBody {
            filters: serde_json::json!({ "user_id": "_health" }),
            page: 1,
            page_size: 1,
        };
        let resp = self
            .client
            .post(self.endpoint("/v3/memories/"))
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .json(&body)
            .send()
            .await?;
        if resp.status().is_success() || resp.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(());
        }
        Err(lift_error(resp).await)
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Mem0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_terminal_states() {
        let done = EventStatus {
            status: Some("completed".into()),
            memory_id: None,
            memory_ids: vec![],
            results: vec![],
        };
        assert!(matches!(done.classify(), EventClass::Done));

        let failed = EventStatus {
            status: Some("ERROR".into()),
            memory_id: None,
            memory_ids: vec![],
            results: vec![],
        };
        assert!(matches!(failed.classify(), EventClass::Failed));

        let pending = EventStatus {
            status: Some("PENDING".into()),
            memory_id: None,
            memory_ids: vec![],
            results: vec![],
        };
        assert!(matches!(pending.classify(), EventClass::Pending));

        let missing = EventStatus { status: None, memory_id: None, memory_ids: vec![], results: vec![] };
        assert!(matches!(missing.classify(), EventClass::Pending));
    }

    #[test]
    fn first_memory_id_prefers_singular_then_list_then_results() {
        let singular = EventStatus {
            status: None,
            memory_id: Some("m1".into()),
            memory_ids: vec!["m2".into()],
            results: vec![],
        };
        assert_eq!(singular.first_memory_id(), "m1");

        let list = EventStatus {
            status: None,
            memory_id: None,
            memory_ids: vec!["m2".into(), "m3".into()],
            results: vec![],
        };
        assert_eq!(list.first_memory_id(), "m2");

        let results = EventStatus {
            status: None,
            memory_id: None,
            memory_ids: vec![],
            results: vec![IdOnly { id: Some("m4".into()) }],
        };
        assert_eq!(results.first_memory_id(), "m4");

        let none = EventStatus { status: None, memory_id: None, memory_ids: vec![], results: vec![] };
        assert_eq!(none.first_memory_id(), "");
    }

    #[test]
    fn memory_content_aliases_memory_field() {
        let m: Mem0Memory = serde_json::from_str(
            r#"{"id":"x","memory":"aliased body","categories":["fact"]}"#,
        )
        .expect("parse");
        let rec = m.into_record("agent-A");
        assert_eq!(rec.content, "aliased body");
        assert_eq!(rec.agent_id, "agent-A");
        assert_eq!(rec.tags, vec!["fact".to_string()]);
    }
}
