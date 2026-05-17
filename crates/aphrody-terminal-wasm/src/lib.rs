// SPDX-License-Identifier: Apache-2.0
//! wasm-bindgen DOM renderer for [`aphrody_terminal_vt::TerminalState`].
//!
//! This crate is unconditionally gated to `target_arch = "wasm32"` — every
//! public item is wrapped in `#[cfg(target_arch = "wasm32")]`.  On native
//! targets `cargo check --workspace` sees an empty crate and passes cleanly.
//!
//! # Usage (JS/TS side)
//!
//! ```js
//! import init, { TerminalHandle } from './pkg/aphrody_terminal_wasm.js';
//! await init();
//! const term = TerminalHandle.new("terminal-div", 80, 24);
//! term.set_on_data(bytes => { ws.send(bytes); });
//! ws.onmessage = e => term.write(new Uint8Array(e.data));
//! ```

#![cfg(target_arch = "wasm32")]

use aphrody_terminal_vt::{Color, TerminalState};
use js_sys::Function;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent, Node};

// ── Panic hook (installed once at module initialisation) ─────────────────────

#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

// ── Colour helpers ────────────────────────────────────────────────────────────

/// Format an [`Color`] as a CSS `rgb(r, g, b)` string.
fn css_rgb(c: Color) -> String {
    format!("rgb({},{},{})", c.r, c.g, c.b)
}

// ── M3 surface colour ─────────────────────────────────────────────────────────

/// The M3 baseline surface background colour as a CSS hex string.
///
/// `BASELINE.surface` is an ARGB `u32` (`0xFFrrggbb`).  We strip the alpha
/// byte and format the RGB part.
fn m3_surface_css() -> String {
    let argb = m3_tokens::color::BASELINE.surface;
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    format!("#{r:02X}{g:02X}{b:02X}")
}

// ── TerminalHandle ────────────────────────────────────────────────────────────

/// A live terminal instance bound to a DOM element.
///
/// The struct owns the [`TerminalState`] (VT parser + cell grid) and keeps a
/// reference to the root `<div>` element in the browser document.  The DOM
/// grid is a flat `<span>` per cell arranged inside `<div class="row">` rows.
///
/// The `on_data` callback is invoked with a `Uint8Array` whenever the user
/// generates input (key presses).  Connect it to a WebSocket or any other
/// transport to implement the pty round-trip.
#[wasm_bindgen]
pub struct TerminalHandle {
    state: TerminalState,
    /// Root container element (the div passed by `target_id`).
    root: HtmlElement,
    /// Approximate character width in pixels (used for sizing).
    cell_w_px: f64,
    /// Approximate character height in pixels.
    cell_h_px: f64,
    /// Optional JS callback invoked with a `Uint8Array` on user input.
    on_data: Option<Function>,
}

#[wasm_bindgen]
impl TerminalHandle {
    /// Create a new terminal attached to the DOM element with `id = target_id`.
    ///
    /// # Errors
    ///
    /// Returns a [`JsValue`] error string if:
    /// - `window` or `document` is unavailable (non-browser context).
    /// - No element with `target_id` exists in the document.
    #[wasm_bindgen]
    pub fn new(target_id: &str, cols: u16, rows: u16) -> Result<TerminalHandle, JsValue> {
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("no window object"))?;
        let document: Document = window.document()
            .ok_or_else(|| JsValue::from_str("no document"))?;

        let root: HtmlElement = document
            .get_element_by_id(target_id)
            .ok_or_else(|| JsValue::from_str(&format!("element #{target_id} not found")))?
            .dyn_into::<HtmlElement>()
            .map_err(|_| JsValue::from_str("element is not an HtmlElement"))?;

        // Apply M3 surface background and monospace font.
        let style = root.style();
        style.set_property("background-color", &m3_surface_css())?;
        style.set_property("font-family", "monospace")?;
        style.set_property("font-size", "14px")?;
        style.set_property("line-height", "1.2")?;
        style.set_property("white-space", "pre")?;
        style.set_property("overflow", "hidden")?;
        style.set_property("cursor", "text")?;
        // Make the container focusable so it can receive keyboard events.
        root.set_tab_index(0);

