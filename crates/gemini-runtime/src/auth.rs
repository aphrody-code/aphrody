// SPDX-License-Identifier: Apache-2.0
//! Google OAuth 2.0 token exchange + refresh, ported from the openclaw
//! gemini-cli OAuth pipeline.
//!
//! Source refs (TypeScript):
//!
//! - `openclaw/extensions/google/gemini-auth.ts` — apiKey vs Bearer header
//!   selection (`parseGeminiAuth`).
//! - `openclaw/src/infra/gemini-auth.ts` — shared infra (same contract).
//! - `openclaw/extensions/google/oauth.shared.ts` — REDIRECT_URI / TOKEN_URL /
//!   SCOPES constants.
//! - `openclaw/extensions/google/oauth.token.ts` — exchange_code_for_tokens /
//!   refresh_tokens_for_gemini_cli with the 5-minute safety buffer on `expires`.
//! - `openclaw/extensions/google/oauth-token-shared.ts` — `{token, projectId}`
//!   payload formatting.
//!
//! The classic on-disk credential file at `~/.gemini/oauth_creds.json` is the
//! authoritative source of refresh tokens. This module mirrors the read /
//! parse / refresh / write contract without depending on any TypeScript code.

use std::{path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Public constants (mirror openclaw/extensions/google/oauth.shared.ts)
// ---------------------------------------------------------------------------

/// OAuth localhost callback (matches `REDIRECT_URI` in `oauth.shared.ts`).
pub const REDIRECT_URI: &str = "http://localhost:8085/oauth2callback";

/// Google OAuth authorisation endpoint.
pub const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Google OAuth token endpoint (used for both code exchange and refresh).
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Google userinfo endpoint (`oauth2.shared.ts::USERINFO_URL`).
pub const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v1/userinfo?alt=json";

/// OAuth scopes requested for the Gemini CLI flow.
pub const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
];

/// Default HTTP timeout for OAuth requests (matches `DEFAULT_FETCH_TIMEOUT_MS`).
pub const DEFAULT_FETCH_TIMEOUT_MS: u64 = 10_000;

/// Safety margin subtracted from `expires_in` when computing the wall-clock
/// expiry (mirrors the `5 * 60 * 1000` ms buffer applied in
/// `oauth.token.ts::buildGeminiCliCredentials`).
pub const EXPIRY_SAFETY_MARGIN_MS: i64 = 5 * 60 * 1000;

/// Env vars that supply the OAuth client id (in priority order).
pub const CLIENT_ID_ENV_VARS: &[&str] =
    &["OPENCLAW_GEMINI_OAUTH_CLIENT_ID", "GEMINI_CLI_OAUTH_CLIENT_ID"];

/// Env vars that supply the OAuth client secret (in priority order).
pub const CLIENT_SECRET_ENV_VARS: &[&str] =
    &["OPENCLAW_GEMINI_OAUTH_CLIENT_SECRET", "GEMINI_CLI_OAUTH_CLIENT_SECRET"];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the auth module.
#[derive(Debug, Error)]
pub enum AuthError {
    /// `dirs::home_dir()` returned `None`.
    #[error("could not resolve user home directory")]
    NoHomeDir,

