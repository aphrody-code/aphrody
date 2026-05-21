// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Adobe **cloud Photoshop API** client (Firefly Services family).
//!
//! Headless PSD manipulation over REST — the in-policy answer to the
//! TypeScript `photoshop-mcp` (which drives a *locally installed* Photoshop via
//! ExtendScript/COM). This talks to `image.adobe.io/pie/psdService` with the
//! **same IMS token** as the Firefly image API (shared [`TokenCache`]): no
//! Photoshop install, cross-platform, pure Rust.
//!
//! Verified protocol (Adobe Photoshop API SDK, 2026-05):
//! - Base `https://image.adobe.io/pie/psdService/`; ops `documentManifest`,
//!   `documentOperations`, `smartObject`, `renditionCreate`.
//! - Auth headers `Authorization: Bearer <token>` + `x-api-key: <client_id>`.
//! - Inputs/outputs are `{ href, storage }` (+ `type`, `overwrite` for outputs).
//!   `storage ∈ { aio, adobe, external, azure, dropbox }`.
//! - POST returns `{ _links: { self: { href } } }`; poll that href until every
//!   `outputs[].status` is terminal (`succeeded`/`failed`); intermediate states
//!   are `pending`/`running`/`uploading`.

use crate::auth::{ImsCredentials, TokenCache};
use crate::error::{FireflyError, Result};
use crate::client::PollConfig;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Base for every Photoshop service operation.
pub const PSD_SERVICE_BASE: &str = "https://image.adobe.io/pie/psdService";
/// `documentManifest` — return the PSD layer tree.
pub const OP_DOCUMENT_MANIFEST: &str = "https://image.adobe.io/pie/psdService/documentManifest";
/// `documentOperations` — edit layers / create renditions.
pub const OP_DOCUMENT_OPERATIONS: &str =
    "https://image.adobe.io/pie/psdService/documentOperations";
/// `smartObject` — replace a smart-object layer's contents.
pub const OP_SMART_OBJECT: &str = "https://image.adobe.io/pie/psdService/smartObject";
/// `renditionCreate` — render a PSD to one or more output formats.
pub const OP_RENDITION_CREATE: &str = "https://image.adobe.io/pie/psdService/renditionCreate";
/// `productCrop` — content-aware product crop (psdService; preview surface).
pub const OP_PRODUCT_CROP: &str = "https://image.adobe.io/pie/psdService/productCrop";
/// `depthBlur` — Neural-Filter depth-of-field blur (psdService; preview surface).
pub const OP_DEPTH_BLUR: &str = "https://image.adobe.io/pie/psdService/depthBlur";

/// Base for the Sensei (Adobe Firefly Services AI) image operations.
pub const SENSEI_BASE: &str = "https://image.adobe.io/sensei";
/// `cutout` — remove the background, returning a transparent cut-out (Sensei).
pub const OP_SENSEI_CUTOUT: &str = "https://image.adobe.io/sensei/cutout";
/// `mask` — produce a subject/background alpha mask (Sensei).
pub const OP_SENSEI_MASK: &str = "https://image.adobe.io/sensei/mask";

/// Base for the Adobe **Lightroom** API (Camera-Raw-grade edits, headless).
pub const LR_SERVICE_BASE: &str = "https://image.adobe.io/lrService";
/// `autoTone` — AI auto exposure/contrast/highlights/shadows/whites/blacks/vibrance.
pub const OP_LR_AUTO_TONE: &str = "https://image.adobe.io/lrService/autoTone";
/// `autoStraighten` — Upright perspective correction.
pub const OP_LR_AUTO_STRAIGHTEN: &str = "https://image.adobe.io/lrService/autoStraighten";
/// `edit` — apply explicit Camera-Raw edit parameters.
pub const OP_LR_EDIT: &str = "https://image.adobe.io/lrService/edit";
/// `presets` — apply a Lightroom `.xmp` preset.
pub const OP_LR_PRESET: &str = "https://image.adobe.io/lrService/presets";

/// Where an input/output blob lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Storage {
    /// Adobe I/O Files.
    Aio,
    /// Creative Cloud.
    Adobe,
    /// A pre-signed external URL (e.g. S3).
    External,
    /// An Azure SAS URL.
    Azure,
    /// A Dropbox temporary link.
    Dropbox,
}