        let state = TerminalState::new(cols, rows);

        // Build the initial DOM grid.
        let mut handle = TerminalHandle {
            state,
            root,
            cell_w_px: 8.4,
            cell_h_px: 16.8,
            on_data: None,
        };
        handle.build_dom_grid(&document, cols, rows)?;

        Ok(handle)
    }

    /// Feed raw bytes from the pty into the terminal state and repaint dirty rows.
    pub fn write(&mut self, bytes: &[u8]) {
        self.state.feed(bytes);
        let dirty = self.state.dirty_rows_drain();
        if dirty.is_empty() {
            return;
        }
        // Repaint only the dirty rows.
        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                for row_idx in dirty {
                    let _ = self.repaint_row(&doc, row_idx);
                }
            }
        }
    }

    /// Resize the terminal grid and rebuild the DOM to match.
    ///
    /// # Errors
    ///
    /// Propagates any DOM manipulation error.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), JsValue> {
        self.state.resize(cols, rows);
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window.document()
            .ok_or_else(|| JsValue::from_str("no document"))?;
        // Clear existing children.
        while let Some(child) = self.root.first_child() {
            self.root.remove_child(&child)?;
        }
        self.build_dom_grid(&document, cols, rows)?;
        Ok(())
    }

    /// Focus the terminal root element so it receives keyboard events.
    pub fn focus(&self) {
        let _ = self.root.focus();
    }

    /// Register a JS callback invoked with a `Uint8Array` on each user input event.
    pub fn set_on_data(&mut self, cb: Function) {
        self.on_data = Some(cb);
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Build the DOM grid from scratch: `rows` × `<div class="row">` each
    /// containing `cols` × `<span>` elements.  One span per cell.
    fn build_dom_grid(
        &mut self,
        document: &Document,
        cols: u16,
        rows: u16,
    ) -> Result<(), JsValue> {
        for r in 0..rows {
            let row_div: HtmlElement = document
                .create_element("div")?
                .dyn_into::<HtmlElement>()?;
            row_div.set_class_name("row");
            let row_style = row_div.style();
            row_style.set_property("display", "block")?;
            row_style.set_property("height", &format!("{}px", self.cell_h_px))?;
            row_style.set_property("white-space", "pre")?;

            for c in 0..cols {
                let span: HtmlElement = document
                    .create_element("span")?
                    .dyn_into::<HtmlElement>()?;
                let cell = self.state.cell(r, c);
                span.set_text_content(Some(&char_to_str(cell.ch)));
                let s = span.style();
                s.set_property("color", &css_rgb(cell.fg))?;
                s.set_property("background-color", &css_rgb(cell.bg))?;
                apply_attr_style(&s, cell.attr)?;
                row_div.append_child(&span)?;
            }
            self.root.append_child(&row_div)?;
        }

        // Attach keydown listener on the root element.
        // We clone the `on_data` callback into the closure.
        let on_data_ref = self.on_data.clone();
        let closure = Closure::wrap(Box::new(move |evt: KeyboardEvent| {
            let bytes = key_to_ansi_bytes(&evt);
            if bytes.is_empty() {
                return;
            }
            // Prevent default browser action (e.g. scrolling on Arrow keys).
            evt.prevent_default();
            if let Some(cb) = &on_data_ref {
                let arr = js_sys::Uint8Array::from(bytes.as_slice());
                let _ = cb.call1(&JsValue::NULL, &arr);
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);

        self.root
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        // Leak the closure so it remains alive for the lifetime of the element.
        // This is intentional: the listener lives as long as the DOM element does.
        closure.forget();

        Ok(())
    }

    /// Repaint a single row by updating the text content and styles of its spans.
    fn repaint_row(&self, document: &Document, row: u16) -> Result<(), JsValue> {
        let _ = document; // document not needed; we traverse the existing DOM.
        let row_node: Node = self
            .root
            .child_nodes()
            .item(u32::from(row))
            .ok_or_else(|| JsValue::from_str("row out of range"))?;
        let row_el: HtmlElement = row_node
            .dyn_into::<HtmlElement>()
            .map_err(|_| JsValue::from_str("row is not HtmlElement"))?;
        let spans = row_el.child_nodes();

        for c in 0..self.state.cols() {
            let span_node = spans
                .item(u32::from(c))
                .ok_or_else(|| JsValue::from_str("span out of range"))?;
            let span: HtmlElement = span_node
                .dyn_into::<HtmlElement>()
                .map_err(|_| JsValue::from_str("span is not HtmlElement"))?;
            let cell = self.state.cell(row, c);
            span.set_text_content(Some(&char_to_str(cell.ch)));
            let s = span.style();
            s.set_property("color", &css_rgb(cell.fg))?;
            s.set_property("background-color", &css_rgb(cell.bg))?;
            apply_attr_style(&s, cell.attr)?;
        }
        Ok(())
    }
}

// ── DOM helpers ───────────────────────────────────────────────────────────────

/// Render a cell character to a one-char string suitable for `textContent`.
///
/// Non-printable characters (including NUL) are replaced with a space so that
/// the DOM span always has exactly one visible glyph width.
fn char_to_str(ch: char) -> String {
    if ch == '\0' || ch.is_control() {
        " ".to_owned()
    } else {
        ch.to_string()
    }
}

/// Apply attribute flags to a CSS style declaration.
fn apply_attr_style(
    style: &web_sys::CssStyleDeclaration,
    attr: aphrody_terminal_vt::Attr,
) -> Result<(), JsValue> {
    use aphrody_terminal_vt::Attr;
    style.set_property(
        "font-weight",
        if attr.contains(Attr::BOLD) { "bold" } else { "normal" },
    )?;
    style.set_property(
        "font-style",
        if attr.contains(Attr::ITALIC) { "italic" } else { "normal" },
    )?;
    let decoration = if attr.contains(Attr::UNDERLINE) {
        "underline"
    } else {
        "none"
    };
    style.set_property("text-decoration", decoration)?;
    // INVERSE: swap fg/bg at the call site when building cell colours, so no
    // extra CSS property is needed here — the colours are already swapped in
    // the cell data by the VT parser.
    Ok(())
}

// ── Keyboard mapping ──────────────────────────────────────────────────────────

/// Convert a [`KeyboardEvent`] to the ANSI byte sequence it should generate.
///
/// Returns an empty `Vec` for keys that produce no terminal input (modifier
/// keys alone, unrecognised keys, etc.).
fn key_to_ansi_bytes(evt: &KeyboardEvent) -> Vec<u8> {
    let key = evt.key();
    match key.as_str() {
        // Named keys with fixed ANSI/VT mappings.
        "Enter"     => vec![b'\r'],
        "Backspace" => vec![0x7F],
        "Tab"       => vec![b'\t'],
        "Escape"    => vec![0x1B],
        "ArrowUp"   => vec![0x1B, b'[', b'A'],
        "ArrowDown" => vec![0x1B, b'[', b'B'],
        "ArrowRight"=> vec![0x1B, b'[', b'C'],
        "ArrowLeft" => vec![0x1B, b'[', b'D'],
        "Home"      => vec![0x1B, b'[', b'H'],
        "End"       => vec![0x1B, b'[', b'F'],
        "PageUp"    => vec![0x1B, b'[', b'5', b'~'],
        "PageDown"  => vec![0x1B, b'[', b'6', b'~'],
        "Delete"    => vec![0x1B, b'[', b'3', b'~'],
        "Insert"    => vec![0x1B, b'[', b'2', b'~'],
        // Printable single-character keys: use the UTF-8 encoding directly.
        k if k.chars().count() == 1 => {
            let ch = k.chars().next();
            // invariant: k has exactly one char, so next() returns Some
            let ch = ch.unwrap_or(' ');
            let mut buf = [0u8; 4];
            ch.encode_utf8(&mut buf);
            buf[..ch.len_utf8()].to_vec()
        }
        // Modifier-only keys and unrecognised sequences produce no output.
        _ => vec![],
    }
}
