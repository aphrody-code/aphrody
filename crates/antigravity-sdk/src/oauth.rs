// SPDX-License-Identifier: Apache-2.0
//! Native Google OAuth 2.0 Authorization-Code + PKCE (S256) loopback login.
//!
//! This module reproduces the sign-in flow performed by the Antigravity
//! desktop client (verified against the running 2.0.1 client, 2026-05-21):
//!
//! 1. Generate a PKCE verifier/challenge pair ([`PkcePair::generate`]).
//! 2. Build an authorize URL pointing at
//!    `https://accounts.google.com/o/oauth2/v2/auth` ([`authorize_url`]).
//! 3. Open it in the user's browser, start an ephemeral loopback listener on
//!    `127.0.0.1:9109`, accept the single `GET /oauth-callback?code=…&state=…`
//!    redirect, validate `state`, then exchange `code` for an
//!    [`crate::auth::OAuthToken`] at `https://oauth2.googleapis.com/token`
//!    ([`login`]).
//! 4. Persist the token: Windows Credential Manager `gemini:antigravity`
//!    (`CredWriteW`) or `~/.config/aphrody/antigravity-token.json` with `0600`
//!    permissions on Unix ([`persist`]).
//!
//! No secrets are embedded — the OAuth `client_id` is public and the verifier
//! is generated fresh on every call.
//!
//! ## Example (no network)
//!
//! ```rust
//! use antigravity_sdk::oauth::{authorize_url, PkcePair};
//!
//! let pkce = PkcePair::generate();
//! let url = authorize_url(&pkce, "csrf-state-123");
//! assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
//! assert!(url.contains("code_challenge_method=S256"));
//! ```

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::Rng as _;
use rand::distributions::Alphanumeric;
use sha2::{Digest, Sha256};

use crate::auth::{ANTIGRAVITY_CLIENT_ID, ANTIGRAVITY_SCOPES, OAuthToken};
use crate::error::SdkError;

// ---------------------------------------------------------------------------
// Endpoints / fixed parameters (all public; verified against client 2.0.1)
// ---------------------------------------------------------------------------

/// Google OAuth 2.0 authorization endpoint.
const AUTHORIZE_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Google OAuth 2.0 token endpoint.
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

/// Loopback redirect URI registered for the Antigravity desktop client.
pub const REDIRECT_URI: &str = "http://localhost:9109/oauth-callback";

/// Loopback socket address the ephemeral listener binds to.
const LOOPBACK_ADDR: &str = "127.0.0.1:9109";

/// Path component of the loopback redirect URI.
const CALLBACK_PATH: &str = "/oauth-callback";

/// Default wait timeout for the browser redirect, in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Number of characters in the generated PKCE verifier (RFC 7636 permits
/// 43..=128; the desktop client uses 64).
const VERIFIER_LEN: usize = 64;

// ---------------------------------------------------------------------------
// PKCE
// ---------------------------------------------------------------------------

/// A PKCE (RFC 7636) verifier/challenge pair using the `S256` method.
///
/// The `verifier` is a random 64-character `[A-Za-z0-9]` string; the
/// `challenge` is `base64url-nopad(SHA256(verifier))`.
#[derive(Debug, Clone)]
pub struct PkcePair {
    /// High-entropy code verifier sent in the token exchange.
    pub verifier: String,
    /// `base64url-nopad(SHA256(verifier))` sent in the authorize request.
    pub challenge: String,
}

impl PkcePair {
    /// Generate a fresh PKCE pair using the operating-system RNG.
    #[must_use]
    pub fn generate() -> Self {
        let verifier: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(VERIFIER_LEN)
            .map(char::from)
            .collect();
        let challenge = Self::challenge_for(&verifier);
        Self {
            verifier,
            challenge,
        }
    }

    /// Compute the `S256` challenge for a given verifier.
    ///
    /// `base64url-nopad(SHA256(verifier))`.
    #[must_use]
    pub fn challenge_for(verifier: &str) -> String {
        let digest = Sha256::digest(verifier.as_bytes());
        URL_SAFE_NO_PAD.encode(digest)
    }
}

// ---------------------------------------------------------------------------
// Authorize URL
// ---------------------------------------------------------------------------

