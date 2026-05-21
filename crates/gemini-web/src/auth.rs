// SPDX-License-Identifier: Apache-2.0
//! Authentication for the Gemini web client.
//!
//! Gemini reuses the signed-in Google session cookies. The user exports them
//! from a logged-in browser (e.g. the Cookie-Editor extension) into a JSON
//! file; the headless client replays the jar on every `batchexecute` POST. The
//! anti-CSRF `at` token is NOT a cookie — it is scraped from the app page at
//! bootstrap (see [`crate::bootstrap`]).
//!
//! Secrets stay on disk under the caller's home (`~/.aphrody/google-cookies.json`
//! by default); they are never written into the aphrody workspace.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{GeminiError, Result};

/// One entry in the cookie jar. Field aliases accept both the Cookie-Editor
/// (`httpOnly`, camel-case) and the snake-case (`http_only`) spellings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCookie {
    pub name: String,
    pub value: String,
    /// Domain the browser bound the cookie to (e.g. `".google.com"`).
    #[serde(default)]
    pub domain: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    /// Accepts `httpOnly` (Cookie-Editor) or `http_only`.
    #[serde(default, alias = "httpOnly")]
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, cookie: SessionCookie) {
        self.cookies.insert(cookie.name.clone(), cookie);
    }

    /// Build the flat `Cookie:` header value the Boq endpoint expects.
    #[must_use]
    pub fn header_value(&self) -> String {
        self.cookies
            .values()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Fetch a cookie value by name (used for `SAPISIDHASH` minting).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(|c| c.value.as_str())
    }

    /// Required tokens for a Gemini session — fail fast if any are missing.
    ///
    /// # Errors
    ///
    /// Returns [`GeminiError::Auth`] when a mandatory cookie is absent.
    pub fn require_google_session(&self) -> Result<()> {
        for needed in ["SAPISID", "__Secure-1PSID"] {
            if !self.cookies.contains_key(needed) {
                return Err(GeminiError::Auth(format!(
                    "cookie jar is missing `{needed}` — re-export the Google session"
                )));
            }
        }
        Ok(())
    }
}

/// Cookie-jar credential carrier for the Gemini web surface.
#[derive(Debug, Clone)]
pub struct Auth {
    jar: CookieJar,
}

impl Auth {
    /// Wrap a pre-built jar.
    #[must_use]
    pub fn from_jar(jar: CookieJar) -> Self {
        Self { jar }
    }

    /// Parse the JSON array produced by the Cookie-Editor browser extension
    /// (a flat `Vec<SessionCookie>`).
    ///
    /// # Errors
    ///
    /// Returns [`GeminiError::Auth`] when the JSON is malformed or the jar is
    /// missing a mandatory Google session cookie.
    pub fn from_cookie_editor_json(payload: &str) -> Result<Self> {
        let raw: Vec<SessionCookie> = serde_json::from_str(payload)
            .map_err(|e| GeminiError::Auth(format!("malformed cookie JSON: {e}")))?;
        let mut jar = CookieJar::new();
        for cookie in raw {
            jar.insert(cookie);
        }
        jar.require_google_session()?;
        Ok(Self { jar })
    }

    /// Load a Cookie-Editor export from a file (default
    /// `~/.aphrody/google-cookies.json`).
    ///
    /// # Errors
    ///
    /// Returns [`GeminiError::Network`] on IO failure or [`GeminiError::Auth`]
    /// on a malformed / incomplete jar.
    pub async fn from_cookie_file(path: impl AsRef<Path>) -> Result<Self> {
        let payload = tokio::fs::read_to_string(path.as_ref()).await?;
        Self::from_cookie_editor_json(&payload)
    }

    /// Borrow the underlying jar (for `SAPISIDHASH` minting / diagnostics).
    #[must_use]
    pub fn jar(&self) -> &CookieJar {
        &self.jar
    }

    /// Materialise the HTTP headers reqwest should ship with every request.
    #[must_use]
    pub fn request_headers(&self) -> Vec<(&'static str, String)> {
        vec![("Cookie", self.jar.header_value())]
    }
}

/// Compute the origin-bound `SAPISIDHASH` value the Google APIs gateway accepts
/// (`<unix_seconds>_<sha256(unix_seconds + ' ' + SAPISID + ' ' + origin)>`).
///
/// Not required by `batchexecute` (which uses the page `at` token), but exposed
/// so callers can mint the `Authorization: SAPISIDHASH` header for the public
/// Google APIs gateway with the same jar.
#[must_use]
pub fn sapisidhash(sapisid: &str, origin: &str, unix_seconds: u64) -> String {
    use sha2::Digest;
    let payload = format!("{unix_seconds} {sapisid} {origin}");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_editor_camelcase_httponly_parses() {
        let json = r#"[
            {"name":"SAPISID","value":"v1","domain":".google.com","path":"/","secure":true,"httpOnly":false},
            {"name":"__Secure-1PSID","value":"v2","domain":".google.com","path":"/","secure":true,"httpOnly":true}
        ]"#;
        let auth = Auth::from_cookie_editor_json(json).unwrap();
        assert_eq!(auth.jar().get("SAPISID"), Some("v1"));
        assert!(auth.jar().cookies["__Secure-1PSID"].http_only);
    }

    #[test]
    fn missing_required_cookie_is_rejected() {
        let json = r#"[{"name":"NID","value":"x","domain":".google.com"}]"#;
        let err = Auth::from_cookie_editor_json(json).unwrap_err();
        assert!(matches!(err, GeminiError::Auth(_)));
    }

    #[test]
    fn header_value_is_semicolon_joined() {
        let json = r#"[
            {"name":"SAPISID","value":"a"},
            {"name":"__Secure-1PSID","value":"b"}
        ]"#;
        let auth = Auth::from_cookie_editor_json(json).unwrap();
        // BTreeMap orders by name; 'S' (0x53) < '_' (0x5F), so SAPISID first.
        assert_eq!(auth.jar().header_value(), "SAPISID=a; __Secure-1PSID=b");
    }

    #[test]
    fn sapisidhash_is_deterministic() {
        let h = sapisidhash("SAPISIDVALUE", "https://gemini.google.com", 1_700_000_000);
        assert!(h.starts_with("1700000000_"));
        assert_eq!(h.len(), "1700000000_".len() + 64);
    }
}