impl Storage {
    /// Parse from a CLI / config string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "aio" => Some(Self::Aio),
            "adobe" => Some(Self::Adobe),
            "external" => Some(Self::External),
            "azure" => Some(Self::Azure),
            "dropbox" => Some(Self::Dropbox),
            _ => None,
        }
    }
}

/// Output media type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputType {
    /// `image/jpeg`
    #[serde(rename = "image/jpeg")]
    Jpeg,
    /// `image/png`
    #[serde(rename = "image/png")]
    Png,
    /// `image/vnd.adobe.photoshop` (a `.psd`)
    #[serde(rename = "image/vnd.adobe.photoshop")]
    Psd,
    /// `image/tiff`
    #[serde(rename = "image/tiff")]
    Tiff,
    /// `image/x-adobe-dng`
    #[serde(rename = "image/x-adobe-dng")]
    Dng,
}

impl OutputType {
    /// Infer the output type from a file extension.
    #[must_use]
    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "psd" => Some(Self::Psd),
            "tif" | "tiff" => Some(Self::Tiff),
            "dng" => Some(Self::Dng),
            _ => None,
        }
    }
}

/// An input blob reference (e.g. the source PSD).
#[derive(Debug, Clone, Serialize)]
pub struct Input {
    /// URL of the input.
    pub href: String,
    /// Where the URL points.
    pub storage: Storage,
}

impl Input {
    /// A pre-signed external (S3-style) input URL.
    #[must_use]
    pub fn external(href: impl Into<String>) -> Self {
        Self { href: href.into(), storage: Storage::External }
    }
}

/// An output blob destination.
#[derive(Debug, Clone, Serialize)]
pub struct Output {
    /// Destination URL (must be writable for `external`/`azure`/`dropbox`).
    pub href: String,
    /// Where to write.
    pub storage: Storage,
    /// Output media type.
    #[serde(rename = "type")]
    pub kind: OutputType,
    /// Overwrite an existing object at `href`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwrite: Option<bool>,
}

impl Output {
    /// A pre-signed external output URL with an inferred type.
    #[must_use]
    pub fn external(href: impl Into<String>, kind: OutputType) -> Self {
        Self { href: href.into(), storage: Storage::External, kind, overwrite: Some(true) }
    }
}

/// Per-output job status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PsJobStatus {
    /// Queued.
    Pending,
    /// Running.
    Running,
    /// Uploading the result to the destination.
    Uploading,
    /// Done.
    Succeeded,
    /// Failed.
    Failed,
    /// Any future status this enum does not yet model.
    #[serde(other)]
    Unknown,
}

impl PsJobStatus {
    /// `true` for a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

/// A hypermedia link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// The link target.
    pub href: String,
    /// The storage backing the target, when present.
    #[serde(default)]
    pub storage: Option<Storage>,
}

/// `_links` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    /// The canonical self/status link.
    #[serde(rename = "self")]
    pub self_link: Link,
}

/// Submission response from a POST to an operation.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitResponse {
    /// `_links.self.href` is the status-poll URL.
    #[serde(rename = "_links")]
    pub links: Links,
}

/// One output entry in a job status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobOutput {
    /// The input this output derives from.
    #[serde(default)]
    pub input: Option<String>,
    /// Per-output status.
    pub status: PsJobStatus,
    /// Result links (the produced asset / the manifest payload).
    #[serde(rename = "_links", default)]
    pub links: Option<Links>,
    /// For `documentManifest`, the layer tree / document info is inlined here.
    #[serde(default)]
    pub layer: Option<serde_json::Value>,
    /// Any error detail attached to a failed output.
    #[serde(default)]
    pub errors: Option<serde_json::Value>,
}

/// A Photoshop / Lightroom / Sensei async job's status payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoshopJob {
    /// The job id.
    #[serde(rename = "jobId", default)]
    pub job_id: Option<String>,
    /// A top-level status, used by some services (Lightroom, Sensei) that
    /// report a single status instead of a per-output array.
    #[serde(default)]
    pub status: Option<PsJobStatus>,
    /// Per-output statuses.
    #[serde(default)]
    pub outputs: Vec<JobOutput>,
}