/// Whether a byte is `unreserved` per RFC 3986
/// (`ALPHA / DIGIT / "-" / "." / "_" / "~"`) and so needs no percent-encoding.
fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

/// Percent-encode a string for use as a query value, encoding spaces as `%20`
/// (not `+`) to match Google's authorize-endpoint parser.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for &b in value.as_bytes() {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(
                char::from_digit((u32::from(b) >> 4) & 0xF, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            out.push(
                char::from_digit(u32::from(b) & 0xF, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
        }
    }
    out
}

/// Build the full Google OAuth 2.0 authorize URL for the loopback PKCE flow.
///
/// Mirrors the Antigravity desktop client: `response_type=code`,
/// `access_type=offline`, `prompt=consent`, `code_challenge_method=S256`,
/// the public [`ANTIGRAVITY_CLIENT_ID`], the [`REDIRECT_URI`] loopback, and
/// the space-joined [`ANTIGRAVITY_SCOPES`].
#[must_use]
pub fn authorize_url(pkce: &PkcePair, state: &str) -> String {
    let scopes = ANTIGRAVITY_SCOPES.join(" ");
    let params = [
        ("client_id", ANTIGRAVITY_CLIENT_ID),
        ("redirect_uri", REDIRECT_URI),
        ("response_type", "code"),
        ("scope", scopes.as_str()),
        ("access_type", "offline"),
        ("prompt", "consent"),
        ("code_challenge", pkce.challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("state", state),
    ];

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{AUTHORIZE_ENDPOINT}?{query}")
}

// ---------------------------------------------------------------------------
// Login flow
// ---------------------------------------------------------------------------

/// Perform the full interactive OAuth 2.0 loopback PKCE login.
///
/// Generates a PKCE pair and a random `state`, opens the authorize URL in the
/// user's browser (best-effort), binds an ephemeral loopback listener on
/// `127.0.0.1:9109`, accepts a single callback request, validates `state`,
/// exchanges `code` for a token, and returns the resulting [`OAuthToken`].
///
/// The wait for the browser redirect is bounded by ~120 seconds.
///
/// # Errors
///
/// * [`SdkError::Io`] — the loopback listener could not be bound or accepted.
/// * [`SdkError::OAuthFlow`] — the callback was malformed or `state` mismatched.
/// * [`SdkError::OAuthTimeout`] — no redirect arrived within the timeout.
/// * [`SdkError::Http`] / [`SdkError::OAuthServer`] / [`SdkError::TokenParse`]
///   — the token exchange failed.
pub async fn login() -> Result<OAuthToken, SdkError> {
    login_with_timeout(DEFAULT_TIMEOUT_SECS).await
}

/// [`login`] with an explicit timeout (seconds) for the browser redirect.
///
/// # Errors
///
/// See [`login`].
pub async fn login_with_timeout(timeout_secs: u64) -> Result<OAuthToken, SdkError> {
    use tokio::time::{Duration, timeout};

    let pkce = PkcePair::generate();
    let state: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let url = authorize_url(&pkce, &state);

    // Bind the loopback listener BEFORE opening the browser so the redirect
    // never races a missing socket.
    let listener = tokio::net::TcpListener::bind(LOOPBACK_ADDR).await?;
    tracing::debug!(addr = LOOPBACK_ADDR, "OAuth loopback listener bound");

    // Best-effort browser launch; if it fails the user can paste the URL.
    open_in_browser(&url);
    tracing::info!("Opened Google sign-in in your browser. Complete the consent screen.");

    let callback = timeout(Duration::from_secs(timeout_secs), accept_callback(&listener))
        .await
        .map_err(|_| SdkError::OAuthTimeout(timeout_secs))??;

    if callback.state != state {
        return Err(SdkError::OAuthFlow(
            "state mismatch in OAuth callback (possible CSRF)".to_owned(),
        ));
    }

    exchange_code(&callback.code, &pkce.verifier).await
}

/// Parsed contents of the loopback callback request.
#[derive(Debug)]
struct Callback {
    code: String,
    state: String,
}

/// Accept a single connection, parse `code` / `state` from the HTTP request
/// line, write a small HTML response, and return the parsed callback.
async fn accept_callback(listener: &tokio::net::TcpListener) -> Result<Callback, SdkError> {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let (mut stream, _peer) = listener.accept().await?;

    // Read the start of the request; the GET line carries the query string.
    // Cap the read to avoid unbounded buffering on a hostile client.
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let request_line = request
        .lines()
        .next()
        .ok_or_else(|| SdkError::OAuthFlow("empty callback request".to_owned()))?;

    // "GET /oauth-callback?code=...&state=... HTTP/1.1"
    let target = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| SdkError::OAuthFlow("malformed request line".to_owned()))?;

    let result = parse_callback_target(target);

    // Always respond so the browser tab shows a friendly message.
    let (status, body) = if result.is_ok() {
        (
            "200 OK",
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>aphrody</title></head>\
             <body style=\"font-family:system-ui;text-align:center;margin-top:4rem\">\
             <h2>Sign-in complete</h2><p>You can close this tab and return to aphrody.</p>\
             </body></html>",
        )
    } else {
        (
            "400 Bad Request",
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>aphrody</title></head>\
             <body style=\"font-family:system-ui;text-align:center;margin-top:4rem\">\
             <h2>Sign-in failed</h2><p>The callback was missing an authorization code.</p>\
             </body></html>",
        )
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // Best-effort: even if the write fails we still return the parse result.
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
    let _ = stream.shutdown().await;

    result
}

/// Parse the request target `/oauth-callback?code=…&state=…` into a [`Callback`].
fn parse_callback_target(target: &str) -> Result<Callback, SdkError> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != CALLBACK_PATH {
        return Err(SdkError::OAuthFlow(format!(
            "unexpected callback path: {path}"
        )));
    }

    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut error: Option<String> = None;
    for pair in query.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        match k {
            "code" => code = Some(percent_decode(v)),
            "state" => state = Some(percent_decode(v)),
            "error" => error = Some(percent_decode(v)),
            _ => {}
        }
    }

    if let Some(err) = error {
        return Err(SdkError::OAuthFlow(format!("authorization denied: {err}")));
    }

    let code = code.ok_or_else(|| SdkError::OAuthFlow("missing `code` parameter".to_owned()))?;
    let state =
        state.ok_or_else(|| SdkError::OAuthFlow("missing `state` parameter".to_owned()))?;
    Ok(Callback { code, state })
}

