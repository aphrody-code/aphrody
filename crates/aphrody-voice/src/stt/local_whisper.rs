// SPDX-License-Identifier: Apache-2.0
//! Local offline Whisper backend via `whisper.cpp` bindings.
//!
//! # Feature gate
//!
//! This module is only compiled when the `local-whisper` Cargo feature is
//! enabled.  Without the feature the module still exists but every public item
//! is a documented stub that returns [`SttError::NotImplemented`].
//!
//! # Migration path
//!
//! When you are ready to implement fully offline transcription:
//!
//! 1. Enable the `local-whisper` feature in your crate: ```toml aphrody-voice-stt = { path = "…",
//!    features = ["local-whisper"] } ```
//!
//! 2. Download a GGML model weight file from <https://huggingface.co/ggerganov/whisper.cpp>.
//!
//! 3. Replace the `NotImplemented` body in [`LocalWhisperBackend::transcribe`] with real
//!    `whisper_rs` calls: ```rust,ignore use whisper_rs::{WhisperContext, FullParams,
//!    SamplingStrategy}; let ctx = WhisperContext::new(&self.model_path)?; let mut state =
//!    ctx.create_state()?; let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 0
//!    }); // … set language, prompt, temperature … let samples: Vec<f32> =
//!    convert_bytes_to_f32_pcm(audio); state.full(params, &samples)?; let n =
//!    state.full_n_segments()?; // … collect text + timing … ```
//!
//! 4. Remove the `#[cfg(not(feature = "local-whisper"))]` guard on the real implementation block
//!    and delete the stub block.
//!
//! # Resource requirements
//!
//! | Model size | VRAM / RAM | WER (en) |
//! |------------|------------|----------|
//! | tiny.en    | ~39 MB     | ~5.7 %   |
//! | base.en    | ~74 MB     | ~4.2 %   |
//! | small.en   | ~244 MB    | ~3.4 %   |
//! | medium.en  | ~769 MB    | ~3.0 %   |
//! | large-v3   | ~1550 MB   | ~2.7 %   |
//!
//! The backend runs on CPU by default; CUDA/Metal acceleration is available
//! via `whisper-rs` feature flags.

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;

use crate::stt::{SttError, SttOptions, SttProvider, Transcript, TranscriptDelta};

// ── LocalWhisperBackend ───────────────────────────────────────────────────────

/// Fully offline STT backend using `whisper.cpp` via [`whisper_rs`].
///
/// # Current status
///
/// This backend is a **documented stub**.  Every method returns
/// [`SttError::NotImplemented`] until the `local-whisper` feature body is
/// filled in (see module-level doc for the migration path).
///
/// # Construction
///
/// ```rust,no_run
/// # use aphrody_voice_stt::local_whisper::LocalWhisperBackend;
/// let backend = LocalWhisperBackend::new("/path/to/ggml-base.en.bin");
/// ```
///
/// `model_path` must point to a GGML-format Whisper model weight file.  The
/// file is not validated at construction time; the error is deferred to the
/// first [`transcribe`](SttProvider::transcribe) call.
#[derive(Debug, Clone)]
pub struct LocalWhisperBackend {
    /// Filesystem path to the GGML Whisper model weights.
    pub model_path: String,
}

impl LocalWhisperBackend {
    /// Create a backend that will load `model_path` on first use.
    ///
    /// Construction is infallible and does not touch the filesystem; any
    /// I/O errors are deferred to the first transcription call.
    pub fn new(model_path: impl Into<String>) -> Self {
        Self { model_path: model_path.into() }
    }
}

// ── SttProvider impl (stub) ───────────────────────────────────────────────────

#[async_trait]
impl SttProvider for LocalWhisperBackend {
    /// Transcribe `audio` locally using `whisper.cpp`.
    ///
    /// # Current status
    ///
    /// Returns [`SttError::NotImplemented`] unconditionally.
    /// See the [module-level documentation](self) for the migration path.
    async fn transcribe(&self, _audio: &[u8], _opts: SttOptions) -> Result<Transcript, SttError> {
        Err(SttError::NotImplemented {
            reason: "LocalWhisperBackend: fill in whisper_rs calls (see local_whisper module doc)",
        })
    }

    /// Stream-transcribe audio locally using `whisper.cpp`.
    ///
    /// # Current status
    ///
    /// Returns [`SttError::NotImplemented`] unconditionally.
    /// See the [module-level documentation](self) for the migration path.
    async fn transcribe_stream(
        &self,
        _audio: impl Stream<Item = Bytes> + Send + 'static,
        _opts: SttOptions,
    ) -> Result<impl Stream<Item = Result<TranscriptDelta, SttError>> + Send, SttError> {
        Err::<futures::stream::Empty<Result<TranscriptDelta, SttError>>, _>(
            SttError::NotImplemented {
                reason: "LocalWhisperBackend::transcribe_stream: fill in whisper_rs calls (see \
                         local_whisper module doc)",
            },
        )
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_stores_path() {
        let backend = LocalWhisperBackend::new("/models/ggml-base.en.bin");
        assert_eq!(backend.model_path, "/models/ggml-base.en.bin");
    }

    #[tokio::test]
    async fn transcribe_returns_not_implemented() {
        let backend = LocalWhisperBackend::new("/dev/null");
        let err = backend
            .transcribe(b"fake audio", SttOptions::default())
            .await
            .expect_err("expected NotImplemented");
        assert!(matches!(err, SttError::NotImplemented { .. }), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn transcribe_stream_returns_not_implemented() {
        let backend = LocalWhisperBackend::new("/dev/null");
        let audio_stream = futures::stream::empty::<Bytes>();
        let result = backend.transcribe_stream(audio_stream, SttOptions::default()).await;
        // `unwrap_err` / `expect_err` require T: Debug which opaque `impl Stream`
        // does not satisfy.  Extract the error via pattern matching instead.
        match result {
            Err(SttError::NotImplemented { .. }) => {},
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("expected Err from transcribe_stream, got Ok"),
        }
    }
}
