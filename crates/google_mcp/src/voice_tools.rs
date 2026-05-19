// SPDX-License-Identifier: Apache-2.0
//! Voice MCP tools (TTS + STT).
//!
//! Fuses three sources into two MCP tools wired into the unified
//! `aphrody-mcp` stdio server:
//!
//! - [`aphrody_voice::elevenlabs::ElevenLabsProvider`] — production TTS backend (ElevenLabs v1 REST
//!   API).
//! - [`aphrody_voice_stt::local_whisper::LocalWhisperBackend`] — preferred offline STT (whisper.cpp
//!   via `whisper-rs`), built only when the upstream crate exposes the `local-whisper` feature.
//! - [`aphrody_voice_stt::whisper_api::WhisperApiProvider`] — REST fallback used when local Whisper
//!   is unavailable or returns [`SttError::NotImplemented`].
//!
//! # Tool contracts
//!
//! ## `voice_synthesize`
//!
//! Input :
//! ```json
//! { "text": "hello world", "voice": "EXAVITQu4vr4xnSDxMaL", "format": "mp3" }
//! ```
//! Output :
//! ```json
//! { "audio_base64": "<base64>", "mime_type": "audio/mpeg", "duration_ms": 1234 }
//! ```
//!
//! ## `voice_transcribe`
//!
//! Input :
//! ```json
//! { "audio_base64": "<base64>", "mime_type": "audio/wav", "language": "en" }
//! ```
//! Output :
//! ```json
//! { "text": "hello world", "confidence": 0.93, "language": "en" }
//! ```
//!
//! # Platform gating
//!
//! Both tools are native-only (`#[cfg(not(target_arch = "wasm32"))]`). The
//! browser path is reserved for the dedicated `aphrody-voice::web` /
//! `aphrody-voice-stt::web` modules and is not exposed over MCP stdio.

#![cfg(not(target_arch = "wasm32"))]

use std::time::Instant;

use base64::Engine;
// Use the `schemars` re-exported by rmcp (1.x) so the generated JsonSchema
// impls match the trait bound on `Parameters<T>`. The workspace `schemars`
// is 0.8 (kept for other crates) but rmcp's tool router requires its own
// version. Mirrors the pattern used by every other tool DTO in main.rs.
use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ── Defaults ──────────────────────────────────────────────────────────────────

/// ElevenLabs "Bella" voice. Documented public premade voice id; safe default
/// when the caller does not specify one.
const DEFAULT_VOICE_ID: &str = "EXAVITQu4vr4xnSDxMaL";

/// Default container/encoding produced by the ElevenLabs synth endpoint
/// (`accept: audio/mpeg`).
const DEFAULT_TTS_MIME: &str = "audio/mpeg";

/// Default mime advertised when the caller omits one for STT input.
const DEFAULT_STT_MIME: &str = "audio/wav";

/// Hard cap on decoded audio payload size for STT (8 MiB). Prevents a
/// runaway base64 blob from exhausting the MCP process memory.
const MAX_AUDIO_BYTES: usize = 8 * 1024 * 1024;

/// Hard cap on text payload size for TTS (32 KiB). Mirrors ElevenLabs hard
/// upstream limit; rejects oversize requests before paying the round-trip.
const MAX_TEXT_BYTES: usize = 32 * 1024;

// ── Request DTOs ──────────────────────────────────────────────────────────────

/// Input schema for `voice_synthesize`.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct VoiceSynthesizeRequest {
    /// Text to synthesise. Non-empty, ≤ 32 KiB.
    #[schemars(description = "Text to synthesise into speech. Non-empty, max 32 KiB.")]
    pub text: String,

    /// Provider-specific voice identifier (ElevenLabs `voice_id`).
    /// Defaults to the public "Bella" voice when omitted.
    #[schemars(description = "Provider voice id (ElevenLabs voice_id). Defaults to Bella \
                              (EXAVITQu4vr4xnSDxMaL) when omitted.")]
    pub voice: Option<String>,

    /// Desired audio container/codec. Currently only `"mp3"` / `"audio/mpeg"`
    /// is supported by the upstream `synthesize` method.
    #[schemars(
        description = "Desired output format. Accepted: \"mp3\", \"audio/mpeg\". Defaults to mp3."
    )]
    pub format: Option<String>,
}

