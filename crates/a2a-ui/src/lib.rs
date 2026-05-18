// SPDX-License-Identifier: Apache-2.0
//! A2A protocol UI — WASM channel viewer for the `.coord` file-based mailbox.
//!
//! This crate renders A2A envelopes (the JSONL format used by
//! `C:\winclean\.coord\inbox-from-aphrody.jsonl`) into DOM nodes inside a
//! browser page.  It is the UI complement to the wire-side `a2a`, `a2a-client`,
//! and `a2a-pb` crates.
//!
//! # Usage
//!
//! ```js
//! import init, { init_panic_hook, render_envelope_list } from "./a2a_ui.js";
//!
//! await init();
//! init_panic_hook();
//!
//! const resp = await fetch("./envelopes.json");
//! const json = await resp.text();
//! render_envelope_list("channel-view", json);
//! ```
//!
//! The `envelopes.json` payload is a JSON array of [`Envelope`] objects.
//! Each item in the `.coord/*.jsonl` files is one JSONL line that maps
//! directly to an [`Envelope`].

#![forbid(unsafe_code)]

// Native ratatui TUI backend — only compiled when the `native` feature is
// enabled.  WASM builds are unaffected.
#[cfg(feature = "native")] pub mod native;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Envelope type — cross-platform (native + wasm32).
//
// Fields derived from inspection of `C:\winclean\.coord\inbox-from-aphrody.jsonl`.
// The v1 schema is authoritative in `schemas/ai.json/v1.json`.
//
//   { "v": 1, "ts": "2026-05-17T20:02:27Z", "from": "aphrody",
//     "to": "winclean", "kind": "fact", "topic": "...", "body": "..." }
//
// Legacy envelopes (pre-v1) omit the `v` field and use `type` instead of
// `kind`; both are handled via `#[serde(alias)]`.
// ---------------------------------------------------------------------------

/// A single A2A channel envelope as stored in the `.coord/*.jsonl` mailboxes.
///
/// The struct is intentionally flat: nested structure is avoided because the
/// file-based mailbox format prioritises human-editability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    /// Protocol version.  `1` for current envelopes; absent on legacy v0.
    #[serde(default)]
    pub v: Option<u32>,

    /// ISO-8601 UTC timestamp at which the sender wrote the envelope.
    #[serde(default)]
    pub ts: String,

    /// Sender identifier (e.g. `"aphrody"` or `"winclean"`).
    #[serde(default)]
    pub from: String,

    /// Recipient identifier.  Optional on broadcast envelopes.
    #[serde(default)]
    pub to: Option<String>,

    /// Envelope kind / type.  Values: `fact`, `ask`, `ack`, `ping`, `error`.
    /// `type` is accepted as a legacy alias (pre-v1 mailboxes).
    #[serde(alias = "type", default)]
    pub kind: String,

    /// Human-readable topic / subject line.
    /// `subject` is accepted as a legacy alias.
    #[serde(alias = "subject", default)]
    pub topic: String,

    /// Free-form body.  May be a plain string or a JSON-stringified object.
    #[serde(default)]
    pub body: String,
}

impl Envelope {
    /// Parse a `.coord/*.jsonl` mailbox content (one envelope per line, blank
    /// lines and `//`-prefixed comment lines skipped).
    ///
    /// # Errors
    ///
    /// Returns the first `serde_json::Error` encountered. Subsequent lines
    /// are not inspected.
    pub fn parse_jsonl(content: &str) -> Result<Vec<Self>, serde_json::Error> {
        content
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with("//")
            })
            .map(serde_json::from_str)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// WASM surface — only compiled when targeting wasm32.