impl PhotoshopJob {
    /// `true` when the job has reached a terminal state. Prefers the per-output
    /// array (Photoshop `psdService`); falls back to the top-level `status`
    /// (Lightroom / Sensei single-status shape).
    #[must_use]
    pub fn all_terminal(&self) -> bool {
        if !self.outputs.is_empty() {
            return self.outputs.iter().all(|o| o.status.is_terminal());
        }
        matches!(&self.status, Some(s) if s.is_terminal())
    }

    /// `true` when the job succeeded (all outputs, or the top-level status).
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        if !self.outputs.is_empty() {
            return self.outputs.iter().all(|o| o.status == PsJobStatus::Succeeded);
        }
        matches!(&self.status, Some(PsJobStatus::Succeeded))
    }
}

/// Request body for `documentOperations` (layer edits + optional renditions).
#[derive(Debug, Clone, Serialize)]
pub struct DocumentOperationsRequest {
    /// Source PSD(s).
    pub inputs: Vec<Input>,
    /// Output rendition(s).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<Output>,
    /// The `options` block (layer edit tree). Passed through verbatim because
    /// the layer-edit schema is large and open-ended; callers build it as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

/// Explicit Lightroom Camera-Raw edit parameters (Process Version 2012).
///
/// Field names serialize to the canonical Camera-Raw XMP keys the Lightroom
/// `edit` endpoint expects (e.g. `Exposure2012`, `Contrast2012`). Every field
/// is optional; only the set ones are sent. Build with the `with_*` helpers
/// and pass to [`PhotoshopClient::lr_edit`].
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LrEdit {
    /// Exposure, in stops (`-5.0` … `+5.0`).
    #[serde(rename = "Exposure2012", skip_serializing_if = "Option::is_none")]
    pub exposure: Option<f64>,
    /// Contrast (`-100` … `+100`).
    #[serde(rename = "Contrast2012", skip_serializing_if = "Option::is_none")]
    pub contrast: Option<i32>,
    /// Highlights (`-100` … `+100`).
    #[serde(rename = "Highlights2012", skip_serializing_if = "Option::is_none")]
    pub highlights: Option<i32>,
    /// Shadows (`-100` … `+100`).
    #[serde(rename = "Shadows2012", skip_serializing_if = "Option::is_none")]
    pub shadows: Option<i32>,
    /// Whites (`-100` … `+100`).
    #[serde(rename = "Whites2012", skip_serializing_if = "Option::is_none")]
    pub whites: Option<i32>,
    /// Blacks (`-100` … `+100`).
    #[serde(rename = "Blacks2012", skip_serializing_if = "Option::is_none")]
    pub blacks: Option<i32>,
    /// White-balance temperature (`-100` … `+100` as a relative shift).
    #[serde(rename = "Temperature", skip_serializing_if = "Option::is_none")]
    pub temperature: Option<i32>,
    /// White-balance tint (`-100` … `+100`).
    #[serde(rename = "Tint", skip_serializing_if = "Option::is_none")]
    pub tint: Option<i32>,
    /// Vibrance (`-100` … `+100`).
    #[serde(rename = "Vibrance", skip_serializing_if = "Option::is_none")]
    pub vibrance: Option<i32>,
    /// Saturation (`-100` … `+100`).
    #[serde(rename = "Saturation", skip_serializing_if = "Option::is_none")]
    pub saturation: Option<i32>,
    /// Clarity (`-100` … `+100`).
    #[serde(rename = "Clarity2012", skip_serializing_if = "Option::is_none")]
    pub clarity: Option<i32>,
    /// Dehaze (`-100` … `+100`).
    #[serde(rename = "Dehaze", skip_serializing_if = "Option::is_none")]
    pub dehaze: Option<i32>,
    /// Texture (`-100` … `+100`).
    #[serde(rename = "Texture", skip_serializing_if = "Option::is_none")]
    pub texture: Option<i32>,
    /// Sharpness (`0` … `150`).
    #[serde(rename = "Sharpness", skip_serializing_if = "Option::is_none")]
    pub sharpness: Option<i32>,
}

impl LrEdit {
    /// A new, empty edit set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` when no field is set (nothing would be sent).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Set exposure (stops).
    #[must_use]
    pub fn with_exposure(mut self, v: f64) -> Self {
        self.exposure = Some(v);
        self
    }
    /// Set contrast.
    #[must_use]
    pub fn with_contrast(mut self, v: i32) -> Self {
        self.contrast = Some(v);
        self
    }
    /// Set highlights.
    #[must_use]
    pub fn with_highlights(mut self, v: i32) -> Self {
        self.highlights = Some(v);
        self
    }
    /// Set shadows.
    #[must_use]
    pub fn with_shadows(mut self, v: i32) -> Self {
        self.shadows = Some(v);
        self
    }
    /// Set whites.
    #[must_use]
    pub fn with_whites(mut self, v: i32) -> Self {
        self.whites = Some(v);
        self
    }
    /// Set blacks.
    #[must_use]
    pub fn with_blacks(mut self, v: i32) -> Self {
        self.blacks = Some(v);
        self
    }
    /// Set white-balance temperature.
    #[must_use]
    pub fn with_temperature(mut self, v: i32) -> Self {
        self.temperature = Some(v);
        self
    }
    /// Set white-balance tint.
    #[must_use]
    pub fn with_tint(mut self, v: i32) -> Self {
        self.tint = Some(v);
        self
    }
    /// Set vibrance.
    #[must_use]
    pub fn with_vibrance(mut self, v: i32) -> Self {
        self.vibrance = Some(v);
        self
    }
    /// Set saturation.
    #[must_use]
    pub fn with_saturation(mut self, v: i32) -> Self {
        self.saturation = Some(v);
        self
    }
    /// Set clarity.
    #[must_use]
    pub fn with_clarity(mut self, v: i32) -> Self {
        self.clarity = Some(v);
        self
    }
    /// Set dehaze.
    #[must_use]
    pub fn with_dehaze(mut self, v: i32) -> Self {
        self.dehaze = Some(v);
        self
    }
    /// Set texture.
    #[must_use]
    pub fn with_texture(mut self, v: i32) -> Self {
        self.texture = Some(v);
        self
    }
    /// Set sharpness.
    #[must_use]
    pub fn with_sharpness(mut self, v: i32) -> Self {
        self.sharpness = Some(v);
        self
    }
}

/// Build a Lightroom request body. Lightroom takes `inputs` as a single
/// object (not an array) plus an `outputs` array and an optional `options`.
fn lr_body(input: &Input, output: &Output, options: Option<serde_json::Value>) -> serde_json::Value {
    let mut body = serde_json::json!({ "inputs": input, "outputs": [output] });
    if let Some(opts) = options {
        if !opts.is_null() {
            body["options"] = opts;
        }
    }
    body
}

/// Build a Sensei request body (`input`/`output` are singular objects).
fn sensei_body(input: &Input, output: &Output) -> serde_json::Value {
    serde_json::json!({ "input": input, "output": output })
}

/// Build a single-input `psdService` body (`inputs`/`outputs` arrays + options).
fn psd_single_body(
    input: &Input,
    output: &Output,
    options: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut body = serde_json::json!({ "inputs": [input], "outputs": [output] });
    if let Some(opts) = options {
        if !opts.is_null() {
            body["options"] = opts;
        }
    }
    body
}

/// The cloud Photoshop API client (shares the IMS [`TokenCache`]).
pub struct PhotoshopClient {
    http: reqwest::Client,
    tokens: TokenCache,
    poll: PollConfig,
}

impl PhotoshopClient {
    /// Build from explicit credentials.
    ///
    /// # Errors
    ///
    /// [`FireflyError::Http`] if the HTTP client cannot be built.
    pub fn new(creds: ImsCredentials) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("aphrody-firefly/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http, tokens: TokenCache::new(creds), poll: PollConfig::default() })
    }