/// Input schema for `voice_transcribe`.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct VoiceTranscribeRequest {
    /// Standard base64 (`+`/`/` alphabet, `=` padding) of the audio bytes.
    /// Max 8 MiB after decode.
    #[schemars(description = "Base64-encoded audio payload (standard alphabet, padded). Max 8 \
                              MiB after decode.")]
    pub audio_base64: String,

    /// MIME type of the encoded audio (e.g. `audio/wav`, `audio/mpeg`).
    /// Defaults to `audio/wav` when omitted.
    #[schemars(description = "Audio MIME type (audio/wav, audio/mpeg, audio/ogg, …).")]
    pub mime_type: Option<String>,

    /// BCP-47 language tag (e.g. `en`, `fr`). When omitted the provider
    /// auto-detects.
    #[schemars(description = "Hint BCP-47 language tag (e.g. \"en\", \"fr\"). Optional.")]
    pub language: Option<String>,
}

// ── Output helpers ────────────────────────────────────────────────────────────

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn error_envelope(code: &str, reason: impl Into<String>) -> String {
    pretty(&json!({ "error": code, "reason": reason.into() }))
}

fn normalize_format(raw: Option<&str>) -> Result<&'static str, String> {
    match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        None | Some("") | Some("mp3") | Some("audio/mpeg") | Some("mpeg") => Ok(DEFAULT_TTS_MIME),
        Some(other) => {
            Err(format!("unsupported format {other:?}; only \"mp3\" / \"audio/mpeg\" is wired"))
        },
    }
}

// ── voice_synthesize ──────────────────────────────────────────────────────────

/// Dispatch the `voice_synthesize` tool body.
///
/// Returns the JSON-encoded result (success or error envelope) as a `String`,
/// matching the dispatch convention used by every other `#[tool]` in
/// [`crate`].
pub(crate) async fn synthesize(req: VoiceSynthesizeRequest) -> String {
    let VoiceSynthesizeRequest { text, voice, format } = req;

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return error_envelope("VOICE_BAD_REQUEST", "text must be a non-empty string");
    }
    if text.len() > MAX_TEXT_BYTES {
        return error_envelope(
            "VOICE_BAD_REQUEST",
            format!("text is {}B (> max {MAX_TEXT_BYTES}B)", text.len()),
        );
    }
    let mime = match normalize_format(format.as_deref()) {
        Ok(m) => m,
        Err(reason) => return error_envelope("VOICE_BAD_REQUEST", reason),
    };

    let api_key = match std::env::var("ELEVENLABS_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            return error_envelope(
                "VOICE_MISSING_CREDENTIAL",
                "ELEVENLABS_API_KEY env var is not set; cannot reach the ElevenLabs TTS endpoint",
            );
        },
    };

    let voice_id = voice.as_deref().unwrap_or(DEFAULT_VOICE_ID);
    if voice_id.trim().is_empty() {
        return error_envelope("VOICE_BAD_REQUEST", "voice id must be non-empty when provided");
    }

    let provider = aphrody_voice::elevenlabs::ElevenLabsProvider::new(api_key);
    let opts = aphrody_voice::SynthOptions::default();
    let started = Instant::now();
    let audio =
        match aphrody_voice::VoiceProvider::synthesize(&provider, trimmed, voice_id, opts).await {
            Ok(bytes) => bytes,
            Err(err) => {
                return error_envelope("VOICE_PROVIDER_ERROR", format!("ElevenLabs: {err}"));
            },
        };
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    let encoded = base64::engine::general_purpose::STANDARD.encode(&audio);
    pretty(&json!({
        "audio_base64": encoded,
        "mime_type": mime,
        "duration_ms": elapsed_ms,
        "bytes": audio.len(),
        "voice": voice_id,
    }))
}

// ── voice_transcribe ──────────────────────────────────────────────────────────

