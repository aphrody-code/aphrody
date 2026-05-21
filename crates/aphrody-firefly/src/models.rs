// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Typed request / response models for the Firefly v3 image API.
//!
//! Field names and the async job shape are taken from the Adobe Firefly
//! Services `OpenAPI` surface (2026-05): `POST /v3/images/generate-async`
//! returns `{ jobId, statusUrl, cancelUrl }`; polling `statusUrl` yields a
//! `{ status, result }` envelope where `result.outputs[].image.url` holds the
//! pre-signed download links.

use serde::{Deserialize, Serialize};

/// Image dimensions in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Size {
    /// 2048×2048 — the Firefly v3 default square output.
    pub const SQUARE_2K: Self = Self { width: 2048, height: 2048 };
    /// 2304×1792 — landscape 4:3-ish.
    pub const LANDSCAPE: Self = Self { width: 2304, height: 1792 };
    /// 1792×2304 — portrait.
    pub const PORTRAIT: Self = Self { width: 1792, height: 2304 };
    /// 2688×1536 — widescreen 16:9-ish.
    pub const WIDESCREEN: Self = Self { width: 2688, height: 1536 };

    /// Parse an `WxH` string (e.g. `"2048x2048"`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let (w, h) = s.split_once(['x', 'X', '*'])?;
        Some(Self {
            width: w.trim().parse().ok()?,
            height: h.trim().parse().ok()?,
        })
    }
}

/// The Firefly `contentClass` hint — biases the model toward photographic or
/// illustrated output. Omitted from the request when `Auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentClass {
    /// Let Firefly infer the class from the prompt (field omitted).
    #[default]
    Auto,
    /// Photographic realism.
    Photo,
    /// Illustration / art.
    Art,
}

impl ContentClass {
    /// The wire token, or `None` for [`ContentClass::Auto`].
    #[must_use]
    pub fn wire(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Photo => Some("photo"),
            Self::Art => Some("art"),
        }
    }

    /// Parse from a CLI string (`photo` / `art` / `auto`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Some(Self::Auto),
            "photo" | "photograph" | "photographic" => Some(Self::Photo),
            "art" | "illustration" => Some(Self::Art),
            _ => None,
        }
    }
}

/// The request body for `POST /v3/images/generate-async`.
///
/// Only `prompt` is required; every other field is omitted from the wire when
/// unset (`skip_serializing_if`) so the server applies its defaults.
#[derive(Debug, Clone, Serialize)]
pub struct GenerateImageRequest {
    /// The text prompt (required).
    pub prompt: String,

    /// A negative prompt — concepts to avoid.
    #[serde(rename = "negativePrompt", skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,

    /// Number of variations to produce (1–4).
    #[serde(rename = "numVariations", skip_serializing_if = "Option::is_none")]
    pub num_variations: Option<u8>,

    /// Output dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,

    /// Photographic vs illustrated bias.
    #[serde(rename = "contentClass", skip_serializing_if = "Option::is_none")]
    pub content_class: Option<&'static str>,

    /// Per-variation seeds for reproducibility (length must match
    /// `num_variations` when both are set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeds: Option<Vec<u64>>,

    /// Strength of the prompt's influence (1–10).
    #[serde(rename = "visualIntensity", skip_serializing_if = "Option::is_none")]
    pub visual_intensity: Option<u8>,

    /// BCP-47 locale used to bias the prompt (e.g. `"en-US"`, `"fr-FR"`).
    #[serde(rename = "promptBiasingLocaleCode", skip_serializing_if = "Option::is_none")]
    pub prompt_biasing_locale_code: Option<String>,
}

impl GenerateImageRequest {
    /// Build a minimal request from just a prompt.
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            negative_prompt: None,
            num_variations: None,
            size: None,
            content_class: None,
            seeds: None,
            visual_intensity: None,
            prompt_biasing_locale_code: None,
        }
    }

    /// Set the number of variations (clamped to 1..=4).
    #[must_use]
    pub fn with_variations(mut self, n: u8) -> Self {
        self.num_variations = Some(n.clamp(1, 4));
        self
    }

    /// Set the output size.
    #[must_use]
    pub fn with_size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the content class.
    #[must_use]
    pub fn with_content_class(mut self, class: ContentClass) -> Self {
        self.content_class = class.wire();
        self
    }

    /// Set the negative prompt.
    #[must_use]
    pub fn with_negative_prompt(mut self, neg: impl Into<String>) -> Self {
        self.negative_prompt = Some(neg.into());
        self
    }

    /// Set the prompt-biasing locale.
    #[must_use]
    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.prompt_biasing_locale_code = Some(locale.into());
        self
    }
}

/// Response from submitting an async generate job.
#[derive(Debug, Clone, Deserialize)]
pub struct AsyncJobSubmission {
    /// The Firefly job id, e.g. `urn:ff:jobs:<org>:<uuid>`.
    #[serde(rename = "jobId")]
    pub job_id: String,
    /// Absolute URL to poll for status.
    #[serde(rename = "statusUrl")]
    pub status_url: String,
    /// Absolute URL to cancel the job (optional in some responses).
    #[serde(rename = "cancelUrl", default)]
    pub cancel_url: Option<String>,
}

/// Terminal / transient status of an async Firefly job.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Queued, not yet started.
    Pending,
    /// Currently running.
    Running,
    /// Completed successfully — `result` is populated.
    Succeeded,
    /// Failed — terminal.
    Failed,
    /// Cancelled — terminal.
    Cancelled,
    /// Any status string Firefly introduces that this enum does not yet model.
    #[serde(other)]
    Unknown,
}