    /// Build from `FIREFLY_CLIENT_ID` / `FIREFLY_CLIENT_SECRET`.
    ///
    /// # Errors
    ///
    /// [`FireflyError::MissingCredential`] / [`FireflyError::Http`].
    pub fn from_env() -> Result<Self> {
        Self::new(ImsCredentials::from_env()?)
    }

    /// Override polling cadence.
    #[must_use]
    pub fn with_poll_config(mut self, poll: PollConfig) -> Self {
        self.poll = poll;
        self
    }

    /// Fetch the layer manifest for one or more PSDs.
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn document_manifest(&self, inputs: Vec<Input>) -> Result<PhotoshopJob> {
        let body = serde_json::json!({ "inputs": inputs });
        self.run(OP_DOCUMENT_MANIFEST, &body).await
    }

    /// Render a PSD to the given output(s).
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn create_rendition(
        &self,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
    ) -> Result<PhotoshopJob> {
        let body = serde_json::json!({ "inputs": inputs, "outputs": outputs });
        self.run(OP_RENDITION_CREATE, &body).await
    }

    /// Apply layer edits (and optionally render) via `documentOperations`.
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn document_operations(
        &self,
        req: &DocumentOperationsRequest,
    ) -> Result<PhotoshopJob> {
        self.run(OP_DOCUMENT_OPERATIONS, req).await
    }

