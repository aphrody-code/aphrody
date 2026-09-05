// SPDX-License-Identifier: Apache-2.0
//! Speech-to-text (STT) abstractions for the aphrody CLI.
//!
//! # Overview
//!
//! This crate provides:
//! - [`SttProvider`] — the core async trait for STT providers (native only).
//! - [`WhisperApiProvider`] — production adapter for the OpenAI Whisper v1 REST API (native only).
//! - [`ElevenLabsSttProvider`] — production adapter for the ElevenLabs STT v1 REST API (native
//!   only).
//! - [`LocalWhisperBackend`] — optional, fully offline backend via `whisper.cpp` bindings (feature
//!   `local-whisper`, native only).
//! - [`web::WebSpeechRecognition`] — browser-native STT via `SpeechRecognition` (wasm32 only).
//!
//! # Feature flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `local-whisper` | off | Enables [`LocalWhisperBackend`]; requires `whisper-rs` and a C++ toolchain. |
//!
//! # Authentication
//!
//! Credentials are read from environment variables at construction time:
//! - [`WhisperApiProvider`]: `OPENAI_API_KEY`
//! - [`ElevenLabsSttProvider`]: `ELEVENLABS_API_KEY`
//!
//! # Relationship to `aphrody-voice`
//!
//! The companion crate `aphrody-voice` handles TTS (text-to-speech).  This
//! crate handles the reverse direction: audio in, text out.  The two crates are
//! intentionally separate so consumers can depend on only what they need.

#![forbid(unsafe_code)]

// Native-only modules: reqwest + tokio do not compile for wasm32.
#[cfg(not(target_arch = "wasm32"))] pub mod elevenlabs_stt;
#[cfg(not(target_arch = "wasm32"))] pub mod local_whisper;
#[cfg(not(target_arch = "wasm32"))] pub mod whisper_api;

// Browser-native module: only compiled when targeting wasm32.
#[cfg(target_arch = "wasm32")] pub mod web;

// ── Native-only: trait + error types ─────────────────────────────────────────
//
// All items below reference `reqwest` which is absent on wasm32 targets.
// The browser path uses `JsValue` errors directly in `web::WebSpeechRecognition`.

#[cfg(not(target_arch = "wasm32"))] use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))] use futures::Stream;
#[cfg(not(target_arch = "wasm32"))] use serde::{Deserialize, Serialize};

// ── SttError ──────────────────────────────────────────────────────────────────

/// Error type for all native STT provider operations.
///
/// Not available on `wasm32` targets — the browser path uses [`wasm_bindgen::JsValue`]
/// errors directly via [`web::WebSpeechRecognition`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, thiserror::Error)]
pub enum SttError {
    /// An HTTP-level transport error (connection reset, timeout, etc.).
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// The remote API returned a non-2xx status code.
    #[error("API error {status}: {body}")]
    Api { status: u16, body: String },

