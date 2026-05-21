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

/// A Photoshop async job's status payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoshopJob {
    /// The job id.
    #[serde(rename = "jobId", default)]
    pub job_id: Option<String>,
    /// Per-output statuses.
    #[serde(default)]
    pub outputs: Vec<JobOutput>,
}

impl PhotoshopJob {
    /// `true` when every output has reached a terminal state (or there are
    /// none yet — caller keeps polling until at least one appears).
    #[must_use]
    pub fn all_terminal(&self) -> bool {
        !self.outputs.is_empty() && self.outputs.iter().all(|o| o.status.is_terminal())
    }

    /// `true` when every output succeeded.
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        !self.outputs.is_empty()
            && self.outputs.iter().all(|o| o.status == PsJobStatus::Succeeded)
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
