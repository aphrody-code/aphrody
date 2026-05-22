// SPDX-License-Identifier: Apache-2.0
//! HTTP client construction and generic GraphQL invoker for the X private web
//! API.
//!
//! Builds a `reqwest::Client` pre-loaded with the authentication headers
//! that X's private API expects on every request. A cookie store is
//! enabled so that X's session-refresh set-cookie responses are honoured
//! automatically.
//!
//! # Generic GraphQL invoker
//!
//! [`XClient::graphql`] dispatches any of the 158 catalog operations without
//! requiring a hand-written method per operation.  Typed helpers in `api.rs`
//! wrap it for common use cases.
//!
//! # Rate-limit tracking
//!
//! Every response's `x-rate-limit-*` headers are captured into
//! `last_rate_limit`.  [`XClient::graphql_waiting`] uses this to
//! transparently sleep until the reset epoch when `remaining == 0`, bounded
//! by a caller-supplied `max_wait` duration.
//!
//! # Client-transaction-ID
//!
//! X is progressively enforcing `x-client-transaction-id` on write
//! mutations. This header is computed client-side using a keyed HMAC over
//! the endpoint path + a random nonce, with a rotating key extracted from
//! X's main JS bundle. No stable open-source Rust implementation exists yet
//! (see lib.rs module-level docs). We send a static placeholder here; if
//! you need the real value, set `XSession::transaction_id` and it will be
//! forwarded as-is. If X returns error code 353, that is the signal.

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;

use crate::catalog::{self, OpType};
use crate::features;
use crate::session::XSession;
use crate::{Result, XError};

/// The static public bearer token embedded in X's web JavaScript bundle.
///
/// This is NOT a personal token — it is the same for every logged-in browser
/// session and can be extracted from `main.<hash>.js` on x.com. X rotates it
/// very rarely (last change was from the Twitter-era bearer to the x.com
/// bearer in early 2024). Update this constant if requests start returning
/// HTTP 403 with no error body.
pub const WEB_BEARER: &str =
    "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

/// Chrome 124 on Windows — realistic UA that X's bot-detection accepts.
///
/// Update this string if X starts returning 403 / Forbidden for all
/// requests (UA-based detection tightened), matching a then-current
/// stable Chrome version.
pub const CHROME_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
     AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/124.0.0.0 Safari/537.36";

/// Static placeholder for `x-client-transaction-id`.
///
/// Most accounts accept this placeholder. If you receive API error code 353,
/// set `XSession::transaction_id` to a real value extracted from a browser
/// DevTools session (Network tab → CreateTweet request → x-client-transaction-id).
const TRANSACTION_ID_PLACEHOLDER: &str = "placeholder-see-session-transaction-id";

/// Base URL prefix for all X private API calls.
pub const API_BASE: &str = "https://x.com/i/api";

// ---------------------------------------------------------------------------
// Rate-limit snapshot
// ---------------------------------------------------------------------------

/// A snapshot of the X API rate-limit headers for the most recent response.
///
/// X includes these headers on all GraphQL and REST v1.1 responses.  Values
/// are per-endpoint, per-15-minute-window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimit {
    /// The maximum number of requests allowed in the window.
    pub limit: i64,
    /// Remaining requests in the current window.
    pub remaining: i64,
    /// Unix epoch second at which the window resets.
    pub reset_epoch: i64,
}

// ---------------------------------------------------------------------------
// XClient
// ---------------------------------------------------------------------------

/// Stateless X API client.
///
/// Holds a `reqwest::Client` configured with auth headers and a cookie jar.
/// All methods take `&self` and are safe to call from concurrent tasks.
///
/// The `last_rate_limit` field is protected by a `Mutex` so that concurrent
/// requests race only over a tiny critical section (header parsing).
#[derive(Debug)]
pub struct XClient {
    pub(crate) inner: reqwest::Client,
    pub(crate) session: XSession,
    /// Most recent rate-limit snapshot captured from response headers.
    last_rate_limit: Mutex<Option<RateLimit>>,
}

impl Clone for XClient {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            session: self.session.clone(),
            last_rate_limit: Mutex::new(
                *self.last_rate_limit.lock().unwrap_or_else(|e| e.into_inner()),
            ),
        }
    }
}

