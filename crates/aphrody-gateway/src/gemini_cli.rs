// SPDX-License-Identifier: Apache-2.0
//! Gemini CLI provider — delegates to the locally installed `gemini` binary
//! (or `$APHRODY_GEMINI_BIN`) instead of hitting an HTTP endpoint.
//!
//! Mirrors the behaviour of `openclaw/extensions/google/gemini-cli-provider.ts`:
//! credentials are extracted from `~/.gemini/oauth_creds.json` (same shape used
//! by `crates/cli/src/commands.rs::A2aCommand`) and the CLI is invoked
//! non-interactively for each turn.
//!
//! # Auth & credential file
//!
//! The classic Gemini CLI persists OAuth credentials in
//! `~/.gemini/oauth_creds.json` with a payload such as:
//!
//! ```json
//! { "access_token": "ya29...", "refresh_token": "1//...",
//!   "expiry_date": 1700000000000 }
//! ```
//!
//! [`GeminiCliCredentials::from_default_path`] reads that file and returns the
//! parsed credentials; [`GeminiCliCredentials::is_expired`] inspects the
//! `expiry_date` field (millis-since-epoch) so callers can decide to refresh
//! upstream before re-invoking the CLI.
//!
//! # Binary location
//!
//! `GeminiCliAdapter::resolve_bin()` honours `$APHRODY_GEMINI_BIN` first (this
//! is the override already used elsewhere in the repo per `CLAUDE.md`), then
//! falls back to `which("gemini")`.

use std::{env, path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ChatRequest, ChatResponse, ChunkStream, GatewayAdapter, GatewayError};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Env var that overrides which `gemini` binary the adapter executes.
pub const ENV_GEMINI_BIN: &str = "APHRODY_GEMINI_BIN";

/// Default classic location of the Gemini CLI OAuth credential file.
///
/// Equivalent to `~/.gemini/oauth_creds.json`.
pub const GEMINI_OAUTH_CREDS_FILENAME: &str = "oauth_creds.json";

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

/// Parsed contents of `~/.gemini/oauth_creds.json`.
///
/// All fields are optional because different CLI versions persist slightly
/// different shapes (some add `id_token`, others omit `refresh_token` for
/// short-lived tokens, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiCliCredentials {
    /// Bearer token used as `Authorization: Bearer <access_token>`.
    pub access_token: Option<String>,

    /// Long-lived refresh token (used by the CLI to rotate `access_token`).
    pub refresh_token: Option<String>,

    /// Token expiry, in milliseconds since the Unix epoch.
    pub expiry_date: Option<i64>,

    /// Optional Google Cloud project id (Antigravity / Vertex routes use this).
    pub project_id: Option<String>,

    /// Token type as reported by Google OAuth (typically `"Bearer"`).
    pub token_type: Option<String>,
}

