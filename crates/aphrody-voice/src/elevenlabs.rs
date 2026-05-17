// SPDX-License-Identifier: Apache-2.0
//! ElevenLabs TTS adapter.
//!
//! Implements [`VoiceProvider`] against the ElevenLabs v1 REST API:
//! - `GET /v1/voices` — enumerate available voices.
//! - `POST /v1/text-to-speech/{voice_id}` — synthesise (buffered).
//! - `POST /v1/text-to-speech/{voice_id}/stream` — synthesise (streaming).
//!
//! # Authentication
//!
//! Pass the API key via [`ElevenLabsProvider::new`] or
//! [`ElevenLabsProvider::with_base_url`].  The key is sent in the
//! `xi-api-key` header on every request.
//!
//! # Wire format
//!
//! Request bodies are JSON.  Audio is returned as `audio/mpeg` by default.
//! The `accept` header is set to `audio/mpeg` on every synthesis request.

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use serde::Serialize;
use serde_json::Value;

use crate::{SynthOptions, VoiceError, VoiceInfo, VoiceProvider};

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL: &str = "https://api.elevenlabs.io";
const AUDIO_MPEG: &str = "audio/mpeg";

// ── ElevenLabsProvider ────────────────────────────────────────────────────────

/// TTS provider backed by the ElevenLabs REST API.
///
/// Clone is cheap: the inner [`reqwest::Client`] is `Arc`-wrapped.
#[derive(Debug, Clone)]
pub struct ElevenLabsProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl ElevenLabsProvider {
    /// Construct a provider using the ElevenLabs production endpoint.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    /// Construct a provider with a custom `base_url` (useful for testing
    /// against a mock server).
    ///
    /// Trailing slashes on `base_url` are stripped automatically.
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let raw = base_url.into();
        let base_url = raw.trim_end_matches('/').to_owned();
        Self { api_key: api_key.into(), base_url, client: reqwest::Client::new() }
    }
}

// ── Wire request body ─────────────────────────────────────────────────────────

/// JSON body sent to the synthesis endpoints.
#[derive(Debug, Serialize)]
struct SynthRequest<'a> {
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_settings: Option<VoiceSettings>,
}

#[derive(Debug, Serialize)]
struct VoiceSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    stability: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    similarity_boost: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_speaker_boost: Option<bool>,
}

impl<'a> SynthRequest<'a> {
    fn from_opts(text: &'a str, opts: &'a SynthOptions) -> Self {
        let has_settings = opts.stability.is_some()
            || opts.similarity_boost.is_some()
            || opts.style.is_some()
            || opts.use_speaker_boost.is_some();

        let voice_settings = has_settings.then(|| VoiceSettings {
            stability: opts.stability,
            similarity_boost: opts.similarity_boost,
            style: opts.style,
            use_speaker_boost: opts.use_speaker_boost,
        });

        Self { text, model_id: opts.model_id.as_deref(), voice_settings }
    }
}

// ── Public payload parser (also used by tests in lib.rs) ─────────────────────

/// Parse a raw `GET /v1/voices` JSON payload into a `Vec<VoiceInfo>`.
///
/// Unknown or malformed voice entries are silently skipped (same policy as the
/// upstream TypeScript `normalizeVoice` helper).
pub fn parse_voices_payload(payload: &Value) -> Result<Vec<VoiceInfo>, VoiceError> {
    let raw_voices = match payload.get("voices").and_then(Value::as_array) {
        Some(arr) => arr,
        None => return Ok(Vec::new()),
    };

    let voices = raw_voices.iter().filter_map(normalize_voice).collect::<Vec<_>>();

    Ok(voices)
}

/// Map a single raw voice JSON object to [`VoiceInfo`].
///
/// Returns `None` if the entry lacks a non-empty `voice_id`.
fn normalize_voice(value: &Value) -> Option<VoiceInfo> {
    let obj = value.as_object()?;

    let id = read_string(obj.get("voice_id")?)?;

    let name = obj.get("name").and_then(|v| read_string(v)).unwrap_or_else(|| id.clone());

    let preview_url = obj.get("preview_url").and_then(|v| read_string(v));

    // Gender is buried inside the `labels` map.
    let labels = obj.get("labels").and_then(Value::as_object);
    let gender = labels.and_then(|m| m.get("gender")).and_then(|v| read_string(v));

    // `language` is not a top-level field in the standard v1 payload; it may
    // appear in fine-tuned or shared voice metadata under `fine_tuning`.
    // We also accept it as a direct top-level field for forward-compat.
    let language = obj.get("language").and_then(|v| read_string(v));

    Some(VoiceInfo { id, name, language, gender, preview_url })
}

/// Return `Some(trimmed)` only when `value` is a non-empty string.
fn read_string(value: &Value) -> Option<String> {
    let s = value.as_str()?.trim();
    if s.is_empty() { None } else { Some(s.to_owned()) }
}

// ── Helper: check HTTP status and extract error body ─────────────────────────

async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, VoiceError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let code = status.as_u16();
    let body = resp.text().await.unwrap_or_else(|_| String::from("<unreadable body>"));
    Err(VoiceError::Api { status: code, body })
}

// ── VoiceProvider impl ────────────────────────────────────────────────────────

#[async_trait]
impl VoiceProvider for ElevenLabsProvider {
    /// List voices available on the account.
    ///
    /// Calls `GET /v1/voices` with the `xi-api-key` header.
    async fn list_voices(&self) -> Result<Vec<VoiceInfo>, VoiceError> {
        let url = format!("{}/v1/voices", self.base_url);

        let resp = self
            .client
            .get(&url)
            .header("xi-api-key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;

        let resp = check_status(resp).await?;
        let payload: Value = resp.json().await?;
        parse_voices_payload(&payload)
    }

    /// Synthesise `text` into audio and return the full payload as bytes.
    ///
    /// Calls `POST /v1/text-to-speech/{voice_id}` with `accept: audio/mpeg`.
    async fn synthesize(
        &self,
        text: &str,
        voice_id: &str,
        opts: SynthOptions,
    ) -> Result<Vec<u8>, VoiceError> {
        let url = format!("{}/v1/text-to-speech/{}", self.base_url, voice_id);
        let body = SynthRequest::from_opts(text, &opts);

        let resp = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header(reqwest::header::ACCEPT, AUDIO_MPEG)
            .json(&body)
            .send()
            .await?;

        let resp = check_status(resp).await?;
        let audio = resp.bytes().await?;
        Ok(audio.to_vec())
    }

    /// Synthesise `text` as a streaming byte sequence.
    ///
    /// Calls `POST /v1/text-to-speech/{voice_id}/stream`.  The returned
    /// stream yields [`bytes::Bytes`] chunks as they arrive from the API.
    async fn synthesize_stream(
        &self,
        text: &str,
        voice_id: &str,
        opts: SynthOptions,
    ) -> Result<impl Stream<Item = Result<Bytes, VoiceError>> + Send, VoiceError> {
        let url = format!("{}/v1/text-to-speech/{}/stream", self.base_url, voice_id);
        let body = SynthRequest::from_opts(text, &opts);

        let resp = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header(reqwest::header::ACCEPT, AUDIO_MPEG)
            .json(&body)
            .send()
            .await?;

        let resp = check_status(resp).await?;

        // Map the reqwest byte-stream errors into VoiceError.
        let stream = futures::StreamExt::map(resp.bytes_stream(), |chunk| {
            chunk.map_err(VoiceError::Transport)
        });

        Ok(stream)
    }
}
