// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Adobe IMS OAuth **server-to-server** (client-credentials) token acquisition.
//!
//! This is the auth core shared across all Firefly Services REST APIs — the
//! Firefly image API *and* the cloud Photoshop / Lightroom APIs all accept the
//! same IMS access token (different `scope` is not required; the
//! `firefly_api,ff_apis` scopes cover the family). Acquire once, reuse until it
//! is near expiry.
//!
//! Verified against the Adobe getting-started docs (2026-05):
//! `POST https://ims-na1.adobelogin.com/ims/token/v3`, form-encoded
//! `grant_type=client_credentials&client_id&client_secret&scope=...`.

use crate::error::{FireflyError, Result};
use std::time::{Duration, Instant};

/// Adobe IMS token endpoint (North-America region — the documented default).
pub const IMS_TOKEN_ENDPOINT: &str = "https://ims-na1.adobelogin.com/ims/token/v3";

/// Scope string required for the Firefly Services family (Firefly + Photoshop +
/// Lightroom share these scopes), copied verbatim from the Adobe docs.
pub const FIREFLY_SCOPE: &str =
    "openid,AdobeID,session,additional_info,read_organizations,firefly_api,ff_apis";

/// Refresh the token this many seconds *before* its real expiry, so an in-flight
/// request never races the boundary.
const EXPIRY_SAFETY_MARGIN_SECS: u64 = 60;

/// OAuth server-to-server credentials (a Developer Console project's
/// client id + secret). The secret is held only in memory and never logged.
#[derive(Clone)]
pub struct ImsCredentials {
    /// The Developer Console **Client ID** (also sent as the `x-api-key`
    /// header on Firefly REST calls).
    pub client_id: String,
    /// The Developer Console **Client Secret**.
    pub client_secret: String,
}

impl std::fmt::Debug for ImsCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never leak the secret through Debug.
        f.debug_struct("ImsCredentials")
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .finish()
    }
}

impl ImsCredentials {
    /// Read credentials from `FIREFLY_CLIENT_ID` / `FIREFLY_CLIENT_SECRET`.
    ///
    /// # Errors
    ///
    /// [`FireflyError::MissingCredential`] when either variable is absent or empty.
    pub fn from_env() -> Result<Self> {
        let client_id = non_empty_env("FIREFLY_CLIENT_ID")
            .ok_or(FireflyError::MissingCredential("FIREFLY_CLIENT_ID"))?;
        let client_secret = non_empty_env("FIREFLY_CLIENT_SECRET")
            .ok_or(FireflyError::MissingCredential("FIREFLY_CLIENT_SECRET"))?;
        Ok(Self { client_id, client_secret })
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// The raw shape of a successful IMS token response.
#[derive(serde::Deserialize)]
struct ImsTokenResponse {
    access_token: String,
    /// Lifetime of the token. **Adobe IMS reports this in milliseconds**, not
    /// seconds (a well-known quirk: `~86_399_999` for a 24 h token). See
    /// [`interpret_expires_in`].
    expires_in: u64,
}

/// A cached bearer token plus the instant after which it must be refreshed.
#[derive(Clone)]
pub struct AccessToken {
    /// The bearer token value (sent as `Authorization: Bearer <token>`).
    pub token: String,
    /// Monotonic instant at which the token should be considered expired
    /// (already includes the safety margin).
    pub refresh_after: Instant,
}

impl AccessToken {
    /// `true` when the token is still safely usable.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.refresh_after
    }
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessToken")
            .field("token", &"<redacted>")
            .field("valid", &self.is_valid())
            .finish()
    }
}

/// Interpret the IMS `expires_in` field as a [`Duration`], compensating for
/// Adobe's millisecond reporting.
///
/// Heuristic: any value above `100_000` is treated as milliseconds (no real
/// IMS token lives ~27 hours when read as seconds; a 24 h token reads as
/// `~86_400_000` ms). Values at or below the threshold are treated as seconds.
/// A `EXPIRY_SAFETY_MARGIN_SECS` margin is subtracted so callers refresh early.
#[must_use]
pub fn interpret_expires_in(expires_in: u64) -> Duration {
    let secs = if expires_in > 100_000 { expires_in / 1000 } else { expires_in };
    let usable = secs.saturating_sub(EXPIRY_SAFETY_MARGIN_SECS);
    Duration::from_secs(usable)
}