/// Decode a percent-encoded query value (`%XX` plus `+` → space).
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push(((hi << 4) | lo) as u8);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Exchange the authorization `code` (with the PKCE `code_verifier`) for an
/// [`OAuthToken`] at the Google token endpoint.
async fn exchange_code(code: &str, verifier: &str) -> Result<OAuthToken, SdkError> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", ANTIGRAVITY_CLIENT_ID),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", verifier),
    ];

    let response = client.post(TOKEN_ENDPOINT).form(&params).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(SdkError::OAuthServer {
            status: status.as_u16(),
            body,
        });
    }

    #[derive(serde::Deserialize)]
    struct TokenResponse {
        access_token: String,
        #[serde(default)]
        refresh_token: Option<String>,
        #[serde(default)]
        expires_in: Option<u64>,
    }

    let body_text = response.text().await?;
    let resp: TokenResponse = serde_json::from_str(&body_text)?;

    Ok(OAuthToken {
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expiry: resp.expires_in.map(|s| s.to_string()),
    })
}

/// Best-effort: open `url` in the system default browser.
///
/// Uses the platform launcher (`start` / `open` / `xdg-open`) and never fails
/// the flow — if it cannot launch, the URL is logged for manual use.
fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let spawn = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();

    #[cfg(target_os = "macos")]
    let spawn = std::process::Command::new("open").arg(url).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let spawn = std::process::Command::new("xdg-open").arg(url).spawn();

    #[cfg(not(any(windows, unix)))]
    let spawn: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "no browser launcher for this platform",
    ));

    if spawn.is_err() {
        tracing::warn!(%url, "could not launch a browser; open this URL manually");
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Persist an [`OAuthToken`] for later use by the SDK.
///
/// * **Windows** — written to the Credential Manager generic credential
///   `gemini:antigravity` (matches the Antigravity desktop client) via
///   `CredWriteW`, in the same `{"token": {…}}` envelope the reader expects.
/// * **Unix (Linux/macOS)** — written as JSON to
///   `~/.config/aphrody/antigravity-token.json` with `0600` permissions.
/// * **Other** — [`SdkError::Unsupported`].
///
/// # Errors
///
/// * [`SdkError::Io`] — the file or its parent directory could not be written.
/// * [`SdkError::CredentialManager`] — `CredWriteW` failed (Windows).
/// * [`SdkError::Unsupported`] — no persistence backend on this platform.
pub fn persist(token: &OAuthToken) -> Result<(), SdkError> {
    #[cfg(target_os = "windows")]
    {
        windows_persist::write_credential(token)
    }
    #[cfg(unix)]
    {
        unix_persist::write_token_file(token)
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        let _ = token;
        Err(SdkError::Unsupported(
            "no token persistence backend on this platform",
        ))
    }
}

/// Serialize a token into the `{"token": {…}}` envelope used on disk and in
/// the Credential Manager.
#[cfg_attr(
    not(any(target_os = "windows", unix)),
    expect(dead_code, reason = "only used by the windows/unix persist backends")
)]
fn token_envelope_json(token: &OAuthToken) -> Result<String, SdkError> {
    let envelope = serde_json::json!({ "token": token });
    Ok(serde_json::to_string(&envelope)?)
}

