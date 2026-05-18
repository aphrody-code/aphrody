// SPDX-License-Identifier: Apache-2.0
//! Gemini Web Search via Google grounding (`tools: [{google_search: ...}]`).
//!
//! Ports `openclaw/extensions/google/src/gemini-web-search-provider.runtime.ts`
//! + `.shared.ts` to Rust. The provider issues a single `generateContent`
//! call with `google_search` grounding, parses the returned `candidates →
//! content → parts` + `groundingMetadata.groundingChunks` payload, and
//! returns structured [`SearchResults`].
//!
//! Default model: `gemini-2.5-flash` (matches `DEFAULT_GEMINI_WEB_SEARCH_MODEL`
//! in the upstream TS file). Default base URL:
//! `https://generativelanguage.googleapis.com/v1beta` (matches
//! `DEFAULT_GOOGLE_API_BASE_URL` in `provider-policy.ts`).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default Gemini API base URL.
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default model id for web-search-grounded queries.
pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";

/// Default request timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All errors that the web search module can return.
#[derive(Debug, Error)]
pub enum WebSearchError {
    /// The API key was blank or otherwise invalid for header injection.
    #[error("missing or blank Gemini API key")]
    MissingApiKey,

    /// The base URL could not be parsed / joined with the request path.
    #[error("invalid base URL: {0}")]
    Url(#[from] url::ParseError),

    /// Transport failure.
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// Upstream returned a non-2xx status.
    #[error("Gemini API HTTP {status}: {body}")]
    UpstreamStatus {
        /// HTTP status code.
        status: u16,
        /// Response body (trimmed, with sensitive query strings scrubbed).
        body: String,
    },

    /// Gemini returned a structured `error` payload (5xx, quota, etc).
    #[error("Gemini API error {code:?}: {message}")]
    Api {
        /// Numeric error code reported by Gemini.
        code: Option<i64>,
        /// Human-readable error message (scrubbed of `key=` query params).
        message: String,
    },

    /// Response JSON did not match the expected grounding-metadata shape.
    #[error("Gemini API returned malformed JSON: {0}")]
    Malformed(String),
}

// ---------------------------------------------------------------------------
// Config + Results
// ---------------------------------------------------------------------------

/// Per-call configuration for [`web_search`].
#[derive(Debug, Clone)]
pub struct WebSearchConfig {
    /// Bearer / API key (sent as `x-goog-api-key` header).
    pub api_key: String,

    /// Base URL of the Gemini API. Defaults to [`DEFAULT_BASE_URL`].
    pub base_url: String,

    /// Model id. Defaults to [`DEFAULT_MODEL`].
    pub model: String,

    /// Optional time-range filter forwarded to `tools.google_search.timeRangeFilter`.
    pub time_range: Option<TimeRange>,

    /// Optional request timeout. Defaults to [`DEFAULT_TIMEOUT`].
    pub timeout: Option<Duration>,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: DEFAULT_BASE_URL.to_owned(),
            model: DEFAULT_MODEL.to_owned(),
            time_range: None,
            timeout: None,
        }
    }
}

impl WebSearchConfig {
    /// Construct a config from an API key, using all defaults.
    #[must_use]
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self { api_key: api_key.into(), ..Self::default() }
    }
}

/// RFC 3339 time range filter (mirrors the `GeminiTimeRangeFilter` type in
/// `gemini-web-search-provider.runtime.ts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    /// Inclusive start time (RFC 3339 / ISO 8601).
    pub start_time: String,
    /// Exclusive end time (RFC 3339 / ISO 8601).
    pub end_time: String,
}

/// Parsed search response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResults {
    /// Original query echoed back.
    pub query: String,
    /// Aggregated text response (parts joined with `"\n"`).
    pub content: String,
    /// Cited URLs from the grounding metadata.
    pub citations: Vec<Citation>,
}

/// A single grounding-metadata citation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    /// Cited URL.
    pub url: String,
    /// Optional title of the cited page.
    pub title: Option<String>,
}

// ---------------------------------------------------------------------------
// URL builder
// ---------------------------------------------------------------------------

/// Build the fully-qualified endpoint URL for a model's `generateContent`
/// call.
///
/// # Errors
///
/// Returns [`WebSearchError::Url`] when `base_url` or `model` cannot be
/// joined into a valid URL.
pub fn build_endpoint_url(base_url: &str, model: &str) -> Result<Url, WebSearchError> {
    let trimmed_base = base_url.trim_end_matches('/');
    let trimmed_model = model.trim();
    let composed = format!("{trimmed_base}/models/{trimmed_model}:generateContent");
    Ok(Url::parse(&composed)?)
}

// ---------------------------------------------------------------------------
// Request body builder
// ---------------------------------------------------------------------------