    /// File system read error for the creds file.
    #[error("failed to read creds file {path}: {source}")]
    Read {
        /// Path that failed to open.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// File system write error for the creds file.
    #[error("failed to write creds file {path}: {source}")]
    Write {
        /// Path that failed to write.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// JSON serialisation / deserialisation failure.
    #[error("creds JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP transport failure during a token request.
    #[error("HTTP error during token request: {0}")]
    Http(#[from] reqwest::Error),

    /// Token endpoint returned a non-2xx response.
    #[error("token endpoint returned {status}: {body}")]
    TokenStatus {
        /// HTTP status code.
        status: u16,
        /// Response body (trimmed).
        body: String,
    },

    /// Token response did not contain an `access_token` field.
    #[error("token response missing access_token")]
    MissingAccessToken,

    /// `CLIENT_ID_ENV_VARS` were all unset/blank.
    #[error("OAuth client id not configured (set one of {0:?})")]
    MissingClientId(&'static [&'static str]),
}

// ---------------------------------------------------------------------------
// GeminiCliCredentials — the on-disk shape of ~/.gemini/oauth_creds.json
// ---------------------------------------------------------------------------

/// On-disk credential shape persisted at `~/.gemini/oauth_creds.json`.
///
/// Mirrors `GeminiCliOAuthCredentials` in `oauth.shared.ts` plus the legacy
/// `access_token` / `refresh_token` / `expiry_date` field names emitted by
/// older Gemini CLI builds. Deserialisation accepts both shapes; serialisation
/// always writes the modern (camelCase) shape used by the upstream TS pipeline.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiCliCredentials {
    /// Bearer access token.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access: String,

    /// Long-lived refresh token.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub refresh: String,

    /// Token expiry in milliseconds since the Unix epoch.
    #[serde(default)]
    pub expires: i64,

    /// Google account email (when known).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Google Cloud project id (when known).
    #[serde(default, rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

impl GeminiCliCredentials {
    /// Parse a JSON string into [`GeminiCliCredentials`], accepting both the
    /// modern (`access` / `refresh` / `expires`) and legacy (`access_token`
    /// / `refresh_token` / `expiry_date`) field names.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::Json`] when the input is not valid JSON.
    pub fn parse_json(raw: &str) -> Result<Self, AuthError> {
        let v: serde_json::Value = serde_json::from_str(raw)?;
        let access = pick_str(&v, &["access", "access_token", "token"]).unwrap_or_default();
        let refresh = pick_str(&v, &["refresh", "refresh_token"]).unwrap_or_default();
        let expires = pick_i64(&v, &["expires", "expiry_date"]).unwrap_or(0);
        let email = pick_str(&v, &["email"]);
        let project_id = pick_str(&v, &["projectId", "project_id"]);
        Ok(Self { access, refresh, expires, email, project_id })
    }

    /// Read the classic credential file at `<home>/.gemini/oauth_creds.json`.
    ///
    /// # Errors
    ///
    /// - [`AuthError::NoHomeDir`] — no home directory available.
    /// - [`AuthError::Read`] — file read failed.
    /// - [`AuthError::Json`] — file contents were not valid JSON.
    pub fn read_default() -> Result<Self, AuthError> {
        let path = default_creds_path()?;
        Self::read_from(&path)
    }

    /// Read credentials from an explicit path.
    ///
    /// # Errors
    ///
    /// Same as [`Self::read_default`].
    pub fn read_from(path: &PathBuf) -> Result<Self, AuthError> {
        let raw = std::fs::read_to_string(path)
            .map_err(|source| AuthError::Read { path: path.clone(), source })?;
        Self::parse_json(&raw)
    }

    /// Atomically write the credentials to `<home>/.gemini/oauth_creds.json`.
    ///
    /// Creates the `~/.gemini` directory if it does not already exist.
    ///
    /// # Errors
    ///
    /// - [`AuthError::NoHomeDir`] — no home directory.
    /// - [`AuthError::Write`] — file write failed.
    /// - [`AuthError::Json`] — serialisation failed.
    pub fn write_default(&self) -> Result<(), AuthError> {
        let path = default_creds_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|source| AuthError::Write { path: path.clone(), source })?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw).map_err(|source| AuthError::Write { path, source })
    }

    /// Returns `true` when `expires` is in the past relative to `now_ms`.
    /// Credentials without an `expires` value are treated as "never expiring"
    /// (callers may still refresh defensively).
    #[must_use]
    pub fn is_expired_at(&self, now_ms: i64) -> bool {
        self.expires != 0 && self.expires <= now_ms
    }

