// SPDX-License-Identifier: Apache-2.0
//! Browser-native STT via the Web Speech Recognition API.
//!
//! Gated `#[cfg(target_arch = "wasm32")]`. Exposes [`WebSpeechRecognition`] as
//! a `wasm-bindgen` class consumable from JavaScript / the
//! `packages/gemini-app-aphrody/` TypeScript package.
//!
//! # Browser support
//!
//! Requires `SpeechRecognition` or `webkitSpeechRecognition` (Chrome 33+,
//! Edge 79+). Returns `Err(JsValue)` on Firefox/Safari where the API is absent.
//!
//! # Constructor strategy
//!
//! The constructor probes `window["SpeechRecognition"]` first (standard), then
//! falls back to `window["webkitSpeechRecognition"]` (Chrome/Edge legacy prefix)
//! using `js_sys::Reflect` so that both code paths compile to the same Wasm
//! without hard-coding either class name.

#![cfg(target_arch = "wasm32")]

use js_sys::{Array, Function, Reflect};
use wasm_bindgen::prelude::*;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Obtain the `SpeechRecognition` constructor from `window`, trying the
/// unprefixed form first and falling back to `webkitSpeechRecognition`.
///
/// Returns `Err` when neither form is available.
fn get_recognition_ctor() -> Result<Function, JsValue> {
    let win: JsValue = web_sys::window()
        .ok_or_else(|| JsValue::from_str("no global `window` object"))?
        .into();

    // Try standard name first.
    let standard = Reflect::get(&win, &JsValue::from_str("SpeechRecognition"))?;
    if standard.is_function() {
        return Ok(standard.unchecked_into::<Function>());
    }

    // Fall back to webkit-prefixed variant (Chrome / Edge).
    let webkit = Reflect::get(&win, &JsValue::from_str("webkitSpeechRecognition"))?;
    if webkit.is_function() {
        return Ok(webkit.unchecked_into::<Function>());
    }

    Err(JsValue::from_str(
        "SpeechRecognition is unavailable in this browser",
    ))
}

/// Call a named method on a `JsValue` object with no arguments.
///
/// Returns the method's return value as `JsValue`, or `Err` if the method
/// does not exist or throws.
fn call_method(target: &JsValue, method: &str) -> Result<JsValue, JsValue> {
    let f: Function = Reflect::get(target, &JsValue::from_str(method))?.dyn_into()?;
    f.call0(target)
}

// ── WebSpeechRecognition ──────────────────────────────────────────────────────

/// Browser-native speech recogniser backed by `SpeechRecognition` /
/// `webkitSpeechRecognition`.
///
/// # Example (JavaScript / TypeScript)
///
/// ```js
/// import init, { WebSpeechRecognition } from "./aphrody_voice_stt.js";
/// await init();
///
/// const rec = new WebSpeechRecognition("en-US");
/// rec.start(
///   (event) => console.log("result", event),
///   (err)   => console.error("error", err),
/// );
/// ```
#[wasm_bindgen]
pub struct WebSpeechRecognition {
    inner: JsValue,
    lang: String,
}

#[wasm_bindgen]
impl WebSpeechRecognition {
    /// Construct a `WebSpeechRecognition` for the given BCP-47 `lang` tag
    /// (e.g. `"en-US"`, `"fr-FR"`).
    ///
    /// Configures:
    /// - `interimResults = true` — partial results are delivered to `onresult`.
    /// - `continuous = false` — recognition stops automatically after silence.
    /// - `lang` — set to the provided value.
    ///
    /// # Errors
    ///
    /// Returns `Err(JsValue)` when neither `SpeechRecognition` nor
    /// `webkitSpeechRecognition` is available, or when construction fails.
    #[wasm_bindgen(constructor)]
    pub fn new(lang: &str) -> Result<WebSpeechRecognition, JsValue> {
        let ctor = get_recognition_ctor()?;

        // Construct `new SpeechRecognition()` via Reflect so that we do not
        // depend on a specific class name being wired in web-sys features.
        let args = Array::new();
        let inner: JsValue = Reflect::construct(&ctor, &args)?;

        // Configure the recogniser before the first `start()`.
        Reflect::set(&inner, &JsValue::from_str("interimResults"), &JsValue::from_bool(true))?;
        Reflect::set(&inner, &JsValue::from_str("continuous"), &JsValue::from_bool(false))?;
        Reflect::set(&inner, &JsValue::from_str("lang"), &JsValue::from_str(lang))?;

        Ok(WebSpeechRecognition {
            inner,
            lang: lang.to_owned(),
        })
    }

    /// The BCP-47 language tag this recogniser is configured for.
    pub fn lang(&self) -> String {
        self.lang.clone()
    }

    /// Begin recognition.
    ///
    /// * `on_result` — JavaScript function called with each `SpeechRecognitionEvent`.
    /// * `on_error` — JavaScript function called with each `SpeechRecognitionErrorEvent`.
    ///
    /// Both callbacks are set on the underlying object before `start()` is
    /// called so that no event is missed.
    ///
    /// # Errors
    ///
    /// Returns `Err(JsValue)` when `start()` itself throws (e.g. when called
    /// on an already-active recogniser).
    pub fn start(&self, on_result: Function, on_error: Function) -> Result<(), JsValue> {
        Reflect::set(&self.inner, &JsValue::from_str("onresult"), on_result.as_ref())?;
        Reflect::set(&self.inner, &JsValue::from_str("onerror"), on_error.as_ref())?;
        call_method(&self.inner, "start")?;
        Ok(())
    }

    /// Stop recognition gracefully: processes buffered audio before ending.
    ///
    /// # Errors
    ///
    /// Returns `Err(JsValue)` when the underlying `stop()` call throws.
    pub fn stop(&self) -> Result<(), JsValue> {
        call_method(&self.inner, "stop")?;
        Ok(())
    }

    /// Abort recognition immediately, discarding any buffered audio.
    ///
    /// # Errors
    ///
    /// Returns `Err(JsValue)` when the underlying `abort()` call throws.
    pub fn abort(&self) -> Result<(), JsValue> {
        call_method(&self.inner, "abort")?;
        Ok(())
    }
}