    /// Replace a smart-object layer's contents (`smartObject`). The `body` is a
    /// full request object (`inputs`, `outputs`, `options.layers`), passed
    /// through so callers can express the full smart-object schema.
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn smart_object(&self, body: &serde_json::Value) -> Result<PhotoshopJob> {
        self.run(OP_SMART_OBJECT, body).await
    }

    /// Play an `ActionJSON` program over a PSD/image via `documentOperations`.
    /// `actions` is the `actionJSON` array (a recorded Photoshop action set
    /// translated to JSON); it is placed under `options.actionJSON`.
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn action_json(
        &self,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        actions: serde_json::Value,
    ) -> Result<PhotoshopJob> {
        let body = serde_json::json!({
            "inputs": inputs,
            "outputs": outputs,
            "options": { "actionJSON": actions },
        });
        self.run(OP_DOCUMENT_OPERATIONS, &body).await
    }

    /// Content-aware **product crop** (psdService preview surface).
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn product_crop(
        &self,
        input: Input,
        output: Output,
        options: Option<serde_json::Value>,
    ) -> Result<PhotoshopJob> {
        let body = psd_single_body(&input, &output, options);
        self.run(OP_PRODUCT_CROP, &body).await
    }

    /// **Depth blur** (Neural-Filter depth-of-field; psdService preview surface).
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn depth_blur(
        &self,
        input: Input,
        output: Output,
        options: Option<serde_json::Value>,
    ) -> Result<PhotoshopJob> {
        let body = psd_single_body(&input, &output, options);
        self.run(OP_DEPTH_BLUR, &body).await
    }

    /// **Remove background** — Sensei `cutout`, returning a transparent PNG.
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn remove_background(&self, input: Input, output: Output) -> Result<PhotoshopJob> {
        let body = sensei_body(&input, &output);
        self.run(OP_SENSEI_CUTOUT, &body).await
    }

    /// **Create mask** — Sensei `mask`, returning a subject/background alpha mask.
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn create_mask(&self, input: Input, output: Output) -> Result<PhotoshopJob> {
        let body = sensei_body(&input, &output);
        self.run(OP_SENSEI_MASK, &body).await
    }

    /// Lightroom **auto-tone**: AI exposure/contrast/highlights/shadows/etc.
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn lr_auto_tone(&self, input: Input, output: Output) -> Result<PhotoshopJob> {
        let body = lr_body(&input, &output, None);
        self.run(OP_LR_AUTO_TONE, &body).await
    }

    /// Lightroom **auto-straighten** (Upright perspective correction).
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn lr_auto_straighten(&self, input: Input, output: Output) -> Result<PhotoshopJob> {
        let body = lr_body(&input, &output, None);
        self.run(OP_LR_AUTO_STRAIGHTEN, &body).await
    }