//
// On native targets these items are stripped, enabling `cargo check` on
// Linux/Windows hosts to pass cleanly without any browser-specific linkage.
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;
    use web_sys::{Document, Element};

    use super::Envelope;

    /// Install the `console_error_panic_hook` panic handler.
    ///
    /// Must be called once at WASM module start before any other function.
    /// Subsequent calls are no-ops (the hook is registered via an internal
    /// `Once` cell inside the crate).
    #[wasm_bindgen]
    pub fn init_panic_hook() {
        console_error_panic_hook::set_once();
    }

    /// Parse `json` as a JSON array of [`Envelope`] objects and render each
    /// one as a DOM subtree appended to the element identified by
    /// `container_id`.
    ///
    /// # DOM output
    ///
    /// For each envelope a `<div class="a2a-envelope">` is appended to the
    /// container.  The inner structure is:
    ///
    /// ```html
    /// <div class="a2a-envelope" data-kind="fact">
    ///   <div class="a2a-header">
    ///     <span class="a2a-from">aphrody</span>
    ///     <span class="a2a-arrow">→</span>
    ///     <span class="a2a-to">winclean</span>
    ///     <span class="a2a-kind">fact</span>
    ///     <span class="a2a-ts">2026-05-17T20:02:27Z</span>
    ///   </div>
    ///   <div class="a2a-topic">yolo-grind-tick34-mini</div>
    ///   <pre class="a2a-body">...</pre>
    /// </div>
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a `JsValue` error string if:
    /// - The element `container_id` does not exist in the document.
    /// - `json` is not a valid JSON array of envelope objects.
    /// - Any DOM mutation call fails.
    #[wasm_bindgen]
    pub fn render_envelope_list(container_id: &str, json: &str) -> Result<(), JsValue> {
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("render_envelope_list: no global window"))?;

        let document: Document = window
            .document()
            .ok_or_else(|| JsValue::from_str("render_envelope_list: no document on window"))?;

        let container: Element = document.get_element_by_id(container_id).ok_or_else(|| {
            JsValue::from_str(&format!("render_envelope_list: element #{container_id} not found"))
        })?;

        let envelopes: Vec<Envelope> = serde_json::from_str(json).map_err(|e| {
            JsValue::from_str(&format!("render_envelope_list: JSON parse error: {e}"))
        })?;

        for envelope in &envelopes {
            let card = build_envelope_card(&document, envelope)?;
            container.append_child(&card)?;
        }

        Ok(())
    }

    /// Build a single envelope DOM card.
    fn build_envelope_card(document: &Document, env: &Envelope) -> Result<Element, JsValue> {
        let card = document.create_element("div")?;
        card.set_class_name("a2a-envelope");
        card.set_attribute("data-kind", &env.kind)?;

        let header = document.create_element("div")?;
        header.set_class_name("a2a-header");

        let from_el = document.create_element("span")?;
        from_el.set_class_name("a2a-from");
        from_el.set_text_content(Some(&env.from));
        header.append_child(&from_el)?;

        if let Some(ref to) = env.to {
            let arrow = document.create_element("span")?;
            arrow.set_class_name("a2a-arrow");
            arrow.set_text_content(Some("\u{2192}"));
            header.append_child(&arrow)?;

            let to_el = document.create_element("span")?;
            to_el.set_class_name("a2a-to");
            to_el.set_text_content(Some(to));
            header.append_child(&to_el)?;
        }

        let kind_el = document.create_element("span")?;
        kind_el.set_class_name("a2a-kind");
        kind_el.set_text_content(Some(&env.kind));
        header.append_child(&kind_el)?;

        let ts_el = document.create_element("span")?;
        ts_el.set_class_name("a2a-ts");
        ts_el.set_text_content(Some(&env.ts));
        header.append_child(&ts_el)?;

        card.append_child(&header)?;

        let topic_el = document.create_element("div")?;
        topic_el.set_class_name("a2a-topic");
        topic_el.set_text_content(Some(&env.topic));
        card.append_child(&topic_el)?;

        let body_el = document.create_element("pre")?;
        body_el.set_class_name("a2a-body");
        body_el.set_text_content(Some(&env.body));
        card.append_child(&body_el)?;

        Ok(card)
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::{init_panic_hook, render_envelope_list};

// ---------------------------------------------------------------------------
// Native unit tests — exercise the Envelope serde surface (no DOM required).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::Envelope;

    const SAMPLE_V1: &str = r#"{"v":1,"ts":"2026-05-17T20:02:27Z","from":"aphrody","to":"winclean","kind":"fact","topic":"yolo-grind-tick34-mini","body":"workspace gate exit 0"}"#;

    const SAMPLE_LEGACY: &str =
        r#"{"ts":"2026-05-17T19:00:00Z","from":"winclean","type":"ping","subject":"hello"}"#;

    #[test]
    fn parses_v1_envelope() {
        let e: Envelope = serde_json::from_str(SAMPLE_V1).expect("parse v1");
        assert_eq!(e.v, Some(1));
        assert_eq!(e.from, "aphrody");
        assert_eq!(e.to.as_deref(), Some("winclean"));
        assert_eq!(e.kind, "fact");
        assert_eq!(e.topic, "yolo-grind-tick34-mini");
        assert!(!e.body.is_empty());
    }

    #[test]
    fn parses_legacy_aliases() {
        let e: Envelope = serde_json::from_str(SAMPLE_LEGACY).expect("parse legacy");
        assert_eq!(e.v, None);
        assert_eq!(e.kind, "ping"); // via #[serde(alias = "type")]
        assert_eq!(e.topic, "hello"); // via #[serde(alias = "subject")]
        assert_eq!(e.to, None);
    }

    #[test]
    fn parses_jsonl_mailbox() {
        let mailbox = format!("// comment line\n{SAMPLE_V1}\n\n{SAMPLE_LEGACY}\n");
        let envelopes = Envelope::parse_jsonl(&mailbox).expect("parse mailbox");
        assert_eq!(envelopes.len(), 2);
        assert_eq!(envelopes[0].kind, "fact");
        assert_eq!(envelopes[1].kind, "ping");
    }

    #[test]
    fn rejects_malformed_jsonl() {
        let bad = format!("{SAMPLE_V1}\n{{not valid json}}\n");
        assert!(Envelope::parse_jsonl(&bad).is_err());
    }
}
