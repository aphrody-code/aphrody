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
    ContentClass, ExpandRequest, FillRequest, FireflyClient, GenerateImageRequest, ImageSourceRef,
    LrEdit, OutputType, PhotoshopClient, PsInput, PsOutput, Size, Storage,
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

// ---------------------------------------------------------------------------
// Lightroom / Sensei single-input edit tools — mirror the verbs of the
// official "Adobe for creativity" connector (image_apply_auto_tone,
// image_auto_straighten, image_adjust_*, image_select_subject/mask, remove
// background), backed by our own Rust client on the same IMS token.
// ---------------------------------------------------------------------------

/// A single-input → single-output edit request shared by the Lightroom/Sensei
/// tools (auto-tone, auto-straighten, remove-background, create-mask, etc.).
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PsEditRequest {
    /// URL of the input image the Adobe API can read.
    pub input_url: String,
    /// Writable destination URL for the edited output.
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

impl PsEditRequest {
    fn input(&self) -> PsInput {
        PsInput { href: self.input_url.clone(), storage: parse_storage(self.input_storage.as_deref()) }
    }
    fn output(&self) -> PsOutput {
        PsOutput {
            href: self.output_url.clone(),
            storage: parse_storage(self.output_storage.as_deref()),
            kind: parse_output_type(self.format.as_deref()),
            overwrite: Some(true),
        }
    }
}

/// Request for [`edit`] — explicit Camera-Raw adjustments (Lightroom `edit`).
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PsAdjustRequest {
    /// URL of the input image the Adobe API can read.
    pub input_url: String,
    /// Writable destination URL for the edited output.
    pub output_url: String,
    /// Output format by extension (default jpg).
    #[serde(default)]
    pub format: Option<String>,
    /// Storage backing `input_url` (default external).
    #[serde(default)]
    pub input_storage: Option<String>,
    /// Storage backing `output_url` (default external).
    #[serde(default)]
    pub output_storage: Option<String>,
    /// Exposure in stops (-5.0 … +5.0).
    #[serde(default)]
    pub exposure: Option<f64>,
    /// Contrast (-100 … +100).
    #[serde(default)]
    pub contrast: Option<i32>,
    /// Highlights (-100 … +100).
    #[serde(default)]
    pub highlights: Option<i32>,
    /// Shadows / dark portions (-100 … +100).
    #[serde(default)]
    pub shadows: Option<i32>,
    /// Whites / bright portions (-100 … +100).
    #[serde(default)]
    pub whites: Option<i32>,
    /// Blacks (-100 … +100).
    #[serde(default)]
    pub blacks: Option<i32>,
    /// White-balance temperature shift (-100 … +100).
    #[serde(default)]
    pub temperature: Option<i32>,
    /// White-balance tint (-100 … +100).
    #[serde(default)]
    pub tint: Option<i32>,
    /// Vibrance (-100 … +100).
    #[serde(default)]
    pub vibrance: Option<i32>,
    /// Saturation (-100 … +100).
    #[serde(default)]
    pub saturation: Option<i32>,
    /// Clarity (-100 … +100).
    #[serde(default)]
    pub clarity: Option<i32>,
    /// Dehaze (-100 … +100).
    #[serde(default)]
    pub dehaze: Option<i32>,
    /// Texture (-100 … +100).
    #[serde(default)]
    pub texture: Option<i32>,
    /// Sharpness (0 … 150).
    #[serde(default)]
    pub sharpness: Option<i32>,
}

impl PsAdjustRequest {
    fn input(&self) -> PsInput {
        PsInput { href: self.input_url.clone(), storage: parse_storage(self.input_storage.as_deref()) }
    }
    fn output(&self) -> PsOutput {
        PsOutput {
            href: self.output_url.clone(),
            storage: parse_storage(self.output_storage.as_deref()),
            kind: self.format.as_deref().and_then(OutputType::from_ext).unwrap_or(OutputType::Jpeg),
            overwrite: Some(true),
        }
    }
    fn edit(&self) -> LrEdit {
        let mut e = LrEdit::new();
        e.exposure = self.exposure;
        e.contrast = self.contrast;
        e.highlights = self.highlights;
        e.shadows = self.shadows;
        e.whites = self.whites;
        e.blacks = self.blacks;
        e.temperature = self.temperature;
        e.tint = self.tint;
        e.vibrance = self.vibrance;
        e.saturation = self.saturation;
        e.clarity = self.clarity;
        e.dehaze = self.dehaze;
        e.texture = self.texture;
        e.sharpness = self.sharpness;
        e
    }
}

/// Request for [`action_json`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PsActionJsonRequest {
    /// URL of the input PSD/image.
    pub input_url: String,
    /// Writable destination URL for the result.
    pub output_url: String,
    /// Output format by extension (default png).
    #[serde(default)]
    pub format: Option<String>,
    /// The `actionJSON` array (a recorded Photoshop action set as JSON).
    pub actions: Value,
}

