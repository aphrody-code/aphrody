// SPDX-License-Identifier: Apache-2.0
//! Authenticated HTTP client for the Antigravity API.
//!
//! [`AntigravityClient`] wraps a [`reqwest::Client`] together with an
//! [`OAuthToken`] and automatically injects a `Bearer` authorization header
//! on every request.

use tracing::{debug, warn};

use crate::auth::{OAuthToken, token_from_credential_manager};
use crate::error::SdkError;
use crate::models::{
    CloudCodeGenerateContentResponse, FetchAvailableModelsRequest, FetchAvailableModelsResponse,
    GenerateContentRequest, GenerateContentResponse, LoadCodeAssistRequest, LoadCodeAssistResponse,
    OnboardUserRequest, OnboardUserResponse,
};

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
    /// Automatically retries on HTTP 429 (`RESOURCE_EXHAUSTED`): the Cloud Code
    /// modelbackend enforces a tight per-model quota on lower Code Assist tiers
    /// ("Your quota will reset after Ns"), and a single retryable 429 should not
    /// fail the whole turn. See [`post_json_with_retry`](Self::post_json_with_retry).
    ///
    /// # Errors
    ///
    /// Same as [`AntigravityClient::post_json`]; a 429 is only surfaced after the
    /// retry budget is exhausted.
    pub async fn cloud_code(
        &self,
        endpoint: crate::endpoints::CloudCodeEndpoint,
        method: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        self.post_json_with_retry(&endpoint.url(method), body).await
    }

    /// [`post_json`](Self::post_json) with automatic back-off on HTTP 429.
    ///
    /// On a `429 RESOURCE_EXHAUSTED` the Cloud Code body carries the reset delay
    /// (a structured `google.rpc.RetryInfo.retryDelay`, and/or the human message
    /// "…reset after Ns"). We wait that long (plus a small margin, capped) and
    /// retry, up to [`MAX_RETRY_ATTEMPTS`] total attempts. Non-429 errors (401
    /// token expiry, 403 scope, 404 model) are returned immediately — they are
    /// not transient and waiting would not help. Every other status is returned
    /// as-is.
    async fn post_json_with_retry(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        /// Total attempts (1 initial + up to N-1 retries).
        const MAX_RETRY_ATTEMPTS: u32 = 3;
        /// Hard cap on any single back-off wait, so a pathological server delay
        /// cannot hang an interactive turn indefinitely.
        const MAX_WAIT_SECS: u64 = 65;

        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            match self.post_json(url, body).await {
                Err(SdkError::OAuthServer { status: 429, body: err_body })
                    if attempt < MAX_RETRY_ATTEMPTS =>
                {
                    // Prefer the server-advertised delay; fall back to a small
                    // exponential back-off (2, 4, … s) when the body omits it.
                    let advertised = parse_retry_delay_secs(&err_body);
                    let wait = advertised
                        .unwrap_or_else(|| 2u64.saturating_pow(attempt))
                        .saturating_add(1) // +1s margin so the window has surely rolled over
                        .min(MAX_WAIT_SECS);
                    warn!(
                        attempt,
                        wait_secs = wait,
                        advertised = advertised.is_some(),
                        "Cloud Code 429 RESOURCE_EXHAUSTED — backing off then retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                },
                other => return other,
            }
        }
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

    // -- Typed wrappers ------------------------------------------------------

    /// Typed `loadCodeAssist`: serialize `req`, post it via the existing
    /// [`load_code_assist`](Self::load_code_assist) building block, and
    /// deserialize the JSON response into [`LoadCodeAssistResponse`].
    ///
    /// # Errors
    ///
    /// * Any [`SdkError`] from [`load_code_assist`](Self::load_code_assist).
    /// * [`SdkError::TokenParse`] if the request cannot be serialized or the
    ///   response cannot be deserialized into the typed shape.
    pub async fn load_code_assist_typed(
        &self,
        req: &LoadCodeAssistRequest,
    ) -> Result<LoadCodeAssistResponse, SdkError> {
        let body = serde_json::to_value(req)?;
        let value = self.load_code_assist(&body).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Typed `fetchAvailableModels`: serialize `req`, post it via the existing
    /// [`fetch_available_models`](Self::fetch_available_models) building block,
    /// and deserialize the JSON response into [`FetchAvailableModelsResponse`].
    ///
    /// # Errors
    ///
    /// * Any [`SdkError`] from
    ///   [`fetch_available_models`](Self::fetch_available_models).
    /// * [`SdkError::TokenParse`] on (de)serialization failure.
    pub async fn fetch_available_models_typed(
        &self,
        req: &FetchAvailableModelsRequest,
    ) -> Result<FetchAvailableModelsResponse, SdkError> {
        let body = serde_json::to_value(req)?;
        let value = self.fetch_available_models(&body).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Typed `onboardUser`: serialize `req`, post it to the production Cloud
    /// Code `v1internal:onboardUser` method, and deserialize the JSON response
    /// into [`OnboardUserResponse`].
    ///
    /// # Errors
    ///
    /// * Any [`SdkError`] from [`cloud_code`](Self::cloud_code).
    /// * [`SdkError::TokenParse`] on (de)serialization failure.
    pub async fn onboard_user(
        &self,
        req: &OnboardUserRequest,
    ) -> Result<OnboardUserResponse, SdkError> {
        let body = serde_json::to_value(req)?;
        let value = self
            .cloud_code(
                crate::endpoints::CloudCodeEndpoint::Prod,
                crate::endpoints::METHOD_ONBOARD_USER,
                &body,
            )
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Gemini `generateContent`: post `req` to
    /// `{GEMINI_API_HOST}/v1beta/models/{model}:generateContent` and
    /// deserialize the response into [`GenerateContentResponse`].
    ///
    /// `model` is a bare model id such as `"gemini-2.0-flash"`.
    ///
    /// # Errors
    ///
    /// * Any [`SdkError`] from [`post_json`](Self::post_json).
    /// * [`SdkError::TokenParse`] on (de)serialization failure.
    pub async fn generate_content(
        &self,
        model: &str,
        req: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse, SdkError> {
        let url = format!(
            "{}/v1beta/models/{model}:generateContent",
            crate::endpoints::GEMINI_API_HOST
        );
        let body = serde_json::to_value(req)?;
        let value = self.post_json(&url, &body).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Run `generateContent` against the **regional Vertex AI** endpoint.
    ///
    /// Unlike [`generate_content`](Self::generate_content) (which targets the
    /// public `generativelanguage` host and is rejected by the agy OAuth token
    /// with `401 ACCESS_TOKEN_TYPE_UNSUPPORTED`), this routes through
    /// `{location}-aiplatform.googleapis.com`, which accepts the Antigravity
    /// access token scoped to `project` + `location`. Mirrors the working
    /// Python keyless path (`google-genai` with `vertexai=True`).
    ///
    /// # Errors
    /// [`SdkError`] on transport failure, a non-2xx Vertex envelope, or a
    /// response body that does not deserialise into [`GenerateContentResponse`].
    pub async fn generate_content_vertex(
        &self,
        model: &str,
        project: &str,
        location: &str,
        req: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse, SdkError> {
        let url = crate::endpoints::vertex_generate_content_url(project, location, model);
        let body = serde_json::to_value(req)?;
        let value = self.post_json(&url, &body).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Run `generateContent` against the **Cloud Code modelbackend** — the exact
    /// path agy.exe uses (`cloudcode-pa.googleapis.com/v1internal:generateContent`).
    ///
    /// The agy OAuth token is scoped for this host, so this is the faithful
    /// reproduction of agy's LLM access (it carries the account's Code Assist
    /// tier, e.g. Google One AI Ultra). The request is wrapped in the
    /// `{ model, project, request }` envelope and the reply is unwrapped from
    /// its `response` field.
    ///
    /// # Errors
    /// [`SdkError`] on transport failure, a non-2xx Cloud Code envelope, or a
    /// response body that does not deserialise into the expected shape.
    pub async fn generate_content_cloud_code(
        &self,
        endpoint: crate::endpoints::CloudCodeEndpoint,
        model: &str,
        project: &str,
        req: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse, SdkError> {
        let body = serde_json::json!({
            "model": model,
            "project": project,
            "request": req,
        });
        let value = self
            .cloud_code(endpoint, crate::endpoints::METHOD_GENERATE_CONTENT, &body)
            .await?;
        let wrapped: CloudCodeGenerateContentResponse = serde_json::from_value(value)?;
        Ok(wrapped.response)
    }

    /// Resolve the Cloud AI Companion project bound to the signed-in account via
    /// `loadCodeAssist` (the same bootstrap agy.exe performs at startup).
    ///
    /// Posts an empty body (the server resolves the user's default project) and
    /// returns the `cloudaicompanionProject` field, or `None` when the account
    /// has no resolved project yet (needs onboarding).
    ///
    /// # Errors
    /// Any [`SdkError`] from [`load_code_assist`](Self::load_code_assist) or a
    /// response that does not parse as JSON object.
    pub async fn resolve_cloudcode_project(&self) -> Result<Option<String>, SdkError> {
        let value = self.load_code_assist(&serde_json::json!({})).await?;
        Ok(value
            .get("cloudaicompanionProject")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned))
    }
}

/// Extract the quota-reset delay (whole seconds) from a Cloud Code `429`
/// response body, used to back off before a retry.
///
/// Two sources are checked, in order:
/// 1. the structured `google.rpc.RetryInfo` detail (`retryDelay: "12s"` /
///    `"12.5s"`) under `error.details[]`;
/// 2. the human-readable message `error.message` ("…reset after 50s.").
///
/// Returns `None` when neither is present (the caller then uses a default
/// exponential back-off). A fractional `retryDelay` is rounded **up** so we
/// never wake before the window rolls over.
fn parse_retry_delay_secs(body: &str) -> Option<u64> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;

    // 1) Structured RetryInfo.retryDelay.
    if let Some(details) = v.pointer("/error/details").and_then(serde_json::Value::as_array) {
        for d in details {
            let is_retry_info = d
                .get("@type")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|t| t.ends_with("RetryInfo"));
            if is_retry_info
                && let Some(delay) = d.get("retryDelay").and_then(serde_json::Value::as_str)
                && let Some(secs) = parse_duration_secs_ceil(delay)
            {
                return Some(secs);
            }
        }
    }

    // 2) Human message: "...reset after <N>s.".
    let msg = v.pointer("/error/message").and_then(serde_json::Value::as_str)?;
    let after = msg.split("reset after ").nth(1)?;
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u64>().ok()
}

/// Parse a protobuf-style duration string (`"12s"`, `"12.5s"`) to whole seconds,
/// rounding any fraction **up**.
fn parse_duration_secs_ceil(s: &str) -> Option<u64> {
    let trimmed = s.trim().strip_suffix('s').unwrap_or(s.trim());
    match trimmed.split_once('.') {
        None => trimmed.parse::<u64>().ok(),
        Some((whole, frac)) => {
            let base = whole.parse::<u64>().ok()?;
            // Any non-zero fractional part rounds the second up.
            let bump = u64::from(frac.bytes().any(|b| b != b'0'));
            Some(base.saturating_add(bump))
        },
    }
}

#[cfg(test)]
mod retry_delay_tests {
    use super::parse_retry_delay_secs;

    #[test]
    fn parses_human_message_reset_after() {
        let body = r#"{"error":{"code":429,"message":"You have exhausted your capacity on this model. Your quota will reset after 50s.","status":"RESOURCE_EXHAUSTED"}}"#;
        assert_eq!(parse_retry_delay_secs(body), Some(50));
    }

    #[test]
    fn prefers_structured_retry_info() {
        let body = r#"{"error":{"code":429,"message":"reset after 9s.","status":"RESOURCE_EXHAUSTED","details":[{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"12.5s"}]}}"#;
        // Structured RetryInfo (12.5s -> 13) wins over the message (9s).
        assert_eq!(parse_retry_delay_secs(body), Some(13));
    }

    #[test]
    fn integer_retry_delay() {
        let body = r#"{"error":{"details":[{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"7s"}]}}"#;
        assert_eq!(parse_retry_delay_secs(body), Some(7));
    }

    #[test]
    fn none_when_absent() {
        assert_eq!(parse_retry_delay_secs(r#"{"error":{"code":403,"message":"nope"}}"#), None);
        assert_eq!(parse_retry_delay_secs("not json"), None);
    }
}
