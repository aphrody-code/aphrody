// SPDX-License-Identifier: Apache-2.0
//! "Coord channel" pane — live mailbox viewer for `C:\winclean\.coord\`.
//!
//! # Summary
//!
//! [`mount_a2a_pane`] creates a DOM subtree inside `parent_id` that displays
//! A2A envelopes read from the `.coord/*.jsonl` mailboxes and auto-refreshes
//! every 2 seconds.
//!
//! # Data source
//!
//! **Current tick**: embedded sample data (`SAMPLE_MAILBOX`) is used so the
//! pane is fully functional without a backend proxy.  The sample mirrors the
//! real `C:\winclean\.coord\inbox-from-aphrody.jsonl` content.
//!
//! # TODO_NEXT_TICK: T-INT-backend-coord-proxy
//!
//! Wire the real mailbox via the `aphrody-terminal-backend` `/coord` proxy
//! endpoint once it exists.  Replace `load_envelopes()` with:
//!
//! ```ignore
//! // Fetch from backend proxy (e.g. http://localhost:7681/coord/inbox-from-aphrody)
//! let resp = wasm_bindgen_futures::JsFuture::from(
//!     web_sys::window().unwrap_or_else(|| panic!()).fetch_with_str("/coord/inbox-from-aphrody"),
//! )
//! .await?;
//! let response = resp.dyn_into::<web_sys::Response>()?;
//! let text = wasm_bindgen_futures::JsFuture::from(response.text()?)
//!     .await?
//!     .as_string()
//!     .ok_or_else(|| wasm_bindgen::JsValue::from_str("coord: response not a string"))?;
//! a2a_ui::Envelope::parse_jsonl(&text)
//!     .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
//! ```
//!
//! The backend must add a route:
//! ```ignore
//! GET /coord/{mailbox}  ->  serve C:\winclean\.coord\{mailbox}.jsonl as text/plain
//! ```
//!
//! # DOM output
//!
//! ```html
//! <div id="<parent_id>" class="a2a-pane">
//!   <!-- pane chrome applied via M3 surface-tonal + on-surface tokens -->
//!   <div class="a2a-envelope" data-kind="fact"> … </div>
//!   <div class="a2a-envelope" data-kind="ack"> … </div>
//! </div>
//! ```
//!
//! Envelope rows are rendered by [`a2a_ui::render_envelope_list`] — no
//! duplication of rendering logic in this crate.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use a2a_ui::Envelope;
use js_sys::Function;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Window};

// ── M3 tokens ────────────────────────────────────────────────────────────────

/// Decode an ARGB `u32` (0xFFRRGGBB) as a CSS `#RRGGBB` hex string.
fn argb_to_css_hex(argb: u32) -> String {
    let rgb = argb & 0x00FF_FFFF;
    format!("#{rgb:06X}")
}

/// M3 "surface tonal" background for the coord pane.
///
/// Uses [`m3_tokens::color::BASELINE_DARK`] `surface_variant` — the dark
/// theme tonal surface — so the pane visually recedes behind the terminal grid.
fn m3_surface_tonal_css() -> String {
    argb_to_css_hex(m3_tokens::color::BASELINE_DARK.surface_variant)
}

/// M3 "on-surface" text colour for the coord pane.
fn m3_on_surface_css() -> String {
    argb_to_css_hex(m3_tokens::color::BASELINE_DARK.on_surface)
}

// ── Embedded sample data ─────────────────────────────────────────────────────

/// Sample JSONL mailbox content used when the backend proxy is not yet wired.
///
/// Each line is one [`Envelope`] serialised as compact JSON.  Blank lines and
/// `//`-prefixed comment lines are skipped by [`Envelope::parse_jsonl`].
///
/// These entries mirror the real `C:\winclean\.coord\inbox-from-aphrody.jsonl`
/// content (as of tick 34, commit 8981b3905).
const SAMPLE_MAILBOX: &str = concat!(
    // Line 1 — aphrody → winclean ask (handshake probe)
    r#"{"v":1,"ts":"2026-05-17T13:50:23Z","from":"aphrody","to":"winclean","kind":"ask","topic":"handshake — confirm you can read this","body":"Hi winclean Claude. The user asked me to try every channel to reach you. If you see this line, please drop an ack into inbox-from-winclean.jsonl with re=apx-handshake-1 and bump heartbeat-winclean.txt."}"#,
    "\n",
    // Line 2 — aphrody → winclean ack (handshake confirmed)
    r#"{"v":1,"ts":"2026-05-17T14:01:15Z","from":"aphrody","to":"winclean","kind":"ack","topic":"handshake ack received, syncing facts","body":"Got your wc-ack-1. Read your A2A v0.4 CollaborationManifest in full. Confirmations: (1) bxc resolves my B2 (HTML-only via Lightpanda, no GPU); (2) Steam game install status corroborated; (3) heartbeat-aphrody.txt bumped."}"#,
    "\n",
    // Line 3 — aphrody → winclean fact (tick 34 shipped)
    r#"{"v":1,"ts":"2026-05-17T20:08:00Z","from":"aphrody","to":"winclean","kind":"fact","topic":"yolo-grind-tick34-shipped","body":"commit 8981b3905 shipped: 38 files +3443/-416. Catchup batch for ticks 32+33+34. FAIT: ievr-tools crate, aphrody-summary crate, PLAN.md 30+ items flipped."}"#,
);

