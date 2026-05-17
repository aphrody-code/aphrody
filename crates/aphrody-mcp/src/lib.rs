// SPDX-License-Identifier: Apache-2.0

//! Native Rust OAuth 2.1 client for HTTP/SSE MCP servers.
//!
//! # RFC coverage
//!
//! | RFC   | Topic                                  | Module           |
//! |-------|----------------------------------------|------------------|
//! | 9728  | Protected Resource Metadata discovery  | [`discovery`]    |
//! | 8414  | Authorization Server Metadata          | [`discovery`]    |
//! | 7591  | Dynamic Client Registration            | [`registration`] |
//! | 7636  | PKCE (S256)                            | [`pkce`]         |
//! | 6749  | Authorization Code Grant + Refresh     | [`flow`]         |
//! | 8707  | Resource Indicators (optional param)   | [`flow`]         |
//!
//! Token persistence is the caller's responsibility — this crate is the
//! pure protocol layer.
// (implementation module-level comments continue below)
//
// RFC coverage:
//   - RFC 9728 : Protected Resource Metadata discovery  (`discovery`)
//   - RFC 8414 : Authorization Server Metadata discovery (`discovery`)
//   - RFC 7591 : Dynamic Client Registration            (`registration`)
//   - RFC 7636 : PKCE (S256)                           (`pkce`)
//   - RFC 6749 : Authorization Code Grant + Refresh    (`flow`)
//   - RFC 8707 : Resource Indicators                   (`flow` — optional param)
//
// Token persistence is the caller's responsibility.  This crate is the
// protocol layer only.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// ── Public modules ────────────────────────────────────────────────────────────

/// RFC 9728 + RFC 8414 well-known endpoint discovery.
pub mod discovery;

/// RFC 7591 dynamic client registration + disk cache.
pub mod registration;

/// RFC 7636 PKCE: code_verifier generation and S256 challenge derivation.
pub mod pkce;

/// RFC 6749 §4.1 authorization URL builder, code exchange, token refresh.
pub mod flow;

/// In-memory pending-authorization-state cache (single-use, TTL-bounded).
pub mod callback_state;

// ── Re-exports ────────────────────────────────────────────────────────────────

pub use callback_state::{PendingAuthCache, PendingAuthState};
pub use discovery::{AuthorizationServerMetadata, ProtectedResourceMetadata};
pub use flow::{AuthorizeUrlParams, ExchangeCodeParams, RefreshParams, TokenSet};
pub use pkce::{Pkce, derive_challenge, generate_state};
pub use registration::RegisteredClient;

// ── Error type ────────────────────────────────────────────────────────────────

/// All errors produced by this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A URL could not be parsed as a valid absolute URI.
    #[error("invalid URL `{url}`: {source}")]
    InvalidUrl {
        /// The unparseable URL string.
        url: String,
        /// Underlying parse error.
        #[source]
        source: url::ParseError,
    },

    /// An HTTP request failed at the transport layer (before a status code was
    /// received, or while reading the response body).
    #[error("HTTP request to `{url}` failed: {source}")]
    Http {
        /// Target URL.
        url: String,
        /// Underlying reqwest error.
        #[source]
        source: reqwest::Error,
    },

    /// The authorization server's `registration_endpoint` returned a non-2xx
    /// status or a body that did not include `client_id`.
    #[error("dynamic client registration at `{endpoint}` failed (HTTP {status}): {body}")]
    RegistrationFailed {
        /// Registration endpoint URL.
        endpoint: String,
        /// HTTP status code, or 0 if the failure was at the body-parse stage.
        status: u16,
        /// First 500 bytes of the response body (for diagnostics).
        body: String,
    },

    /// The auth server does not publish a `registration_endpoint` and no
    /// cached client exists for the requested `(issuer, redirect_uri)` pair.
    #[error("auth server `{issuer}` has no registration_endpoint and no cached client")]
    NoRegistrationEndpoint {
        /// Issuer URI of the auth server.
        issuer: String,
    },

    /// The token endpoint returned a non-2xx status or a body without
    /// `access_token`.
    #[error("token endpoint `{endpoint}` rejected request (HTTP {status}): {body}")]
    TokenEndpointFailed {
        /// Token endpoint URL.
        endpoint: String,
        /// HTTP status code.
        status: u16,
        /// First 500 bytes of the response body.
        body: String,
    },

    /// Failed to read or write the client-registration cache file.
    #[error("cache I/O error at `{path}`: {source}")]
    CacheIo {
        /// Filesystem path involved in the failed operation.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

// ── Top-level flow orchestration ──────────────────────────────────────────────

use std::path::Path;

use reqwest::Client;

/// Parameters for the full pre-redirect half of the OAuth dance.
#[derive(Debug)]
pub struct BeginAuthParams<'a> {
    /// Unique identifier for the MCP server being authenticated.
    pub server_id: &'a str,
    /// MCP server URL (used for RFC 9728 discovery + RFC 8707 resource).
    pub server_url: &'a str,
    /// OAuth redirect URI that will receive the authorization code callback.
    pub redirect_uri: &'a str,
    /// Directory where `mcp-oauth-clients.json` cache is stored.
    pub data_dir: &'a Path,
    /// Optional scope override.  When `None`, scopes are inferred from the
    /// protected-resource or authorization-server metadata.
    pub scope: Option<&'a str>,
}