    /// Lightroom **edit**: apply explicit Camera-Raw parameters ([`LrEdit`]).
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn lr_edit(
        &self,
        input: Input,
        output: Output,
        edit: &LrEdit,
    ) -> Result<PhotoshopJob> {
        let opts = serde_json::to_value(edit).unwrap_or(serde_json::Value::Null);
        let body = lr_body(&input, &output, Some(opts));
        self.run(OP_LR_EDIT, &body).await
    }

    /// Lightroom **apply preset**: render with a Lightroom `.xmp` preset, passed
    /// either as an input reference under `options` or as inline XMP.
    ///
    /// # Errors
    ///
    /// As [`PhotoshopClient::run`].
    pub async fn lr_apply_preset(
        &self,
        input: Input,
        output: Output,
        preset_options: serde_json::Value,
    ) -> Result<PhotoshopJob> {
        let body = lr_body(&input, &output, Some(preset_options));
        self.run(OP_LR_PRESET, &body).await
    }

    /// Submit an operation and poll its status URL until every output is
    /// terminal (or the poll budget is exhausted).
    ///
    /// # Errors
    ///
    /// * [`FireflyError::Auth`] on IMS failure.
    /// * [`FireflyError::Api`] on a non-2xx submit/poll response.
    /// * [`FireflyError::JobFailed`] when any output ends `failed`.
    /// * [`FireflyError::JobTimeout`] when the budget is exhausted.
    pub async fn run<B: Serialize + ?Sized>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> Result<PhotoshopJob> {
        let status_href = self.submit(endpoint, body).await?;
        self.await_job(&status_href).await
    }

    async fn submit<B: Serialize + ?Sized>(&self, endpoint: &str, body: &B) -> Result<String> {
        let token = self.tokens.bearer(&self.http).await?;
        let resp = self
            .http
            .post(endpoint)
            .header("x-api-key", self.tokens.client_id())
            .bearer_auth(&token)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(FireflyError::Api {
                status: status.as_u16(),
                endpoint: endpoint.to_string(),
                body: crate::auth::truncate(&text, 512),
            });
        }
        let submit: SubmitResponse = serde_json::from_str(&text)
            .map_err(|source| FireflyError::Decode { endpoint: endpoint.to_string(), source })?;
        Ok(submit.links.self_link.href)
    }

    async fn await_job(&self, status_href: &str) -> Result<PhotoshopJob> {
        let started = Instant::now();
        for _ in 0..self.poll.max_polls {
            let job = self.poll_status(status_href).await?;
            if job.all_terminal() {
                if job.all_succeeded() {
                    return Ok(job);
                }
                let detail = job
                    .outputs
                    .iter()
                    .find(|o| o.status == PsJobStatus::Failed)
                    .and_then(|o| o.errors.as_ref())
                    .map(|e| crate::auth::truncate(&e.to_string(), 512))
                    .unwrap_or_default();
                return Err(FireflyError::JobFailed {
                    job_id: job.job_id.clone().unwrap_or_else(|| status_href.to_string()),
                    status: "failed".to_string(),
                    detail,
                });
            }
            tokio::time::sleep(self.poll.interval).await;
        }
        Err(FireflyError::JobTimeout {
            job_id: status_href.to_string(),
            waited_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            polls: self.poll.max_polls,
        })
    }

    async fn poll_status(&self, status_href: &str) -> Result<PhotoshopJob> {
        let token = self.tokens.bearer(&self.http).await?;
        let resp = self
            .http
            .get(status_href)
            .header("x-api-key", self.tokens.client_id())
            .bearer_auth(&token)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(FireflyError::Api {
                status: status.as_u16(),
                endpoint: status_href.to_string(),
                body: crate::auth::truncate(&text, 512),
            });
        }
        serde_json::from_str(&text).map_err(|source| FireflyError::Decode {
            endpoint: status_href.to_string(),
            source,
        })
    }
}

