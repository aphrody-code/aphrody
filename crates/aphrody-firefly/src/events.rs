// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Adobe I/O Events **journaling** consumer.
//!
//! The journaling API is a pull-based, at-least-once event log. Instead of
//! polling each Firefly job's `statusUrl`, a single journal stream delivers
//! every event (e.g. async-job completion) the registration is subscribed to —
//! the latency-optimal completion path.
//!
//! Verified protocol (Adobe docs, 2026-05):
//! `GET <journal_url>[?latest=true|since=<pos>&limit=<n>]` with headers
//! `Authorization: Bearer <ims_token>`, `x-api-key: <client_id>`,
//! `x-ims-org-id: <org>@AdobeOrg`. A `200` returns
//! `{ "events": [{ "position", "event" }], "_page": { "last", "count" } }` and
//! an HTTP `Link: <…?since=…>; rel="next"` header. A `204 No Content` means no
//! events are available yet; the `retry-after` header (seconds) says when to
//! retry, and the `Link` rel="next" points at the same position to resume from.
//!
//! Credentials are read from the environment; nothing here is ever logged or
//! serialized to disk.

use crate::auth::{ImsCredentials, TokenCache};
use crate::error::{FireflyError, Result};
use serde::Deserialize;

/// Where to start reading the journal.
#[derive(Debug, Clone)]
pub enum Position {
    /// The oldest events available (the journal endpoint with no query params).
    Oldest,
    /// Jump to the end — only events arriving after this call (`?latest=true`).
    Latest,
    /// Resume strictly after a known position (`?since=<pos>`), as returned in a
    /// previous `next` link / event `position`.
    Since(String),
    /// Follow a full `next` link URL verbatim (already carries `since`+`limit`).
    NextLink(String),
}

/// A single journaled event: its opaque journal position plus the raw payload.
#[derive(Debug, Clone, Deserialize)]
pub struct JournalEvent {
    /// Opaque, monotonically increasing journal position for this event.
    pub position: String,
    /// The event payload (Adobe wraps the `CloudEvent` under `event`).
    pub event: serde_json::Value,
}

/// The `_page` metadata block.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PageInfo {
    /// Position of the last event in this batch.
    #[serde(default)]
    pub last: Option<String>,
    /// Number of events in this batch.
    #[serde(default)]
    pub count: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct JournalBody {
    #[serde(default)]
    events: Vec<JournalEvent>,
    #[serde(rename = "_page", default)]
    page: PageInfo,
}

/// One journal read result.
#[derive(Debug, Clone)]
pub struct JournalBatch {
    /// Events returned in this batch (empty on a 204).
    pub events: Vec<JournalEvent>,
    /// The `rel="next"` link to resume from (full URL), if the server gave one.
    pub next: Option<String>,
    /// `true` when the server returned `204 No Content` (no events yet).
    pub no_content: bool,
    /// Seconds to wait before retrying, parsed from `retry-after` (204 only).
    pub retry_after_secs: Option<u64>,
    /// `_page` metadata (200 only).
    pub page: PageInfo,
}

impl JournalBatch {
    /// `true` when there is more to read right now (events present, or a next
    /// link without a 204 back-off).
    #[must_use]
    pub fn has_more_now(&self) -> bool {
        !self.no_content && (!self.events.is_empty() || self.next.is_some())
    }
}

/// Parse the `rel="next"` target out of an HTTP `Link` header value.
///
/// Handles the standard `<url>; rel="next"` form (and comma-separated multi-link
/// headers), tolerating `rel=next` without quotes and extra params.
#[must_use]
pub fn parse_link_next(header: &str) -> Option<String> {
    for part in header.split(',') {
        let part = part.trim();
        let Some(open) = part.find('<') else { continue };
        let Some(close) = part[open + 1..].find('>') else { continue };
        let url = &part[open + 1..open + 1 + close];
        let params = &part[open + 1 + close + 1..];
        let is_next = params
            .split(';')
            .map(str::trim)
            .any(|p| p == "rel=\"next\"" || p == "rel=next" || p == "rel='next'");
        if is_next {
            return Some(url.to_string());
        }
    }
    None
}

/// Configuration for a journaling consumer.
#[derive(Debug, Clone)]
pub struct JournalConfig {
    /// The registration's journal endpoint (the long `events-*.adobe.io/...` URL).
    pub journal_url: String,
    /// The IMS org id in `XXXX@AdobeOrg` form (sent as `x-ims-org-id`).
    pub ims_org_id: String,
}

