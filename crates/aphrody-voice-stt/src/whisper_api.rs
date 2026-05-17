// SPDX-License-Identifier: Apache-2.0
//! OpenAI Whisper STT adapter.
//!
//! Implements [`SttProvider`] against the OpenAI audio transcription endpoint:
//! - `POST /v1/audio/transcriptions` — multipart/form-data upload.
//!
//! # Authentication
//!
//! Pass the API key via [`WhisperApiProvider::new`] or read it from the
//! `OPENAI_API_KEY` environment variable via [`WhisperApiProvider::from_env`].
//! The key is sent as `Authorization: Bearer <key>` on every request.
//!
//! # Wire format
//!
//! The request is `multipart/form-data` with the following fields:
//! - `file` — the audio bytes with a synthetic filename (`audio.wav`).
//! - `model` — e.g. `"whisper-1"`.
//! - `response_format` — one of `text`, `verbose_json`, `srt`, `vtt`.
//! - `language` — BCP-47 tag, omitted when `None`.
//! - `prompt` — optional context hint, omitted when `None`.
//! - `temperature` — only forwarded when `> 0.0`.
//!
//! # Streaming
//!
//! The Whisper REST API does not support true token-level streaming.
//! [`SttProvider::transcribe_stream`] accumulates the input stream in memory,
//! calls the buffered endpoint, and emits a single final [`TranscriptDelta`].

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use serde_json::Value;

use crate::{
    SttError, SttOptions, SttProvider, Transcript, TranscriptDelta, TranscriptFormat,
    TranscriptSegment,
};

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL: &str = "https://api.openai.com";
/// Environment variable consulted by [`WhisperApiProvider::from_env`].
pub const API_KEY_ENV: &str = "OPENAI_API_KEY";

// ── WhisperApiProvider ────────────────────────────────────────────────────────

/// STT provider backed by the OpenAI Whisper REST API.
///
/// Clone is cheap: the inner [`reqwest::Client`] is `Arc`-wrapped.
#[derive(Debug, Clone)]
pub struct WhisperApiProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl WhisperApiProvider {
    /// Construct a provider using the OpenAI production endpoint.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    /// Construct a provider with a custom `base_url`.
    ///
    /// Useful for testing against a mock server or a compatible proxy.
    /// Trailing slashes are stripped automatically.
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let raw = base_url.into();
        let base_url = raw.trim_end_matches('/').to_owned();
        Self { api_key: api_key.into(), base_url, client: reqwest::Client::new() }
    }

    /// Construct by reading `OPENAI_API_KEY` from the environment.
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

// ── Response parsers (pub for unit tests in lib.rs) ───────────────────────────

/// Parse an OpenAI Whisper `verbose_json` response payload into a [`Transcript`].
///
/// Unknown or malformed segment entries are silently skipped.
pub fn parse_verbose_response(payload: &Value) -> Result<Transcript, SttError> {
    let text = payload.get("text").and_then(Value::as_str).unwrap_or("").to_owned();

    let language = payload.get("language").and_then(Value::as_str).map(str::to_owned);

    let duration_s = payload.get("duration").and_then(Value::as_f64).map(|d| d as f32);

    let segments = payload
        .get("segments")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(parse_segment).collect::<Vec<_>>())
        .unwrap_or_default();

    Ok(Transcript { text, language, duration_s, segments })
}

/// Parse a plain-text response (response_format = "text") into a [`Transcript`].
///
/// Trims trailing whitespace / newlines; language, duration, and segments are
/// left empty because the plain-text format carries no metadata.
pub fn parse_text_response(body: &str) -> Transcript {
    Transcript {
        text: body.trim_end().to_owned(),
        language: None,
        duration_s: None,
        segments: vec![],
    }
}

/// Parse a single segment object from the verbose_json `segments` array.
fn parse_segment(value: &Value) -> Option<TranscriptSegment> {
    let obj = value.as_object()?;
    let id = obj.get("id").and_then(Value::as_u64)? as u32;
    let start = obj.get("start").and_then(Value::as_f64)? as f32;
    let end = obj.get("end").and_then(Value::as_f64)? as f32;
    let text = obj.get("text").and_then(Value::as_str)?.to_owned();
    Some(TranscriptSegment { id, start, end, text })
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

/// Build the multipart form for a transcription request.
fn build_form(audio: Vec<u8>, opts: &SttOptions) -> reqwest::multipart::Form {
    let format_str = match opts.response_format {
        TranscriptFormat::Text => "text",
        TranscriptFormat::VerboseJson => "verbose_json",
        TranscriptFormat::Srt => "srt",
        TranscriptFormat::Vtt => "vtt",
    };

    // The API infers the codec from the filename extension; use `.wav` as a
    // safe default that is always accepted.  Callers that supply different
    // formats (MP3, FLAC, OGG, etc.) should note that the current API does
    // not use the extension for codec detection — it relies on the file
    // magic bytes — so `audio.wav` is harmless for all supported formats.
    let audio_part = reqwest::multipart::Part::bytes(audio)
        .file_name("audio.wav")
        .mime_str("application/octet-stream")
        .expect("static MIME string is always valid");

    let mut form = reqwest::multipart::Form::new()
        .part("file", audio_part)
        .text("model", opts.model.clone())
        .text("response_format", format_str);

    if let Some(lang) = &opts.language {
        form = form.text("language", lang.clone());
    }
    if let Some(prompt) = &opts.prompt {
        form = form.text("prompt", prompt.clone());
    }
    if opts.temperature > 0.0 {
        form = form.text("temperature", opts.temperature.to_string());
    }

    form
}

// ── SttProvider impl ──────────────────────────────────────────────────────────

#[async_trait]
impl SttProvider for WhisperApiProvider {
    /// Transcribe `audio` by posting it to `POST /v1/audio/transcriptions`.
    ///
    /// # Errors
    ///
    /// - [`SttError::EmptyInput`] — `audio` is empty.
    /// - [`SttError::Transport`] — network or TLS failure.
    /// - [`SttError::Api`] — the API returned a non-2xx status.
    /// - [`SttError::Deserialise`] — response JSON could not be parsed (only when `response_format`
    ///   is `VerboseJson`).
    async fn transcribe(&self, audio: &[u8], opts: SttOptions) -> Result<Transcript, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyInput);
        }

        let url = format!("{}/v1/audio/transcriptions", self.base_url);
        let form = build_form(audio.to_vec(), &opts);

        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        let resp = check_status(resp).await?;

        match opts.response_format {
            TranscriptFormat::VerboseJson => {
                let payload: Value = resp.json().await?;
                parse_verbose_response(&payload)
            },
            // text / srt / vtt: all return a UTF-8 body; treat as plain text.
            _ => {
                let body = resp.text().await.map_err(SttError::Transport)?;
                Ok(parse_text_response(&body))
            },
        }
    }

    /// Accumulate the input stream and transcribe it as a single request.
    ///
    /// The Whisper REST API has no true streaming endpoint.  This
    /// implementation collects all chunks, calls [`transcribe`](Self::transcribe),
    /// and emits one final [`TranscriptDelta`].
    async fn transcribe_stream(
        &self,
        audio: impl Stream<Item = Bytes> + Send + 'static,
        opts: SttOptions,
    ) -> Result<impl Stream<Item = Result<TranscriptDelta, SttError>> + Send, SttError> {
        use futures::StreamExt as _;

        // Collect all chunks into a single buffer.
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