/// Errors specific to gemini-cli credential / binary discovery.
#[derive(Debug, Error)]
pub enum GeminiCliError {
    /// `~/.gemini/oauth_creds.json` could not be opened.
    #[error("could not read gemini creds file at {path}: {source}")]
    CredsRead {
        /// Path that was attempted.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The creds file is present but not parseable JSON.
    #[error("could not parse gemini creds JSON: {0}")]
    CredsParse(#[from] serde_json::Error),

    /// `dirs::home_dir()` returned `None` — no home dir.
    #[error("could not determine the user's home directory")]
    NoHomeDir,

    /// Neither `$APHRODY_GEMINI_BIN` nor `which gemini` resolved a binary.
    #[error("gemini binary not found (set {ENV_GEMINI_BIN} or install on PATH): {0}")]
    BinNotFound(#[from] which::Error),
}

impl GeminiCliCredentials {
    /// Parse a JSON string into a [`GeminiCliCredentials`].
    ///
    /// Accepts both the modern (`access_token`, `expiry_date`) and the
    /// project-aware (`projectId`) shapes; unknown fields are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`GeminiCliError::CredsParse`] when the input is not valid JSON.
    pub fn parse_json(raw: &str) -> Result<Self, GeminiCliError> {
        // Use a permissive intermediate value so we can support both
        // `expiry_date` (number-of-ms) and the OAuth `expires_in` (seconds)
        // shape without exploding on either.
        let v: serde_json::Value = serde_json::from_str(raw)?;
        let access_token = v
            .get("access_token")
            .or_else(|| v.get("token"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let refresh_token = v
            .get("refresh_token")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let expiry_date = v
            .get("expiry_date")
            .and_then(serde_json::Value::as_i64)
            .or_else(|| {
                // Some CLI versions store `expires_in` (seconds) instead.
                v.get("expires_in").and_then(serde_json::Value::as_i64).and_then(|sec| {
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                        .and_then(|d| i64::try_from(d.as_millis()).ok())
                        .map(|now_ms| now_ms + sec * 1000)
                })
            });
        let project_id = v
            .get("projectId")
            .or_else(|| v.get("project_id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let token_type =
            v.get("token_type").and_then(serde_json::Value::as_str).map(str::to_owned);

        Ok(Self { access_token, refresh_token, expiry_date, project_id, token_type })
    }

    /// Reads `<home>/.gemini/oauth_creds.json` and parses it.
    ///
    /// # Errors
    ///
    /// - [`GeminiCliError::NoHomeDir`] — `dirs::home_dir()` returned `None`.
    /// - [`GeminiCliError::CredsRead`] — file system read error.
    /// - [`GeminiCliError::CredsParse`] — file contents were not valid JSON.
    pub fn from_default_path() -> Result<Self, GeminiCliError> {
        let home = dirs::home_dir().ok_or(GeminiCliError::NoHomeDir)?;
        let path = home.join(".gemini").join(GEMINI_OAUTH_CREDS_FILENAME);
        let raw = std::fs::read_to_string(&path)
            .map_err(|source| GeminiCliError::CredsRead { path: path.clone(), source })?;
        Self::parse_json(&raw)
    }

    /// Returns `true` when the credentials have an `expiry_date` that is in
    /// the past relative to `now_ms`. Tokens without an `expiry_date` are
    /// assumed to never expire (the caller may still want to refresh).
    #[must_use]
    pub fn is_expired_at(&self, now_ms: i64) -> bool {
        self.expiry_date.is_some_and(|exp| exp <= now_ms)
    }

    /// Convenience wrapper around [`Self::is_expired_at`] using the wall clock.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or(0);
        self.is_expired_at(now_ms)
    }
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Adapter that delegates chat completions to the local `gemini` CLI.
///
/// Constructed via [`GeminiCliAdapter::from_env`] which picks up
/// `$APHRODY_GEMINI_BIN` (override) or falls back to `which("gemini")`, and
/// loads credentials from `~/.gemini/oauth_creds.json`.
///
/// The adapter does NOT itself spawn the CLI — instead it surfaces the
/// resolved binary path + credentials so the higher-level `gemini-runtime`
/// crate (which knows the full NDJSON streaming protocol) can execute turns.
/// The [`GatewayAdapter::chat`] / [`GatewayAdapter::chat_stream`] impls
/// document this contract by returning a structured [`GatewayError::Config`]
/// error pointing the caller at `gemini-runtime::detect` + `GeminiRuntime::turn`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliAdapter {
    /// Absolute path to the resolved `gemini` binary.
    pub bin: PathBuf,

    /// OAuth credentials parsed from `~/.gemini/oauth_creds.json`.
    pub creds: GeminiCliCredentials,
}

impl GeminiCliAdapter {
    /// Resolve the gemini binary location.
    ///
    /// Honours `$APHRODY_GEMINI_BIN` first, then falls back to
    /// `which("gemini")`.
    ///
    /// # Errors
    ///
    /// Returns [`GeminiCliError::BinNotFound`] when neither resolves.
    pub fn resolve_bin() -> Result<PathBuf, GeminiCliError> {
        if let Ok(explicit) = env::var(ENV_GEMINI_BIN) {
            let trimmed = explicit.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
        Ok(which::which("gemini")?)
    }

    /// Construct an adapter from environment + classic creds path.
    ///
    /// # Errors
    ///
    /// Surfaces any [`GeminiCliError`] from binary discovery or creds parsing.
    pub fn from_env() -> Result<Self, GeminiCliError> {
        let bin = Self::resolve_bin()?;
        let creds = GeminiCliCredentials::from_default_path()?;
        Ok(Self { bin, creds })
    }

    /// Bearer token suitable for `Authorization: Bearer <token>` headers, if
    /// the parsed credentials include one.
    #[must_use]
    pub fn bearer_token(&self) -> Option<&str> {
        self.creds.access_token.as_deref()
    }
}

#[async_trait::async_trait]
impl GatewayAdapter for GeminiCliAdapter {
    async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse, GatewayError> {
        // Intentionally do NOT shell out from inside aphrody-gateway: that is
        // the job of the `gemini-runtime` crate (`detect()` + `turn()`).
        // Returning a structured error keeps the contract explicit.
        Err(GatewayError::Config(
            "GeminiCliAdapter: use gemini_runtime::detect() + GeminiRuntime::turn() to drive \
             the local gemini binary; aphrody-gateway only surfaces creds + bin path"
                .to_owned(),
        ))
    }

    async fn chat_stream(&self, _req: ChatRequest) -> Result<ChunkStream, GatewayError> {
        Err(GatewayError::Config(
            "GeminiCliAdapter: streaming is provided by gemini_runtime::GeminiRuntime::turn (\
             stream-json NDJSON) — call that crate directly"
                .to_owned(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_classic_creds_json() {
        let raw = r#"{
            "access_token": "ya29.AAAA",
            "refresh_token": "1//RRRR",
            "expiry_date": 1700000000000,
            "token_type": "Bearer"
        }"#;
        let creds = GeminiCliCredentials::parse_json(raw).expect("parse");
        assert_eq!(creds.access_token.as_deref(), Some("ya29.AAAA"));
        assert_eq!(creds.refresh_token.as_deref(), Some("1//RRRR"));
        assert_eq!(creds.expiry_date, Some(1_700_000_000_000));
        assert_eq!(creds.token_type.as_deref(), Some("Bearer"));
        assert!(creds.project_id.is_none());
    }

    #[test]
    fn parse_oauth_token_shared_shape() {
        // Mirrors `formatGoogleOauthApiKey` output in
        // openclaw/extensions/google/oauth-token-shared.ts which uses
        // `{ "token": "...", "projectId": "..." }`.
        let raw = r#"{"token":"ya29.ZZ","projectId":"my-proj"}"#;
        let creds = GeminiCliCredentials::parse_json(raw).expect("parse");
        assert_eq!(creds.access_token.as_deref(), Some("ya29.ZZ"));
        assert_eq!(creds.project_id.as_deref(), Some("my-proj"));
    }

    #[test]
    fn parse_malformed_json_errors() {
        let err = GeminiCliCredentials::parse_json("{not json").unwrap_err();
        assert!(matches!(err, GeminiCliError::CredsParse(_)));
    }

    #[test]
    fn expiry_checks_use_provided_clock() {
        let creds = GeminiCliCredentials {
            expiry_date: Some(1_000),
            ..GeminiCliCredentials::default()
        };
        assert!(creds.is_expired_at(2_000));
        assert!(!creds.is_expired_at(500));

        let no_expiry = GeminiCliCredentials::default();
        assert!(!no_expiry.is_expired_at(i64::MAX));
    }

    #[test]
    fn resolve_bin_honors_aphrody_env_override() {
        // Use a path we control rather than depending on `which`.
        let fake = if cfg!(windows) { r"C:\fake\gemini.exe" } else { "/opt/fake/gemini" };
        // SAFETY: tests are single-threaded by default for env mutation here.
        // We restore the variable on exit.
        let prev = env::var(ENV_GEMINI_BIN).ok();
        unsafe {
            env::set_var(ENV_GEMINI_BIN, fake);
        }
        let resolved = GeminiCliAdapter::resolve_bin().expect("resolved");
        assert_eq!(resolved, PathBuf::from(fake));
        unsafe {
            match prev {
                Some(v) => env::set_var(ENV_GEMINI_BIN, v),
                None => env::remove_var(ENV_GEMINI_BIN),
            }
        }
    }

    #[tokio::test]
    async fn chat_returns_config_error_pointing_to_runtime_crate() {
        let adapter = GeminiCliAdapter {
            bin: PathBuf::from("/dev/null"),
            creds: GeminiCliCredentials::default(),
        };
        let err = adapter
            .chat(ChatRequest::new("gemini-2.5-flash", vec![]))
            .await
            .unwrap_err();
        match err {
            GatewayError::Config(msg) => {
                assert!(msg.contains("gemini_runtime"), "msg={msg}");
            },
            other => panic!("expected Config error, got {other:?}"),
        }
    }
}
