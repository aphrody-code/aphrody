// SPDX-License-Identifier: Apache-2.0
//! `agy` (Antigravity) chat backend — drives the OAuth token written by the
//! **agy CLI** to talk to Gemini `generateContent`, with zero dependency on the
//! legacy `gemini` CLI binary.
//!
//! The token is read at **runtime** from the platform credential store (Windows
//! Credential Manager entry `gemini:antigravity`) that the agy CLI persists; no
//! secret is embedded in the binary. This is the same authentication path
//! `aphrody antigravity chat` uses (see [`crate::antigravity_cmd`]), surfaced
//! here as an [`aphrody_chat::backend::ModelBackend`] so the unified chat
//! turn-loop can default to agy instead of bootstrapping the `gemini` CLI.

use antigravity_sdk::{
    SdkError,
    client::AntigravityClient,
    models::{Content, GenerateContentRequest, Part},
};
use aphrody_chat::{
    Message, MessageRole,
    backend::{BackendResponse, ModelBackend},
    error::ChatError,
};
use async_trait::async_trait;

/// Default bare Gemini model id used when `--model` is not supplied. The Cloud
/// Code `:generateContent` envelope expects the bare id (no `models/` prefix).
///
/// This is exactly the model `agy.exe` itself sends when its UI shows
/// **"Gemini 3.5 Flash"**: the agy binary contains no literal `gemini-3.5-flash`
/// id (that one lives only on the public `generativelanguage` API the OAuth
/// token cannot reach — it 404s on Cloud Code); its "3.5 Flash" label maps to
/// the wire id `gemini-3-flash-preview`. We pair it with the **Daily** Cloud
/// Code endpoint (see [`AGY_ENDPOINT`]), which is the host agy uses and which
/// serves this model with a usable quota (Prod 429s it almost immediately).
const DEFAULT_AGY_MODEL: &str = "gemini-3-flash-preview";

/// Cloud Code endpoint the agy backend targets. `agy.exe` talks to the **Daily**
/// host (`daily-cloudcode-pa.googleapis.com`), confirmed from its own logs; it
/// serves the Gemini 3 preview models with a far more generous per-model quota
/// than Prod (which rate-limits `gemini-3-flash-preview` after a single call).
/// Transient 429s are still absorbed by the SDK's back-off retry.
const AGY_ENDPOINT: antigravity_sdk::endpoints::CloudCodeEndpoint =
    antigravity_sdk::endpoints::CloudCodeEndpoint::Daily;

/// Chat backend authenticating via the agy (Antigravity) token and dispatching
/// turns through the **Cloud Code modelbackend** (`daily-cloudcode-pa.googleapis.com/
/// v1internal:generateContent`, see [`AGY_ENDPOINT`]) — the exact host+path
/// `agy.exe` uses. The agy OAuth token is scoped for this host (the public
/// `generativelanguage` host rejects it with `403 ACCESS_TOKEN_SCOPE_INSUFFICIENT`),
/// and it carries the account's Code Assist tier (e.g. Google One AI Ultra).
pub(crate) struct AgyBackend {
    client: AntigravityClient,
    model: String,
    /// Cloud AI Companion project. Resolved lazily on first turn via
    /// `loadCodeAssist` (like agy's startup bootstrap) unless preset from the
    /// `APHRODY_CLOUDCODE_PROJECT` / `GOOGLE_CLOUD_PROJECT` env override.
    project: tokio::sync::OnceCell<String>,
}

impl AgyBackend {
    /// Build the backend, reading the agy OAuth token from the platform
    /// credential store. `model` falls back to [`DEFAULT_AGY_MODEL`] when it is
    /// `None`, empty, or a vendor-qualified slug whose bare id cannot be derived.
    ///
    /// # Errors
    ///
    /// [`ChatError::BackendFailure`] when the credential store has no usable
    /// `gemini:antigravity` token (sign in via the agy CLI first) or the host
    /// has no credential store (non-Windows).
    pub(crate) fn connect(model: Option<&str>) -> Result<Self, ChatError> {
        // rustls 0.23 requires a CryptoProvider before the first reqwest client;
        // idempotent — `main` may already have installed one (CLAUDE.md §7).
        let _ = rustls::crypto::ring::default_provider().install_default();

        let client = AntigravityClient::from_credential_manager()
            .map_err(|e| ChatError::BackendFailure(format!("agy credential store: {e}")))?;

        // Env override skips the loadCodeAssist round-trip (lower latency).
        let project = tokio::sync::OnceCell::new();
        if let Ok(p) = std::env::var("APHRODY_CLOUDCODE_PROJECT")
            .or_else(|_| std::env::var("GOOGLE_CLOUD_PROJECT"))
        {
            if !p.is_empty() {
                let _ = project.set(p);
            }
        }

        Ok(Self { client, model: normalise_model(model), project })
    }