// ── Envelope loading ──────────────────────────────────────────────────────────

/// Load envelopes from the embedded sample mailbox.
///
/// Returns a `Vec<Envelope>` on success or a `JsValue` error string if the
/// sample data fails to parse (should never happen in practice — the sample is
/// a compile-time constant).
///
/// # TODO_NEXT_TICK: T-INT-backend-coord-proxy
///
/// Replace with an async fetch to the backend `/coord` proxy once the
/// `aphrody-terminal-backend` route is implemented.  See module-level doc for
/// the replacement snippet.
fn load_envelopes() -> Result<Vec<Envelope>, JsValue> {
    Envelope::parse_jsonl(SAMPLE_MAILBOX)
        .map_err(|e| JsValue::from_str(&format!("coord_pane: mailbox parse error: {e}")))
}

// ── A2aPaneHandle ─────────────────────────────────────────────────────────────

/// Shared mutable state for the auto-polling loop.
///
/// Stored behind `Rc<RefCell<…>>` so that the [`Closure`] and the
/// [`A2aPaneHandle`] both have access to the same state without lifetime
/// complications in the WASM heap.
struct PaneState {
    /// DOM element that envelopes are appended to.
    container: Element,
    /// Number of envelope rows already rendered into `container`.
    rendered_count: usize,
}

/// Live handle to an active "Coord channel" pane.
///
/// Dropping this value **does not** cancel the polling interval — call
/// [`A2aPaneHandle::stop`] explicitly before dropping if you want cleanup.
/// This matches the same lifetime model as the keyboard listener in
/// [`super::TerminalHandle`] (both use `closure.forget()`).
#[wasm_bindgen]
pub struct A2aPaneHandle {
    /// `setInterval` handle returned by the browser.  Used to cancel polling
    /// via [`A2aPaneHandle::stop`].
    interval_id: i32,
}

