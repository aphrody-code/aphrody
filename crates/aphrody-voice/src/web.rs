// SPDX-License-Identifier: Apache-2.0
//! Browser-native TTS via the Web Speech Synthesis API.
//!
//! Gated `#[cfg(target_arch = "wasm32")]`. Exposes [`WebSpeechSynth`] as a
//! `wasm-bindgen` class consumable directly from JavaScript / the
//! `packages/gemini-app-aphrody/` TypeScript package.
//!
//! # Browser support
//!
//! Requires `window.speechSynthesis` (Chrome 33+, Firefox 49+, Safari 7+).
//! Returns `Err(JsValue)` on browsers that lack the API.

#![cfg(target_arch = "wasm32")]

use js_sys::Array;
use wasm_bindgen::prelude::*;
use web_sys::{SpeechSynthesis, SpeechSynthesisUtterance, SpeechSynthesisVoice};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Retrieve `window.speechSynthesis`, returning a descriptive `JsValue` error
/// when the browser does not expose it.
fn get_synth() -> Result<SpeechSynthesis, JsValue> {
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("no global `window` object"))?
        .speech_synthesis()
        .map_err(|_| JsValue::from_str("window.speechSynthesis is unavailable"))
}

// ── WebSpeechSynth ────────────────────────────────────────────────────────────

/// Browser-native text-to-speech synthesiser backed by `window.speechSynthesis`.
///
/// # Example (JavaScript / TypeScript)
///
/// ```js
/// import init, { WebSpeechSynth } from "./aphrody_voice.js";
/// await init();
/// const synth = new WebSpeechSynth();
/// synth.speak("Hello from aphrody!", null, 1.0, 1.0);
/// ```
#[wasm_bindgen]
pub struct WebSpeechSynth {
    inner: SpeechSynthesis,
}

#[wasm_bindgen]
impl WebSpeechSynth {
    /// Construct a `WebSpeechSynth` by acquiring `window.speechSynthesis`.
    ///
    /// # Errors
    ///
    /// Returns `Err(JsValue)` when called outside a browser context or when the
    /// browser does not implement `SpeechSynthesis`.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WebSpeechSynth, JsValue> {
        let inner = get_synth()?;
        Ok(WebSpeechSynth { inner })
    }

    /// Enqueue `text` for synthesis with optional voice, rate, and pitch.
    ///
    /// * `voice_name` — BCP-47 name of a voice returned by [`list_voices`].
    ///   When `None` the browser chooses its default voice.
    /// * `rate` — speech rate multiplier (0.1 – 10.0; 1.0 = normal).
    /// * `pitch` — pitch multiplier (0.0 – 2.0; 1.0 = normal).
    ///
    /// # Errors
    ///
    /// Returns `Err(JsValue)` when `SpeechSynthesisUtterance` cannot be
    /// constructed (non-browser environment).
    pub fn speak(
        &self,
        text: &str,
        voice_name: Option<String>,
        rate: f32,
        pitch: f32,
    ) -> Result<(), JsValue> {
        let utterance = SpeechSynthesisUtterance::new_with_text(text)?;
        utterance.set_rate(rate);
        utterance.set_pitch(pitch);

        // If a voice name was requested, scan `getVoices()` for a match.
        if let Some(ref name) = voice_name {
            let voices = self.inner.get_voices();
            let n = voices.length();
            for i in 0..n {
                let v: SpeechSynthesisVoice = voices.get(i).unchecked_into();
                if &v.name() == name {
                    utterance.set_voice(Some(&v));
                    break;
                }
            }
        }

        self.inner.speak(&utterance);
        Ok(())
    }

    /// Cancel all pending and current utterances.
    pub fn cancel(&self) {
        self.inner.cancel();
    }

    /// Returns `true` when one or more utterances are queued but not yet being
    /// spoken.
    pub fn pending(&self) -> bool {
        self.inner.pending()
    }

    /// Returns `true` when an utterance is actively being spoken.
    pub fn speaking(&self) -> bool {
        self.inner.speaking()
    }

    /// Returns a `js_sys::Array` of `SpeechSynthesisVoice` objects available on
    /// this browser.
    ///
    /// The array may be empty on first call in Chrome because `getVoices()` is
    /// asynchronously populated; callers should listen for the
    /// `voiceschanged` event and re-query.
    pub fn list_voices(&self) -> Array {
        self.inner.get_voices()
    }
}