    /// Resolve (and cache) the Cloud AI Companion project for this account,
    /// mirroring agy's `loadCodeAssist` bootstrap.
    async fn project(&self) -> Result<&str, ChatError> {
        self.project
            .get_or_try_init(|| async {
                self.client
                    .resolve_cloudcode_project()
                    .await
                    .map_err(|e| classify_agy_error("loadCodeAssist", e))?
                    .ok_or_else(|| {
                        ChatError::BackendFailure(
                            "no cloudaicompanionProject for this account (run `aphrody \
                             antigravity onboard` first)"
                                .to_owned(),
                        )
                    })
            })
            .await
            .map(String::as_str)
    }
}

/// Map a (possibly vendor-qualified) model slug to the bare Gemini id the
/// `generateContent` path expects. `gemini/default` and the empty string both
/// resolve to [`DEFAULT_AGY_MODEL`]; a `gemini/<id>` slug is reduced to `<id>`.
///
/// The public-API id `gemini-3.5-flash` (which Cloud Code 404s) is rewritten to
/// the wire id Cloud Code actually serves for that model, `gemini-3-flash-preview`
/// — the same mapping `agy.exe` applies behind its "Gemini 3.5 Flash" label — so
/// a caller (e.g. the desktop GUI) that requests `gemini-3.5-flash` on the agy
/// path still gets a working turn instead of a 404.
fn normalise_model(model: Option<&str>) -> String {
    let raw = model.map(str::trim).unwrap_or("");
    // Strip a leading `gemini/` vendor qualifier if present.
    let bare = raw.strip_prefix("gemini/").unwrap_or(raw);
    if bare.is_empty() || bare == "default" {
        DEFAULT_AGY_MODEL.to_owned()
    } else if matches!(bare, "gemini-3.5-flash" | "3.5-flash" | "gemini-flash") {
        // No literal gemini-3.5-flash on Cloud Code -> the served Gemini 3 Flash.
        "gemini-3-flash-preview".to_owned()
    } else {
        bare.to_owned()
    }
}

/// Classify an [`SdkError`] from an authenticated Cloud Code call into a
/// [`ChatError`].
///
/// A `401`/`403` from Google means the agy (Antigravity) OAuth token is expired
/// or lacks the required scope/tier. We surface a SHORT, actionable message —
/// and deliberately drop the raw Google JSON error body — so every caller
/// (including the desktop GUI, which renders the captured `stderr` verbatim) can
/// prompt a re-auth instead of dumping a nested error blob. Every other error
/// keeps the contextual `agy {stage}: {err}` form for debugging.
fn classify_agy_error(stage: &str, err: SdkError) -> ChatError {
    match err {
        SdkError::OAuthServer { status: status @ (401 | 403), .. } => {
            ChatError::BackendFailure(format!(
                "agy (Antigravity) token rejected by Google (HTTP {status}). The Google AI \
                 session has expired — re-authenticate with `aphrody antigravity login`, or use \
                 `aphrody chat --web` (keyless web transport) or `--stub` (offline)."
            ))
        },
        // 429 only reaches here after the SDK has already retried with back-off
        // (see `AntigravityClient::post_json_with_retry`). A clean, actionable
        // message — never the raw quota JSON, which the GUI renders verbatim.
        SdkError::OAuthServer { status: 429, .. } => ChatError::BackendFailure(
            "agy: model quota exhausted (HTTP 429) and still rate-limited after automatic \
             retries. This Code Assist tier enforces a tight per-model quota — wait ~1 min, \
             switch to a higher-quota model (e.g. `--model gemini-2.5-flash`), or use `aphrody \
             chat --web` (keyless web transport, no Cloud Code quota)."
                .to_owned(),
        ),
        other => ChatError::BackendFailure(format!("agy {stage}: {other}")),
    }
}

#[async_trait]
impl ModelBackend for AgyBackend {
    fn name(&self) -> &str {
        "agy/antigravity"
    }