/// Exchange client credentials for an IMS access token via `client`.
///
/// # Errors
///
/// * [`FireflyError::Auth`] when IMS returns a non-2xx status.
/// * [`FireflyError::Http`] on transport failure.
/// * [`FireflyError::Decode`] when the token JSON cannot be parsed.
pub async fn fetch_token(
    client: &reqwest::Client,
    creds: &ImsCredentials,
) -> Result<AccessToken> {
    let form = [
        ("grant_type", "client_credentials"),
        ("client_id", creds.client_id.as_str()),
        ("client_secret", creds.client_secret.as_str()),
        ("scope", FIREFLY_SCOPE),
    ];

    let resp = client
        .post(IMS_TOKEN_ENDPOINT)
        .form(&form)
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        return Err(FireflyError::Auth {
            status: status.as_u16(),
            body: truncate(&body, 512),
        });
    }

    let parsed: ImsTokenResponse = serde_json::from_str(&body).map_err(|source| {
        FireflyError::Decode { endpoint: IMS_TOKEN_ENDPOINT.to_string(), source }
    })?;

    let lifetime = interpret_expires_in(parsed.expires_in);
    tracing::debug!(
        valid_for_secs = lifetime.as_secs(),
        "acquired Firefly IMS access token",
    );

    Ok(AccessToken {
        token: parsed.access_token,
        refresh_after: Instant::now() + lifetime,
    })
}

/// Truncate a string to at most `max` bytes on a char boundary (for safe error
/// bodies; never includes secrets — IMS error bodies are non-secret).
pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_in_milliseconds_is_converted_to_seconds() {
        // A 24 h token reported in ms.
        let d = interpret_expires_in(86_399_999);
        // 86_399 - 60 margin = 86_339 s.
        assert_eq!(d.as_secs(), 86_339);
    }

    #[test]
    fn expires_in_seconds_under_threshold_is_used_directly() {
        // A short token already reported in seconds.
        let d = interpret_expires_in(3600);
        assert_eq!(d.as_secs(), 3540); // minus 60 s margin
    }

    #[test]
    fn expires_in_below_margin_saturates_to_zero() {
        let d = interpret_expires_in(30);
        assert_eq!(d.as_secs(), 0);
    }

    #[test]
    fn scope_contains_firefly_apis() {
        assert!(FIREFLY_SCOPE.contains("firefly_api"));
        assert!(FIREFLY_SCOPE.contains("ff_apis"));
    }

    #[test]
    fn debug_redacts_secret() {
        let c = ImsCredentials {
            client_id: "pub-id".into(),
            client_secret: "TOPSECRET".into(),
        };
        let rendered = format!("{c:?}");
        assert!(rendered.contains("pub-id"));
        assert!(!rendered.contains("TOPSECRET"));
        assert!(rendered.contains("redacted"));
    }

    #[test]
    fn token_validity_reflects_refresh_instant() {
        let valid = AccessToken {
            token: "x".into(),
            refresh_after: Instant::now() + Duration::from_secs(60),
        };
        assert!(valid.is_valid());

        let expired = AccessToken {
            token: "x".into(),
            refresh_after: Instant::now() - Duration::from_secs(1),
        };
        assert!(!expired.is_valid());

        // Debug never leaks the token.
        assert!(!format!("{valid:?}").contains('x') || format!("{valid:?}").contains("redacted"));
    }

    #[test]
    fn truncate_respects_char_boundary() {
        let s = "ééééééé"; // multi-byte chars
        let t = truncate(s, 5);
        assert!(t.ends_with('…'));
        // Must not panic / split a char — the prefix is valid UTF-8 by construction.
        assert!(t.len() <= 5 + '…'.len_utf8() + 1);
    }
}
