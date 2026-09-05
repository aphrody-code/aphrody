// SPDX-License-Identifier: Apache-2.0
//! Voice I/O abstractions for the aphrody CLI (TTS + STT).
//!
//! # Overview
//!
//! This crate provides:
//! - [`VoiceProvider`] — the core async trait for TTS providers (native only).
//! - [`ElevenLabsProvider`] — production adapter for the ElevenLabs v1 REST API (native only).
//! - [`DiscordVoiceShim`] — thin HTTP forwarder that delegates synthesised audio to the openclaw
//!   Gateway endpoint (`OPENCLAW_GATEWAY_URL`) (native only).
//! - [`web::WebSpeechSynth`] — browser-native TTS via `window.speechSynthesis` (wasm32 only).
//! - [`stt`] — speech-to-text (STT) module: [`stt::SttProvider`] trait, OpenAI Whisper, ElevenLabs
//!   STT, optional local Whisper backend, and browser-native `SpeechRecognition`.
//!
//! # Module layout
//!
//! The `stt` submodule was formerly published as `aphrody-voice-stt`. It is now
//! unified here so consumers depend on a single crate for all voice I/O.

#![forbid(unsafe_code)]

// Native-only modules: reqwest + tokio do not compile for wasm32.
#[cfg(not(target_arch = "wasm32"))] pub mod discord_shim;
#[cfg(not(target_arch = "wasm32"))] pub mod elevenlabs;

// Browser-native module: only compiled when targeting wasm32.
#[cfg(target_arch = "wasm32")] pub mod web;

// Speech-to-text (STT) — unified from the former `aphrody-voice-stt` crate.
pub mod stt;

// ── Native-only: trait + error types ─────────────────────────────────────────
//
// All items below reference `reqwest` which is absent on wasm32 targets.
// The browser path uses `JsValue` errors directly in `web::WebSpeechSynth`.

#[cfg(not(target_arch = "wasm32"))] use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))] use bytes::Bytes;
#[cfg(not(target_arch = "wasm32"))] use futures::Stream;
#[cfg(not(target_arch = "wasm32"))] use serde::{Deserialize, Serialize};

/// Error type for all native voice provider operations.
///
/// Not available on `wasm32` targets — the browser path uses [`wasm_bindgen::JsValue`]
/// errors directly via [`web::WebSpeechSynth`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    /// An HTTP-level transport error (connection reset, timeout, …).
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// The remote API returned a non-2xx status code.
    #[error("API error {status}: {body}")]
    Api { status: u16, body: String },

    /// Response payload could not be deserialised.
    #[error("deserialise error: {0}")]
    Deserialise(#[from] serde_json::Error),

    /// A required environment variable (`OPENCLAW_GATEWAY_URL`, …) is absent.
    #[error("missing env var `{var}`: {source}")]
    MissingEnv { var: &'static str, source: std::env::VarError },

    /// Any other error not covered by the variants above.
    #[error("voice error: {0}")]
    Other(String),
}

// ── VoiceInfo ─────────────────────────────────────────────────────────────────

/// Metadata returned by [`VoiceProvider::list_voices`].
///
/// Field names mirror the ElevenLabs `/v1/voices` JSON shape so that
/// `serde_json::from_value` works directly on the raw API response.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    /// Provider-internal voice identifier (e.g. `"EXAVITQu4vr4xnSDxMaL"`).
    pub id: String,
    /// Human-readable display name (e.g. `"Bella"`).
    pub name: String,
    /// BCP-47 language/locale tag if known (e.g. `"en-US"`).
    pub language: Option<String>,
    /// Reported gender label (`"male"`, `"female"`, or provider-specific).
    pub gender: Option<String>,
    /// URL to a short preview audio clip, if provided by the API.
    pub preview_url: Option<String>,
}

// ── SynthOptions ──────────────────────────────────────────────────────────────

/// Synthesis tuning knobs forwarded verbatim to the provider.
///
/// All fields are optional at the call site; each provider falls back to its
/// own defaults when `None`.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthOptions {
    /// Voice stability (0.0 – 1.0).  Higher = more consistent but less
    /// expressive.  ElevenLabs default: `0.5`.
    pub stability: Option<f32>,

    /// Similarity boost / speaker similarity (0.0 – 1.0).
    /// ElevenLabs default: `0.75`.
    pub similarity_boost: Option<f32>,

    /// Style exaggeration (0.0 – 1.0, v2 models only).
    pub style: Option<f32>,

    /// Whether to apply the speaker boost post-process (v2 models only).
    pub use_speaker_boost: Option<bool>,

    /// Model identifier, e.g. `"eleven_monolingual_v1"`,
    /// `"eleven_multilingual_v2"`.  Falls back to provider default when
    /// `None`.
    pub model_id: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for SynthOptions {
    fn default() -> Self {
        Self {
            stability: None,
            similarity_boost: None,
            style: None,
            use_speaker_boost: None,
            model_id: None,
        }
    }
}

// ── VoiceProvider ─────────────────────────────────────────────────────────────