impl XClient {
    /// Build a new `XClient` from a loaded `XSession`.
    ///
    /// Installs the ring rustls CryptoProvider (idempotent) and constructs
    /// a `reqwest::Client` with:
    /// - cookie store enabled
    /// - default headers carrying authentication signals
    /// - `https://x.com` as origin / referer
    ///
    /// # Errors
    ///
    /// Returns `XError::Auth` if `reqwest::Client` construction fails (rare —
    /// would indicate a system TLS configuration problem).
    pub fn new(session: XSession) -> Result<Self> {
        // rustls 0.23 requires an explicit CryptoProvider before the first
        // Client is constructed (cf. CLAUDE.md §7). The error is ignored
        // because another crate may have already installed one.
        let _ = rustls::crypto::ring::default_provider().install_default();

        let headers = auth_headers(&session);

        let inner = reqwest::Client::builder()
            .user_agent(CHROME_UA)
            .default_headers(headers)
            .cookie_store(true)
            .build()
            .map_err(|e| XError::Auth(format!("failed to build reqwest::Client: {e}")))?;

        Ok(Self {
            inner,
            session,
            last_rate_limit: Mutex::new(None),
        })
    }

    /// Returns the underlying `reqwest::Client` for ad-hoc requests.
    pub fn inner(&self) -> &reqwest::Client {
        &self.inner
    }

    /// Returns the session this client was built from.
    pub fn session(&self) -> &XSession {
        &self.session
    }

    /// Returns the most recently captured rate-limit snapshot, or `None` if
    /// no response has been received yet.
    pub fn last_rate_limit(&self) -> Option<RateLimit> {
        *self.last_rate_limit.lock().unwrap_or_else(|e| e.into_inner())
    }

    // -----------------------------------------------------------------------
    // Generic GraphQL invoker
    // -----------------------------------------------------------------------

    /// Invoke any operation from the embedded GraphQL catalog.
    ///
    /// - `op_name` — exact operation name from the catalog (e.g. `"CreateTweet"`).
    /// - `variables` — the `variables` JSON object for this call.
    /// - `extra_features` — optional caller overrides merged on top of the
    ///   catalog-derived feature flags (caller values win on conflicts).
    ///
    /// **GET queries** are dispatched as `GET /i/api/graphql/{qid}/{op}?variables=...&features=...`.
    ///
    /// **POST mutations** are dispatched as `POST /i/api/graphql/{qid}/{op}`
    /// with body `{"variables":..,"features":..,"queryId":..}`.
    ///
    /// Rate-limit headers on every response are captured into
    /// [`XClient::last_rate_limit`].
    ///
    /// # Errors
    ///
    /// - [`XError::UnknownOperation`] — `op_name` not found in catalog.
    /// - [`XError::Api`] — X returned a structured error in the `errors[]` array.
    /// - [`XError::Http`] — transport / TLS failure.
    pub async fn graphql(
        &self,
        op_name: &str,
        variables: Value,
        extra_features: Option<Value>,
    ) -> Result<Value> {
        let op = catalog::operation(op_name)
            .ok_or_else(|| XError::UnknownOperation(op_name.to_owned()))?;

        let url = format!("{}/graphql/{}/{}", API_BASE, op.query_id, op_name);

        // Build feature flags: catalog-derived base, then merge caller overrides.
        let mut feat = features::features_for(op);
        if let Some(extra) = extra_features {
            if let (Some(base_obj), Some(extra_obj)) =
                (feat.as_object_mut(), extra.as_object())
            {
                for (k, v) in extra_obj {
                    base_obj.insert(k.clone(), v.clone());
                }
            }
        }

        let resp = match op.op_type {
            OpType::Query => {
                let vars_str = serde_json::to_string(&variables)?;
                let feat_str = serde_json::to_string(&feat)?;
                self.inner
                    .get(&url)
                    .query(&[("variables", &vars_str), ("features", &feat_str)])
                    .send()
                    .await?
            }
            OpType::Mutation | OpType::Subscription => {
                let body = serde_json::json!({
                    "variables": variables,
                    "features": feat,
                    "queryId": op.query_id,
                });
                self.inner.post(&url).json(&body).send().await?
            }
        };

        // Capture rate-limit headers before consuming the response body.
        self.capture_rate_limit(resp.headers());

        let status = resp.status();
        let json: Value = resp.json().await?;

        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;

        Ok(json)
    }

