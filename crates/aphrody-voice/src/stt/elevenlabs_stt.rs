// SPDX-License-Identifier: Apache-2.0
//! ElevenLabs STT adapter.
//!
//! Implements [`SttProvider`] against the ElevenLabs Speech-to-Text endpoint:
//! - `POST /v1/speech-to-text` — multipart/form-data upload.
//!
//! # Authentication
//!
//! Pass the API key via [`ElevenLabsSttProvider::new`] or read it from the
//! `ELEVENLABS_API_KEY` environment variable via
//! [`ElevenLabsSttProvider::from_env`].  The key is sent in the `xi-api-key`
//! header on every request.
//!
//! # Wire format
//!
//! The request is `multipart/form-data` with the following fields:
//! - `audio` — the audio bytes with a synthetic filename (`audio.wav`).
//! - `model_id` — e.g. `"scribe_v1"`.
//! - `language_code` — BCP-47 tag, omitted when `None`.
//!
//! The response is always JSON:
//! ```json
//! {
//!   "text": "Hello world.",
//!   "language_code": "en",
//!   "language_probability": 0.99,
//!   "words": [...]
//! }
//! ```
//!
//! # Streaming
//!
//! ElevenLabs STT does not expose a token-streaming endpoint.
//! [`SttProvider::transcribe_stream`] accumulates the input stream, calls the
//! buffered endpoint, and emits a single final [`TranscriptDelta`].

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use serde::Deserialize;
use serde_json::Value;

use crate::stt::{SttError, SttOptions, SttProvider, Transcript, TranscriptDelta, TranscriptSegment};

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL: &str = "https://api.elevenlabs.io";
/// Environment variable consulted by [`ElevenLabsSttProvider::from_env`].
pub const API_KEY_ENV: &str = "ELEVENLABS_API_KEY";

// ── ElevenLabsSttProvider ─────────────────────────────────────────────────────

/// STT provider backed by the ElevenLabs REST API.
///
/// Clone is cheap: the inner [`reqwest::Client`] is `Arc`-wrapped.
#[derive(Debug, Clone)]
pub struct ElevenLabsSttProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl ElevenLabsSttProvider {
    /// Construct a provider using the ElevenLabs production endpoint.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    /// Construct a provider with a custom `base_url`.
    ///
    /// Trailing slashes are stripped automatically.
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let raw = base_url.into();
        let base_url = raw.trim_end_matches('/').to_owned();
        Self { api_key: api_key.into(), base_url, client: reqwest::Client::new() }
    }

    /// Construct by reading `ELEVENLABS_API_KEY` from the environment.
    ///
    /// # Errors
    ///
    /// Returns [`SttError::MissingEnv`] when the variable is absent or not
    /// valid Unicode.
    pub fn from_env() -> Result<Self, SttError> {
        Self::from_env_with_key_var(API_KEY_ENV)
    }

    /// Construct by reading a custom environment variable name.
    ///
    /// Useful in tests that want to check the error path without clobbering
    /// real credentials.
    pub fn from_env_with_key_var(var: &'static str) -> Result<Self, SttError> {
        let key = std::env::var(var).map_err(|source| SttError::MissingEnv { var, source })?;
        Ok(Self::new(key))
    }
}

// ── Wire response shapes ──────────────────────────────────────────────────────

/// Top-level response from `POST /v1/speech-to-text`.
#[derive(Debug, Deserialize)]
struct SttResponse {
    text: String,
    #[serde(default)]
    language_code: Option<String>,
    #[serde(default)]
    words: Vec<WordEntry>,
}

/// A single word-level timing entry from the ElevenLabs STT response.
#[derive(Debug, Deserialize)]
struct WordEntry {
    #[serde(rename = "type")]
    kind: String,
    text: String,
    #[serde(default)]
    start: f32,
    #[serde(default)]
    end: f32,
}

// ── Response parser (pub for potential unit tests) ────────────────────────────