/// Async trait that every TTS backend must implement.
///
/// Implementations must be [`Send`] + [`Sync`] so they can be stored in
/// `Arc<dyn VoiceProvider>` and shared across async tasks.
///
/// Not available on `wasm32` — use [`web::WebSpeechSynth`] there.
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait VoiceProvider: Send + Sync {
    /// Enumerate voices available on this provider.
    async fn list_voices(&self) -> Result<Vec<VoiceInfo>, VoiceError>;

    /// Synthesise `text` using `voice_id` and return the complete audio payload
    /// as a `Vec<u8>` (MPEG audio by default for ElevenLabs).
    async fn synthesize(
        &self,
        text: &str,
        voice_id: &str,
        opts: SynthOptions,
    ) -> Result<Vec<u8>, VoiceError>;

    /// Synthesise `text` as a streaming sequence of [`Bytes`] chunks.
    ///
    /// Callers can pipe the stream directly to an audio sink without buffering
    /// the full payload in memory.
    async fn synthesize_stream(
        &self,
        text: &str,
        voice_id: &str,
        opts: SynthOptions,
    ) -> Result<impl Stream<Item = Result<Bytes, VoiceError>> + Send, VoiceError>;
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use serde_json::json;

    use super::*;

    // ── SynthOptions serde ────────────────────────────────────────────────────

    #[test]
    fn synth_options_round_trip_all_fields() {
        let original = SynthOptions {
            stability: Some(0.4),
            similarity_boost: Some(0.8),
            style: Some(0.2),
            use_speaker_boost: Some(true),
            model_id: Some("eleven_multilingual_v2".to_owned()),
        };
        let serialised = serde_json::to_string(&original).expect("serialise");
        let restored: SynthOptions = serde_json::from_str(&serialised).expect("deserialise");
        assert_eq!(restored.stability, Some(0.4_f32));
        assert_eq!(restored.similarity_boost, Some(0.8_f32));
        assert_eq!(restored.style, Some(0.2_f32));
        assert_eq!(restored.use_speaker_boost, Some(true));
        assert_eq!(restored.model_id.as_deref(), Some("eleven_multilingual_v2"));
    }

    #[test]
    fn synth_options_round_trip_all_none() {
        let original = SynthOptions::default();
        let serialised = serde_json::to_string(&original).expect("serialise");
        let restored: SynthOptions = serde_json::from_str(&serialised).expect("deserialise");
        assert!(restored.stability.is_none());
        assert!(restored.similarity_boost.is_none());
        assert!(restored.style.is_none());
        assert!(restored.use_speaker_boost.is_none());
        assert!(restored.model_id.is_none());
    }

    // ── VoiceInfo deserialisation ─────────────────────────────────────────────
    //
    // Sample shaped from the real ElevenLabs GET /v1/voices response.
    // No network call; all data is hard-coded.

    const VOICES_PAYLOAD: &str = r#"
    {
      "voices": [
        {
          "voice_id": "EXAVITQu4vr4xnSDxMaL",
          "name": "Bella",
          "category": "premade",
          "labels": {
            "accent": "american",
            "description": "soft",
            "age": "young",
            "gender": "female",
            "use_case": "narration"
          },
          "preview_url": "https://storage.googleapis.com/eleven-public-prod/premade/voices/Bella.mp3"
        },
        {
          "voice_id": "N2lVS1w4EtoT3dr4eOWO",
          "name": "Callum",
          "category": "premade",
          "labels": {
            "accent": "american",
            "description": "hoarse",
            "age": "middle_aged",
            "gender": "male",
            "use_case": "video_games"
          },
          "preview_url": "https://storage.googleapis.com/eleven-public-prod/premade/voices/Callum.mp3"
        },
        {
          "voice_id": "orphan-no-preview",
          "name": "TestVoice"
        }
      ]
    }
    "#;

    /// Parse the upstream-shaped payload and map to our [`VoiceInfo`].
    ///
    /// This exercises [`elevenlabs::parse_voices_payload`] without any network
    /// call.
    #[test]
    fn voice_info_deserialise_from_elevenlabs_sample() {
        use crate::elevenlabs::parse_voices_payload;

        let raw: serde_json::Value = serde_json::from_str(VOICES_PAYLOAD).expect("parse payload");
        let voices = parse_voices_payload(&raw).expect("parse voices");

        assert_eq!(voices.len(), 3);

        let bella = &voices[0];
        assert_eq!(bella.id, "EXAVITQu4vr4xnSDxMaL");
        assert_eq!(bella.name, "Bella");
        assert_eq!(bella.gender.as_deref(), Some("female"));
        assert!(bella.preview_url.is_some());

        let callum = &voices[1];
        assert_eq!(callum.id, "N2lVS1w4EtoT3dr4eOWO");
        assert_eq!(callum.gender.as_deref(), Some("male"));

        // Voice with no optional fields must still parse without error.
        let orphan = &voices[2];
        assert_eq!(orphan.id, "orphan-no-preview");
        assert!(orphan.preview_url.is_none());
        assert!(orphan.gender.is_none());
    }

    /// Payload that has no `voices` array should produce an empty list.
    #[test]
    fn voice_info_empty_when_no_array() {
        use crate::elevenlabs::parse_voices_payload;

        let raw = json!({ "some_other_key": 42 });
        let voices = parse_voices_payload(&raw).expect("parse should succeed");
        assert!(voices.is_empty());
    }
}