    /// Like [`XClient::graphql`] but transparently waits out soft rate limits.
    ///
    /// If the last known [`RateLimit`] has `remaining == 0`, this method
    /// sleeps until `reset_epoch` before issuing the call.  If the required
    /// wait would exceed `max_wait`, it returns
    /// [`XError::RateLimited`] immediately without sleeping.
    ///
    /// # Note on hard limits
    ///
    /// X's hard per-account limits (e.g. error code 344 — daily tweet cap)
    /// are enforced server-side and **cannot** be bypassed by waiting.  This
    /// method only helps with soft window-based rate limits (remaining == 0
    /// from the response headers).  A 344 error will still propagate as
    /// `XError::Api { code: 344, .. }`.
    pub async fn graphql_waiting(
        &self,
        op_name: &str,
        variables: Value,
        extra_features: Option<Value>,
        max_wait: Duration,
    ) -> Result<Value> {
        // Check whether we already know we are rate-limited.
        if let Some(rl) = self.last_rate_limit() {
            if rl.remaining == 0 {
                let now_epoch = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                let wait_secs = (rl.reset_epoch - now_epoch).max(0) as u64;
                let wait = Duration::from_secs(wait_secs);

                if wait > max_wait {
                    return Err(XError::RateLimited {
                        reset_epoch: rl.reset_epoch,
                    });
                }

                if wait > Duration::ZERO {
                    tokio::time::sleep(wait).await;
                }
            }
        }

        self.graphql(op_name, variables, extra_features).await
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Parse `x-rate-limit-*` headers from a response and store them.
    pub(crate) fn capture_rate_limit(&self, headers: &reqwest::header::HeaderMap) {
        let parse_i64 = |name: &str| -> Option<i64> {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<i64>().ok())
        };

        let limit = parse_i64("x-rate-limit-limit");
        let remaining = parse_i64("x-rate-limit-remaining");
        let reset = parse_i64("x-rate-limit-reset");

        if let (Some(limit), Some(remaining), Some(reset_epoch)) = (limit, remaining, reset) {
            let rl = RateLimit {
                limit,
                remaining,
                reset_epoch,
            };
            if let Ok(mut guard) = self.last_rate_limit.lock() {
                *guard = Some(rl);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Auth header builder (unchanged from original)
// ---------------------------------------------------------------------------

/// Build the default HTTP headers required for every X API request.
///
/// These headers collectively satisfy X's private API auth checks:
/// - `Authorization` carries the public web bearer token (not personal).
/// - `Cookie` carries `auth_token` + `ct0`.
/// - `X-Csrf-Token` must equal the `ct0` cookie value (double-submit CSRF).
/// - `X-Twitter-Auth-Type`, `X-Twitter-Active-User`, language headers
///   convince X's server-side checks that the request originates from a
///   normal browser session.
pub(crate) fn auth_headers(session: &XSession) -> HeaderMap {
    let mut map = HeaderMap::new();

    let insert = |map: &mut HeaderMap, k: &'static str, v: &str| {
        if let Ok(val) = HeaderValue::from_str(v) {
            map.insert(HeaderName::from_static(k), val);
        }
    };

    // Bearer token — same value for all browser sessions (public constant).
    insert(
        &mut map,
        "authorization",
        &format!("Bearer {WEB_BEARER}"),
    );

    // Cookie header — auth_token + ct0 are the only two X checks.
    insert(&mut map, "cookie", &session.cookie_header());

    // CSRF double-submit: ct0 cookie value must equal this header value.
    insert(&mut map, "x-csrf-token", &session.ct0);

    // X session-type marker.
    insert(&mut map, "x-twitter-auth-type", "OAuth2Session");

    // Active-user marker (required on all authenticated GraphQL calls).
    insert(&mut map, "x-twitter-active-user", "yes");

    // Language / locale — must be present; "en" is the safest value.
    insert(&mut map, "x-twitter-client-language", "en");

    // Content negotiation.
    insert(&mut map, "content-type", "application/json");
    insert(&mut map, "accept", "*/*");

    // Origin + Referer — required to pass X's CORS-like server checks.
    insert(&mut map, "origin", "https://x.com");
    insert(&mut map, "referer", "https://x.com/");

    // Transaction-ID: use the session-provided value if present, otherwise
    // the static placeholder. If X returns error 353, override via session.
    let txn_id = session
        .transaction_id
        .as_deref()
        .unwrap_or(TRANSACTION_ID_PLACEHOLDER);
    insert(&mut map, "x-client-transaction-id", txn_id);

    map
}

// ---------------------------------------------------------------------------
// Shared response checker (also used in api.rs)
// ---------------------------------------------------------------------------

/// Extract the first X `errors[]` entry, if present, and return it as
/// `XError::Api`. Returns `Ok(())` when no `errors` key is found.
pub(crate) fn check_api_errors(body: &Value) -> Result<()> {
    if let Some(errors) = body.get("errors").and_then(Value::as_array) {
        if let Some(first) = errors.first() {
            let code = first
                .get("code")
                .and_then(Value::as_i64)
                .unwrap_or(-1);
            let message = first
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_owned();
            return Err(XError::Api { code, message });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::XSession;

    fn make_session() -> XSession {
        XSession::new("AUTH_TOKEN_PLACEHOLDER", "CT0_PLACEHOLDER")
    }

    #[test]
    fn auth_headers_contains_bearer() {
        let session = make_session();
        let headers = auth_headers(&session);
        let auth = headers
            .get("authorization")
            .expect("authorization header missing")
            .to_str()
            .unwrap();
        assert!(auth.starts_with("Bearer "), "expected Bearer prefix");
        assert!(
            auth.contains(WEB_BEARER),
            "expected WEB_BEARER constant in auth header"
        );
    }

    #[test]
    fn auth_headers_csrf_token_equals_ct0() {
        let session = make_session();
        let headers = auth_headers(&session);
        let csrf = headers
            .get("x-csrf-token")
            .expect("x-csrf-token header missing")
            .to_str()
            .unwrap();
        assert_eq!(csrf, session.ct0, "x-csrf-token must equal ct0");
    }

    #[test]
    fn auth_headers_cookie_contains_auth_token() {
        let session = make_session();
        let headers = auth_headers(&session);
        let cookie = headers
            .get("cookie")
            .expect("cookie header missing")
            .to_str()
            .unwrap();
        assert!(
            cookie.contains("auth_token=AUTH_TOKEN_PLACEHOLDER"),
            "cookie header must contain auth_token"
        );
        assert!(
            cookie.contains("ct0=CT0_PLACEHOLDER"),
            "cookie header must contain ct0"
        );
    }

    #[test]
    fn auth_headers_uses_session_transaction_id_when_set() {
        let mut session = make_session();
        session.transaction_id = Some("custom-txn-id".into());
        let headers = auth_headers(&session);
        let txn = headers
            .get("x-client-transaction-id")
            .expect("x-client-transaction-id missing")
            .to_str()
            .unwrap();
        assert_eq!(txn, "custom-txn-id");
    }

    #[test]
    fn auth_headers_falls_back_to_placeholder_transaction_id() {
        let session = make_session();
        let headers = auth_headers(&session);
        let txn = headers
            .get("x-client-transaction-id")
            .expect("x-client-transaction-id missing")
            .to_str()
            .unwrap();
        assert_eq!(txn, TRANSACTION_ID_PLACEHOLDER);
    }

    #[test]
    fn xclient_new_succeeds_with_valid_session() {
        let session = make_session();
        let client = XClient::new(session).expect("XClient::new must succeed");
        assert_eq!(client.session().ct0, "CT0_PLACEHOLDER");
    }

    #[test]
    fn last_rate_limit_is_none_initially() {
        let session = make_session();
        let client = XClient::new(session).unwrap();
        assert!(client.last_rate_limit().is_none());
    }

    #[test]
    fn capture_rate_limit_parses_headers() {
        let session = make_session();
        let client = XClient::new(session).unwrap();

        // Build a minimal HeaderMap with rate-limit headers.
        let mut hmap = reqwest::header::HeaderMap::new();
        hmap.insert("x-rate-limit-limit", "100".parse().unwrap());
        hmap.insert("x-rate-limit-remaining", "0".parse().unwrap());
        hmap.insert("x-rate-limit-reset", "9999999999".parse().unwrap());

        client.capture_rate_limit(&hmap);

        let rl = client.last_rate_limit().expect("rate limit must be set");
        assert_eq!(rl.limit, 100);
        assert_eq!(rl.remaining, 0);
        assert_eq!(rl.reset_epoch, 9_999_999_999);
    }

    #[test]
    fn check_api_errors_returns_ok_when_no_errors_key() {
        let body = serde_json::json!({"data": {"foo": "bar"}});
        assert!(check_api_errors(&body).is_ok());
    }

    #[test]
    fn check_api_errors_surfaces_code_and_message() {
        let body = serde_json::json!({
            "errors": [{"code": 32, "message": "Could not authenticate you"}]
        });
        let err = check_api_errors(&body).unwrap_err();
        match err {
            XError::Api { code, message } => {
                assert_eq!(code, 32);
                assert!(message.contains("authenticate"));
            }
            other => panic!("expected XError::Api, got {other:?}"),
        }
    }

    #[test]
    fn check_api_errors_empty_errors_array_is_ok() {
        let body = serde_json::json!({"errors": []});
        assert!(check_api_errors(&body).is_ok());
    }
}