impl JournalConfig {
    /// Read `FIREFLY_JOURNAL_URL` and `FIREFLY_IMS_ORG_ID` from the environment.
    ///
    /// # Errors
    ///
    /// [`FireflyError::MissingCredential`] when either is absent/empty.
    pub fn from_env() -> Result<Self> {
        let journal_url = non_empty_env("FIREFLY_JOURNAL_URL")
            .ok_or(FireflyError::MissingCredential("FIREFLY_JOURNAL_URL"))?;
        let ims_org_id = non_empty_env("FIREFLY_IMS_ORG_ID")
            .ok_or(FireflyError::MissingCredential("FIREFLY_IMS_ORG_ID"))?;
        Ok(Self { journal_url, ims_org_id })
    }

    /// Origin (`scheme://host[:port]`) of the journal URL, used to resolve
    /// relative `next` links into absolute URLs.
    #[must_use]
    pub fn origin(&self) -> Option<String> {
        let after_scheme = self.journal_url.split_once("://")?;
        let host = after_scheme.1.split('/').next()?;
        Some(format!("{}://{}", after_scheme.0, host))
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// A journaling consumer: holds the cached IMS token, the journal URL and org.
pub struct JournalClient {
    http: reqwest::Client,
    tokens: TokenCache,
    config: JournalConfig,
}

impl JournalClient {
    /// Build a consumer from explicit credentials + config.
    ///
    /// # Errors
    ///
    /// [`FireflyError::Http`] if the underlying HTTP client cannot be built.
    pub fn new(creds: ImsCredentials, config: JournalConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("aphrody-firefly/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http, tokens: TokenCache::new(creds), config })
    }

    /// Build a consumer entirely from the environment
    /// (`FIREFLY_CLIENT_ID/SECRET`, `FIREFLY_JOURNAL_URL`, `FIREFLY_IMS_ORG_ID`).
    ///
    /// # Errors
    ///
    /// [`FireflyError::MissingCredential`] for any missing var, or
    /// [`FireflyError::Http`] if the client cannot be built.
    pub fn from_env() -> Result<Self> {
        Self::new(ImsCredentials::from_env()?, JournalConfig::from_env()?)
    }

    /// Resolve a [`Position`] into the absolute URL to GET.
    fn url_for(&self, pos: &Position) -> String {
        match pos {
            Position::NextLink(link) => self.absolutize(link),
            Position::Oldest => self.config.journal_url.clone(),
            Position::Latest => format!("{}?latest=true", self.config.journal_url),
            Position::Since(p) => {
                format!("{}?since={}", self.config.journal_url, urlencode(p))
            },
        }
    }

    /// Turn a possibly-relative `next` link into an absolute URL.
    fn absolutize(&self, link: &str) -> String {
        if link.starts_with("http://") || link.starts_with("https://") {
            return link.to_string();
        }
        match self.config.origin() {
            Some(origin) if link.starts_with('/') => format!("{origin}{link}"),
            Some(origin) => format!("{origin}/{link}"),
            None => link.to_string(),
        }
    }

    /// Read one batch from the journal at `pos`.
    ///
    /// # Errors
    ///
    /// * [`FireflyError::Auth`] on IMS failure.
    /// * [`FireflyError::Api`] on a non-2xx/204 status.
    /// * [`FireflyError::Decode`] when a 200 body cannot be parsed.
    pub async fn read(&self, pos: &Position) -> Result<JournalBatch> {
        let url = self.url_for(pos);
        let token = self.tokens.bearer(&self.http).await?;
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .header("x-api-key", self.tokens.client_id())
            .header("x-ims-org-id", &self.config.ims_org_id)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;

        let status = resp.status();
        let next = resp
            .headers()
            .get(reqwest::header::LINK)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_link_next)
            .map(|l| self.absolutize(&l));

        if status.as_u16() == 204 {
            let retry_after_secs = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok());
            return Ok(JournalBatch {
                events: Vec::new(),
                next,
                no_content: true,
                retry_after_secs,
                page: PageInfo::default(),
            });
        }

        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(FireflyError::Api {
                status: status.as_u16(),
                endpoint: url,
                body: crate::auth::truncate(&body_text, 512),
            });
        }

        let body: JournalBody = serde_json::from_str(&body_text)
            .map_err(|source| FireflyError::Decode { endpoint: url, source })?;
        Ok(JournalBatch {
            events: body.events,
            next,
            no_content: false,
            retry_after_secs: None,
            page: body.page,
        })
    }

    /// Drain every event currently available starting from `start`, following
    /// `next` links until the journal returns `204 No Content`.
    ///
    /// This does **not** wait on the `retry-after` back-off — it returns as soon
    /// as the journal is caught up, along with the `next` position so a caller
    /// can resume later. `max_batches` bounds the number of HTTP round-trips.
    ///
    /// # Errors
    ///
    /// As [`JournalClient::read`].
    pub async fn drain(
        &self,
        start: Position,
        max_batches: u32,
    ) -> Result<(Vec<JournalEvent>, Option<String>)> {
        let mut collected = Vec::new();
        let mut pos = start;
        let mut last_next = None;
        for _ in 0..max_batches {
            let batch = self.read(&pos).await?;
            last_next.clone_from(&batch.next);
            let caught_up = batch.no_content || batch.events.is_empty();
            collected.extend(batch.events);
            match (&batch.next, caught_up) {
                (Some(link), false) => pos = Position::NextLink(link.clone()),
                _ => break,
            }
        }
        Ok((collected, last_next))
    }
}

