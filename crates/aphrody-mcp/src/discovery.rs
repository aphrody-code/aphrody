// SPDX-License-Identifier: Apache-2.0
//
// RFC 9728 — OAuth 2.0 Protected Resource Metadata discovery.
// RFC 8414 — OAuth 2.0 Authorization Server Metadata discovery.
//
// Discovery strategy mirrors the reference TypeScript implementation in
// `open-design/apps/daemon/src/mcp-oauth.ts`:
//
//   - RFC 9728: try path-suffixed well-known first, then bare root.
//   - RFC 8414: try `oauth-authorization-server` + `openid-configuration` variants, with and
//     without path suffix.  Accept whichever has both `authorization_endpoint` and `token_endpoint`
//     fields populated.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use url::Url;

use crate::Error;

// ── RFC 9728 ────────────────────────────────────────────────────────────────

/// Subset of the RFC 9728 `oauth-protected-resource` document fields used
/// by the OAuth 2.1 flow.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    /// Canonical resource identifier (URI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// List of authorization server issuer URIs that protect this resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_servers: Option<Vec<String>>,
    /// Scopes advertised by the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
}

// ── RFC 8414 ────────────────────────────────────────────────────────────────

/// Subset of the RFC 8414 (and OIDC Discovery) authorization server metadata
/// document fields used by the OAuth 2.1 flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationServerMetadata {
    /// Issuer identifier URI.  Always set (falls back to the discovery URL
    /// origin when the server does not include it).
    pub issuer: String,
    /// Authorization endpoint URI (RFC 6749 §3.1).
    pub authorization_endpoint: String,
    /// Token endpoint URI (RFC 6749 §3.2).
    pub token_endpoint: String,
    /// Dynamic Client Registration endpoint (RFC 7591).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_endpoint: Option<String>,
    /// Scopes the server supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
    /// Response types supported (e.g. `["code"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_types_supported: Option<Vec<String>>,
    /// Grant types supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_types_supported: Option<Vec<String>>,
    /// PKCE challenge methods (must contain `"S256"` for RFC 7636 compliance).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_challenge_methods_supported: Option<Vec<String>>,
    /// Token endpoint authentication methods.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint_auth_methods_supported: Option<Vec<String>>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Attempt to fetch the RFC 9728 protected-resource metadata for a given
/// MCP server URL.
///
/// Tries the path-suffixed well-known endpoint first, then the bare root
/// endpoint, returning the first valid JSON document or `None` if both fail.
///
/// # Errors
///
/// Returns `Err` only if `resource_url` cannot be parsed as a valid URL.
/// Network failures are silently treated as a missing document (`None`).
pub async fn discover_protected_resource(
    client: &Client,
    resource_url: &str,
) -> Result<Option<ProtectedResourceMetadata>, Error> {
    let parsed = Url::parse(resource_url)
        .map_err(|e| Error::InvalidUrl { url: resource_url.to_owned(), source: e })?;

    let origin = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
    let path = parsed.path().trim_end_matches('/');

    let candidates = [
        format!("{origin}/.well-known/oauth-protected-resource{path}"),
        format!("{origin}/.well-known/oauth-protected-resource"),
    ];

    for url in &candidates {
        if let Some(meta) = fetch_json::<ProtectedResourceMetadata>(client, url).await {
            debug!(%url, "RFC 9728 protected-resource metadata found");
            return Ok(Some(meta));
        }
    }
    debug!(resource_url, "no RFC 9728 metadata found");
    Ok(None)
}

/// Attempt to fetch the RFC 8414 authorization-server metadata for an issuer.
///
/// Tries four candidate URLs in order:
///   1. `/.well-known/oauth-authorization-server<path>`
///   2. `/.well-known/openid-configuration<path>`
///   3. `/.well-known/oauth-authorization-server`
///   4. `/.well-known/openid-configuration`
///
/// Accepts the first document that has both `authorization_endpoint` and
/// `token_endpoint` set.  When the document does not include an `issuer`
/// field, the provided `issuer` string is used as a fallback.
///
/// # Errors
///
/// Returns `Err` if `issuer` cannot be parsed as a valid URL.
/// Network failures are silently treated as a missing document (`None`).
pub async fn discover_auth_server(
    client: &Client,
    issuer: &str,
) -> Result<Option<AuthorizationServerMetadata>, Error> {
    let parsed =
        Url::parse(issuer).map_err(|e| Error::InvalidUrl { url: issuer.to_owned(), source: e })?;

    let origin = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
    let path = parsed.path().trim_end_matches('/');

    let candidates = [
        format!("{origin}/.well-known/oauth-authorization-server{path}"),
        format!("{origin}/.well-known/openid-configuration{path}"),
        format!("{origin}/.well-known/oauth-authorization-server"),
        format!("{origin}/.well-known/openid-configuration"),
    ];

    for url in &candidates {
        if let Some(mut meta) = fetch_json::<AuthorizationServerMetadata>(client, url).await {
            // Validate minimum required fields.
            if meta.authorization_endpoint.is_empty() || meta.token_endpoint.is_empty() {
                warn!(%url, "auth server metadata missing required endpoint fields — skipping");
                continue;
            }
            // If the document omits `issuer`, fall back to the discovery input.
            if meta.issuer.is_empty() {
                meta.issuer = issuer.to_owned();
            }
            debug!(%url, issuer = %meta.issuer, "RFC 8414 auth server metadata found");
            return Ok(Some(meta));
        }
    }
    debug!(issuer, "no RFC 8414 metadata found");
    Ok(None)
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Perform a GET to `url`, deserialize JSON into `T`, return `None` on any
/// non-success status or transport error (mirrors the TS `fetchJson` helper).
async fn fetch_json<T: for<'de> Deserialize<'de>>(client: &Client, url: &str) -> Option<T> {
    let resp =
        client.get(url).header(reqwest::header::ACCEPT, "application/json").send().await.ok()?;

    if !resp.status().is_success() {
        debug!(url, status = %resp.status(), "fetch_json non-success");
        return None;
    }

    match resp.json::<T>().await {
        Ok(v) => Some(v),
        Err(e) => {
            warn!(url, err = %e, "fetch_json JSON decode failed");
            None
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// Initialize the rustls ring provider once per test process.
    ///
    /// Required because reqwest with `rustls-no-provider` panics with
    /// "No provider set" if no `CryptoProvider` is installed before the first
    /// `Client::new()` (see CLAUDE.md §7 — rustls 0.23 CryptoProvider pitfall).
    fn init_rustls() {
        // `install_default` is idempotent if called multiple times — it
        // returns `Err` when already set, which we intentionally ignore.
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    // Parsing an invalid URL must surface an Error, not panic.
    #[test]
    fn invalid_url_returns_error() {
        init_rustls();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = Client::new();
        let err = rt.block_on(discover_protected_resource(&client, "not a url")).unwrap_err();
        assert!(matches!(err, Error::InvalidUrl { .. }));
    }
}