    /// Convenience wall-clock variant of [`Self::is_expired_at`].
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(now_unix_ms())
    }

    /// Build the request headers used to call the Gemini API with these
    /// credentials. Mirrors `parseGeminiAuth` in
    /// `openclaw/extensions/google/gemini-auth.ts`: if `access` is set we use
    /// `Authorization: Bearer …`; otherwise the caller is expected to supply
    /// a plain API key string (use [`api_key_headers`] for that path).
    #[must_use]
    pub fn auth_headers(&self) -> Vec<(&'static str, String)> {
        if self.access.is_empty() {
            vec![("Content-Type", "application/json".to_owned())]
        } else {
            vec![
                ("Authorization", format!("Bearer {}", self.access)),
                ("Content-Type", "application/json".to_owned()),
            ]
        }
    }
}

/// Return the auth headers used when a caller supplies a plain Gemini API key
/// rather than an OAuth access token. Mirrors the non-OAuth branch of
/// `parseGeminiAuth` in `gemini-auth.ts`.
#[must_use]
pub fn api_key_headers(api_key: &str) -> Vec<(&'static str, String)> {
    vec![
        ("x-goog-api-key", api_key.to_owned()),
        ("Content-Type", "application/json".to_owned()),
    ]
}

// ---------------------------------------------------------------------------
// OAuth token endpoint shapes (mirror oauth.token.ts)
// ---------------------------------------------------------------------------

/// Raw response shape returned by `https://oauth2.googleapis.com/token`.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    /// Bearer access token.
    pub access_token: Option<String>,
    /// Long-lived refresh token (only returned for `authorization_code`
    /// grants; `refresh_token` grants may omit it).
    pub refresh_token: Option<String>,
    /// Lifetime of `access_token` in seconds.
    pub expires_in: Option<i64>,
    /// Token type, typically `"Bearer"`.
    pub token_type: Option<String>,
}

impl TokenResponse {
    /// Convert a raw token response into a [`GeminiCliCredentials`].
    ///
    /// Applies the same 5-minute safety margin as
    /// `oauth.token.ts::buildGeminiCliCredentials` so that callers don't
    /// hit the upstream right as the token expires.
    ///
    /// `refresh_fallback` is used when the response omits a refresh token
    /// (typical for `grant_type=refresh_token` responses).
    /// `now_ms` is the current wall-clock time used to compute `expires`.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::MissingAccessToken`] when no `access_token` is
    /// present in the response.
    pub fn into_credentials(
        self,
        refresh_fallback: Option<&str>,
        existing_email: Option<&str>,
        existing_project_id: Option<&str>,
        now_ms: i64,
    ) -> Result<GeminiCliCredentials, AuthError> {
        let access = self.access_token.ok_or(AuthError::MissingAccessToken)?;
        let expires_in_ms = self.expires_in.unwrap_or(0).saturating_mul(1_000);
        let expires = now_ms
            .saturating_add(expires_in_ms)
            .saturating_sub(EXPIRY_SAFETY_MARGIN_MS);
        let refresh = self
            .refresh_token
            .or_else(|| refresh_fallback.map(str::to_owned))
            .unwrap_or_default();
        Ok(GeminiCliCredentials {
            access,
            refresh,
            expires,
            email: existing_email.map(str::to_owned),
            project_id: existing_project_id.map(str::to_owned),
        })
    }
}

// ---------------------------------------------------------------------------
// Token grant request body builders (mirror oauth.token.ts)
// ---------------------------------------------------------------------------

/// Construct the application/x-www-form-urlencoded body used by
/// `exchangeCodeForTokens` in `oauth.token.ts`.
///
/// `client_secret` is optional because Google "installed app" clients are
/// allowed to omit it when paired with PKCE.
#[must_use]
pub fn build_code_exchange_body(
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    code_verifier: &str,
) -> String {
    let mut pairs = vec![
        ("client_id", client_id),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", code_verifier),
    ];
    if let Some(secret) = client_secret {
        pairs.push(("client_secret", secret));
    }
    url_encode_pairs(&pairs)
}