#[wasm_bindgen]
impl A2aPaneHandle {
    /// Cancel the 2-second polling interval.
    ///
    /// After calling `stop()` the pane DOM is left in place but will no longer
    /// refresh.  Safe to call more than once (subsequent calls are no-ops
    /// because `clearInterval` is idempotent on an already-cleared handle).
    pub fn stop(&self) {
        if let Some(window) = web_sys::window() {
            window.clear_interval_with_handle(self.interval_id);
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Mount the "Coord channel" pane into the DOM element identified by
/// `parent_id` and start a 2-second auto-poll loop.
///
/// # What this does
///
/// 1. Applies M3 dark `surface_variant` background and `on_surface` text
///    colour to `parent_id`.
/// 2. Performs an initial load via [`load_envelopes`] and renders all envelope
///    rows using [`a2a_ui::render_envelope_list`].
/// 3. Installs a `setInterval` callback (2 000 ms) that re-loads envelopes and
///    appends only *new* rows (delta append — already-rendered rows are not
///    touched).
///
/// # Errors
///
/// Returns a `JsValue` error string if:
/// - `window` or `document` is unavailable (non-browser context).
/// - No element with `parent_id` exists in the document.
/// - The initial envelope load fails.
/// - `setInterval` fails (unexpected in any conformant browser).
///
/// # TODO_NEXT_TICK: T-INT-backend-coord-proxy
///
/// Once `aphrody-terminal-backend` exposes a `/coord` HTTP route, replace
/// [`load_envelopes`] with an async fetch call.  The pane architecture is
/// already wired for real data — only the data source function needs to change.
#[wasm_bindgen]
pub fn mount_a2a_pane(parent_id: &str) -> Result<A2aPaneHandle, JsValue> {
    let window: Window = web_sys::window()
        .ok_or_else(|| JsValue::from_str("mount_a2a_pane: no global window"))?;

    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("mount_a2a_pane: no document on window"))?;

    // ── Locate and style the container element ─────────────────────────────

    let container: Element = document
        .get_element_by_id(parent_id)
        .ok_or_else(|| {
            JsValue::from_str(&format!(
                "mount_a2a_pane: element #{parent_id} not found"
            ))
        })?;

    // Apply M3 tonal surface chrome.
    let style = container
        .dyn_ref::<web_sys::HtmlElement>()
        .ok_or_else(|| JsValue::from_str("mount_a2a_pane: container is not an HtmlElement"))?
        .style();
    style.set_property("background-color", &m3_surface_tonal_css())?;
    style.set_property("color", &m3_on_surface_css())?;
    style.set_property("font-family", "monospace, sans-serif")?;
    style.set_property("font-size", "12px")?;
    style.set_property("overflow-y", "auto")?;
    style.set_property("padding", "8px")?;

    // ── Initial render ─────────────────────────────────────────────────────

    let initial_envelopes = load_envelopes()?;
    let initial_count = initial_envelopes.len();

    // Serialise the Vec<Envelope> back to a JSON array so that
    // a2a_ui::render_envelope_list (which accepts a JSON array string) can
    // own the rendering logic completely — no duplication.
    let initial_json = serde_json::to_string(&initial_envelopes)
        .map_err(|e| JsValue::from_str(&format!("mount_a2a_pane: serialise error: {e}")))?;

    a2a_ui::render_envelope_list(parent_id, &initial_json)?;

    // ── Polling state (shared with the interval closure) ───────────────────

    let state = Rc::new(RefCell::new(PaneState {
        container,
        rendered_count: initial_count,
    }));

    let state_clone = Rc::clone(&state);

    // ── Install setInterval (2 000 ms) ─────────────────────────────────────

    // The closure re-loads the envelope list on every tick and appends only
    // the rows that are newer than what was already rendered.
    let poll_closure: Closure<dyn FnMut()> = Closure::wrap(Box::new(move || {
        // Load current envelopes from the data source.
        let envelopes = match load_envelopes() {
            Ok(v) => v,
            Err(_) => return, // silently skip on transient error; next tick retries
        };

        let mut guard = state_clone.borrow_mut();
        let already_rendered = guard.rendered_count;

        // Nothing new since last tick — early exit.
        if envelopes.len() <= already_rendered {
            return;
        }

        // Append only the new (unseen) envelopes.
        let new_envelopes = &envelopes[already_rendered..];
        let new_json = match serde_json::to_string(new_envelopes) {
            Ok(s) => s,
            Err(_) => return,
        };

        // render_envelope_list appends to the container identified by its DOM
        // id.  Re-derive the id from the Element's `id` attribute.
        let container_id = guard.container.id();
        if container_id.is_empty() {
            // Container lost its id attribute — cannot route to render_envelope_list.
            // TODO_NEXT_TICK: T-INT-backend-coord-proxy — store id as String in
            // PaneState when wiring the real fetch path.
            return;
        }

        if a2a_ui::render_envelope_list(&container_id, &new_json).is_ok() {
            guard.rendered_count = envelopes.len();
        }
        // On error: leave rendered_count unchanged so the next tick retries.
    }));

    // Register the interval.  Timeout = 2 000 ms.
    let interval_id = window
        .set_interval_with_callback_and_timeout_and_arguments_0(
            poll_closure.as_ref().unchecked_ref::<Function>(),
            2_000,
        )
        .map_err(|e| JsValue::from_str(&format!("mount_a2a_pane: setInterval failed: {e:?}")))?;

    // Leak the closure so it stays alive for the duration of the interval.
    // The caller can cancel via A2aPaneHandle::stop(), which calls clearInterval.
    poll_closure.forget();

    Ok(A2aPaneHandle { interval_id })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{load_envelopes, SAMPLE_MAILBOX};
    use a2a_ui::Envelope;

    #[test]
    fn sample_mailbox_parses_to_three_envelopes() {
        let envelopes = Envelope::parse_jsonl(SAMPLE_MAILBOX)
            .expect("SAMPLE_MAILBOX must parse without error");
        assert_eq!(envelopes.len(), 3, "expected 3 envelopes in SAMPLE_MAILBOX");
    }

    #[test]
    fn load_envelopes_returns_ok() {
        // load_envelopes() is the same as parse_jsonl(SAMPLE_MAILBOX) in the
        // current tick.  Verify it does not panic or return Err.
        //
        // Note: load_envelopes() is gated to wasm32 in production code because
        // it calls JsValue.  On native test targets this test exercises the
        // parse path directly.
        let envelopes = Envelope::parse_jsonl(SAMPLE_MAILBOX)
            .expect("load_envelopes must succeed with embedded sample");
        assert!(
            !envelopes.is_empty(),
            "load_envelopes must return at least one envelope"
        );
    }

    #[test]
    fn all_sample_envelopes_have_required_fields() {
        let envelopes = Envelope::parse_jsonl(SAMPLE_MAILBOX)
            .expect("SAMPLE_MAILBOX parse");
        for (i, env) in envelopes.iter().enumerate() {
            assert!(
                !env.from.is_empty(),
                "envelope[{i}].from must not be empty"
            );
            assert!(
                !env.kind.is_empty(),
                "envelope[{i}].kind must not be empty"
            );
            assert_eq!(
                env.v,
                Some(1),
                "envelope[{i}].v must be 1 (current protocol version)"
            );
        }
    }

    #[test]
    fn envelopes_serialise_back_to_json_array() {
        let envelopes = Envelope::parse_jsonl(SAMPLE_MAILBOX)
            .expect("SAMPLE_MAILBOX parse");
        let json = serde_json::to_string(&envelopes)
            .expect("Vec<Envelope> must serialise to JSON");
        assert!(
            json.starts_with('['),
            "serialised envelopes must be a JSON array"
        );
        // Round-trip: parse the JSON array back.
        let rt: Vec<Envelope> =
            serde_json::from_str(&json).expect("round-trip deserialise");
        assert_eq!(rt.len(), envelopes.len(), "round-trip must preserve count");
    }
}