    /// Response payload could not be deserialised.
    #[error("deserialise error: {0}")]
    Deserialise(#[from] serde_json::Error),

    /// A required environment variable is absent or not valid Unicode.
    #[error("missing env var `{var}`: {source}")]
    MissingEnv { var: &'static str, source: std::env::VarError },

    /// Audio input bytes were empty or too short to be transcribed.
    #[error("audio input is empty")]
    EmptyInput,

    /// The operation is defined by the trait but not yet implemented by this
    /// backend.  See the module-level doc of [`local_whisper`] for the planned
    /// migration path.
    #[error("not implemented: {reason}")]
    NotImplemented { reason: &'static str },

    /// Any other error not covered by the variants above.
    #[error("stt error: {0}")]
    Other(String),
}

// ── TranscriptFormat ──────────────────────────────────────────────────────────

/// Requested output format for the transcript text.
///
/// Maps directly to the `response_format` parameter accepted by both the
/// OpenAI Whisper API and the ElevenLabs STT API.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptFormat {
    /// Plain UTF-8 text (default).
    Text,
    /// Verbose JSON including word-level timestamps and confidence scores.
    VerboseJson,
    /// SubRip subtitle format (`.srt`).
    Srt,
    /// WebVTT subtitle format (`.vtt`).
    Vtt,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for TranscriptFormat {
    fn default() -> Self {
        Self::Text
    }
}

// ── SttOptions ────────────────────────────────────────────────────────────────

/// Transcription tuning knobs forwarded verbatim to the provider.
///
/// All optional fields fall back to the provider's default when `None`.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttOptions {
    /// BCP-47 language tag for the source audio (e.g. `"en"`, `"fr"`).
    ///
    /// When `None` the provider auto-detects the language.  Supplying a known
    /// language improves accuracy and reduces latency.
    pub language: Option<String>,

    /// Provider model identifier.
    ///
    /// For OpenAI Whisper this is e.g. `"whisper-1"`.
    /// For ElevenLabs STT this is e.g. `"scribe_v1"`.
    pub model: String,

    /// Optional prompt to guide the transcription style or vocabulary.
    ///
    /// For OpenAI Whisper: a hint to the model (max 224 tokens).
    /// For ElevenLabs STT: passed as `context` when supported.
    pub prompt: Option<String>,

    /// Sampling temperature (0.0 – 1.0).
    ///
    /// Lower values produce more deterministic output.  `0.0` maps to greedy
    /// decoding.  Providers that do not support this knob silently ignore it.
    pub temperature: f32,

    /// Requested transcript output format.
    pub response_format: TranscriptFormat,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for SttOptions {
    fn default() -> Self {
        Self {
            language: None,
            model: String::from("whisper-1"),
            prompt: None,
            temperature: 0.0,
            response_format: TranscriptFormat::default(),
        }
    }
}

// ── TranscriptSegment ─────────────────────────────────────────────────────────

/// A timed segment within a full transcript.
///
/// Populated only when the provider returns verbose/timestamped output
/// (e.g. `response_format = VerboseJson`).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    /// Zero-based segment index as returned by the provider.
    pub id: u32,
    /// Segment start time in seconds from the beginning of the audio.
    pub start: f32,
    /// Segment end time in seconds from the beginning of the audio.
    pub end: f32,
    /// Transcribed text for this segment.
    pub text: String,
}

// ── Transcript ────────────────────────────────────────────────────────────────

/// Full transcript returned by a non-streaming [`SttProvider::transcribe`] call.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// The full transcribed text (concatenation of all segments).
    pub text: String,

    /// Detected or specified BCP-47 language tag (e.g. `"en"`).
    ///
    /// `None` when the provider did not return language information (e.g. when
    /// `response_format` is `Text`).
    pub language: Option<String>,

    /// Total audio duration in seconds as reported by the provider.
    ///
    /// `None` when the provider did not return duration information.
    pub duration_s: Option<f32>,

    /// Timed segments, if the provider returned detailed timing data.
    ///
    /// Empty when `response_format` is `Text` or the provider does not support
    /// word/segment-level timestamps.
    pub segments: Vec<TranscriptSegment>,
}

// ── TranscriptDelta ───────────────────────────────────────────────────────────

/// A partial transcript update emitted by a streaming transcription.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptDelta {
    /// The partial text for this delta.
    ///
    /// When `is_final` is `false` this may be revised in a future delta.
    pub text: String,

    /// `true` when this delta closes a confirmed segment that will not change.
    pub is_final: bool,

    /// Provider-assigned segment identifier.
    ///
    /// Allows the caller to correlate a final delta with earlier partial deltas
    /// for the same segment.
    pub segment_id: u32,
}

// ── SttProvider ───────────────────────────────────────────────────────────────

