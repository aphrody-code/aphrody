// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! The high-level Firefly Services client: token caching, async-job submit +
//! poll, and output download.

use crate::auth::{self, ImsCredentials, TokenCache};
use crate::error::{FireflyError, Result};
use crate::models::{
    AsyncJobSubmission, ExpandRequest, FillRequest, GenerateImageRequest, GenerateResult,
    JobStatusEnvelope,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Firefly REST base host.
pub const FIREFLY_API_BASE: &str = "https://firefly-api.adobe.io";

/// Submit endpoint for async text-to-image generation (Firefly v3).
pub const GENERATE_ASYNC_ENDPOINT: &str =
    "https://firefly-api.adobe.io/v3/images/generate-async";

/// Submit endpoint for async generative expand (Firefly v3).
pub const EXPAND_ASYNC_ENDPOINT: &str = "https://firefly-api.adobe.io/v3/images/expand-async";

/// Submit endpoint for async generative fill (Firefly v3).
pub const FILL_ASYNC_ENDPOINT: &str = "https://firefly-api.adobe.io/v3/images/fill-async";

/// Polling configuration for async jobs.
#[derive(Debug, Clone, Copy)]
pub struct PollConfig {
    /// Delay between status polls.
    pub interval: Duration,
    /// Maximum number of polls before giving up with [`FireflyError::JobTimeout`].
    pub max_polls: u32,
}

impl Default for PollConfig {
    fn default() -> Self {
        // Firefly image jobs typically finish in a few seconds; poll once a
        // second for up to ~2 minutes.
        Self { interval: Duration::from_secs(1), max_polls: 120 }
    }
}

/// One downloaded output image, in memory.
#[derive(Debug, Clone)]
pub struct FireflyImage {
    /// Raw image bytes.
    pub bytes: Vec<u8>,
    /// `Content-Type` reported when downloading (e.g. `image/jpeg`).
    pub content_type: String,
    /// The seed that produced this variation, if Firefly reported one.
    pub seed: Option<i64>,
    /// Zero-based variation index.
    pub index: usize,
}

impl FireflyImage {
    /// Map the download `Content-Type` to a file extension.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        let base = self.content_type.split(';').next().unwrap_or("").trim();
        match base {
            "image/png" => "png",
            "image/webp" => "webp",
            // image/jpeg, image/jpg and anything else: Firefly emits JPEG.
            _ => "jpg",
        }
    }

    /// Persist this image to `dir` as `{prefix}_{index}.{ext}` (atomic write).
    ///
    /// # Errors
    ///
    /// [`FireflyError::Io`] on any filesystem failure.
    pub async fn save_to_dir(&self, dir: &Path, prefix: &str) -> Result<PathBuf> {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|source| FireflyError::Io { path: dir.to_path_buf(), source })?;
        let path = dir.join(format!("{prefix}_{}.{}", self.index, self.extension()));
        let tmp = path.with_extension(format!("{}.tmp", self.extension()));
        tokio::fs::write(&tmp, &self.bytes)
            .await
            .map_err(|source| FireflyError::Io { path: tmp.clone(), source })?;
        tokio::fs::rename(&tmp, &path)
            .await
            .map_err(|source| FireflyError::Io { path: path.clone(), source })?;
        tracing::debug!(path = %path.display(), bytes = self.bytes.len(), "saved Firefly output");
        Ok(path)
    }
}

/// A cached, reusable Adobe Firefly Services client.
///
/// The HTTP client and IMS token are created once and reused — minimising cold
/// starts and avoiding a token round-trip on every call (cf. the project-wide
/// latency objective).
pub struct FireflyClient {
    http: reqwest::Client,
    tokens: TokenCache,
    poll: PollConfig,
}

impl FireflyClient {
    /// Build a client from explicit credentials.
    ///
    /// # Errors
    ///
    /// [`FireflyError::Http`] if the underlying reqwest client cannot be built.
    pub fn new(creds: ImsCredentials) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("aphrody-firefly/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http, tokens: TokenCache::new(creds), poll: PollConfig::default() })
    }

    /// Build a client from `FIREFLY_CLIENT_ID` / `FIREFLY_CLIENT_SECRET`.
    ///
    /// # Errors
    ///
    /// [`FireflyError::MissingCredential`] when the env vars are absent, or
    /// [`FireflyError::Http`] if the client cannot be built.
    pub fn from_env() -> Result<Self> {
        Self::new(ImsCredentials::from_env()?)
    }

    /// Override the async-job polling configuration.
    #[must_use]
    pub fn with_poll_config(mut self, poll: PollConfig) -> Self {
        self.poll = poll;
        self
    }

    /// Return a valid bearer token, refreshing it through IMS if needed.
    async fn bearer(&self) -> Result<String> {
        self.tokens.bearer(&self.http).await
    }