/// Lightroom auto-tone: AI exposure/contrast/highlights/shadows/vibrance.
pub(crate) async fn auto_tone(req: PsEditRequest) -> String {
    match ps_client().await {
        Ok(c) => match c.lr_auto_tone(req.input(), req.output()).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("autoTone: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Lightroom auto-straighten (Upright perspective correction).
pub(crate) async fn auto_straighten(req: PsEditRequest) -> String {
    match ps_client().await {
        Ok(c) => match c.lr_auto_straighten(req.input(), req.output()).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("autoStraighten: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Lightroom edit: apply explicit Camera-Raw adjustments.
pub(crate) async fn edit(req: PsAdjustRequest) -> String {
    let edit = req.edit();
    match ps_client().await {
        Ok(c) => match c.lr_edit(req.input(), req.output(), &edit).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("edit: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Sensei cutout: remove the background, returning a transparent PNG.
pub(crate) async fn remove_background(req: PsEditRequest) -> String {
    match ps_client().await {
        Ok(c) => match c.remove_background(req.input(), req.output()).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("removeBackground: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Sensei mask: produce a subject/background alpha mask.
pub(crate) async fn create_mask(req: PsEditRequest) -> String {
    match ps_client().await {
        Ok(c) => match c.create_mask(req.input(), req.output()).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("createMask: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// `psdService` product crop (content-aware).
pub(crate) async fn product_crop(req: PsEditRequest) -> String {
    match ps_client().await {
        Ok(c) => match c.product_crop(req.input(), req.output(), None).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("productCrop: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// `psdService` depth blur (Neural-Filter depth-of-field).
pub(crate) async fn depth_blur(req: PsEditRequest) -> String {
    match ps_client().await {
        Ok(c) => match c.depth_blur(req.input(), req.output(), None).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("depthBlur: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Play an `actionJSON` program over a PSD/image (documentOperations).
pub(crate) async fn action_json(req: PsActionJsonRequest) -> String {
    let inputs = vec![PsInput::external(req.input_url)];
    let outputs = vec![PsOutput::external(req.output_url, parse_output_type(req.format.as_deref()))];
    match ps_client().await {
        Ok(c) => match c.action_json(inputs, outputs, req.actions).await {
            Ok(job) => job_json(&job),
            Err(e) => json!({ "error": format!("actionJSON: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

// ---------------------------------------------------------------------------
// Firefly generative edit tools — expand (enlarge canvas, AI-fill) and fill
// (replace a masked region). Both take Adobe-readable URLs and download the
// result. Mirror the connector's image_generative_expand.
// ---------------------------------------------------------------------------

/// Request for [`generative_expand`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FireflyExpandRequest {
    /// URL of the source image (Adobe-readable / presigned).
    pub image_url: String,
    /// Target width in pixels.
    pub width: u32,
    /// Target height in pixels.
    pub height: u32,
    /// Optional prompt guiding the generated fill content.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Number of variations (1–4, default 1).
    #[serde(default)]
    pub variations: Option<u8>,
    /// Optional directory to save the downloaded outputs.
    #[serde(default)]
    pub save_dir: Option<String>,
}

/// Request for [`generative_fill`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FireflyFillRequest {
    /// URL of the base image (Adobe-readable / presigned).
    pub image_url: String,
    /// URL of the mask image (white = region to fill).
    pub mask_url: String,
    /// Prompt describing the desired fill content.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Number of variations (1–4, default 1).
    #[serde(default)]
    pub variations: Option<u8>,
    /// Optional directory to save the downloaded outputs.
    #[serde(default)]
    pub save_dir: Option<String>,
}

/// Generative expand: enlarge the canvas of an image, AI-filling the new area.
pub(crate) async fn generative_expand(req: FireflyExpandRequest) -> String {
    let mut fx = ExpandRequest::new(
        ImageSourceRef::url(req.image_url),
        Size { width: req.width, height: req.height },
    );
    if let Some(n) = req.variations {
        fx = fx.with_variations(n);
    }
    if let Some(p) = req.prompt {
        fx = fx.with_prompt(p);
    }
    match ff_client().await {
        Ok(c) => match c.expand(&fx).await {
            Ok(result) => download_and_report(c, &result, req.save_dir, "expand").await,
            Err(e) => json!({ "error": format!("generativeExpand: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Generative fill: replace the masked region of an image with prompt content.
pub(crate) async fn generative_fill(req: FireflyFillRequest) -> String {
    let mut fx = FillRequest::new(
        ImageSourceRef::url(req.image_url),
        ImageSourceRef::url(req.mask_url),
    );
    if let Some(p) = req.prompt {
        fx = fx.with_prompt(p);
    }
    if let Some(n) = req.variations {
        fx = fx.with_variations(n);
    }
    match ff_client().await {
        Ok(c) => match c.fill(&fx).await {
            Ok(result) => download_and_report(c, &result, req.save_dir, "fill").await,
            Err(e) => json!({ "error": format!("generativeFill: {e}") }).to_string(),
        },
        Err(e) => json!({ "error": e }).to_string(),
    }
}

/// Download a Firefly result (optionally to disk) and report URLs/seeds/paths.
async fn download_and_report(
    client: &FireflyClient,
    result: &aphrody_firefly::GenerateResult,
    save_dir: Option<String>,
    prefix: &str,
) -> String {
    let urls: Vec<String> = result.outputs.iter().map(|o| o.image.url.clone()).collect();
    let mut saved: Vec<String> = Vec::new();
    if let Some(dir) = save_dir.as_deref() {
        match client.download_outputs(result).await {
            Ok(images) => {
                for img in &images {
                    match img.save_to_dir(std::path::Path::new(dir), prefix).await {
                        Ok(p) => saved.push(p.display().to_string()),
                        Err(e) => return json!({ "error": format!("save: {e}") }).to_string(),
                    }
                }
            }
            Err(e) => return json!({ "error": format!("download: {e}") }).to_string(),
        }
    }
    json!({
        "count": urls.len(),
        "image_urls": urls,
        "saved_paths": saved,
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// Live Photoshop (in-app UXP plugin) — drive the *running* desktop Photoshop
// from the inside through the WebSocket bridge. batchPlay + eval expose the
// entire Photoshop surface (every menu/filter/action), unlike the headless
// cloud API. Requires the aphrody UXP panel (apps/photoshop-uxp) to be loaded.
// ---------------------------------------------------------------------------

/// Generous budget — a heavy `batchPlay` (e.g. a Neural Filter) can take a
/// while inside Photoshop.
const LIVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

fn bridge_result(r: Result<Value, String>) -> String {
    match r {
        Ok(v) => json!({ "ok": true, "result": v }).to_string(),
        Err(e) => json!({ "ok": false, "error": e }).to_string(),
    }
}

/// Report the live Photoshop state (app version, active document, layer tree).
pub(crate) async fn live_info() -> String {
    bridge_result(crate::photoshop_bridge::call("info", json!({}), LIVE_TIMEOUT).await)
}

/// Request for [`live_batchplay`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PsBatchPlayRequest {
    /// The `batchPlay` command array — an array of ActionDescriptor objects
    /// (the same JSON ScriptListener / Alchemist emit). Drives any Photoshop op.
    pub commands: Value,
    /// Optional `batchPlay` options object (e.g. `{ "synchronousExecution": false }`).
    #[serde(default)]
    pub options: Option<Value>,
}

/// Run a `batchPlay` command array inside the live Photoshop — the universal
/// driver for any Photoshop operation.
pub(crate) async fn live_batchplay(req: PsBatchPlayRequest) -> String {
    let args = json!({ "commands": req.commands, "options": req.options.unwrap_or_else(|| json!({})) });
    bridge_result(crate::photoshop_bridge::call("batchPlay", args, LIVE_TIMEOUT).await)
}

/// Request for [`live_exec`].
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PsExecRequest {
    /// JavaScript evaluated inside the UXP plugin. The async body has `app`,
    /// `photoshop`, `constants`, `core` and `batchPlay` in scope and should
    /// `return` a JSON-serializable value.
    pub code: String,
}

/// Evaluate arbitrary UXP JavaScript inside the live Photoshop (full internal
/// access — DOM, modules, batchPlay). The escape hatch for anything not
/// expressible as a single `batchPlay` array.
pub(crate) async fn live_exec(req: PsExecRequest) -> String {
    bridge_result(crate::photoshop_bridge::call("eval", json!({ "code": req.code }), LIVE_TIMEOUT).await)
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