/// Async trait that every native STT backend must implement.
///
/// Implementations must be [`Send`] + [`Sync`] so they can be stored behind
/// `Arc<dyn SttProvider>` and shared across async tasks.
///
/// Not available on `wasm32` — use [`web::WebSpeechRecognition`] there.
///
/// # Buffered vs streaming
///
/// - [`transcribe`](SttProvider::transcribe) — buffers the full audio payload before uploading;
///   suitable for short clips and batch workflows.
/// - [`transcribe_stream`](SttProvider::transcribe_stream) — accepts an incoming byte stream (live
///   microphone, chunked download, etc.) and returns a stream of partial results.  Not all
///   providers support true streaming; adapters that do not may accumulate the full input and emit
///   a single final delta.
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait SttProvider: Send + Sync {
    /// Transcribe `audio` bytes and return the complete [`Transcript`].
    ///
    /// # Errors
    ///
    /// - [`SttError::EmptyInput`] — `audio` is empty.
    /// - [`SttError::Transport`] — network or TLS failure.
    /// - [`SttError::Api`] — the provider returned a non-2xx response.
    /// - [`SttError::Deserialise`] — the response body could not be parsed.
    async fn transcribe(&self, audio: &[u8], opts: SttOptions) -> Result<Transcript, SttError>;

    /// Transcribe a streaming audio source and return an async stream of
    /// [`TranscriptDelta`] updates.
    ///
    /// The `audio` input stream yields [`bytes::Bytes`] chunks as they become
    /// available.  Implementations accumulate chunks internally and call the
    /// provider when enough data is buffered (or at stream end).
    ///
    /// # Errors
    ///
    /// Returns `Err(SttError)` when the provider rejects the request before
    /// any deltas are produced.  Errors that occur mid-stream are emitted as
    /// `Err` items inside the returned stream.
    async fn transcribe_stream(
        &self,
        audio: impl Stream<Item = bytes::Bytes> + Send + 'static,
        opts: SttOptions,
    ) -> Result<impl Stream<Item = Result<TranscriptDelta, SttError>> + Send, SttError>;
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use serde_json::json;

    use super::*;

    // ── SttOptions defaults ───────────────────────────────────────────────────

    #[test]
    fn stt_options_defaults() {
        let opts = SttOptions::default();
        assert!(opts.language.is_none());
        assert_eq!(opts.model, "whisper-1");
        assert!(opts.prompt.is_none());
        assert_eq!(opts.temperature, 0.0_f32);
        assert_eq!(opts.response_format, TranscriptFormat::Text);
    }

    // ── SttOptions serde round-trip ───────────────────────────────────────────

    #[test]
    fn stt_options_round_trip_all_fields() {
        let original = SttOptions {
            language: Some("fr".to_owned()),
            model: "whisper-1".to_owned(),
            prompt: Some("Bonjour".to_owned()),
            temperature: 0.2,
            response_format: TranscriptFormat::VerboseJson,
        };
        let serialised = serde_json::to_string(&original).expect("serialise");
        let restored: SttOptions = serde_json::from_str(&serialised).expect("deserialise");
        assert_eq!(restored.language.as_deref(), Some("fr"));
        assert_eq!(restored.model, "whisper-1");
        assert_eq!(restored.prompt.as_deref(), Some("Bonjour"));
        assert!((restored.temperature - 0.2_f32).abs() < f32::EPSILON);
        assert_eq!(restored.response_format, TranscriptFormat::VerboseJson);
    }

    #[test]
    fn stt_options_round_trip_defaults() {
        let original = SttOptions::default();
        let serialised = serde_json::to_string(&original).expect("serialise");
        let restored: SttOptions = serde_json::from_str(&serialised).expect("deserialise");
        assert!(restored.language.is_none());
        assert!(restored.prompt.is_none());
        assert_eq!(restored.response_format, TranscriptFormat::Text);
    }

    // ── Transcript serde round-trip ───────────────────────────────────────────

    #[test]
    fn transcript_round_trip_full() {
        let original = Transcript {
            text: "Hello world.".to_owned(),
            language: Some("en".to_owned()),
            duration_s: Some(1.23),
            segments: vec![TranscriptSegment {
                id: 0,
                start: 0.0,
                end: 1.23,
                text: "Hello world.".to_owned(),
            }],
        };
        let serialised = serde_json::to_string(&original).expect("serialise");
        let restored: Transcript = serde_json::from_str(&serialised).expect("deserialise");
        assert_eq!(restored.text, "Hello world.");
        assert_eq!(restored.language.as_deref(), Some("en"));
        assert!((restored.duration_s.unwrap() - 1.23_f32).abs() < 1e-4);
        assert_eq!(restored.segments.len(), 1);
        assert_eq!(restored.segments[0].text, "Hello world.");
    }

    #[test]
    fn transcript_round_trip_minimal() {
        let original = Transcript {
            text: "Ok.".to_owned(),
            language: None,
            duration_s: None,
            segments: vec![],
        };
        let serialised = serde_json::to_string(&original).expect("serialise");
        let restored: Transcript = serde_json::from_str(&serialised).expect("deserialise");
        assert_eq!(restored.text, "Ok.");
        assert!(restored.language.is_none());
        assert!(restored.duration_s.is_none());
        assert!(restored.segments.is_empty());
    }

    // ── TranscriptDelta serde round-trip ─────────────────────────────────────

    #[test]
    fn transcript_delta_round_trip() {
        let original =
            TranscriptDelta { text: "partial...".to_owned(), is_final: false, segment_id: 3 };
        let serialised = serde_json::to_string(&original).expect("serialise");
        let restored: TranscriptDelta = serde_json::from_str(&serialised).expect("deserialise");
        assert_eq!(restored.text, "partial...");
        assert!(!restored.is_final);
        assert_eq!(restored.segment_id, 3);
    }

    // ── TranscriptFormat serde ────────────────────────────────────────────────

    #[test]
    fn transcript_format_serde_all_variants() {
        let cases = [
            (TranscriptFormat::Text, "\"text\""),
            (TranscriptFormat::VerboseJson, "\"verbose_json\""),
            (TranscriptFormat::Srt, "\"srt\""),
            (TranscriptFormat::Vtt, "\"vtt\""),
        ];
        for (variant, expected_json) in &cases {
            let serialised = serde_json::to_string(variant).expect("serialise");
            assert_eq!(&serialised, expected_json);
            let restored: TranscriptFormat =
                serde_json::from_str(&serialised).expect("deserialise");
            assert_eq!(&restored, variant);
        }
    }

    // ── SttError env-missing path ─────────────────────────────────────────────

    #[test]
    fn stt_error_missing_env_from_provider() {
        // Verify that WhisperApiProvider::from_env surfaces MissingEnv when the
        // key is absent.  Uses a deliberately non-existent var name to avoid
        // contaminating the real environment.
        let key = "APHRODY_TEST_NONEXISTENT_KEY_WHISPER_12345";
        // Safety: guaranteed absent in CI; test is single-threaded by default.
        let result = std::env::var(key);
        assert!(result.is_err(), "test assumes var is absent");

        let err = crate::stt::whisper_api::WhisperApiProvider::from_env_with_key_var(key)
            .expect_err("should fail with missing env");
        assert!(matches!(err, SttError::MissingEnv { .. }), "unexpected error variant: {err}");
    }

    #[test]
    fn stt_error_missing_env_elevenlabs() {
        let key = "APHRODY_TEST_NONEXISTENT_KEY_ELEVENLABS_12345";
        let result = std::env::var(key);
        assert!(result.is_err(), "test assumes var is absent");

        let err = crate::stt::elevenlabs_stt::ElevenLabsSttProvider::from_env_with_key_var(key)
            .expect_err("should fail with missing env");
        assert!(matches!(err, SttError::MissingEnv { .. }), "unexpected error variant: {err}");
    }

    // ── Verbose JSON payload parsing ──────────────────────────────────────────

    /// Exercises [`whisper_api::parse_verbose_response`] with a payload shaped
    /// after the real OpenAI Whisper verbose_json response.  No network call.
    #[test]
    fn whisper_verbose_json_parse() {
        let payload = json!({
            "text": "Hello world.",
            "language": "english",
            "duration": 1.23,
            "segments": [
                {
                    "id": 0,
                    "start": 0.0,
                    "end": 1.23,
                    "text": " Hello world."
                }
            ]
        });
        let transcript =
            crate::stt::whisper_api::parse_verbose_response(&payload).expect("parse verbose response");
        assert_eq!(transcript.text, "Hello world.");
        assert_eq!(transcript.language.as_deref(), Some("english"));
        assert!((transcript.duration_s.unwrap() - 1.23_f32).abs() < 1e-4);
        assert_eq!(transcript.segments.len(), 1);
        assert_eq!(transcript.segments[0].text, " Hello world.");
    }

    /// A plain-text response (response_format = "text") produces a minimal
    /// Transcript with no language / duration / segments.
    #[test]
    fn whisper_text_response_parse() {
        let transcript = crate::stt::whisper_api::parse_text_response("Hello world.\n");
        assert_eq!(transcript.text, "Hello world.");
        assert!(transcript.language.is_none());
        assert!(transcript.duration_s.is_none());
        assert!(transcript.segments.is_empty());
    }
}