    /// Submit a generate request and wait for the job to complete, returning the
    /// raw [`GenerateResult`] (output URLs not yet downloaded).
    ///
    /// # Errors
    ///
    /// * [`FireflyError::Auth`] on IMS failure.
    /// * [`FireflyError::Api`] on a non-2xx submit/poll response.
    /// * [`FireflyError::JobFailed`] / [`FireflyError::JobTimeout`] on job error.
    pub async fn generate(&self, req: &GenerateImageRequest) -> Result<GenerateResult> {
        let submission = self.submit(req).await?;
        self.await_job(&submission).await
    }

    /// Submit a generate request, wait for completion, then download every
    /// output variation into memory.
    ///
    /// # Errors
    ///
    /// As [`FireflyClient::generate`], plus [`FireflyError::Http`] on download.
    pub async fn generate_and_download(
        &self,
        req: &GenerateImageRequest,
    ) -> Result<Vec<FireflyImage>> {
        let result = self.generate(req).await?;
        self.download_outputs(&result).await
    }

    /// Generative **expand**: enlarge the canvas of an image, AI-filling the new
    /// area. Submit → poll → returns the [`GenerateResult`] (output URLs).
    ///
    /// # Errors
    ///
    /// As [`FireflyClient::generate`].
    pub async fn expand(&self, req: &ExpandRequest) -> Result<GenerateResult> {
        let submission = self.submit_to(EXPAND_ASYNC_ENDPOINT, req).await?;
        self.await_job(&submission).await
    }

    /// Generative expand, then download every variation into memory.
    ///
    /// # Errors
    ///
    /// As [`FireflyClient::expand`], plus [`FireflyError::Http`] on download.
    pub async fn expand_and_download(&self, req: &ExpandRequest) -> Result<Vec<FireflyImage>> {
        let result = self.expand(req).await?;
        self.download_outputs(&result).await
    }

    /// Generative **fill**: replace the masked region of an image with
    /// prompt-guided content. Submit → poll → returns the [`GenerateResult`].
    ///
    /// # Errors
    ///
    /// As [`FireflyClient::generate`].
    pub async fn fill(&self, req: &FillRequest) -> Result<GenerateResult> {
        let submission = self.submit_to(FILL_ASYNC_ENDPOINT, req).await?;
        self.await_job(&submission).await
    }

    /// Generative fill, then download every variation into memory.
    ///
    /// # Errors
    ///
    /// As [`FireflyClient::fill`], plus [`FireflyError::Http`] on download.
    pub async fn fill_and_download(&self, req: &FillRequest) -> Result<Vec<FireflyImage>> {
        let result = self.fill(req).await?;
        self.download_outputs(&result).await
    }

    /// Submit the async generate job (returns immediately with a job handle).
    async fn submit(&self, req: &GenerateImageRequest) -> Result<AsyncJobSubmission> {
        self.submit_to(GENERATE_ASYNC_ENDPOINT, req).await
    }

    /// Submit any async-image job to `endpoint`, returning its job handle. The
    /// generate / expand / fill endpoints all share this submission shape.
    async fn submit_to<B: Serialize + ?Sized>(
        &self,
        endpoint: &str,
        req: &B,
    ) -> Result<AsyncJobSubmission> {
        let token = self.bearer().await?;
        let resp = self
            .http
            .post(endpoint)
            .header("x-api-key", self.tokens.client_id())
            .bearer_auth(&token)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(req)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(FireflyError::Api {
                status: status.as_u16(),
                endpoint: endpoint.to_string(),
                body: auth::truncate(&body, 512),
            });
        }
        serde_json::from_str(&body).map_err(|source| FireflyError::Decode {
            endpoint: endpoint.to_string(),
            source,
        })
    }

    /// Poll a job's status URL until it reaches a terminal state.
    async fn await_job(&self, submission: &AsyncJobSubmission) -> Result<GenerateResult> {
        let started = Instant::now();
        for poll in 1..=self.poll.max_polls {
            let envelope = self.poll_status(&submission.status_url).await?;
            if envelope.status.is_success() {
                return envelope.result.ok_or_else(|| FireflyError::JobFailed {
                    job_id: submission.job_id.clone(),
                    status: "succeeded".to_string(),
                    detail: "job succeeded but carried no result payload".to_string(),
                });
            }
            if envelope.status.is_terminal() {
                let detail = envelope
                    .error
                    .map(|e| auth::truncate(&e.to_string(), 512))
                    .unwrap_or_default();
                return Err(FireflyError::JobFailed {
                    job_id: submission.job_id.clone(),
                    status: format!("{:?}", envelope.status).to_lowercase(),
                    detail,
                });
            }
            tracing::trace!(poll, job = %submission.job_id, "Firefly job still running");
            tokio::time::sleep(self.poll.interval).await;
        }
        Err(FireflyError::JobTimeout {
            job_id: submission.job_id.clone(),
            waited_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            polls: self.poll.max_polls,
        })
    }

    /// Single status poll against `status_url`.
    async fn poll_status(&self, status_url: &str) -> Result<JobStatusEnvelope> {
        let token = self.bearer().await?;
        let resp = self
            .http
            .get(status_url)
            .header("x-api-key", self.tokens.client_id())
            .bearer_auth(&token)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(FireflyError::Api {
                status: status.as_u16(),
                endpoint: status_url.to_string(),
                body: auth::truncate(&body, 512),
            });
        }
        serde_json::from_str(&body)
            .map_err(|source| FireflyError::Decode { endpoint: status_url.to_string(), source })
    }

    /// Download every output URL in `result` concurrently.
    ///
    /// # Errors
    ///
    /// [`FireflyError::Http`] if any download fails.
    pub async fn download_outputs(&self, result: &GenerateResult) -> Result<Vec<FireflyImage>> {
        let mut set = tokio::task::JoinSet::new();
        for (index, output) in result.outputs.iter().enumerate() {
            let http = self.http.clone();
            let url = output.image.url.clone();
            let seed = output.seed;
            set.spawn(async move {
                let resp = http.get(&url).send().await?;
                let resp = resp.error_for_status()?;
                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("image/jpeg")
                    .to_string();
                let bytes = resp.bytes().await?.to_vec();
                Ok::<FireflyImage, FireflyError>(FireflyImage {
                    bytes,
                    content_type,
                    seed,
                    index,
                })
            });
        }

        let mut images = Vec::with_capacity(result.outputs.len());
        while let Some(joined) = set.join_next().await {
            // A panicked download task is a bug; surface it as a transport-shaped
            // error rather than silently dropping the variation.
            match joined {
                Ok(res) => images.push(res?),
                Err(join_err) => {
                    return Err(FireflyError::JobFailed {
                        job_id: "<download>".to_string(),
                        status: "panicked".to_string(),
                        detail: join_err.to_string(),
                    })
                }
            }
        }
        images.sort_by_key(|img| img.index);
        Ok(images)
    }
}

