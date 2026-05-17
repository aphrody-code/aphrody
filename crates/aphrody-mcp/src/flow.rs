// SPDX-License-Identifier: Apache-2.0
//
// RFC 6749 §4.1 — Authorization Code Grant: code exchange + token refresh.
// RFC 7636      — PKCE: code_verifier sent at exchange time.
// RFC 6749 §6   — Refreshing an Access Token.
// RFC 8707      — Resource Indicators for OAuth 2.0 (optional `resource` param).
//
// Token endpoint uses `application/x-www-form-urlencoded` per RFC 6749 §4.1.3.
// Confidential clients (those with a `client_secret`) authenticate via HTTP
// Basic (RFC 6749 §2.3.1); public clients send only the form parameters.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use url::Url;

use crate::Error;

// ── Token set ────────────────────────────────────────────────────────────────

/// RFC 6749 §5.1 token endpoint response (subset used by the MCP flow).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    /// Bearer access token.
    pub access_token: String,
    /// Token type (almost always `"Bearer"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    /// Lifetime in seconds, if provided by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
    /// Refresh token, if issued.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Space-delimited scope(s) granted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

// ── Authorization URL builder ─────────────────────────────────────────────────

/// Parameters for building the user-facing authorize redirect URL.
#[derive(Debug)]
pub struct AuthorizeUrlParams<'a> {
    /// Full authorization endpoint URI from the auth server metadata.
    pub authorization_endpoint: &'a str,
    /// OAuth `client_id` from the registered client.
    pub client_id: &'a str,
    /// Redirect URI that will receive the authorization code.
    pub redirect_uri: &'a str,
    /// Opaque anti-CSRF state value (use `pkce::generate_state()`).
    pub state: &'a str,
    /// S256 code_challenge (from `Pkce::code_challenge`).
    pub code_challenge: &'a str,
    /// Space-delimited scopes to request.
    pub scope: Option<&'a str>,
    /// RFC 8707 resource indicator — scopes the issued token to this resource.
    pub resource: Option<&'a str>,
}

/// Build the RFC 6749 §4.1.1 authorization request URL with PKCE (RFC 7636)
/// and optional RFC 8707 resource indicator.
///
/// # Errors
///
/// Returns `Err` if `authorization_endpoint` cannot be parsed as a URL.
pub fn build_authorize_url(params: &AuthorizeUrlParams<'_>) -> Result<String, Error> {
    let mut url = Url::parse(params.authorization_endpoint).map_err(|e| Error::InvalidUrl {
        url: params.authorization_endpoint.to_owned(),
        source: e,
    })?;

    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", params.client_id);
        q.append_pair("redirect_uri", params.redirect_uri);
        q.append_pair("state", params.state);
        q.append_pair("code_challenge", params.code_challenge);
        q.append_pair("code_challenge_method", "S256");
        if let Some(scope) = params.scope {
            q.append_pair("scope", scope);
        }
        if let Some(resource) = params.resource {
            q.append_pair("resource", resource);
        }
    }

    Ok(url.to_string())
}

// ── Code exchange (RFC 6749 §4.1.3 + RFC 7636) ───────────────────────────────

/// Parameters for the authorization code → token exchange.
#[derive(Debug)]
pub struct ExchangeCodeParams<'a> {
    /// Token endpoint URL from the auth server metadata.
    pub token_endpoint: &'a str,
    /// OAuth `client_id`.
    pub client_id: &'a str,
    /// Present for confidential clients; absent for public/PKCE-only clients.
    pub client_secret: Option<&'a str>,
    /// Redirect URI used in the original authorize request.
    pub redirect_uri: &'a str,
    /// Authorization code received from the callback.
    pub code: &'a str,
    /// Raw PKCE `code_verifier` (from `Pkce::code_verifier`).
    pub code_verifier: &'a str,
    /// RFC 8707 resource indicator (must match the authorize request value).
    pub resource: Option<&'a str>,
}

/// Exchange an authorization code for a token set.
///
/// # Errors
///
/// Returns `Err` on transport failure or a non-2xx response from the token
/// endpoint.
#[instrument(skip(http, params), fields(endpoint = params.token_endpoint))]
pub async fn exchange_code(
    http: &Client,
    params: &ExchangeCodeParams<'_>,
) -> Result<TokenSet, Error> {
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "authorization_code"),
        ("code", params.code),
        ("redirect_uri", params.redirect_uri),
        ("client_id", params.client_id),
        ("code_verifier", params.code_verifier),
    ];
    if let Some(resource) = params.resource {
        form.push(("resource", resource));
    }

    debug!("exchanging authorization code for token");
    token_request(http, params.token_endpoint, &form, params.client_secret).await
}

// ── Token refresh (RFC 6749 §6) ───────────────────────────────────────────────

/// Parameters for a refresh-token grant.
#[derive(Debug)]
pub struct RefreshParams<'a> {
    /// Token endpoint URL from the auth server metadata.
    pub token_endpoint: &'a str,
    /// OAuth `client_id`.
    pub client_id: &'a str,
    /// Present for confidential clients.
    pub client_secret: Option<&'a str>,
    /// Refresh token from a prior `TokenSet`.
    pub refresh_token: &'a str,
    /// Subset scope to request (omit to preserve the original grant's scope).
    pub scope: Option<&'a str>,
    /// RFC 8707 resource indicator.
    pub resource: Option<&'a str>,
}