/// Convert the raw ElevenLabs JSON response into a [`Transcript`].
///
/// Word entries of type `"word"` are mapped to [`TranscriptSegment`]s with
/// auto-incrementing IDs.  Punctuation entries (`"spacing"`, etc.) are
/// included in the full `text` but not emitted as separate segments.
pub fn parse_elevenlabs_response(payload: &Value) -> Result<Transcript, SttError> {
    let resp: SttResponse = serde_json::from_value(payload.clone())?;

    // Build segments from word-type entries only.
    let segments = resp
        .words
        .iter()
        .filter(|w| w.kind == "word")
        .enumerate()
        .map(|(i, w)| TranscriptSegment {
            id: i as u32,
            start: w.start,
            end: w.end,
            text: w.text.clone(),
        })
        .collect::<Vec<_>>();

    Ok(Transcript {
        text: resp.text,
        language: resp.language_code,
        // ElevenLabs does not return a total duration field; derive it from
        // the last word end time when words are present.
        duration_s: segments.last().map(|s| s.end),
        segments,
    })
}

// ── Helper: check HTTP status ─────────────────────────────────────────────────

async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, SttError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let code = status.as_u16();
    let body = resp.text().await.unwrap_or_else(|_| String::from("<unreadable body>"));
    Err(SttError::Api { status: code, body })
}

// ── Multipart builder ─────────────────────────────────────────────────────────

fn build_form(audio: Vec<u8>, opts: &SttOptions) -> reqwest::multipart::Form {
    let audio_part = reqwest::multipart::Part::bytes(audio)
        .file_name("audio.wav")
        .mime_str("application/octet-stream")
        .expect("static MIME string is always valid");

    let mut form = reqwest::multipart::Form::new()
        .part("audio", audio_part)
        .text("model_id", opts.model.clone());

    if let Some(lang) = &opts.language {
        form = form.text("language_code", lang.clone());
    }

    form
}

// ── SttProvider impl ──────────────────────────────────────────────────────────

#[async_trait]
impl SttProvider for ElevenLabsSttProvider {
    /// Transcribe `audio` by posting it to `POST /v1/speech-to-text`.
    ///
    /// # Errors
    ///
    /// - [`SttError::EmptyInput`] — `audio` is empty.
    /// - [`SttError::Transport`] — network or TLS failure.
    /// - [`SttError::Api`] — the API returned a non-2xx status.
    /// - [`SttError::Deserialise`] — response JSON could not be parsed.
    async fn transcribe(&self, audio: &[u8], opts: SttOptions) -> Result<Transcript, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyInput);
        }

        let url = format!("{}/v1/speech-to-text", self.base_url);
        let form = build_form(audio.to_vec(), &opts);

        let resp = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .multipart(form)
            .send()
            .await?;

        let resp = check_status(resp).await?;
        let payload: Value = resp.json().await?;
        parse_elevenlabs_response(&payload)
    }

    /// Accumulate the input stream and transcribe it as a single request.
    ///
    /// ElevenLabs STT has no streaming endpoint.  All chunks are buffered and
    /// forwarded as one batch upload.  A single final [`TranscriptDelta`] is
    /// emitted.
    async fn transcribe_stream(
        &self,
        audio: impl Stream<Item = Bytes> + Send + 'static,
        opts: SttOptions,
    ) -> Result<impl Stream<Item = Result<TranscriptDelta, SttError>> + Send, SttError> {
        use futures::StreamExt as _;

        let mut buf: Vec<u8> = Vec::new();
        futures::pin_mut!(audio);
        while let Some(chunk) = audio.next().await {
            buf.extend_from_slice(&chunk);
        }

        let transcript = self.transcribe(&buf, opts).await?;

        let delta = TranscriptDelta { text: transcript.text, is_final: true, segment_id: 0 };

        Ok(futures::stream::once(async move { Ok(delta) }))
    }
}