/// Build the JSON body sent to `models/<model>:generateContent`.
///
/// Mirrors the body constructed in
/// `gemini-web-search-provider.runtime.ts::runGeminiSearch`:
///
/// ```json
/// {
///   "contents": [{"parts": [{"text": "<query>"}]}],
///   "tools": [{"google_search": {... optional timeRangeFilter ...}}]
/// }
/// ```
#[must_use]
pub fn build_request_body(query: &str, time_range: Option<&TimeRange>) -> serde_json::Value {
    let google_search = match time_range {
        Some(tr) => serde_json::json!({
            "timeRangeFilter": {
                "startTime": tr.start_time,
                "endTime": tr.end_time,
            }
        }),
        None => serde_json::json!({}),
    };
    serde_json::json!({
        "contents": [{"parts": [{"text": query}]}],
        "tools": [{"google_search": google_search}],
    })
}

// ---------------------------------------------------------------------------
// HTTP layer
// ---------------------------------------------------------------------------

/// Issue a single Gemini web-search call and return parsed [`SearchResults`].
///
/// # Errors
///
/// - [`WebSearchError::MissingApiKey`] — `cfg.api_key` is blank.
/// - [`WebSearchError::Url`] — `cfg.base_url` is invalid.
/// - [`WebSearchError::Http`] — transport failure.
/// - [`WebSearchError::UpstreamStatus`] — non-2xx HTTP status.
/// - [`WebSearchError::Api`] — Gemini returned a structured error payload.
/// - [`WebSearchError::Malformed`] — response shape did not match expectations.
pub async fn web_search(
    client: &reqwest::Client,
    cfg: &WebSearchConfig,
    query: &str,
) -> Result<SearchResults, WebSearchError> {
    if cfg.api_key.trim().is_empty() {
        return Err(WebSearchError::MissingApiKey);
    }
    let url = build_endpoint_url(&cfg.base_url, &cfg.model)?;
    let body = build_request_body(query, cfg.time_range.as_ref());

    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", &cfg.api_key)
        .json(&body);
    if let Some(timeout) = cfg.timeout {
        req = req.timeout(timeout);
    }

    let resp = req.send().await?;
    let status = resp.status().as_u16();
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(WebSearchError::UpstreamStatus { status, body: scrub_api_key(&text) });
    }

    let v: serde_json::Value = resp.json().await?;
    parse_response(query, &v)
}

// ---------------------------------------------------------------------------
// Response parser
// ---------------------------------------------------------------------------

