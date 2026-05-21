// SPDX-License-Identifier: Apache-2.0
//! Authenticated HTTP client for the Antigravity API.
//!
//! [`AntigravityClient`] wraps a [`reqwest::Client`] together with an
//! [`OAuthToken`] and automatically injects a `Bearer` authorization header
//! on every request.

use tracing::debug;

use crate::auth::{OAuthToken, token_from_credential_manager};
use crate::error::SdkError;

/// Authenticated HTTP client for the Antigravity (Google AI Ultra / Gemini)
/// API surface.
///
/// Construct with [`AntigravityClient::new`] when you already have a token,
/// or with [`AntigravityClient::from_credential_manager`] to load the token
/// from the Windows Credential Manager automatically.
///
/// All requests include `Authorization: Bearer <access_token>`.
pub struct AntigravityClient {
    http: reqwest::Client,
    token: OAuthToken,
}

impl AntigravityClient {
    /// Create a new client from an existing `http` client and `token`.
    ///
    /// The caller is responsible for ensuring the token is valid (not expired).
    /// Use [`AntigravityClient::refresh_token`] to rotate the token when
    /// necessary.
    pub fn new(http: reqwest::Client, token: OAuthToken) -> Self {
        Self { http, token }
    }

    /// Create a new client by reading the OAuth token from the Windows
    /// Credential Manager entry `gemini:antigravity`.
    ///
    /// A default [`reqwest::Client`] is constructed internally.
    ///
    /// # Errors
    ///
    /// Propagates any [`SdkError`] returned by
    /// [`token_from_credential_manager`] (including
    /// [`SdkError::Unsupported`] on non-Windows).
    pub fn from_credential_manager() -> Result<Self, SdkError> {
        let token = token_from_credential_manager()?;
        // Build a minimal reqwest::Client.  The caller must have installed a
        // rustls CryptoProvider before reaching here (cf. CLAUDE.md §7).
        let http = reqwest::Client::new();
        debug!(
            access_token_prefix = &token.access_token[..token.access_token.len().min(10)],
            "AntigravityClient created from credential manager"
        );
        Ok(Self { http, token })
    }

    /// Return a reference to the currently loaded [`OAuthToken`].
    pub fn token(&self) -> &OAuthToken {
        &self.token
    }

    /// Refresh the access token using the stored refresh token and replace the
    /// internal token with the freshly issued one.
    ///
    /// # Errors
    ///
    /// Propagates [`SdkError`] from [`OAuthToken::refresh`].
    pub async fn refresh_token(&mut self) -> Result<(), SdkError> {
        let fresh = self.token.refresh(&self.http).await?;
        debug!("AntigravityClient: token refreshed successfully");
        self.token = fresh;
        Ok(())
    }

    /// Perform an authenticated GET request and deserialize the response body
    /// as a [`serde_json::Value`].
    ///
    /// The `url` parameter must be a valid absolute URL.
    ///
    /// # Errors
    ///
    /// * [`SdkError::Http`] — transport or serialization failure.
    /// * [`SdkError::OAuthServer`] — non-2xx HTTP response.
    /// * [`SdkError::TokenParse`] — response body is not valid JSON.
    pub async fn get_json(&self, url: &str) -> Result<serde_json::Value, SdkError> {
        debug!(url, "AntigravityClient::get_json");
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token.access_token)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SdkError::OAuthServer {
                status: status.as_u16(),
                body,
            });
        }

        let body_text = response.text().await?;
        let value: serde_json::Value = serde_json::from_str(&body_text)?;
        Ok(value)
    }

    /// Perform an authenticated POST request with a JSON body and deserialize
    /// the response as a [`serde_json::Value`].
    ///
    /// This is the lower-level building block for Gemini / Antigravity RPC
    /// endpoints that accept JSON payloads.
    ///
    /// # Errors
    ///
    /// Same as [`AntigravityClient::get_json`].
    pub async fn post_json(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        debug!(url, "AntigravityClient::post_json");
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token.access_token)
            .json(body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(SdkError::OAuthServer {
                status: status.as_u16(),
                body: body_text,
            });
        }

        let body_text = response.text().await?;
        let value: serde_json::Value = serde_json::from_str(&body_text)?;
        Ok(value)
    }

    // -- Typed API surface ---------------------------------------------------

    /// Fetch the signed-in user's profile (email + name) from Google's OpenID
    /// `userinfo` endpoint.
    ///
    /// # Errors
    ///
    /// Same as [`AntigravityClient::get_json`].
    pub async fn userinfo(&self) -> Result<serde_json::Value, SdkError> {
        self.get_json(crate::endpoints::OAUTH_USERINFO_ENDPOINT).await
    }

    /// Call a Cloud Code `v1internal` method, composing the URL from a
    /// [`CloudCodeEndpoint`](crate::endpoints::CloudCodeEndpoint) and a
    /// `METHOD_*` path constant.
    ///
    /// # Errors
    ///
    /// Same as [`AntigravityClient::post_json`].
    pub async fn cloud_code(
        &self,
        endpoint: crate::endpoints::CloudCodeEndpoint,
        method: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        self.post_json(&endpoint.url(method), body).await
    }

    /// `loadCodeAssist` — bootstrap the Code Assist session for the user on the
    /// production Cloud Code endpoint.
    ///
    /// # Errors
    ///
    /// Same as [`AntigravityClient::post_json`].
    pub async fn load_code_assist(
        &self,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        self.cloud_code(
            crate::endpoints::CloudCodeEndpoint::Prod,
            crate::endpoints::METHOD_LOAD_CODE_ASSIST,
            body,
        )
        .await
    }

    /// `fetchAvailableModels` — list models available to the user's tier on the
    /// production Cloud Code endpoint.
    ///
    /// # Errors
    ///
    /// Same as [`AntigravityClient::post_json`].
    pub async fn fetch_available_models(
        &self,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        self.cloud_code(
            crate::endpoints::CloudCodeEndpoint::Prod,
            crate::endpoints::METHOD_FETCH_AVAILABLE_MODELS,
            body,
        )
        .await
    }
}