#[cfg(unix)]
mod unix_persist {
    use std::fs;
    use std::io::Write as _;
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
    use std::path::PathBuf;

    use super::token_envelope_json;
    use crate::auth::OAuthToken;
    use crate::error::SdkError;

    /// Resolve `~/.config/aphrody/antigravity-token.json`, honoring
    /// `$XDG_CONFIG_HOME` then `$HOME`.
    fn token_path() -> Result<PathBuf, SdkError> {
        let base = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            return Err(SdkError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "neither $XDG_CONFIG_HOME nor $HOME is set",
            )));
        };
        Ok(base.join("aphrody").join("antigravity-token.json"))
    }

    pub(super) fn write_token_file(token: &OAuthToken) -> Result<(), SdkError> {
        let path = token_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            // Tighten the directory too (best-effort; ignore if unsupported).
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }

        let json = token_envelope_json(token)?;

        // Create with 0600 from the start so the secret is never world-readable,
        // even transiently. Truncate any pre-existing file.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(json.as_bytes())?;
        file.flush()?;
        // Re-assert mode in case the file pre-existed with looser perms.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;

        tracing::debug!(path = %path.display(), "persisted Antigravity OAuth token (0600)");
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows_persist {
    use windows_sys::Win32::Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CredWriteW, CREDENTIALW,
    };

    use super::token_envelope_json;
    use crate::auth::OAuthToken;
    use crate::error::SdkError;

    /// Credential Manager target name used by the Antigravity CLI.
    const CRED_TARGET: &str = "gemini:antigravity";

    pub(super) fn write_credential(token: &OAuthToken) -> Result<(), SdkError> {
        let json = token_envelope_json(token)?;
        let blob = json.into_bytes();

        let mut target: Vec<u16> = CRED_TARGET
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();

        let cred = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            Comment: std::ptr::null_mut(),
            LastWritten: unsafe { std::mem::zeroed() },
            CredentialBlobSize: u32::try_from(blob.len()).map_err(|_| {
                SdkError::OAuthFlow("token blob too large for Credential Manager".to_owned())
            })?,
            CredentialBlob: blob.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: std::ptr::null_mut(),
            UserName: std::ptr::null_mut(),
        };

        // SAFETY: `cred` points at `target` (NUL-terminated UTF-16) and `blob`
        // (owned `Vec<u8>` of length `CredentialBlobSize`), both of which
        // outlive this synchronous CredWriteW call. CredWriteW copies the
        // credential into the OS store; it borrows nothing past return.
        let ok = unsafe { CredWriteW(&cred, 0) };
        if ok == 0 {
            let code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            return Err(SdkError::CredentialManager { code });
        }
        tracing::debug!(
            target = CRED_TARGET,
            "persisted Antigravity OAuth token to Credential Manager"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests (no network)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_verifier_is_64_alphanumeric() {
        let pkce = PkcePair::generate();
        assert_eq!(pkce.verifier.len(), VERIFIER_LEN);
        assert!(
            pkce.verifier.chars().all(|c| c.is_ascii_alphanumeric()),
            "verifier must be URL-safe alphanumeric"
        );
    }

    #[test]
    fn pkce_challenge_matches_s256_of_verifier() {
        let pkce = PkcePair::generate();
        let expected = PkcePair::challenge_for(&pkce.verifier);
        assert_eq!(pkce.challenge, expected);
    }

    #[test]
    fn pkce_challenge_is_base64url_nopad() {
        let pkce = PkcePair::generate();
        // SHA-256 is 32 bytes → base64url no-pad is 43 chars, no padding.
        assert_eq!(pkce.challenge.len(), 43);
        assert!(!pkce.challenge.contains('='));
        assert!(!pkce.challenge.contains('+'));
        assert!(!pkce.challenge.contains('/'));
    }

    #[test]
    fn pkce_known_vector() {
        // RFC 7636 Appendix B test vector.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = PkcePair::challenge_for(verifier);
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn pkce_pairs_are_distinct() {
        let a = PkcePair::generate();
        let b = PkcePair::generate();
        assert_ne!(a.verifier, b.verifier, "verifiers must be random");
    }

    #[test]
    fn authorize_url_has_required_params() {
        let pkce = PkcePair::generate();
        let url = authorize_url(&pkce, "the-state");

        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(url.contains(&format!("client_id={}", percent_encode(ANTIGRAVITY_CLIENT_ID))));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("code_challenge={}", pkce.challenge)));
        assert!(url.contains("state=the-state"));
        // redirect_uri is percent-encoded (no raw "://").
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A9109%2Foauth-callback"));
    }

    #[test]
    fn authorize_url_scopes_space_encoded() {
        let pkce = PkcePair::generate();
        let url = authorize_url(&pkce, "s");
        // Scopes joined by space → %20, and contains the cloud-platform scope.
        assert!(url.contains("scope="));
        assert!(url.contains("cloud-platform%20"));
        assert!(
            !url.contains("scope=https://"),
            "scope value must be percent-encoded"
        );
    }

    #[test]
    fn percent_encode_basic() {
        assert_eq!(percent_encode("a b"), "a%20b");
        assert_eq!(percent_encode("a/b:c"), "a%2Fb%3Ac");
        assert_eq!(percent_encode("safe-._~AZ09"), "safe-._~AZ09");
    }

    #[test]
    fn percent_decode_roundtrip() {
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("a+b"), "a b");
        assert_eq!(percent_decode("4%2F0Ab%3D"), "4/0Ab=");
        assert_eq!(percent_decode("plain"), "plain");
    }

    #[test]
    fn parse_callback_extracts_code_and_state() {
        let cb = parse_callback_target("/oauth-callback?code=4%2F0Ab&state=xyz").expect("parse");
        assert_eq!(cb.code, "4/0Ab");
        assert_eq!(cb.state, "xyz");
    }

    #[test]
    fn parse_callback_rejects_missing_code() {
        let err = parse_callback_target("/oauth-callback?state=xyz").unwrap_err();
        assert!(matches!(err, SdkError::OAuthFlow(_)));
    }

    #[test]
    fn parse_callback_rejects_wrong_path() {
        let err = parse_callback_target("/wrong?code=a&state=b").unwrap_err();
        assert!(matches!(err, SdkError::OAuthFlow(_)));
    }

    #[test]
    fn parse_callback_surfaces_error_param() {
        let err = parse_callback_target("/oauth-callback?error=access_denied").unwrap_err();
        match err {
            SdkError::OAuthFlow(msg) => assert!(msg.contains("access_denied")),
            other => panic!("expected OAuthFlow, got {other:?}"),
        }
    }

    #[test]
    fn token_envelope_shape() {
        let token = OAuthToken {
            access_token: "ya29.x".to_owned(),
            refresh_token: Some("1//r".to_owned()),
            expiry: Some("3599".to_owned()),
        };
        let json = token_envelope_json(&token).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(value["token"]["access_token"], "ya29.x");
        assert_eq!(value["token"]["refresh_token"], "1//r");
    }
}