/// Parse a Gemini grounding response JSON into [`SearchResults`].
///
/// Pulled out from [`web_search`] so it can be unit-tested without a live
/// network call.
///
/// # Errors
///
/// - [`WebSearchError::Api`] — Gemini returned a structured `error` payload.
/// - [`WebSearchError::Malformed`] — response shape did not match expectations.
pub fn parse_response(
    query: &str,
    v: &serde_json::Value,
) -> Result<SearchResults, WebSearchError> {
    if let Some(err) = v.get("error") {
        let code = err.get("code").and_then(serde_json::Value::as_i64);
        let message = err
            .get("message")
            .or_else(|| err.get("status"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        return Err(WebSearchError::Api { code, message: scrub_api_key(&message) });
    }

    let candidates = v
        .get("candidates")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| WebSearchError::Malformed("missing candidates[]".to_owned()))?;
    let candidate = candidates
        .first()
        .ok_or_else(|| WebSearchError::Malformed("candidates[] is empty".to_owned()))?;
    let content = candidate
        .get("content")
        .ok_or_else(|| WebSearchError::Malformed("missing content".to_owned()))?;
    let parts = content
        .get("parts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| WebSearchError::Malformed("missing parts[]".to_owned()))?;

    let mut text_parts: Vec<&str> = Vec::with_capacity(parts.len());
    for p in parts {
        if let Some(t) = p.get("text").and_then(serde_json::Value::as_str) {
            if !t.is_empty() {
                text_parts.push(t);
            }
        }
    }
    if text_parts.is_empty() {
        return Err(WebSearchError::Malformed("no text parts in response".to_owned()));
    }
    let joined = text_parts.join("\n");

    // Grounding chunks are optional from the API's perspective.
    let citations = candidate
        .get("groundingMetadata")
        .and_then(|g| g.get("groundingChunks"))
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|chunk| {
                    let web = chunk.get("web")?;
                    let url = web.get("uri").and_then(serde_json::Value::as_str)?.to_owned();
                    let title = web
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned);
                    Some(Citation { url, title })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchResults { query: query.to_owned(), content: joined, citations })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Replace any `key=…` query-string fragment with `key=***` so that error
/// messages never leak an API key into logs.
fn scrub_api_key(s: &str) -> String {
    // Lightweight scrub: walk the string and replace `key=<token>` with
    // `key=***` where token = non-whitespace, non-`&` chars.
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 4 <= bytes.len() && &bytes[i..i + 4].to_ascii_lowercase() == b"key=" {
            out.push_str("key=***");
            i += 4;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'&' {
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_search_url_build_default() {
        let url = build_endpoint_url(DEFAULT_BASE_URL, DEFAULT_MODEL).expect("url");
        assert_eq!(
            url.as_str(),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn web_search_url_build_strips_trailing_slash() {
        let url = build_endpoint_url(
            "https://generativelanguage.googleapis.com/v1beta/",
            "gemini-3-flash-preview",
        )
        .expect("url");
        assert!(url.as_str().ends_with("/models/gemini-3-flash-preview:generateContent"));
    }

    #[test]
    fn web_search_request_body_no_time_range() {
        let body = build_request_body("rust async traits 2026", None);
        assert_eq!(body["contents"][0]["parts"][0]["text"], "rust async traits 2026");
        // Empty google_search object (matches upstream TS behaviour).
        assert!(body["tools"][0]["google_search"].is_object());
        assert!(body["tools"][0]["google_search"].as_object().unwrap().is_empty());
    }

    #[test]
    fn web_search_request_body_with_time_range() {
        let tr = TimeRange {
            start_time: "2026-01-01T00:00:00Z".into(),
            end_time: "2026-12-31T23:59:59Z".into(),
        };
        let body = build_request_body("q", Some(&tr));
        assert_eq!(
            body["tools"][0]["google_search"]["timeRangeFilter"]["startTime"],
            "2026-01-01T00:00:00Z"
        );
        assert_eq!(
            body["tools"][0]["google_search"]["timeRangeFilter"]["endTime"],
            "2026-12-31T23:59:59Z"
        );
    }

    // --- response parsing ---------------------------------------------------

    #[test]
    fn parse_response_extracts_text_and_citations() {
        let raw = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {"text": "Rust async traits stabilised in 2024."},
                        {"text": "Further updates landed in 2026."}
                    ]
                },
                "groundingMetadata": {
                    "groundingChunks": [
                        {"web": {"uri": "https://doc.rust-lang.org/std/", "title": "std docs"}},
                        {"web": {"uri": "https://rust-lang.github.io/rfcs/"}}
                    ]
                }
            }]
        });
        let results = parse_response("q", &raw).expect("ok");
        assert_eq!(
            results.content,
            "Rust async traits stabilised in 2024.\nFurther updates landed in 2026."
        );
        assert_eq!(results.citations.len(), 2);
        assert_eq!(results.citations[0].url, "https://doc.rust-lang.org/std/");
        assert_eq!(results.citations[0].title.as_deref(), Some("std docs"));
        assert!(results.citations[1].title.is_none());
    }

    #[test]
    fn parse_response_surfaces_api_error() {
        let raw = serde_json::json!({
            "error": {
                "code": 429,
                "message": "Quota exceeded for key=AIza0123",
                "status": "RESOURCE_EXHAUSTED"
            }
        });
        let err = parse_response("q", &raw).unwrap_err();
        match err {
            WebSearchError::Api { code, message } => {
                assert_eq!(code, Some(429));
                // Key must be scrubbed.
                assert!(message.contains("key=***"), "got: {message}");
                assert!(!message.contains("AIza0123"));
            },
            other => panic!("expected Api, got {other:?}"),
        }
    }

    #[test]
    fn parse_response_malformed_missing_candidates() {
        let raw = serde_json::json!({"foo": "bar"});
        let err = parse_response("q", &raw).unwrap_err();
        assert!(matches!(err, WebSearchError::Malformed(_)));
    }

    #[test]
    fn parse_response_empty_text_parts_is_malformed() {
        let raw = serde_json::json!({
            "candidates": [{"content": {"parts": [{"foo": "bar"}]}}]
        });
        let err = parse_response("q", &raw).unwrap_err();
        assert!(matches!(err, WebSearchError::Malformed(_)));
    }

    // --- key scrubber -------------------------------------------------------

    #[test]
    fn scrub_api_key_replaces_inline_token() {
        let scrubbed = scrub_api_key("oops key=AIzaSyAAA failed");
        assert_eq!(scrubbed, "oops key=*** failed");
    }

    #[test]
    fn scrub_api_key_handles_querystring() {
        let scrubbed = scrub_api_key(
            "https://generativelanguage.googleapis.com/v1beta/models/x?key=AIzaXX&alt=json",
        );
        assert_eq!(
            scrubbed,
            "https://generativelanguage.googleapis.com/v1beta/models/x?key=***&alt=json"
        );
    }

    // --- config defaults ----------------------------------------------------

    #[test]
    fn web_search_config_defaults() {
        let cfg = WebSearchConfig::with_api_key("AIza");
        assert_eq!(cfg.base_url, DEFAULT_BASE_URL);
        assert_eq!(cfg.model, DEFAULT_MODEL);
        assert!(cfg.time_range.is_none());
        assert!(cfg.timeout.is_none());
    }

    #[tokio::test]
    async fn web_search_rejects_blank_api_key() {
        // Install the ring CryptoProvider before constructing a reqwest
        // Client: the workspace pins `reqwest` to `rustls-no-provider`, so
        // `Client::new()` would otherwise panic with "No provider set".
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::new();
        let cfg = WebSearchConfig::default();
        let err = web_search(&client, &cfg, "anything").await.unwrap_err();
        assert!(matches!(err, WebSearchError::MissingApiKey));
    }
}