impl std::fmt::Debug for PhotoshopClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhotoshopClient")
            .field("tokens", &self.tokens)
            .field("poll", &self.poll)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_under_psd_service() {
        for ep in [
            OP_DOCUMENT_MANIFEST,
            OP_DOCUMENT_OPERATIONS,
            OP_SMART_OBJECT,
            OP_RENDITION_CREATE,
        ] {
            assert!(ep.starts_with(PSD_SERVICE_BASE));
        }
    }

    #[test]
    fn storage_parse_and_serialize() {
        assert_eq!(Storage::parse("external"), Some(Storage::External));
        assert_eq!(Storage::parse("ADOBE"), Some(Storage::Adobe));
        assert_eq!(Storage::parse("nope"), None);
        assert_eq!(serde_json::to_string(&Storage::Aio).unwrap(), "\"aio\"");
    }

    #[test]
    fn output_type_from_ext_and_wire() {
        assert_eq!(OutputType::from_ext("png"), Some(OutputType::Png));
        assert_eq!(OutputType::from_ext(".PSD"), Some(OutputType::Psd));
        assert_eq!(OutputType::from_ext("jpeg"), Some(OutputType::Jpeg));
        assert_eq!(OutputType::from_ext("gif"), None);
        assert_eq!(
            serde_json::to_string(&OutputType::Psd).unwrap(),
            "\"image/vnd.adobe.photoshop\""
        );
    }

    #[test]
    fn input_output_serialize_shape() {
        let v = serde_json::to_value(Input::external("https://s3/x.psd")).unwrap();
        assert_eq!(v["href"], "https://s3/x.psd");
        assert_eq!(v["storage"], "external");

        let o = serde_json::to_value(Output::external("https://s3/o.png", OutputType::Png)).unwrap();
        assert_eq!(o["type"], "image/png");
        assert_eq!(o["storage"], "external");
        assert_eq!(o["overwrite"], true);
    }

    #[test]
    fn submit_response_extracts_status_href() {
        let json = r#"{"_links":{"self":{"href":"https://image.adobe.io/pie/psdService/status/J1"}}}"#;
        let s: SubmitResponse = serde_json::from_str(json).unwrap();
        assert_eq!(s.links.self_link.href, "https://image.adobe.io/pie/psdService/status/J1");
    }

    #[test]
    fn job_terminal_and_success_logic() {
        let running: PhotoshopJob = serde_json::from_str(
            r#"{"jobId":"J1","outputs":[{"input":"/i.psd","status":"running"}]}"#,
        )
        .unwrap();
        assert!(!running.all_terminal());

        let ok: PhotoshopJob = serde_json::from_str(
            r#"{"jobId":"J1","outputs":[
                {"input":"/i.psd","status":"succeeded",
                 "_links":{"self":{"href":"https://cc/o.png","storage":"adobe"}}}]}"#,
        )
        .unwrap();
        assert!(ok.all_terminal());
        assert!(ok.all_succeeded());
        assert_eq!(ok.outputs[0].links.as_ref().unwrap().self_link.href, "https://cc/o.png");

        let failed: PhotoshopJob = serde_json::from_str(
            r#"{"jobId":"J1","outputs":[{"status":"failed","errors":{"code":"X"}}]}"#,
        )
        .unwrap();
        assert!(failed.all_terminal());
        assert!(!failed.all_succeeded());
    }

    #[test]
    fn unknown_status_is_non_terminal() {
        let job: PhotoshopJob =
            serde_json::from_str(r#"{"outputs":[{"status":"queued_v2"}]}"#).unwrap();
        assert_eq!(job.outputs[0].status, PsJobStatus::Unknown);
        assert!(!job.all_terminal());
    }

    #[test]
    fn document_operations_request_serializes() {
        let req = DocumentOperationsRequest {
            inputs: vec![Input::external("https://s3/in.psd")],
            outputs: vec![Output::external("https://s3/out.png", OutputType::Png)],
            options: Some(serde_json::json!({ "layers": [{ "name": "Title", "edit": {} }] })),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["inputs"][0]["storage"], "external");
        assert_eq!(v["outputs"][0]["type"], "image/png");
        assert_eq!(v["options"]["layers"][0]["name"], "Title");
    }

    #[test]
    fn empty_outputs_omitted() {
        let req = DocumentOperationsRequest {
            inputs: vec![Input::external("https://s3/in.psd")],
            outputs: vec![],
            options: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("outputs"));
        assert!(!json.contains("options"));
    }

    #[test]
    fn lightroom_endpoints_under_lr_service() {
        for ep in [OP_LR_AUTO_TONE, OP_LR_AUTO_STRAIGHTEN, OP_LR_EDIT, OP_LR_PRESET] {
            assert!(ep.starts_with(LR_SERVICE_BASE));
        }
        for ep in [OP_SENSEI_CUTOUT, OP_SENSEI_MASK] {
            assert!(ep.starts_with(SENSEI_BASE));
        }
        for ep in [OP_PRODUCT_CROP, OP_DEPTH_BLUR] {
            assert!(ep.starts_with(PSD_SERVICE_BASE));
        }
    }

    #[test]
    fn lr_edit_serializes_camera_raw_keys() {
        let edit = LrEdit::new()
            .with_exposure(0.5)
            .with_contrast(20)
            .with_vibrance(15)
            .with_temperature(32);
        let v = serde_json::to_value(&edit).unwrap();
        assert_eq!(v["Exposure2012"], 0.5);
        assert_eq!(v["Contrast2012"], 20);
        assert_eq!(v["Vibrance"], 15);
        assert_eq!(v["Temperature"], 32);
        // Unset fields are omitted.
        assert!(v.get("Shadows2012").is_none());
        assert!(v.get("Dehaze").is_none());
        assert!(!edit.is_empty());
        assert!(LrEdit::new().is_empty());
    }

    #[test]
    fn lr_body_uses_object_inputs() {
        let body = lr_body(
            &Input::external("https://s3/in.jpg"),
            &Output::external("https://s3/out.jpg", OutputType::Jpeg),
            Some(serde_json::json!({ "Exposure2012": 0.3 })),
        );
        // Lightroom: inputs is an object, not an array.
        assert_eq!(body["inputs"]["href"], "https://s3/in.jpg");
        assert_eq!(body["inputs"]["storage"], "external");
        assert_eq!(body["outputs"][0]["type"], "image/jpeg");
        assert_eq!(body["options"]["Exposure2012"], 0.3);
    }

    #[test]
    fn sensei_body_uses_singular_input_output() {
        let body = sensei_body(
            &Input::external("https://s3/in.png"),
            &Output::external("https://s3/cut.png", OutputType::Png),
        );
        assert_eq!(body["input"]["href"], "https://s3/in.png");
        assert_eq!(body["output"]["type"], "image/png");
        // No array wrapping.
        assert!(body["inputs"].is_null());
    }

    #[test]
    fn psd_single_body_uses_arrays() {
        let body = psd_single_body(
            &Input::external("https://s3/in.psd"),
            &Output::external("https://s3/out.jpg", OutputType::Jpeg),
            None,
        );
        assert_eq!(body["inputs"][0]["href"], "https://s3/in.psd");
        assert_eq!(body["outputs"][0]["type"], "image/jpeg");
        assert!(body.get("options").is_none());
    }

    #[test]
    fn top_level_status_drives_terminality() {
        // Lightroom / Sensei single-status shape (no outputs array).
        let ok: PhotoshopJob =
            serde_json::from_str(r#"{"jobId":"L1","status":"succeeded"}"#).unwrap();
        assert!(ok.all_terminal());
        assert!(ok.all_succeeded());

        let running: PhotoshopJob =
            serde_json::from_str(r#"{"jobId":"L1","status":"running"}"#).unwrap();
        assert!(!running.all_terminal());

        let failed: PhotoshopJob =
            serde_json::from_str(r#"{"jobId":"L1","status":"failed"}"#).unwrap();
        assert!(failed.all_terminal());
        assert!(!failed.all_succeeded());

        // Outputs array still takes precedence when present.
        let outputs_win: PhotoshopJob = serde_json::from_str(
            r#"{"status":"succeeded","outputs":[{"status":"running"}]}"#,
        )
        .unwrap();
        assert!(!outputs_win.all_terminal());
    }

    #[test]
    fn client_builds_debug_redacts() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let c = PhotoshopClient::new(ImsCredentials {
            client_id: "ps-id".into(),
            client_secret: "ps-secret".into(),
        })
        .unwrap();
        let dbg = format!("{c:?}");
        assert!(dbg.contains("ps-id"));
        assert!(!dbg.contains("ps-secret"));
    }
}
