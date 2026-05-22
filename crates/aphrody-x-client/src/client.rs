// SPDX-License-Identifier: Apache-2.0
//! HTTP client construction for the X private web API.
//!
//! Builds a `reqwest::Client` pre-loaded with the authentication headers
//! that X's private API expects on every request. A cookie store is
//! enabled so that X's session-refresh set-cookie responses are honoured
//! automatically.
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

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

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

/// Stateless X API client.
///
/// Holds a `reqwest::Client` configured with auth headers and a cookie jar.
/// All methods take `&self` and are safe to call from concurrent tasks.
#[derive(Debug, Clone)]
pub struct XClient {
    pub(crate) inner: reqwest::Client,
    pub(crate) session: XSession,
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

        Ok(Self { inner, session })
    }

    /// Returns the underlying `reqwest::Client` for ad-hoc requests.
    pub fn inner(&self) -> &reqwest::Client {
        &self.inner
    }

    /// Returns the session this client was built from.
    pub fn session(&self) -> &XSession {
        &self.session
    }
}

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
        assert!(auth.contains(WEB_BEARER), "expected WEB_BEARER constant in auth header");
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
}