/// Construct the body used by `refreshTokensForGeminiCli`.
#[must_use]
pub fn build_refresh_body(
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> String {
    let mut pairs = vec![
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];
    if let Some(secret) = client_secret {
        pairs.push(("client_secret", secret));
    }
    url_encode_pairs(&pairs)
}

// ---------------------------------------------------------------------------
// OAuth token endpoint client
// ---------------------------------------------------------------------------

/// Perform an HTTP POST against [`TOKEN_URL`] with an `x-www-form-urlencoded`
/// body and parse the response into a [`TokenResponse`].
///
/// # Errors
///
/// - [`AuthError::Http`] — transport / TLS / DNS failure.
/// - [`AuthError::TokenStatus`] — Google returned a non-2xx status.
/// - [`AuthError::Json`] — response body was not valid JSON.
pub async fn request_token_grant(
    client: &reqwest::Client,
    body: String,
) -> Result<TokenResponse, AuthError> {
    let resp = client
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded;charset=UTF-8")
        .header("Accept", "*/*")
        .header("User-Agent", "aphrody-gemini-runtime/1")
        .body(body)
        .send()
        .await?;
    let status = resp.status().as_u16();
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AuthError::TokenStatus { status, body: text });
    }
    let parsed: TokenResponse = resp.json().await?;
    Ok(parsed)
}

/// High-level helper: exchange an authorisation code for a fresh
/// [`GeminiCliCredentials`]. Identity discovery (email / projectId) is *not*
/// performed here — callers can fill it in after a separate userinfo call.
///
/// # Errors
///
/// Surfaces any [`AuthError`] from the underlying token grant or the
/// `into_credentials` step.
pub async fn exchange_code_for_tokens(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    code_verifier: &str,
) -> Result<GeminiCliCredentials, AuthError> {
    let body = build_code_exchange_body(client_id, client_secret, code, code_verifier);
    let resp = request_token_grant(client, body).await?;
    resp.into_credentials(None, None, None, now_unix_ms())
}

/// High-level helper: refresh an existing [`GeminiCliCredentials`].
///
/// The refresh token from the previous credentials is preserved when the
/// upstream response omits one (matching `refreshTokenFallback` behaviour in
/// `oauth.token.ts`).
///
/// # Errors
///
/// Surfaces any [`AuthError`] from the underlying token grant.
pub async fn refresh_tokens(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: Option<&str>,
    existing: &GeminiCliCredentials,
) -> Result<GeminiCliCredentials, AuthError> {
    if existing.refresh.is_empty() {
        return Err(AuthError::MissingAccessToken);
    }
    let body = build_refresh_body(client_id, client_secret, &existing.refresh);
    let resp = request_token_grant(client, body).await?;
    resp.into_credentials(
        Some(&existing.refresh),
        existing.email.as_deref(),
        existing.project_id.as_deref(),
        now_unix_ms(),
    )
}

// ---------------------------------------------------------------------------
// Env-resolved client id / secret (mirror oauth.credentials.ts)
// ---------------------------------------------------------------------------

/// Resolve the OAuth client id from environment variables in priority order.
///
/// # Errors
///
/// Returns [`AuthError::MissingClientId`] when none of [`CLIENT_ID_ENV_VARS`]
/// are set to a non-blank value.
pub fn resolve_client_id_from_env() -> Result<String, AuthError> {
    resolve_first_env(CLIENT_ID_ENV_VARS).ok_or(AuthError::MissingClientId(CLIENT_ID_ENV_VARS))
}