/// Result of `begin_auth`: everything needed to redirect the user and later
/// complete the exchange at the callback endpoint.
#[derive(Debug)]
pub struct BeginAuthResult {
    /// Full authorization endpoint URL to redirect the user's browser to.
    pub authorize_url: String,
    /// Opaque state token — must be verified at callback time.
    pub state: String,
    /// State value to store in `PendingAuthCache::put(state, pending)`.
    pub pending: PendingAuthState,
}

/// Run the entire pre-redirect half of the OAuth 2.1 dance:
///
/// 1. RFC 9728 protected-resource discovery (to find the auth server).
/// 2. RFC 8414 auth server discovery.
/// 3. RFC 7591 dynamic client registration (cached).
/// 4. RFC 7636 PKCE verifier + challenge generation.
/// 5. RFC 8707 authorize URL construction.
///
/// Returns a `BeginAuthResult` whose `authorize_url` should be opened in the
/// user's browser and whose `pending` should be stored in a `PendingAuthCache`
/// keyed by `state`.
///
/// # Errors
///
/// - `Error::InvalidUrl` — `server_url` is not a valid URL.
/// - `Error::InvalidUrl` — auth server issuer URL unparseable.
/// - Propagates errors from `discover_auth_server`, `get_or_register_client`, and
///   `build_authorize_url`.
pub async fn begin_auth(
    http: &Client,
    params: &BeginAuthParams<'_>,
) -> Result<BeginAuthResult, Error> {
    use std::time::Instant;

    // Step 1: protected-resource discovery → find the auth server issuer.
    let prm = discovery::discover_protected_resource(http, params.server_url).await?;

    let issuer_hint = prm
        .as_ref()
        .and_then(|m| m.authorization_servers.as_deref())
        .and_then(|v| v.first())
        .map(String::as_str);

    let parsed_server = url::Url::parse(params.server_url)
        .map_err(|e| Error::InvalidUrl { url: params.server_url.to_owned(), source: e })?;
    let origin = format!("{}://{}", parsed_server.scheme(), parsed_server.host_str().unwrap_or(""));
    let issuer = issuer_hint.unwrap_or(origin.as_str());

    // Step 2: auth server discovery.
    let auth_server = discovery::discover_auth_server(http, issuer)
        .await?
        .ok_or_else(|| Error::NoRegistrationEndpoint { issuer: issuer.to_owned() })?;

    // Step 3: get or register client (cached DCR).
    let client_reg = registration::get_or_register_client(
        http,
        params.data_dir,
        &auth_server,
        params.redirect_uri,
    )
    .await?;

    // Step 4: PKCE + state.
    let pkce = Pkce::new();
    let state = generate_state();

    // Step 5: resolve scope.
    let scope_owned: Option<String>;
    let scope = if let Some(s) = params.scope {
        Some(s)
    } else if let Some(prm_scopes) = prm.as_ref().and_then(|m| m.scopes_supported.as_deref()) {
        if !prm_scopes.is_empty() {
            scope_owned = Some(prm_scopes.join(" "));
            scope_owned.as_deref()
        } else {
            scope_owned = auth_server.scopes_supported.as_deref().map(|s| s.join(" "));
            scope_owned.as_deref()
        }
    } else {
        scope_owned = auth_server.scopes_supported.as_deref().map(|s| s.join(" "));
        scope_owned.as_deref()
    };

    // Step 6: resource indicator (RFC 8707).
    let resource_url =
        prm.as_ref().and_then(|m| m.resource.as_deref()).unwrap_or(params.server_url);

    let authorize_url = flow::build_authorize_url(&AuthorizeUrlParams {
        authorization_endpoint: &auth_server.authorization_endpoint,
        client_id: &client_reg.client_id,
        redirect_uri: params.redirect_uri,
        state: &state,
        code_challenge: &pkce.code_challenge,
        scope,
        resource: Some(resource_url),
    })?;

    let pending = PendingAuthState {
        server_id: params.server_id.to_owned(),
        auth_server_issuer: auth_server.issuer.clone(),
        token_endpoint: auth_server.token_endpoint.clone(),
        client_id: client_reg.client_id.clone(),
        client_secret: client_reg.client_secret.clone(),
        redirect_uri: params.redirect_uri.to_owned(),
        code_verifier: pkce.code_verifier,
        scope: scope.map(str::to_owned),
        resource_url: Some(resource_url.to_owned()),
        created_at: Instant::now(),
    };

    Ok(BeginAuthResult { authorize_url, state, pending })
}
