// SPDX-License-Identifier: Apache-2.0
//! Cloud Photoshop API MCP tools — `photoshop_manifest`, `photoshop_rendition`,
//! `photoshop_document_operations`, and the `firefly_to_photoshop` bridge,
//! backed by `aphrody_firefly::photoshop` (headless `image.adobe.io` REST API).
//!
//! This is the in-policy MCP endpoint that replaces the TypeScript photoshop-mcp:
//! no local Photoshop, no JS — the headless cloud API on the shared IMS token.
//!
//! Auth = `FIREFLY_CLIENT_ID` / `FIREFLY_CLIENT_SECRET` from the environment.
//! Inputs/outputs are URLs the Adobe API can read/write (presigned S3 / Azure
//! SAS / Dropbox / Creative Cloud). Firefly outputs are presigned URLs, so a
//! generated image feeds straight into a Photoshop op as an `external` input.

use aphrody_firefly::{
    ContentClass, FireflyClient, GenerateImageRequest, OutputType, PhotoshopClient, PsInput,
    PsOutput, Storage,
};
use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::OnceCell;

static PS_CLIENT: OnceCell<PhotoshopClient> = OnceCell::const_new();
static FF_CLIENT: OnceCell<FireflyClient> = OnceCell::const_new();

async fn ps_client() -> Result<&'static PhotoshopClient, String> {
    PS_CLIENT
        .get_or_try_init(|| async {
            PhotoshopClient::from_env().map_err(|e| {
                format!("Photoshop API auth setup failed: {e} \
                    (set FIREFLY_CLIENT_ID and FIREFLY_CLIENT_SECRET)")
            })
        })
        .await
}

async fn ff_client() -> Result<&'static FireflyClient, String> {
    FF_CLIENT
        .get_or_try_init(|| async {
            FireflyClient::from_env()
                .map_err(|e| format!("Firefly auth setup failed: {e}"))
        })
        .await
}

fn parse_storage(s: Option<&str>) -> Storage {
    s.and_then(Storage::parse).unwrap_or(Storage::External)
}

fn parse_output_type(s: Option<&str>) -> OutputType {
    s.and_then(OutputType::from_ext).unwrap_or(OutputType::Png)
}

/// Request for [`manifest`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PsManifestRequest {
    /// URL of the input image/PSD the Adobe API can read.
    pub input_url: String,
    /// Storage backing `input_url`: external (default), adobe, azure, dropbox, aio.
    #[serde(default)]
    pub storage: Option<String>,
}

/// Request for [`rendition`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PsRenditionRequest {
    /// URL of the input image/PSD.
    pub input_url: String,
    /// Writable destination URL for the rendered output.
    pub output_url: String,
    /// Output format by extension: png (default), jpg, psd, tiff, dng.
    #[serde(default)]
    pub format: Option<String>,
    /// Storage backing `input_url` (default external).
    #[serde(default)]
    pub input_storage: Option<String>,
    /// Storage backing `output_url` (default external).
    #[serde(default)]
    pub output_storage: Option<String>,
}

/// Request for [`document_operations`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PsDocOpsRequest {
    /// URL of the input PSD.
    pub input_url: String,
    /// Optional writable output URL (when the edit also renders a result).
    #[serde(default)]
    pub output_url: Option<String>,
    /// Output format by extension when `output_url` is set (default png).
    #[serde(default)]
    pub format: Option<String>,
    /// The `options` layer-edit tree as JSON (passed through to the API).
    pub options: Value,
}

/// Request for [`firefly_to_photoshop`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FireflyToPhotoshopRequest {
    /// Image-generation prompt (rendered by Firefly).
    pub prompt: String,
    /// Optional writable destination — when set, the generated image is
    /// converted to this URL/format via renditionCreate (e.g. an editable PSD).
    /// When omitted, the tool returns the Photoshop layer manifest of the
    /// generated image instead (no writable storage required).
    #[serde(default)]
    pub output_url: Option<String>,
    /// Output format by extension when `output_url` is set (default psd).
    #[serde(default)]
    pub format: Option<String>,
}

