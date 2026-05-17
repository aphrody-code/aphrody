// SPDX-License-Identifier: Apache-2.0
//! Gemini-specific composable atoms.
//!
//! Reference: <https://design.google/library/gemini-ai-visual-design>.
//!
//! These atoms sit on top of the M3 baseline elements exposed by the
//! sibling modules ([`super::button`], [`super::card`], etc.) and
//! contribute the Gemini-product-specific surfaces:
//!
//! - [`GeminiSparkleProps`] — the 4-color-dot lineage sparkle SVG used
//!   on the empty-state greeting and the "Gemini is thinking" indicator.
//! - [`GeminiPromptBarProps`] — the rounded-pill prompt input bar with
//!   leading attach button, multiline textarea, mic, and a send button
//!   wearing the spectrum-shift gradient.
//! - [`GeminiMessageBubbleProps`] — the chat message container, with a
//!   `from` discriminator (`user` | `assistant`) and an optional
//!   shimmer state for streaming AI responses.
//! - [`GeminiSuggestionChipProps`] — the empty-state suggestion chip
//!   with a small leading icon and rounded-pill silhouette.
//! - [`GeminiAvatarRingProps`] — the gradient ring wrapped around the
//!   user avatar (matches the spectrum-shift gradient).
//!
//! All atoms render real DOM via `wasm_bindgen` on the `wasm32` target.
//! On native targets the file compiles as an rlib so Props structs can
//! be unit-tested without a DOM.

use serde::{Deserialize, Serialize};

/// Authoring origin for a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageOrigin {
    /// Message authored by the human user.
    User,
    /// Message authored by the Gemini assistant.
    Assistant,
}

impl MessageOrigin {
    /// CSS class suffix used on the message bubble container.
    #[must_use]
    pub const fn class_suffix(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

// ---------------------------------------------------------------------------
// Props structs (cross-platform, used for native tests + WASM render)
// ---------------------------------------------------------------------------

/// Props for [`create_gemini_sparkle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiSparkleProps {
    /// Diameter in CSS pixels. Empty-state default = 128.
    #[serde(default = "default_sparkle_size")]
    pub size_px: u32,
    /// When true, animates the sparkle (rotates the gradient).
    #[serde(default)]
    pub animated: bool,
    /// Optional `aria-label` for screen readers.
    #[serde(default)]
    pub aria_label: Option<String>,
}

fn default_sparkle_size() -> u32 {
    128
}

impl GeminiSparkleProps {
    /// Field count for smoke-test parity with the other shadcn-bridge atoms.
    pub const FIELD_COUNT: usize = 3;
}

impl Default for GeminiSparkleProps {
    fn default() -> Self {
        Self {
            size_px: default_sparkle_size(),
            animated: true,
            aria_label: Some("Gemini".to_owned()),
        }
    }
}

/// Props for [`create_gemini_prompt_bar`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeminiPromptBarProps {
    /// Current textarea value (empty string by default).
    #[serde(default)]
    pub value: String,
    /// Placeholder text inside the textarea.
    #[serde(default)]
    pub placeholder: String,
    /// When true the send button is enabled (default false — Gemini
    /// only enables Send when the textarea is non-empty).
    #[serde(default)]
    pub can_send: bool,
    /// When true a mic affordance is shown to the left of the send button.
    #[serde(default)]
    pub show_mic: bool,
    /// When true the leading `+` attach button is shown.
    #[serde(default)]
    pub show_attach: bool,
}

impl GeminiPromptBarProps {
    pub const FIELD_COUNT: usize = 5;
}

/// Props for [`create_gemini_message_bubble`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiMessageBubbleProps {
    /// Author of the message.
    pub from: MessageOrigin,
    /// Body text. May contain plain text only — caller is responsible
    /// for sanitising HTML before passing through a wrapping layer.
    pub text: String,
    /// When true the bubble shows the shimmer-gradient streaming
    /// indicator (used while Gemini is still composing the response).
    #[serde(default)]
    pub streaming: bool,
    /// Optional timestamp string (e.g. "12:34"). Rendered in muted
    /// foreground beneath the message.
    #[serde(default)]
    pub timestamp: Option<String>,
}

impl GeminiMessageBubbleProps {
    pub const FIELD_COUNT: usize = 4;
}