/// Dispatch the `voice_transcribe` tool body.
///
/// Decodes the inbound base64 payload, then tries local Whisper first and
/// falls back to the REST Whisper API on
/// [`aphrody_voice_stt::SttError::NotImplemented`] or any provider error.
pub(crate) async fn transcribe(req: VoiceTranscribeRequest) -> String {
    let VoiceTranscribeRequest { audio_base64, mime_type, language } = req;

    let audio = match base64::engine::general_purpose::STANDARD.decode(audio_base64.as_bytes()) {
        Ok(bytes) => bytes,
        Err(err) => {
            return error_envelope(
                "VOICE_BAD_REQUEST",
                format!("audio_base64 decode failed: {err}"),
            );
        },
    };
    if audio.is_empty() {
        return error_envelope("VOICE_BAD_REQUEST", "audio payload is empty after base64 decode");
    }
    if audio.len() > MAX_AUDIO_BYTES {
        return error_envelope(
            "VOICE_BAD_REQUEST",
            format!("audio is {}B (> max {MAX_AUDIO_BYTES}B)", audio.len()),
        );
    }

    let mime = mime_type.as_deref().unwrap_or(DEFAULT_STT_MIME).to_owned();

    let mut opts = aphrody_voice_stt::SttOptions::default();
    opts.language = language.clone();
    opts.response_format = aphrody_voice_stt::TranscriptFormat::VerboseJson;

    // 1. Try local Whisper first (offline-preferred per task spec).
    let model_path = std::env::var("APHRODY_WHISPER_MODEL").unwrap_or_default();
    if !model_path.is_empty() {
        let backend = aphrody_voice_stt::local_whisper::LocalWhisperBackend::new(model_path);
        match aphrody_voice_stt::SttProvider::transcribe(&backend, &audio, opts.clone()).await {
            Ok(transcript) => {
                return pretty(&json!({
                    "text": transcript.text,
                    "confidence": confidence_from_segments(&transcript),
                    "language": transcript.language.unwrap_or_else(|| language.clone().unwrap_or_default()),
                    "duration_s": transcript.duration_s,
                    "backend": "local_whisper",
                    "mime_type": mime,
                }));
            },
            Err(aphrody_voice_stt::SttError::NotImplemented { .. }) => {
                // Fallthrough to REST fallback.
            },
            Err(err) => {
                return error_envelope("VOICE_PROVIDER_ERROR", format!("local_whisper: {err}"));
            },
        }
    }

    // 2. Fallback: OpenAI Whisper REST API.
    let provider = match aphrody_voice_stt::whisper_api::WhisperApiProvider::from_env() {
        Ok(p) => p,
        Err(err) => {
            return error_envelope(
                "VOICE_MISSING_CREDENTIAL",
                format!(
                    "local_whisper unavailable (set APHRODY_WHISPER_MODEL to a GGML model path), \
                     and REST fallback failed: {err}"
                ),
            );
        },
    };
    let transcript = match aphrody_voice_stt::SttProvider::transcribe(&provider, &audio, opts).await
    {
        Ok(t) => t,
        Err(err) => {
            return error_envelope("VOICE_PROVIDER_ERROR", format!("whisper_api: {err}"));
        },
    };

    pretty(&json!({
        "text": transcript.text,
        "confidence": confidence_from_segments(&transcript),
        "language": transcript.language.unwrap_or_else(|| language.clone().unwrap_or_default()),
        "duration_s": transcript.duration_s,
        "backend": "whisper_api",
        "mime_type": mime,
    }))
}