    async fn complete(&self, messages: &[Message]) -> Result<BackendResponse, ChatError> {
        // Project chat messages into the generateContent shape:
        //   * System turns are merged into `system_instruction`.
        //   * Assistant turns map to role "model"; user / tool turns to "user".
        let mut contents: Vec<Content> = Vec::new();
        let mut system_text = String::new();
        for m in messages {
            match m.role {
                MessageRole::System => {
                    if !system_text.is_empty() {
                        system_text.push('\n');
                    }
                    system_text.push_str(&m.content);
                },
                MessageRole::Assistant => contents.push(Content {
                    role: Some("model".to_owned()),
                    parts: vec![Part::text(m.content.clone())],
                }),
                MessageRole::User | MessageRole::Tool => contents.push(Content {
                    role: Some("user".to_owned()),
                    parts: vec![Part::text(m.content.clone())],
                }),
            }
        }

        let system_instruction = (!system_text.is_empty())
            .then(|| Content { role: None, parts: vec![Part::text(system_text)] });

        let req = GenerateContentRequest { contents, generation_config: None, system_instruction };

        let project = self.project().await?;
        let resp = self
            .client
            .generate_content_cloud_code(AGY_ENDPOINT, &self.model, project, &req)
            .await
            .map_err(|e| classify_agy_error("generateContent", e))?;

        let resolved_model = resp.model_version.clone();
        let content = resp.first_text().ok_or_else(|| {
            ChatError::BackendFailure("agy generateContent returned no text candidate".to_owned())
        })?;

        Ok(BackendResponse {
            content,
            tool_calls: Vec::new(),
            // generateContent surfaces usage in `usage_metadata` as raw JSON;
            // the chat loop does not consume per-turn token counts yet.
            prompt_tokens: 0,
            completion_tokens: 0,
            resolved_model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_model_defaults() {
        assert_eq!(normalise_model(None), DEFAULT_AGY_MODEL);
        assert_eq!(normalise_model(Some("")), DEFAULT_AGY_MODEL);
        assert_eq!(normalise_model(Some("  ")), DEFAULT_AGY_MODEL);
        assert_eq!(normalise_model(Some("default")), DEFAULT_AGY_MODEL);
        assert_eq!(normalise_model(Some("gemini/default")), DEFAULT_AGY_MODEL);
    }

    #[test]
    fn normalise_model_strips_vendor_prefix() {
        assert_eq!(normalise_model(Some("gemini/gemini-2.5-flash")), "gemini-2.5-flash");
        assert_eq!(normalise_model(Some("gemini-2.0-flash")), "gemini-2.0-flash");
    }

    #[test]
    fn normalise_model_rewrites_public_3_5_flash_to_served_wire_id() {
        // The public-API id 404s on Cloud Code; it must map to the id agy uses.
        assert_eq!(normalise_model(Some("gemini-3.5-flash")), "gemini-3-flash-preview");
        assert_eq!(normalise_model(Some("gemini/gemini-3.5-flash")), "gemini-3-flash-preview");
        assert_eq!(normalise_model(Some("3.5-flash")), "gemini-3-flash-preview");
        // And the default is already the served wire id.
        assert_eq!(DEFAULT_AGY_MODEL, "gemini-3-flash-preview");
    }

    #[test]
    fn classify_agy_error_maps_401_to_actionable_message() {
        // A 401 carrying a raw Google JSON body must become a short, actionable
        // re-auth message that NEVER leaks the body into the UI.
        let err = SdkError::OAuthServer {
            status: 401,
            body: "{\"error\":{\"message\":\"invalid authentication credentials\"}}".to_owned(),
        };
        let mapped = classify_agy_error("generateContent", err).to_string();
        assert!(mapped.contains("401"), "status surfaced: {mapped}");
        assert!(
            mapped.contains("aphrody antigravity login"),
            "actionable re-auth hint present: {mapped}"
        );
        assert!(
            !mapped.contains("invalid authentication credentials"),
            "raw Google JSON body must not leak: {mapped}"
        );
    }

    #[test]
    fn classify_agy_error_maps_403_scope_failure() {
        let err = SdkError::OAuthServer {
            status: 403,
            body: "ACCESS_TOKEN_SCOPE_INSUFFICIENT".to_owned(),
        };
        let mapped = classify_agy_error("generateContent", err).to_string();
        assert!(mapped.contains("403"), "status surfaced: {mapped}");
        assert!(
            !mapped.contains("ACCESS_TOKEN_SCOPE_INSUFFICIENT"),
            "raw body must not leak: {mapped}"
        );
    }

    #[test]
    fn classify_agy_error_maps_429_to_actionable_message_without_leaking_body() {
        // After the SDK's retries are exhausted, a 429 must become a short,
        // actionable message — never the raw RESOURCE_EXHAUSTED JSON (the GUI
        // renders stderr verbatim).
        let err = SdkError::OAuthServer {
            status: 429,
            body: "{\"error\":{\"message\":\"You have exhausted your capacity on this \
                   model.\",\"status\":\"RESOURCE_EXHAUSTED\"}}"
                .to_owned(),
        };
        let mapped = classify_agy_error("generateContent", err).to_string();
        assert!(mapped.contains("429"), "status surfaced: {mapped}");
        assert!(mapped.contains("--web"), "actionable alternative present: {mapped}");
        assert!(!mapped.contains("RESOURCE_EXHAUSTED"), "raw quota JSON must not leak: {mapped}");
    }

    #[test]
    fn classify_agy_error_preserves_non_auth_errors() {
        // A non-auth SDK error keeps the contextual stage + full message so
        // debugging information is not lost.
        let mapped = classify_agy_error("loadCodeAssist", SdkError::EmptyCredential).to_string();
        assert!(mapped.contains("loadCodeAssist"), "stage preserved: {mapped}");
    }

    #[test]
    fn classify_agy_error_preserves_non_401_oauth_status() {
        // A 5xx from the OAuth server is a transient server fault, not an
        // expired token — it must keep the raw detail (stage + status).
        let err = SdkError::OAuthServer { status: 503, body: "upstream unavailable".to_owned() };
        let mapped = classify_agy_error("generateContent", err).to_string();
        assert!(mapped.contains("generateContent"), "stage preserved: {mapped}");
        assert!(mapped.contains("503"), "status preserved: {mapped}");
    }
}