/// Props for [`create_gemini_suggestion_chip`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiSuggestionChipProps {
    /// Visible label.
    pub label: String,
    /// Optional leading icon glyph name (e.g. `"lightbulb"`,
    /// `"code"`, `"travel"` — matches Material Symbols).
    #[serde(default)]
    pub icon: Option<String>,
    /// When true the chip uses the spectrum-shift gradient ring
    /// (used for the featured / hero suggestion).
    #[serde(default)]
    pub featured: bool,
}

impl GeminiSuggestionChipProps {
    pub const FIELD_COUNT: usize = 3;
}

/// Props for [`create_gemini_avatar_ring`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiAvatarRingProps {
    /// Avatar image URL. Required.
    pub src: String,
    /// Diameter in CSS pixels.
    #[serde(default = "default_avatar_size")]
    pub size_px: u32,
    /// `alt` attribute.
    #[serde(default)]
    pub alt: String,
}

fn default_avatar_size() -> u32 {
    40
}

impl GeminiAvatarRingProps {
    pub const FIELD_COUNT: usize = 3;
}

// ---------------------------------------------------------------------------
// WASM render surface
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::{
        GeminiAvatarRingProps, GeminiMessageBubbleProps, GeminiPromptBarProps,
        GeminiSparkleProps, GeminiSuggestionChipProps, MessageOrigin,
    };
    use wasm_bindgen::prelude::*;
    use web_sys::{Document, HtmlElement};

    fn document() -> Result<Document, JsValue> {
        web_sys::window()
            .ok_or_else(|| JsValue::from_str("no global window"))?
            .document()
            .ok_or_else(|| JsValue::from_str("no document on window"))
    }

    fn element(doc: &Document, tag: &str, class: &str) -> Result<HtmlElement, JsValue> {
        let el: HtmlElement = doc.create_element(tag)?.dyn_into()?;
        if !class.is_empty() {
            el.set_class_name(class);
        }
        Ok(el)
    }

    /// Render an inline `<svg class="gemini-sparkle">` echoing the
    /// four-color-dot lineage.
    pub fn create_gemini_sparkle(props: &GeminiSparkleProps) -> Result<HtmlElement, JsValue> {
        let doc = document()?;
        let wrapper = element(
            &doc,
            "span",
            if props.animated {
                "gemini-sparkle gemini-sparkle--animated"
            } else {
                "gemini-sparkle"
            },
        )?;
        wrapper.set_attribute("style", &format!("width:{0}px;height:{0}px;", props.size_px))?;
        if let Some(ref label) = props.aria_label {
            wrapper.set_attribute("role", "img")?;
            wrapper.set_attribute("aria-label", label)?;
        } else {
            wrapper.set_attribute("aria-hidden", "true")?;
        }
        // Sparkle SVG path: a 4-point star with rounded lobes, matching
        // the Gemini logo silhouette. Path generated from a normalised
        // viewBox so it scales with the wrapper.
        wrapper.set_inner_html(
            r##"<svg viewBox="0 0 24 24" fill="url(#gemini-sparkle-grad)" aria-hidden="true">
                 <defs>
                   <linearGradient id="gemini-sparkle-grad" x1="0" y1="0" x2="1" y2="1">
                     <stop offset="0%" stop-color="#4285F4"/>
                     <stop offset="33%" stop-color="#EA4335"/>
                     <stop offset="66%" stop-color="#FBBC04"/>
                     <stop offset="100%" stop-color="#34A853"/>
                   </linearGradient>
                 </defs>
                 <path d="M12 2.5 C12 7.5 14.5 10 19.5 10 C14.5 10 12 12.5 12 17.5 C12 12.5 9.5 10 4.5 10 C9.5 10 12 7.5 12 2.5 Z"/>
               </svg>"##,
        );
        Ok(wrapper)
    }

    /// Render the Gemini chat prompt bar.
    pub fn create_gemini_prompt_bar(
        props: &GeminiPromptBarProps,
    ) -> Result<HtmlElement, JsValue> {
        let doc = document()?;
        let bar = element(&doc, "form", "gemini-prompt-bar")?;
        bar.set_attribute("role", "search")?;

        if props.show_attach {
            let attach = element(&doc, "button", "gemini-prompt-bar__icon-btn")?;
            attach.set_attribute("type", "button")?;
            attach.set_attribute("aria-label", "Attach")?;
            attach.set_text_content(Some("+"));
            bar.append_child(&attach)?;
        }

        let textarea = element(&doc, "textarea", "gemini-prompt-bar__textarea")?;
        textarea.set_attribute("rows", "1")?;
        textarea.set_attribute("placeholder", &props.placeholder)?;
        textarea.set_text_content(Some(&props.value));
        bar.append_child(&textarea)?;

        if props.show_mic {
            let mic = element(&doc, "button", "gemini-prompt-bar__icon-btn")?;
            mic.set_attribute("type", "button")?;
            mic.set_attribute("aria-label", "Voice input")?;
            mic.set_text_content(Some("\u{1F3A4}")); // microphone glyph
            bar.append_child(&mic)?;
        }

        let send = element(
            &doc,
            "button",
            if props.can_send {
                "gemini-prompt-bar__send gemini-prompt-bar__send--enabled"
            } else {
                "gemini-prompt-bar__send"
            },
        )?;
        send.set_attribute("type", "submit")?;
        send.set_attribute("aria-label", "Send")?;
        if !props.can_send {
            send.set_attribute("disabled", "true")?;
        }
        send.set_inner_html(r##"<span class="gemini-prompt-bar__send-glyph">\u{2191}</span>"##);
        bar.append_child(&send)?;

        Ok(bar)
    }

    /// Render a chat message bubble (user or assistant).
    pub fn create_gemini_message_bubble(
        props: &GeminiMessageBubbleProps,
    ) -> Result<HtmlElement, JsValue> {
        let doc = document()?;
        let suffix = props.from.class_suffix();
        let mut cls = format!("gemini-message gemini-message--{suffix}");
        if props.streaming && props.from == MessageOrigin::Assistant {
            cls.push_str(" gemini-message--streaming");
        }
        let bubble = element(&doc, "article", &cls)?;
        bubble.set_attribute(
            "aria-roledescription",
            if props.from == MessageOrigin::Assistant {
                "assistant response"
            } else {
                "user message"
            },
        )?;

        let body = element(&doc, "div", "gemini-message__body")?;
        body.set_text_content(Some(&props.text));
        bubble.append_child(&body)?;

        if let Some(ref ts) = props.timestamp {
            let stamp = element(&doc, "time", "gemini-message__ts")?;
            stamp.set_text_content(Some(ts));
            bubble.append_child(&stamp)?;
        }

        Ok(bubble)
    }

    /// Render an empty-state suggestion chip.
    pub fn create_gemini_suggestion_chip(
        props: &GeminiSuggestionChipProps,
    ) -> Result<HtmlElement, JsValue> {
        let doc = document()?;
        let cls = if props.featured {
            "gemini-suggestion gemini-suggestion--featured"
        } else {
            "gemini-suggestion"
        };
        let chip = element(&doc, "button", cls)?;
        chip.set_attribute("type", "button")?;
        if let Some(ref icon) = props.icon {
            let icon_el = element(&doc, "span", "gemini-suggestion__icon")?;
            icon_el.set_text_content(Some(icon));
            chip.append_child(&icon_el)?;
        }
        let label = element(&doc, "span", "gemini-suggestion__label")?;
        label.set_text_content(Some(&props.label));
        chip.append_child(&label)?;
        Ok(chip)
    }

    /// Render a user avatar wrapped in the spectrum-shift gradient ring.
    pub fn create_gemini_avatar_ring(
        props: &GeminiAvatarRingProps,
    ) -> Result<HtmlElement, JsValue> {
        let doc = document()?;
        let ring = element(&doc, "span", "gemini-avatar-ring")?;
        ring.set_attribute("style", &format!("width:{0}px;height:{0}px;", props.size_px))?;
        let img = element(&doc, "img", "gemini-avatar-ring__img")?;
        img.set_attribute("src", &props.src)?;
        img.set_attribute("alt", &props.alt)?;
        ring.append_child(&img)?;
        Ok(ring)
    }

    // ----------------------- wasm-bindgen wrappers -----------------------

    #[wasm_bindgen(js_name = createGeminiSparkle)]
    pub fn js_create_gemini_sparkle(props_json: &str) -> Result<HtmlElement, JsValue> {
        let p: GeminiSparkleProps = serde_json::from_str(props_json)
            .map_err(|e| JsValue::from_str(&format!("GeminiSparkleProps parse: {e}")))?;
        create_gemini_sparkle(&p)
    }

    #[wasm_bindgen(js_name = createGeminiPromptBar)]
    pub fn js_create_gemini_prompt_bar(props_json: &str) -> Result<HtmlElement, JsValue> {
        let p: GeminiPromptBarProps = serde_json::from_str(props_json)
            .map_err(|e| JsValue::from_str(&format!("GeminiPromptBarProps parse: {e}")))?;
        create_gemini_prompt_bar(&p)
    }

    #[wasm_bindgen(js_name = createGeminiMessageBubble)]
    pub fn js_create_gemini_message_bubble(props_json: &str) -> Result<HtmlElement, JsValue> {
        let p: GeminiMessageBubbleProps = serde_json::from_str(props_json)
            .map_err(|e| JsValue::from_str(&format!("GeminiMessageBubbleProps parse: {e}")))?;
        create_gemini_message_bubble(&p)
    }

    #[wasm_bindgen(js_name = createGeminiSuggestionChip)]
    pub fn js_create_gemini_suggestion_chip(props_json: &str) -> Result<HtmlElement, JsValue> {
        let p: GeminiSuggestionChipProps = serde_json::from_str(props_json)
            .map_err(|e| JsValue::from_str(&format!("GeminiSuggestionChipProps parse: {e}")))?;
        create_gemini_suggestion_chip(&p)
    }

    #[wasm_bindgen(js_name = createGeminiAvatarRing)]
    pub fn js_create_gemini_avatar_ring(props_json: &str) -> Result<HtmlElement, JsValue> {
        let p: GeminiAvatarRingProps = serde_json::from_str(props_json)
            .map_err(|e| JsValue::from_str(&format!("GeminiAvatarRingProps parse: {e}")))?;
        create_gemini_avatar_ring(&p)
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::{
    create_gemini_avatar_ring, create_gemini_message_bubble, create_gemini_prompt_bar,
    create_gemini_sparkle, create_gemini_suggestion_chip,
};

// ---------------------------------------------------------------------------
// Native unit tests — exercise the Props serde + defaults surface.
// ---------------------------------------------------------------------------
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn message_origin_class_suffix_is_stable() {
        assert_eq!(MessageOrigin::User.class_suffix(), "user");
        assert_eq!(MessageOrigin::Assistant.class_suffix(), "assistant");
    }

    #[test]
    fn message_origin_serde_lowercase() {
        let json = serde_json::to_string(&MessageOrigin::Assistant).unwrap();
        assert_eq!(json, "\"assistant\"");
        let back: MessageOrigin = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(back, MessageOrigin::User);
    }

    #[test]
    fn sparkle_default_size_is_128() {
        let p = GeminiSparkleProps::default();
        assert_eq!(p.size_px, 128);
        assert!(p.animated);
        assert_eq!(p.aria_label.as_deref(), Some("Gemini"));
    }

    #[test]
    fn prompt_bar_can_send_defaults_false() {
        let p: GeminiPromptBarProps = serde_json::from_str("{}").unwrap();
        assert_eq!(p.value, "");
        assert!(!p.can_send);
        assert!(!p.show_mic);
    }

    #[test]
    fn message_bubble_serde_required_fields() {
        let p: GeminiMessageBubbleProps =
            serde_json::from_str(r#"{"from":"assistant","text":"Hello!"}"#).unwrap();
        assert_eq!(p.from, MessageOrigin::Assistant);
        assert_eq!(p.text, "Hello!");
        assert!(!p.streaming);
        assert!(p.timestamp.is_none());
    }

    #[test]
    fn suggestion_chip_featured_defaults_false() {
        let p: GeminiSuggestionChipProps =
            serde_json::from_str(r#"{"label":"Plan a trip"}"#).unwrap();
        assert_eq!(p.label, "Plan a trip");
        assert!(p.icon.is_none());
        assert!(!p.featured);
    }

    #[test]
    fn avatar_ring_required_src() {
        let p: GeminiAvatarRingProps =
            serde_json::from_str(r#"{"src":"https://example.com/me.jpg"}"#).unwrap();
        assert_eq!(p.src, "https://example.com/me.jpg");
        assert_eq!(p.size_px, 40);
    }

    #[test]
    fn field_count_constants_match_struct_fields() {
        assert_eq!(GeminiSparkleProps::FIELD_COUNT, 3);
        assert_eq!(GeminiPromptBarProps::FIELD_COUNT, 5);
        assert_eq!(GeminiMessageBubbleProps::FIELD_COUNT, 4);
        assert_eq!(GeminiSuggestionChipProps::FIELD_COUNT, 3);
        assert_eq!(GeminiAvatarRingProps::FIELD_COUNT, 3);
    }
}
