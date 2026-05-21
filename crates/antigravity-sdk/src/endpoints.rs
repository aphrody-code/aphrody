// SPDX-License-Identifier: Apache-2.0
//! Public network surface of the Antigravity (Google AI Ultra / Gemini)
//! desktop client.
//!
//! These constants were reverse-engineered from the Antigravity 2.0.1 client
//! and confirmed against its live OAuth flow. They are **not secrets**: OAuth
//! client IDs and endpoint hosts are embedded in every distributed copy of the
//! application (desktop OAuth uses the Authorization-Code + PKCE flow, which
//! has no confidential client secret). The user's tokens are *not* stored
//! here — read those at runtime via [`crate::auth::token_from_credential_manager`].
//!
//! ## Architecture
//!
//! The Antigravity desktop app is an Electron shell that launches a
//! Codeium/Windsurf-derived Go `language_server.exe` (`--standalone
//! --subclient_type hub`). All auth + API traffic flows through that local
//! language server, which exposes a self-signed, cert-pinned local HTTPS
//! endpoint speaking Codeium `exa.*` protobuf gRPC. This SDK targets the
//! upstream Google endpoints directly with the OAuth Bearer token rather than
//! the local gRPC bridge, which is the simpler and more portable surface.

// ---------------------------------------------------------------------------
// OAuth 2.0
// ---------------------------------------------------------------------------

/// Secondary OAuth 2.0 client-id also embedded in the desktop binary
/// (the primary one is [`crate::auth::ANTIGRAVITY_CLIENT_ID`]).
pub const ANTIGRAVITY_CLIENT_ID_SECONDARY: &str =
    "884354919052-36trc1jjb3tguiac32ov6cod268c5blh.apps.googleusercontent.com";

/// Loopback redirect URI used by the desktop sign-in flow.
pub const OAUTH_REDIRECT_URI: &str = "http://localhost:9109/oauth-callback";

/// Google OAuth 2.0 authorization endpoint (browser redirect target).
pub const OAUTH_AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Google OAuth 2.0 `tokeninfo` endpoint (validate an access token).
pub const OAUTH_TOKENINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v3/tokeninfo";

/// Google OpenID `userinfo` endpoint (email + profile of the signed-in user).
pub const OAUTH_USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

// ---------------------------------------------------------------------------
// API hosts
// ---------------------------------------------------------------------------

/// Gemini generative-language API host (`generateContent`, models, etc.).
pub const GEMINI_API_HOST: &str = "https://generativelanguage.googleapis.com";

/// Vertex AI host (publishers/google + publishers/anthropic models).
pub const VERTEX_AI_HOST: &str = "https://aiplatform.googleapis.com";

/// Cloud Code "Private API" host. The desktop client defaults its runtime to
/// [`Daily`](CloudCodeEndpoint::Daily) but ships [`Prod`](CloudCodeEndpoint::Prod)
/// as the stable surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudCodeEndpoint {
    /// Production Cloud Code endpoint (`cloudcode-pa.googleapis.com`).
    Prod,
    /// Daily / runtime-default Cloud Code endpoint
    /// (`daily-cloudcode-pa.googleapis.com`).
    Daily,
}

impl CloudCodeEndpoint {
    /// The HTTPS host (no trailing slash) for this endpoint.
    #[must_use]
    pub const fn host(self) -> &'static str {
        match self {
            CloudCodeEndpoint::Prod => "https://cloudcode-pa.googleapis.com",
            CloudCodeEndpoint::Daily => "https://daily-cloudcode-pa.googleapis.com",
        }
    }

    /// Build the absolute URL for a Cloud Code method on this endpoint.
    ///
    /// `method` is one of the `METHOD_*` path constants (e.g.
    /// [`METHOD_LOAD_CODE_ASSIST`]).
    #[must_use]
    pub fn url(self, method: &str) -> String {
        format!("{}{method}", self.host())
    }
}

impl Default for CloudCodeEndpoint {
    /// Mirrors the desktop client's runtime default ([`Daily`](Self::Daily)).
    fn default() -> Self {
        CloudCodeEndpoint::Daily
    }
}

// ---------------------------------------------------------------------------
// Cloud Code `v1internal` method paths
// ---------------------------------------------------------------------------

/// `POST /v1internal:loadCodeAssist` — bootstrap the Code Assist session
/// (project, tier, entitlements) for the signed-in user.
pub const METHOD_LOAD_CODE_ASSIST: &str = "/v1internal:loadCodeAssist";

/// `POST /v1internal:fetchAvailableModels` — list the models available to the
/// user's account / tier.
pub const METHOD_FETCH_AVAILABLE_MODELS: &str = "/v1internal:fetchAvailableModels";

/// `POST /v1internal:onboardUser` — onboarding / first-run provisioning.
pub const METHOD_ONBOARD_USER: &str = "/v1internal:onboardUser";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_code_default_is_daily() {
        assert_eq!(CloudCodeEndpoint::default(), CloudCodeEndpoint::Daily);
    }

    #[test]
    fn cloud_code_url_composition() {
        assert_eq!(
            CloudCodeEndpoint::Prod.url(METHOD_LOAD_CODE_ASSIST),
            "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist"
        );
        assert_eq!(
            CloudCodeEndpoint::Daily.url(METHOD_FETCH_AVAILABLE_MODELS),
            "https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels"
        );
    }

    #[test]
    fn hosts_have_no_trailing_slash() {
        assert!(!CloudCodeEndpoint::Prod.host().ends_with('/'));
        assert!(!CloudCodeEndpoint::Daily.host().ends_with('/'));
        assert!(!GEMINI_API_HOST.ends_with('/'));
        assert!(!VERTEX_AI_HOST.ends_with('/'));
    }

    #[test]
    fn secondary_client_id_is_well_formed() {
        assert!(ANTIGRAVITY_CLIENT_ID_SECONDARY.ends_with(".apps.googleusercontent.com"));
    }
}