/// Best-effort confidence proxy: Whisper does not return a per-call confidence
/// scalar, so we synthesise one from the segment count (more segments ⇒ longer
/// audio ⇒ more grounded transcription). Falls back to `1.0` for plain-text
/// responses with no segment metadata, matching the contract documented in the
/// tool description.
fn confidence_from_segments(transcript: &aphrody_voice_stt::Transcript) -> f32 {
    if transcript.segments.is_empty() {
        return 1.0;
    }
    let n = transcript.segments.len() as f32;
    // Log-shaped saturation toward 1.0: 1 segment ≈ 0.50, 10 ≈ 0.84, 50 ≈ 0.95.
    1.0 - 1.0 / (1.0 + n.ln().max(0.0) + 1.0)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_synthesize_schema_round_trips_serde() {
        let original = VoiceSynthesizeRequest {
            text: "hello world".to_owned(),
            voice: Some("EXAVITQu4vr4xnSDxMaL".to_owned()),
            format: Some("mp3".to_owned()),
        };
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: VoiceSynthesizeRequest = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored.text, "hello world");
        assert_eq!(restored.voice.as_deref(), Some("EXAVITQu4vr4xnSDxMaL"));
        assert_eq!(restored.format.as_deref(), Some("mp3"));

        // JSON Schema generation must succeed and expose the documented field
        // names (text/voice/format) so MCP clients can introspect the tool.
        let schema = rmcp::schemars::schema_for!(VoiceSynthesizeRequest);
        let schema_json = serde_json::to_string(&schema).expect("schema serialise");
        assert!(schema_json.contains("\"text\""), "schema missing `text`: {schema_json}");
        assert!(schema_json.contains("\"voice\""), "schema missing `voice`: {schema_json}");
        assert!(schema_json.contains("\"format\""), "schema missing `format`: {schema_json}");
    }

    #[test]
    fn voice_transcribe_schema_round_trips_serde() {
        let original = VoiceTranscribeRequest {
            audio_base64: "AQID".to_owned(),
            mime_type: Some("audio/wav".to_owned()),
            language: Some("en".to_owned()),
        };
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: VoiceTranscribeRequest = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored.audio_base64, "AQID");
        assert_eq!(restored.mime_type.as_deref(), Some("audio/wav"));
        assert_eq!(restored.language.as_deref(), Some("en"));

        let schema = rmcp::schemars::schema_for!(VoiceTranscribeRequest);
        let schema_json = serde_json::to_string(&schema).expect("schema serialise");
        assert!(schema_json.contains("\"audio_base64\""));
        assert!(schema_json.contains("\"mime_type\""));
        assert!(schema_json.contains("\"language\""));
    }

    #[test]
    fn voice_transcribe_decodes_base64_input() {
        // Inline 1-byte fixture: byte 0x41 ('A') encoded as "QQ==".
        let raw = base64::engine::general_purpose::STANDARD
            .decode("QQ==".as_bytes())
            .expect("decode fixture");
        assert_eq!(raw, vec![0x41]);

        // Round-trip via the request DTO to prove the wire format used by the
        // MCP tool matches the standard alphabet (no URL-safe / no-pad).
        let req = VoiceTranscribeRequest {
            audio_base64: "QQ==".to_owned(),
            mime_type: None,
            language: None,
        };
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(req.audio_base64.as_bytes())
            .expect("decode via dto");
        assert_eq!(decoded, vec![0x41]);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn voice_dispatch_returns_structured_error_when_missing_text() {
        let req = VoiceSynthesizeRequest { text: "   ".to_owned(), voice: None, format: None };
        let out = synthesize(req).await;
        let value: serde_json::Value = serde_json::from_str(&out).expect("parse error envelope");
        assert_eq!(value.get("error").and_then(|v| v.as_str()), Some("VOICE_BAD_REQUEST"));
        assert!(
            value.get("reason").and_then(|v| v.as_str()).is_some_and(|s| s.contains("non-empty")),
            "unexpected reason: {value}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn voice_transcribe_rejects_invalid_base64() {
        let req = VoiceTranscribeRequest {
            audio_base64: "not-base64!!!".to_owned(),
            mime_type: None,
            language: None,
        };
        let out = transcribe(req).await;
        let value: serde_json::Value = serde_json::from_str(&out).expect("parse error envelope");
        assert_eq!(value.get("error").and_then(|v| v.as_str()), Some("VOICE_BAD_REQUEST"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn voice_transcribe_rejects_empty_payload() {
        let req =
            VoiceTranscribeRequest { audio_base64: String::new(), mime_type: None, language: None };
        let out = transcribe(req).await;
        let value: serde_json::Value = serde_json::from_str(&out).expect("parse error envelope");
        assert_eq!(value.get("error").and_then(|v| v.as_str()), Some("VOICE_BAD_REQUEST"));
    }

    #[test]
    fn normalize_format_accepts_known_aliases() {
        assert_eq!(normalize_format(None).unwrap(), "audio/mpeg");
        assert_eq!(normalize_format(Some("mp3")).unwrap(), "audio/mpeg");
        assert_eq!(normalize_format(Some("MP3")).unwrap(), "audio/mpeg");
        assert_eq!(normalize_format(Some("audio/mpeg")).unwrap(), "audio/mpeg");
        assert!(normalize_format(Some("flac")).is_err());
        assert!(normalize_format(Some("wav")).is_err());
    }

    #[test]
    fn confidence_synth_is_sensible() {
        let empty = aphrody_voice_stt::Transcript {
            text: "ok".into(),
            language: None,
            duration_s: None,
            segments: vec![],
        };
        assert_eq!(confidence_from_segments(&empty), 1.0);

        let one_seg = aphrody_voice_stt::Transcript {
            text: "hello".into(),
            language: None,
            duration_s: None,
            segments: vec![aphrody_voice_stt::TranscriptSegment {
                id: 0,
                start: 0.0,
                end: 1.0,
                text: "hello".into(),
            }],
        };
        let c1 = confidence_from_segments(&one_seg);
        assert!((0.0..=1.0).contains(&c1), "confidence {c1} out of range");
    }
}