impl JobStatus {
    /// `true` when the job has reached a terminal state (success or failure).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    /// `true` only for successful completion.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

/// The pre-signed image link inside an output.
#[derive(Debug, Clone, Deserialize)]
pub struct ImageRef {
    /// Pre-signed S3 download URL (short-lived).
    pub url: String,
}

/// A single generated variation.
#[derive(Debug, Clone, Deserialize)]
pub struct Output {
    /// The seed that produced this variation.
    #[serde(default)]
    pub seed: Option<i64>,
    /// The downloadable image reference.
    pub image: ImageRef,
}

/// The `result` payload of a succeeded job.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResult {
    /// Final output dimensions.
    #[serde(default)]
    pub size: Option<Size>,
    /// The generated variations.
    #[serde(default)]
    pub outputs: Vec<Output>,
    /// The content class Firefly applied.
    #[serde(rename = "contentClass", default)]
    pub content_class: Option<String>,
}

/// The envelope returned when polling a job's status URL.
#[derive(Debug, Clone, Deserialize)]
pub struct JobStatusEnvelope {
    /// The job id echoed back.
    #[serde(rename = "jobId", default)]
    pub job_id: Option<String>,
    /// Current status.
    pub status: JobStatus,
    /// Present only once `status == Succeeded`.
    #[serde(default)]
    pub result: Option<GenerateResult>,
    /// Failure reason when `status == Failed`.
    #[serde(rename = "error", default)]
    pub error: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_only_set_fields() {
        let req = GenerateImageRequest::new("a cat coding");
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, r#"{"prompt":"a cat coding"}"#);
    }

    #[test]
    fn request_builder_emits_camel_case() {
        let req = GenerateImageRequest::new("x")
            .with_variations(3)
            .with_size(Size::SQUARE_2K)
            .with_content_class(ContentClass::Photo)
            .with_negative_prompt("blurry")
            .with_locale("fr-FR");
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["numVariations"], 3);
        assert_eq!(v["size"]["width"], 2048);
        assert_eq!(v["contentClass"], "photo");
        assert_eq!(v["negativePrompt"], "blurry");
        assert_eq!(v["promptBiasingLocaleCode"], "fr-FR");
    }

    #[test]
    fn variations_are_clamped() {
        assert_eq!(GenerateImageRequest::new("x").with_variations(0).num_variations, Some(1));
        assert_eq!(GenerateImageRequest::new("x").with_variations(9).num_variations, Some(4));
    }

    #[test]
    fn content_class_auto_omits_field() {
        let req = GenerateImageRequest::new("x").with_content_class(ContentClass::Auto);
        assert!(req.content_class.is_none());
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("contentClass"));
    }

    #[test]
    fn size_parses_various_separators() {
        assert_eq!(Size::parse("2048x2048"), Some(Size { width: 2048, height: 2048 }));
        assert_eq!(Size::parse("1024X768"), Some(Size { width: 1024, height: 768 }));
        assert_eq!(Size::parse("100*200"), Some(Size { width: 100, height: 200 }));
        assert_eq!(Size::parse("garbage"), None);
    }

    #[test]
    fn content_class_parse_roundtrip() {
        assert_eq!(ContentClass::parse("photo"), Some(ContentClass::Photo));
        assert_eq!(ContentClass::parse("ART"), Some(ContentClass::Art));
        assert_eq!(ContentClass::parse("auto"), Some(ContentClass::Auto));
        assert_eq!(ContentClass::parse("nope"), None);
    }

    #[test]
    fn submission_deserializes() {
        let json = r#"{
            "jobId":"urn:ff:jobs:eso851211:86ffe2ea",
            "statusUrl":"https://firefly-api.adobe.io/v3/status/urn:ff:jobs:eso851211:86ffe2ea",
            "cancelUrl":"https://firefly-api.adobe.io/v3/cancel/urn:ff:jobs:eso851211:86ffe2ea"
        }"#;
        let sub: AsyncJobSubmission = serde_json::from_str(json).unwrap();
        assert_eq!(sub.job_id, "urn:ff:jobs:eso851211:86ffe2ea");
        assert!(sub.status_url.ends_with("86ffe2ea"));
        assert!(sub.cancel_url.is_some());
    }

    #[test]
    fn status_running_then_succeeded() {
        let running: JobStatusEnvelope =
            serde_json::from_str(r#"{"status":"running"}"#).unwrap();
        assert_eq!(running.status, JobStatus::Running);
        assert!(!running.status.is_terminal());

        let done: JobStatusEnvelope = serde_json::from_str(
            r#"{"jobId":"j1","status":"succeeded","result":{"size":{"width":2048,"height":2048},
                "outputs":[{"seed":1779323515,"image":{"url":"https://s3/x.png"}}],
                "contentClass":"art"}}"#,
        )
        .unwrap();
        assert!(done.status.is_success());
        let result = done.result.unwrap();
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].image.url, "https://s3/x.png");
        assert_eq!(result.outputs[0].seed, Some(1_779_323_515));
        assert_eq!(result.size, Some(Size::SQUARE_2K));
    }

    #[test]
    fn unknown_status_maps_to_unknown_variant() {
        let env: JobStatusEnvelope =
            serde_json::from_str(r#"{"status":"some_future_state"}"#).unwrap();
        assert_eq!(env.status, JobStatus::Unknown);
        assert!(!env.status.is_terminal());
    }

    #[test]
    fn failed_status_is_terminal_not_success() {
        let env: JobStatusEnvelope =
            serde_json::from_str(r#"{"status":"failed","error":{"code":"x"}}"#).unwrap();
        assert!(env.status.is_terminal());
        assert!(!env.status.is_success());
        assert!(env.error.is_some());
    }
}