impl std::fmt::Debug for JournalClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JournalClient")
            .field("tokens", &self.tokens)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

/// Minimal percent-encoding for a journal `position` used in a `since` query
/// (positions can contain `:` and `+`). Encodes everything outside the
/// unreserved set.
fn urlencode(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            },
            // Writing to a String is infallible.
            _ => {
                let _ = write!(out, "%{b:02X}");
            },
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_next_standard_form() {
        let h = "</events/organizations/1/integrations/2/reg?since=ABC&limit=10>; rel=\"next\"";
        assert_eq!(
            parse_link_next(h).as_deref(),
            Some("/events/organizations/1/integrations/2/reg?since=ABC&limit=10")
        );
    }

    #[test]
    fn link_next_unquoted_and_multi() {
        let h = "<https://x/old>; rel=\"prev\", <https://x/new?since=Z>; rel=next";
        assert_eq!(parse_link_next(h).as_deref(), Some("https://x/new?since=Z"));
    }

    #[test]
    fn link_next_absent() {
        assert_eq!(parse_link_next("<https://x/a>; rel=\"self\""), None);
        assert_eq!(parse_link_next("garbage"), None);
    }

    #[test]
    fn config_origin_extracted() {
        let cfg = JournalConfig {
            journal_url: "https://events-va6.adobe.io/events/organizations/42/integrations/9/r"
                .into(),
            ims_org_id: "X@AdobeOrg".into(),
        };
        assert_eq!(cfg.origin().as_deref(), Some("https://events-va6.adobe.io"));
    }

    #[test]
    fn absolutize_relative_and_absolute() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = JournalClient::new(
            ImsCredentials { client_id: "k".into(), client_secret: "s".into() },
            JournalConfig {
                journal_url: "https://events-va6.adobe.io/events/org/1/int/2/r".into(),
                ims_org_id: "X@AdobeOrg".into(),
            },
        )
        .unwrap();
        assert_eq!(
            client.absolutize("/events/org/1/int/2/r?since=P"),
            "https://events-va6.adobe.io/events/org/1/int/2/r?since=P"
        );
        assert_eq!(client.absolutize("https://other/x"), "https://other/x");
    }

    #[test]
    fn url_for_positions() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let base = "https://events-va6.adobe.io/events/org/1/int/2/r";
        let client = JournalClient::new(
            ImsCredentials { client_id: "k".into(), client_secret: "s".into() },
            JournalConfig { journal_url: base.into(), ims_org_id: "X@AdobeOrg".into() },
        )
        .unwrap();
        assert_eq!(client.url_for(&Position::Oldest), base);
        assert_eq!(client.url_for(&Position::Latest), format!("{base}?latest=true"));
        assert_eq!(
            client.url_for(&Position::Since("a:b+c".into())),
            format!("{base}?since=a%3Ab%2Bc")
        );
        assert_eq!(
            client.url_for(&Position::NextLink("https://x/n".into())),
            "https://x/n"
        );
    }

    #[test]
    fn journal_body_parses_events_and_page() {
        let json = r#"{
            "events":[
                {"position":"pos-1","event":{"type":"firefly.job.completed","jobId":"urn:1"}},
                {"position":"pos-2","event":{"type":"firefly.job.completed","jobId":"urn:2"}}
            ],
            "_page":{"last":"pos-2","count":2}
        }"#;
        let body: JournalBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.events.len(), 2);
        assert_eq!(body.events[0].position, "pos-1");
        assert_eq!(body.events[1].event["jobId"], "urn:2");
        assert_eq!(body.page.last.as_deref(), Some("pos-2"));
        assert_eq!(body.page.count, Some(2));
    }

    #[test]
    fn batch_has_more_now_semantics() {
        let with_events = JournalBatch {
            events: vec![JournalEvent { position: "p".into(), event: serde_json::Value::Null }],
            next: Some("n".into()),
            no_content: false,
            retry_after_secs: None,
            page: PageInfo::default(),
        };
        assert!(with_events.has_more_now());

        let empty204 = JournalBatch {
            events: vec![],
            next: Some("n".into()),
            no_content: true,
            retry_after_secs: Some(10),
            page: PageInfo::default(),
        };
        assert!(!empty204.has_more_now());
    }

    #[test]
    fn urlencode_unreserved_and_reserved() {
        assert_eq!(urlencode("abcDEF123-_.~"), "abcDEF123-_.~");
        assert_eq!(urlencode("a:b+c/d"), "a%3Ab%2Bc%2Fd");
    }
}