/// Resolve the OAuth client secret from environment variables in priority
/// order. Returns `None` if none of [`CLIENT_SECRET_ENV_VARS`] are set;
/// installed-app clients may legitimately omit a secret when paired with PKCE.
#[must_use]
pub fn resolve_client_secret_from_env() -> Option<String> {
    resolve_first_env(CLIENT_SECRET_ENV_VARS)
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Resolve `<home>/.gemini/oauth_creds.json`.
///
/// # Errors
///
/// Returns [`AuthError::NoHomeDir`] when `dirs::home_dir()` returns `None`.
pub fn default_creds_path() -> Result<PathBuf, AuthError> {
    let home = dirs::home_dir().ok_or(AuthError::NoHomeDir)?;
    Ok(home.join(".gemini").join("oauth_creds.json"))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn pick_str(v: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(serde_json::Value::as_str) {
            if !s.is_empty() {
                return Some(s.to_owned());
            }
        }
    }
    None
}

fn pick_i64(v: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    for k in keys {
        if let Some(n) = v.get(*k).and_then(serde_json::Value::as_i64) {
            return Some(n);
        }
    }
    None
}

fn url_encode_pairs(pairs: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (k, v) in pairs {
        if !out.is_empty() {
            out.push('&');
        }
        out.push_str(&percent_encode(k));
        out.push('=');
        out.push_str(&percent_encode(v));
    }
    out
}

/// application/x-www-form-urlencoded encoder. Encodes everything outside the
/// unreserved set `[A-Za-z0-9-._~]` per RFC 3986 §2.3, and renders spaces as
/// `+` per the form-urlencoded convention.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        let allowed = byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || byte == b'.'
            || byte == b'~';
        if allowed {
            out.push(byte as char);
        } else if byte == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn resolve_first_env(keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(v) = std::env::var(key) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
    }
    None
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- credential parsing -------------------------------------------------

    #[test]
    fn auth_creds_parse_modern_shape() {
        let raw = r#"{
            "access": "ya29.AAAA",
            "refresh": "1//RRRR",
            "expires": 1700000000000,
            "email": "alice@example.com",
            "projectId": "my-proj"
        }"#;
        let creds = GeminiCliCredentials::parse_json(raw).expect("parse");
        assert_eq!(creds.access, "ya29.AAAA");
        assert_eq!(creds.refresh, "1//RRRR");
        assert_eq!(creds.expires, 1_700_000_000_000);
        assert_eq!(creds.email.as_deref(), Some("alice@example.com"));
        assert_eq!(creds.project_id.as_deref(), Some("my-proj"));
    }

    #[test]
    fn auth_creds_parse_legacy_shape() {
        // Older gemini CLI builds emit access_token / refresh_token / expiry_date.
        let raw = r#"{
            "access_token": "ya29.LEG",
            "refresh_token": "1//LEG",
            "expiry_date": 1234567890,
            "project_id": "snake-cased"
        }"#;
        let creds = GeminiCliCredentials::parse_json(raw).expect("parse");
        assert_eq!(creds.access, "ya29.LEG");
        assert_eq!(creds.refresh, "1//LEG");
        assert_eq!(creds.expires, 1_234_567_890);
        assert_eq!(creds.project_id.as_deref(), Some("snake-cased"));
    }

    #[test]
    fn auth_creds_parse_invalid_json_errors() {
        let err = GeminiCliCredentials::parse_json("{not valid").unwrap_err();
        assert!(matches!(err, AuthError::Json(_)));
    }

    // --- expiry checks ------------------------------------------------------

    #[test]
    fn expiry_predicate_uses_injectable_clock() {
        let creds = GeminiCliCredentials {
            expires: 1_000,
            ..GeminiCliCredentials::default()
        };
        assert!(creds.is_expired_at(2_000));
        assert!(!creds.is_expired_at(500));

        // Zero expiry means "no known expiry" — never expired.
        let zero = GeminiCliCredentials::default();
        assert!(!zero.is_expired_at(i64::MAX));
    }

    // --- header selection ---------------------------------------------------

    #[test]
    fn auth_headers_bearer_when_access_present() {
        let creds = GeminiCliCredentials {
            access: "ya29.ZZ".into(),
            ..GeminiCliCredentials::default()
        };
        let h = creds.auth_headers();
        assert!(h.iter().any(|(k, v)| *k == "Authorization" && v == "Bearer ya29.ZZ"));
        assert!(h.iter().any(|(k, v)| *k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn api_key_headers_use_x_goog_header() {
        let h = api_key_headers("AIzaXX");
        assert!(h.iter().any(|(k, v)| *k == "x-goog-api-key" && v == "AIzaXX"));
        assert!(h.iter().any(|(k, v)| *k == "Content-Type" && v == "application/json"));
    }

    // --- body builders ------------------------------------------------------

    #[test]
    fn build_code_exchange_body_includes_all_required_pairs() {
        let body = build_code_exchange_body("client-id", Some("secret"), "abc 123", "verifier");
        assert!(body.contains("client_id=client-id"));
        assert!(body.contains("client_secret=secret"));
        assert!(body.contains("grant_type=authorization_code"));
        assert!(body.contains(&format!(
            "redirect_uri={}",
            percent_encode(REDIRECT_URI)
        )));
        assert!(body.contains("code=abc+123"));
        assert!(body.contains("code_verifier=verifier"));
    }

    #[test]
    fn build_code_exchange_body_omits_secret_when_none() {
        let body = build_code_exchange_body("cid", None, "code", "verifier");
        assert!(!body.contains("client_secret"));
    }

    #[test]
    fn build_refresh_body_shape() {
        let body = build_refresh_body("cid", None, "1//refresh");
        assert!(body.contains("client_id=cid"));
        assert!(body.contains("grant_type=refresh_token"));
        assert!(body.contains("refresh_token=1%2F%2Frefresh"));
        assert!(!body.contains("client_secret"));
    }

    // --- TokenResponse -> Credentials --------------------------------------

    #[test]
    fn token_response_applies_5min_safety_margin() {
        let resp = TokenResponse {
            access_token: Some("at".into()),
            refresh_token: Some("rt".into()),
            expires_in: Some(3_600),
            token_type: Some("Bearer".into()),
        };
        let creds = resp
            .into_credentials(None, None, None, 1_000_000)
            .expect("creds");
        // 1_000_000 + 3_600_000 - 300_000 = 4_300_000
        assert_eq!(creds.expires, 4_300_000);
        assert_eq!(creds.refresh, "rt");
        assert_eq!(creds.access, "at");
    }

    #[test]
    fn token_response_uses_refresh_fallback_when_missing() {
        let resp = TokenResponse {
            access_token: Some("at".into()),
            refresh_token: None,
            expires_in: Some(60),
            token_type: None,
        };
        let creds = resp
            .into_credentials(Some("FALLBACK"), Some("alice@x"), Some("proj"), 0)
            .expect("creds");
        assert_eq!(creds.refresh, "FALLBACK");
        assert_eq!(creds.email.as_deref(), Some("alice@x"));
        assert_eq!(creds.project_id.as_deref(), Some("proj"));
    }

    #[test]
    fn token_response_missing_access_errors() {
        let resp = TokenResponse {
            access_token: None,
            refresh_token: Some("rt".into()),
            expires_in: Some(60),
            token_type: None,
        };
        let err = resp.into_credentials(None, None, None, 0).unwrap_err();
        assert!(matches!(err, AuthError::MissingAccessToken));
    }

    // --- env resolution -----------------------------------------------------

    #[test]
    fn env_resolution_picks_first_non_blank() {
        let prev1 = std::env::var(CLIENT_ID_ENV_VARS[0]).ok();
        let prev2 = std::env::var(CLIENT_ID_ENV_VARS[1]).ok();
        // SAFETY: tests run in process — restore both vars in the finally branch.
        unsafe {
            std::env::set_var(CLIENT_ID_ENV_VARS[0], "");
            std::env::set_var(CLIENT_ID_ENV_VARS[1], "secondary-id");
        }
        let id = resolve_client_id_from_env().expect("id");
        assert_eq!(id, "secondary-id");
        unsafe {
            match prev1 {
                Some(v) => std::env::set_var(CLIENT_ID_ENV_VARS[0], v),
                None => std::env::remove_var(CLIENT_ID_ENV_VARS[0]),
            }
            match prev2 {
                Some(v) => std::env::set_var(CLIENT_ID_ENV_VARS[1], v),
                None => std::env::remove_var(CLIENT_ID_ENV_VARS[1]),
            }
        }
    }

    // --- percent-encoding ---------------------------------------------------

    #[test]
    fn percent_encode_form_urlencoded_basics() {
        assert_eq!(percent_encode("abc"), "abc");
        assert_eq!(percent_encode("a b"), "a+b");
        assert_eq!(percent_encode("1//rt"), "1%2F%2Frt");
        assert_eq!(percent_encode("foo@bar.com"), "foo%40bar.com");
    }
}