/// Get the PSD/image layer manifest. Returns the raw Photoshop job JSON.
pub(crate) async fn manifest(req: PsManifestRequest) -> String {
    let storage = parse_storage(req.storage.as_deref());
    let input = PsInput { href: req.input_url, storage };
    match ps_client().await {
        Ok(c) => match c.document_manifest(vec![input]).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("documentManifest: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Render a PSD/image to an output URL.
pub(crate) async fn rendition(req: PsRenditionRequest) -> String {
    let input = PsInput {
        href: req.input_url,
        storage: parse_storage(req.input_storage.as_deref()),
    };
    let output = PsOutput {
        href: req.output_url,
        storage: parse_storage(req.output_storage.as_deref()),
        kind: parse_output_type(req.format.as_deref()),
        overwrite: Some(true),
    };
    match ps_client().await {
        Ok(c) => match c.create_rendition(vec![input], vec![output]).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("renditionCreate: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Apply layer edits (and optionally render) via documentOperations.
pub(crate) async fn document_operations(req: PsDocOpsRequest) -> String {
    let inputs = vec![PsInput::external(req.input_url)];
    let outputs = match req.output_url {
        Some(url) => vec![PsOutput::external(url, parse_output_type(req.format.as_deref()))],
        None => vec![],
    };
    let dop = aphrody_firefly::DocumentOperationsRequest {
        inputs,
        outputs,
        options: Some(req.options),
    };
    match ps_client().await {
        Ok(c) => match c.document_operations(&dop).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("documentOperations: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Bridge: generate an image with Firefly, then route it through the Photoshop
/// API. With `output_url`, converts the generated image to that URL/format
/// (e.g. an editable PSD); otherwise returns the layer manifest of the result.
pub(crate) async fn firefly_to_photoshop(req: FireflyToPhotoshopRequest) -> String {
    match bridge_inner(req).await {
        Ok(v) => v.to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    }
}

async fn bridge_inner(req: FireflyToPhotoshopRequest) -> Result<Value, String> {
    // 1. Generate with Firefly (returns presigned, Adobe-readable URLs).
    let ff = ff_client().await?;
    let gen_req = GenerateImageRequest::new(req.prompt)
        .with_variations(1)
        .with_content_class(ContentClass::Auto);
    let result = ff
        .generate(&gen_req)
        .await
        .map_err(|e| format!("Firefly generation: {e}"))?;
    let image_url = result
        .outputs
        .first()
        .map(|o| o.image.url.clone())
        .ok_or("Firefly returned no image")?;

    // 2. Feed the presigned URL into the Photoshop API as an external input.
    let ps = ps_client().await?;
    let input = PsInput::external(image_url.clone());
    let job = if let Some(out) = req.output_url {
        let kind = req.format.as_deref().and_then(OutputType::from_ext).unwrap_or(OutputType::Psd);
        ps.create_rendition(vec![input], vec![PsOutput::external(out, kind)])
            .await
            .map_err(|e| format!("Photoshop renditionCreate: {e}"))?
    } else {
        ps.document_manifest(vec![input])
            .await
            .map_err(|e| format!("Photoshop documentManifest: {e}"))?
    };

    Ok(json!({
        "firefly_image_url": image_url,
        "photoshop_job": serde_json::to_value(&job).unwrap_or(Value::Null),
    }))
}

/// Serialize a Photoshop job into a tool-result JSON string.
fn job_json(job: &aphrody_firefly::PhotoshopJob) -> String {
    serde_json::to_string(&json!({
        "job_id": job.job_id,
        "all_succeeded": job.all_succeeded(),
        "outputs": serde_json::to_value(&job.outputs).unwrap_or(Value::Null),
    }))
    .unwrap_or_else(|e| json!({ "error": format!("encode: {e}") }).to_string())
}
