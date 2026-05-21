// SPDX-License-Identifier: Apache-2.0
//! Authentication primitives for the NotebookLM RPC client.
//!
//! NotebookLM accepts two flavours of credentials:
//!
//! 1. **Cookie jar** — the user signs in on `accounts.google.com` and the
//!    resulting `__Secure-1PSID`, `SAPISID`, `HSID`, `SSID`, `APISID`
//!    cookies are reused by the headless client. Most existing TS/Python
//!    clients (icebear0828, teng-lin) ship a "browser export" flow that
//!    captures these into a JSON file via the Cookie-Editor extension.
//! 2. **OAuth access token** — for accounts with the Google AI Ultra
//!    subscription, the upstream also accepts a Bearer token issued through
//!    the standard OAuth 2.0 web flow (scope `cloud-platform` is enough).
//!
//! The two are mutually exclusive at construction time.  Both can be loaded
//! from environment variables (`NOTEBOOKLM_COOKIES` JSON / `NOTEBOOKLM_OAUTH_TOKEN`)
//! so secrets never have to be written to disk inside the aphrody workspace.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{NotebookError, Result};

/// One entry in the cookie jar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCookie {
    pub name: String,
    pub value: String,
    /// Domain the browser bound the cookie to (e.g. `".google.com"`).
    pub domain: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
}

fn default_path() -> String {
    "/".to_string()
}

/// Collection of cookies, indexed by name so dedup is cheap on import.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CookieJar {
    pub cookies: BTreeMap<String, SessionCookie>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, cookie: SessionCookie) {
        self.cookies.insert(cookie.name.clone(), cookie);
    }

    /// Build the flat `Cookie:` header value the Boq endpoint expects.
    pub fn header_value(&self) -> String {
        self.cookies
            .values()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Required tokens for a NotebookLM session — fail fast if any are missing.
    pub fn require_google_session(&self) -> Result<()> {
        for needed in ["SAPISID", "__Secure-1PSID"] {
            if !self.cookies.contains_key(needed) {
                return Err(NotebookError::Auth(format!(
                    "cookie jar is missing `{needed}` — re-export the Google session"
                )));
            }
        }
        Ok(())
    }
}

/// Top-level credential carrier.
#[derive(Debug, Clone)]
pub enum Auth {
    /// Reuse cookies harvested from a logged-in browser session.
    Cookies(CookieJar),
    /// Bearer access token (OAuth 2.0).
    OAuthAccessToken(String),
}

impl Auth {
    /// Read credentials from environment variables.
    ///
    /// Order of precedence:
    /// 1. `NOTEBOOKLM_OAUTH_TOKEN` — raw Bearer token.
    /// 2. `NOTEBOOKLM_COOKIES`     — JSON document in Cookie-Editor format.
    pub fn from_env() -> Result<Self> {
        if let Ok(token) = std::env::var("NOTEBOOKLM_OAUTH_TOKEN") {
            let trimmed = token.trim();
            if !trimmed.is_empty() {
                return Ok(Self::OAuthAccessToken(trimmed.to_string()));
            }
        }
        if let Ok(json) = std::env::var("NOTEBOOKLM_COOKIES") {
            return Self::from_chromium_export(&json);
        }
        Err(NotebookError::Auth(
            "no credentials found — set NOTEBOOKLM_OAUTH_TOKEN or NOTEBOOKLM_COOKIES".to_string(),
        ))
    }

    /// Parse the JSON document produced by the Cookie-Editor browser
    /// extension (a flat `Vec<SessionCookie>`).
    pub fn from_chromium_export(payload: &str) -> Result<Self> {
        let raw: Vec<SessionCookie> = serde_json::from_str(payload)
            .map_err(|e| NotebookError::Auth(format!("malformed cookie JSON: {e}")))?;
        let mut jar = CookieJar::new();
        for cookie in raw {
            jar.insert(cookie);
        }
        jar.require_google_session()?;
        Ok(Self::Cookies(jar))
    }

    /// Sniff the auth flavour name (handy for diagnostics).
    pub fn flavour(&self) -> &'static str {
        match self {
            Self::Cookies(_) => "cookies",
            Self::OAuthAccessToken(_) => "oauth",
        }
    }

    /// Materialise the HTTP headers reqwest should ship with every request.
    pub fn request_headers(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::Cookies(jar) => vec![("Cookie", jar.header_value())],
            Self::OAuthAccessToken(tok) => {
                vec![("Authorization", format!("Bearer {tok}"))]
            },
        }
    }
}

/// Compute the `SAPISIDHASH` value the Google APIs gateway accepts for
/// authenticated calls that bypass `XSRF-TOKEN` (origin-bound SHA-1 over
/// `<unix_seconds> <SAPISID> <origin>`).
///
/// Not strictly required by `batchexecute` (which uses the `at` token
/// extracted from the page), but exposed here so external callers can mint
/// the header on demand when they want to talk to the public Google APIs
/// gateway with the same cookie jar.
pub fn sapisidhash(sapisid: &str, origin: &str, unix_seconds: u64) -> String {
    use sha2::Digest;
    let payload = format!("{unix_seconds} {sapisid} {origin}");
    // SAPISIDHASH historically uses SHA-1, but Google nowadays accepts SHA-256
    // through the same scheme (`SAPISIDHASH_256_`). We expose SHA-256 to avoid
    // pulling a SHA-1 crate just for this helper — callers prepend the
    // `SAPISIDHASH_256_` prefix manually if they need the strict variant.
    let mut hasher = sha2::Sha256::new();
    hasher.update(payload.as_bytes());
    let digest = hasher.finalize();
    format!("{unix_seconds}_{}", hex_lowercase(&digest))
}

fn hex_lowercase(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