/// Refresh an access token using a refresh token (RFC 6749 §6).
///
/// Performs a token endpoint request with `grant_type=refresh_token`.
/// Rotation: the server may issue a new refresh token; the caller must
/// persist `TokenSet::refresh_token` from the returned value.
///
/// # Errors
///
/// Returns `Err` on transport failure or non-2xx token endpoint response.
#[instrument(skip(http, params), fields(endpoint = params.token_endpoint))]
pub async fn refresh(http: &Client, params: &RefreshParams<'_>) -> Result<TokenSet, Error> {
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", params.refresh_token),
        ("client_id", params.client_id),
    ];
    if let Some(scope) = params.scope {
        form.push(("scope", scope));
    }
    if let Some(resource) = params.resource {
        form.push(("resource", resource));
    }

    debug!("refreshing access token");
    token_request(http, params.token_endpoint, &form, params.client_secret).await
}

// ── Shared token endpoint helper ──────────────────────────────────────────────

/// POST an `application/x-www-form-urlencoded` request to `token_endpoint`.
///
/// Adds HTTP Basic auth header when `client_secret` is `Some` per
/// RFC 6749 §2.3.1.  Validates that the response contains `access_token`.
async fn token_request(
    http: &Client,
    token_endpoint: &str,
    form: &[(&str, &str)],
    client_secret: Option<&str>,
) -> Result<TokenSet, Error> {
    // Encode the form body manually so we control the exact wire format.
    let body = form
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoded(k), urlencoded(v)))
        .collect::<Vec<_>>()
        .join("&");

    let mut req = http
        .post(token_endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(reqwest::header::ACCEPT, "application/json")
        .body(body);

    // Confidential client: HTTP Basic credentials.
    if let Some(secret) = client_secret {
        // The `client_id` is already in the form; use it for the Basic username.
        let client_id = form.iter().find(|(k, _)| *k == "client_id").map(|(_, v)| *v).unwrap_or("");
        let credentials = STANDARD.encode(format!("{client_id}:{secret}"));
        req = req.header(reqwest::header::AUTHORIZATION, format!("Basic {credentials}"));
    }

    let resp =
        req.send().await.map_err(|e| Error::Http { url: token_endpoint.to_owned(), source: e })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body_text = resp.text().await.unwrap_or_default();
        let snippet = body_text.chars().take(500).collect::<String>();
        return Err(Error::TokenEndpointFailed {
            endpoint: token_endpoint.to_owned(),
            status,
            body: snippet,
        });
    }

    let token_set: TokenSet =
        resp.json().await.map_err(|e| Error::Http { url: token_endpoint.to_owned(), source: e })?;

    if token_set.access_token.is_empty() {
        return Err(Error::TokenEndpointFailed {
            endpoint: token_endpoint.to_owned(),
            status: 0,
            body: "response missing access_token".to_owned(),
        });
    }

    debug!("token endpoint returned valid TokenSet");
    Ok(token_set)
}

/// Hex digit lookup: index 0..=15 → ASCII uppercase hex character.
const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// Percent-encode a form field name or value (application/x-www-form-urlencoded).
///
/// RFC 3986 unreserved characters are left as-is; everything else is `%XX`.
fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            },
            b => {
                out.push('%');
                out.push(HEX[usize::from((b >> 4) & 0xF)] as char);
                out.push(HEX[usize::from(b & 0xF)] as char);
            },
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn token_set_serde_roundtrip() {
        let ts = TokenSet {
            access_token: "at123".to_owned(),
            token_type: Some("Bearer".to_owned()),
            expires_in: Some(3600),
            refresh_token: Some("rt456".to_owned()),
            scope: Some("read write".to_owned()),
        };
        let json = serde_json::to_string(&ts).unwrap();
        let back: TokenSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.access_token, ts.access_token);
        assert_eq!(back.expires_in, ts.expires_in);
        assert_eq!(back.refresh_token, ts.refresh_token);
        assert_eq!(back.scope, ts.scope);
    }

    #[test]
    fn token_set_optional_fields_omitted() {
        let ts = TokenSet {
            access_token: "at".to_owned(),
            token_type: None,
            expires_in: None,
            refresh_token: None,
            scope: None,
        };
        let json = serde_json::to_string(&ts).unwrap();
        assert!(!json.contains("expires_in"));
        assert!(!json.contains("refresh_token"));
        assert!(!json.contains("scope"));
    }

    #[test]
    fn build_authorize_url_contains_required_params() {
        let params = AuthorizeUrlParams {
            authorization_endpoint: "https://auth.example.com/authorize",
            client_id: "myapp",
            redirect_uri: "https://app.example.com/callback",
            state: "xyzstate",
            code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            scope: Some("openid profile"),
            resource: Some("https://api.example.com"),
        };
        let url = build_authorize_url(&params).unwrap();
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=myapp"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=xyzstate"));
        assert!(url.contains("code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"));
        assert!(url.contains("resource="));
    }

    #[test]
    fn urlencoded_encodes_special_chars() {
        assert_eq!(urlencoded("hello world"), "hello%20world");
        assert_eq!(urlencoded("a+b=c"), "a%2Bb%3Dc");
        assert_eq!(urlencoded("abc-_.~"), "abc-_.~"); // unreserved
    }
}
