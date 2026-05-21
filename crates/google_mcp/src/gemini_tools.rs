// SPDX-License-Identifier: Apache-2.0
//! Gemini web app MCP tools — `gemini_chat` (3.5 Flash) and `gemini_image`
//! (Nano Banana image generation), backed by the `gemini-web` SDK.
//!
//! Auth = the signed-in Google cookie jar at `~/.aphrody/google-cookies.json`
//! (no API key). A single [`GeminiWebClient`] is cached process-wide and reused
//! across calls (latency: the page bootstrap + token scrape happens once); on a
//! session-expiry auth error the client re-bootstraps and retries once. Tool
//! results are JSON strings; no secret values are ever emitted.

use gemini_web::{GeminiModel, GeminiWebClient};
// Use the `schemars` re-exported by rmcp so the generated JsonSchema impl
// matches the version rmcp's tool macros expect (avoids the multi-version clash).
use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{Mutex, OnceCell};

/// Process-wide cached client (built once, reused — see latency objective).
static CLIENT: OnceCell<Mutex<GeminiWebClient>> = OnceCell::const_new();

async fn client() -> Result<&'static Mutex<GeminiWebClient>, String> {
    CLIENT
        .get_or_try_init(|| async {
            GeminiWebClient::from_default_cookie_file("en")
                .await
                .map(Mutex::new)
                .map_err(|e| format!("gemini-web connect failed: {e} \
                    (export Google cookies to ~/.aphrody/google-cookies.json)"))
        })
        .await
}

/// Request for [`chat`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GeminiChatRequest {
    /// The prompt to send to Gemini.
    pub prompt: String,
    /// Optional model id: `flash` (3.5, default), `flash-lite`, or `pro`.
    #[serde(default)]
    pub model: Option<String>,
}

/// Request for [`image`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GeminiImageRequest {
    /// The image-generation prompt (routed to the Nano Banana image model).
    pub prompt: String,
}

/// Run one chat turn against the Gemini web app. Returns JSON
/// `{ "text", "model", "conversation_id" }`.
pub(crate) async fn chat(req: GeminiChatRequest) -> String {
    let model = req
        .model
        .as_deref()
        .and_then(GeminiModel::from_id)
        .unwrap_or(GeminiModel::Flash);
    match send_once(&req.prompt, model).await {
        Ok(reply) => json!({
            "text": reply.text,
            "model": model.label(),
            "conversation_id": reply.metadata.conversation_id,
        })
        .to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Generate an image (Nano Banana). Returns JSON
/// `{ "text", "generated_image_urls", "count" }`.
pub(crate) async fn image(req: GeminiImageRequest) -> String {
    // The image model is reached by prompting; Gemini routes image requests and
    // returns the rendered image URLs in the reply.
    let prompt = format!("Generate an image: {}", req.prompt);
    match send_once(&prompt, GeminiModel::Flash).await {
        Ok(reply) => json!({
            "text": reply.text,
            "generated_image_urls": reply.generated_image_urls,
            "count": reply.generated_image_urls.len(),
        })
        .to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Send one prompt, refreshing the session once on an auth error.
async fn send_once(prompt: &str, model: GeminiModel) -> Result<gemini_web::ChatReply, String> {
    let cell = client().await?;
    let header = model.header();
    let meta = gemini_web::ConversationMetadata::default();
    {
        let guard = cell.lock().await;
        match guard.send(prompt, Some(header.as_str()), &meta).await {
            Ok(reply) => return Ok(reply),
            Err(gemini_web::GeminiError::Auth(_)) => {}, // fall through to refresh
            Err(e) => return Err(format!("gemini-web send: {e}")),
        }
    }
    // Refresh tokens and retry once.
    {
        let mut guard = cell.lock().await;
        guard.refresh().await.map_err(|e| format!("gemini-web refresh: {e}"))?;
        guard
            .send(prompt, Some(header.as_str()), &meta)
            .await
            .map_err(|e| format!("gemini-web send (post-refresh): {e}"))
    }
}
