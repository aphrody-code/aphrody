// SPDX-License-Identifier: Apache-2.0
//! Adobe Firefly Services MCP tool — `firefly_generate`, backed by the
//! `aphrody-firefly` crate (IMS OAuth server-to-server + v3 async generate).
//!
//! Auth = `FIREFLY_CLIENT_ID` / `FIREFLY_CLIENT_SECRET` from the environment
//! (a Developer Console project). A single [`FireflyClient`] is cached
//! process-wide and reused across calls (the IMS token is fetched once and
//! refreshed near expiry — latency objective). Tool results are JSON strings;
//! the client secret is never emitted.

use aphrody_firefly::{ContentClass, FireflyClient, GenerateImageRequest, Size};
// schemars re-exported by rmcp so the JsonSchema impl matches the macro version.
use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::OnceCell;

/// Process-wide cached Firefly client (built once, reused).
static CLIENT: OnceCell<FireflyClient> = OnceCell::const_new();

async fn client() -> Result<&'static FireflyClient, String> {
    CLIENT
        .get_or_try_init(|| async {
            FireflyClient::from_env().map_err(|e| {
                format!("Firefly auth setup failed: {e} \
                    (set FIREFLY_CLIENT_ID and FIREFLY_CLIENT_SECRET)")
            })
        })
        .await
}

/// Request for [`generate`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FireflyGenerateRequest {
    /// The image-generation prompt.
    pub prompt: String,
    /// Number of variations to produce (1–4; default 1).
    #[serde(default)]
    pub variations: Option<u8>,
    /// Output size as `WxH` (e.g. `2048x2048`). Default = Firefly's 2048×2048.
    #[serde(default)]
    pub size: Option<String>,
    /// Content class: `auto` (default), `photo`, or `art`.
    #[serde(default)]
    pub content_class: Option<String>,
    /// Concepts to avoid (negative prompt).
    #[serde(default)]
    pub negative_prompt: Option<String>,
    /// Directory to save the generated images to (created if absent). When set,
    /// the saved paths are returned; otherwise only metadata (seeds) is returned.
    #[serde(default)]
    pub save_dir: Option<String>,
}

/// Generate one or more images with Adobe Firefly. Returns JSON
/// `{ "count", "outputs": [{ "seed", "content_type", "bytes", "saved_path"? }] }`.
pub(crate) async fn generate(req: FireflyGenerateRequest) -> String {
    match generate_inner(req).await {
        Ok(value) => value.to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    }
}

async fn generate_inner(req: FireflyGenerateRequest) -> Result<serde_json::Value, String> {
    let class = req
        .content_class
        .as_deref()
        .map_or(Some(ContentClass::Auto), ContentClass::parse)
        .ok_or("invalid content_class (use auto|photo|art)")?;

    let mut body = GenerateImageRequest::new(req.prompt)
        .with_variations(req.variations.unwrap_or(1))
        .with_content_class(class);
    if let Some(s) = req.size.as_deref() {
        let parsed = Size::parse(s).ok_or("invalid size (use WxH, e.g. 2048x2048)")?;
        body = body.with_size(parsed);
    }
    if let Some(neg) = req.negative_prompt {
        body = body.with_negative_prompt(neg);
    }

    let client = client().await?;
    let images = client
        .generate_and_download(&body)
        .await
        .map_err(|e| format!("Firefly generation: {e}"))?;

    let save_dir = req.save_dir.map(std::path::PathBuf::from);
    let mut outputs = Vec::with_capacity(images.len());
    for img in &images {
        let saved_path = if let Some(dir) = save_dir.as_ref() {
            Some(
                img.save_to_dir(dir, "firefly")
                    .await
                    .map_err(|e| format!("save image: {e}"))?
                    .display()
                    .to_string(),
            )
        } else {
            None
        };
        outputs.push(json!({
            "seed": img.seed,
            "content_type": img.content_type,
            "bytes": img.bytes.len(),
            "saved_path": saved_path,
        }));
    }

    Ok(json!({ "count": outputs.len(), "outputs": outputs }))
}