impl std::fmt::Debug for FireflyClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FireflyClient")
            .field("creds", self.tokens.creds())
            .field("poll", &self.poll)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ContentClass, GenerateImageRequest, Size};

    fn test_client() -> FireflyClient {
        // reqwest 0.13 on rustls requires a CryptoProvider before building a
        // client (CLAUDE.md §7). Idempotent: ignore "already installed".
        let _ = rustls::crypto::ring::default_provider().install_default();
        FireflyClient::new(ImsCredentials {
            client_id: "test-id".into(),
            client_secret: "test-secret".into(),
        })
        .unwrap()
    }

    #[test]
    fn endpoints_are_v3() {
        assert!(GENERATE_ASYNC_ENDPOINT.starts_with(FIREFLY_API_BASE));
        assert!(GENERATE_ASYNC_ENDPOINT.ends_with("/v3/images/generate-async"));
        assert!(EXPAND_ASYNC_ENDPOINT.ends_with("/v3/images/expand-async"));
        assert!(FILL_ASYNC_ENDPOINT.ends_with("/v3/images/fill-async"));
        assert!(EXPAND_ASYNC_ENDPOINT.starts_with(FIREFLY_API_BASE));
        assert!(FILL_ASYNC_ENDPOINT.starts_with(FIREFLY_API_BASE));
    }

    #[test]
    fn poll_config_defaults_are_sane() {
        let p = PollConfig::default();
        assert_eq!(p.interval, Duration::from_secs(1));
        assert_eq!(p.max_polls, 120);
    }

    #[test]
    fn image_extension_from_content_type() {
        let mk = |ct: &str| FireflyImage {
            bytes: vec![],
            content_type: ct.into(),
            seed: None,
            index: 0,
        };
        assert_eq!(mk("image/png").extension(), "png");
        assert_eq!(mk("image/jpeg").extension(), "jpg");
        assert_eq!(mk("image/webp; charset=binary").extension(), "webp");
        assert_eq!(mk("application/octet-stream").extension(), "jpg");
    }

    #[tokio::test]
    async fn save_to_dir_writes_indexed_file() {
        let dir = tempfile::tempdir().unwrap();
        let img = FireflyImage {
            bytes: b"\xff\xd8\xff fake jpeg".to_vec(),
            content_type: "image/jpeg".into(),
            seed: Some(42),
            index: 2,
        };
        let path = img.save_to_dir(dir.path(), "firefly").await.unwrap();
        assert!(path.ends_with("firefly_2.jpg"));
        let read = tokio::fs::read(&path).await.unwrap();
        assert_eq!(read, img.bytes);
    }

    #[test]
    fn client_builds_and_debug_redacts() {
        let c = test_client();
        let dbg = format!("{c:?}");
        assert!(dbg.contains("test-id"));
        assert!(!dbg.contains("test-secret"));
    }

    #[test]
    fn request_round_trips_through_builder() {
        let req = GenerateImageRequest::new("northern lights over a fjord")
            .with_variations(2)
            .with_size(Size::WIDESCREEN)
            .with_content_class(ContentClass::Photo);
        assert_eq!(req.num_variations, Some(2));
        assert_eq!(req.size, Some(Size::WIDESCREEN));
        assert_eq!(req.content_class, Some("photo"));
    }
}
