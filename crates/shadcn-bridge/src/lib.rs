// SPDX-License-Identifier: Apache-2.0
//! shadcn-bridge — Rust ⇆ Material Web Components 3 bridge for shadcn primitives.
//!
//! This crate scaffolds the binding surface that lets a Rust/WASM front-end
//! re-create the shadcn React component API on top of Google's official
//! `@material/web` custom elements (Material Web Components 3, MWC3). The
//! bridge is intentionally **thin**: each shadcn primitive maps to one or more
//! MWC3 custom elements, with a `*Props` struct mirroring the original shadcn
//! prop names (PascalCase → snake_case).
//!
//! The intent — per `project_aphrody_ultimate_goals` memory (2026-05-17) — is
//! to migrate shadcn-ui consumers to Material Web Components 3 *natively* via
//! `bxc` scraping + Rust wasm-bindgen, without ever shipping the original
//! React/JS shadcn implementation in the runtime bundle.
//!
//! ## Mapping table
//!
//! | shadcn primitive | MWC3 element(s)                                                       |
//! |------------------|-----------------------------------------------------------------------|
//! | `Button`         | `<md-filled-button>`, `<md-outlined-button>`, `<md-text-button>`, `<md-tonal-button>` |
//! | `Input`          | `<md-outlined-text-field>`, `<md-filled-text-field>`                  |
//! | `Card`           | bespoke `<div class="md-elevated-card">` w/ M3 elevation token        |
//! | `Dialog`         | `<md-dialog>`                                                         |
//! | `Tabs`           | `<md-tabs>` + `<md-primary-tab>` / `<md-secondary-tab>`               |
//! | `Toast`          | `<md-snackbar>` (fallback: bespoke `<div class="md-snackbar">`)       |
//! | `Select`         | `<md-outlined-select>` + `<md-select-option>`                         |
//! | `Checkbox`       | `<md-checkbox>`                                                       |
//! | `RadioGroup`     | `<md-radio>` (one per option)                                         |
//! | `Switch`         | `<md-switch>`                                                         |
//! | `Slider`         | `<md-slider>`                                                         |
//! | `Avatar`         | bespoke `<div class="md-avatar">` wrapping `<img>` + M3 elevation     |
//!
//! ## API shape
//!
//! Each module exposes:
//!
//! 1. A `<Name>Props` struct (`serde`-friendly) reflecting the minimal shadcn
//!    prop surface.
//! 2. A pure-Rust `create_<name>(props: &<Name>Props) -> Result<HtmlElement, JsValue>`
//!    entrypoint that builds the DOM subtree using `web_sys`.
//! 3. A `#[wasm_bindgen]`-annotated thin wrapper that JS callers can invoke
//!    directly (`shadcnBridge.create_button(propsJson)`).
//!
//! The native-host build skips every `web_sys` call — modules still compile so
//! that `cargo check` / `cargo test` succeed on Linux + Windows hosts (only
//! the `Props` structs and their tests are exercised).

#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{Document, HtmlElement};

// ---------------------------------------------------------------------------
// Shared internals — only meaningful on wasm32, but we keep the signatures
// host-portable so consumer code stays uniform.
// ---------------------------------------------------------------------------

/// Material Design 3 default elevation token (in `dp`) for surface containers
/// such as cards and avatars. Values mirror the `md.sys.elevation.level*` spec
/// — see <https://m3.material.io/styles/elevation/tokens>. Level 1 is the
/// default resting elevation for an elevated card; the constant is exported
/// so card / avatar consumers stay consistent without re-typing literals.
pub const M3_ELEVATION_LEVEL_1_DP: u8 = 1;
/// Material Design 3 elevation level 0 — used when a surface is flush with
/// its parent (e.g. flat cards on inverse-surface backgrounds).
pub const M3_ELEVATION_LEVEL_0_DP: u8 = 0;

/// Returns the current `Document`, or a `JsValue` error if it cannot be
/// resolved (e.g. running inside a Worker without a DOM).
#[cfg(target_arch = "wasm32")]
fn document() -> Result<Document, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    window
        .document()
        .ok_or_else(|| JsValue::from_str("no document on window"))
}

/// Creates a Material Web Components custom element by tag name and casts it
/// to [`HtmlElement`] so callers can set common DOM attributes uniformly.
#[cfg(target_arch = "wasm32")]
fn create_mwc_element(tag: &str) -> Result<HtmlElement, JsValue> {
    let el = document()?.create_element(tag)?;
    el.dyn_into::<HtmlElement>()
        .map_err(|_| JsValue::from_str("create_element did not return an HtmlElement"))
}

/// Sets a string attribute on an element, no-op if `value` is `None`.
#[cfg(target_arch = "wasm32")]
fn set_attr_opt(el: &HtmlElement, name: &str, value: Option<&str>) -> Result<(), JsValue> {
    if let Some(v) = value {
        el.set_attribute(name, v)?;
    }
    Ok(())
}

/// Sets a boolean attribute (presence-only) when `flag` is true.
#[cfg(target_arch = "wasm32")]
fn set_attr_bool(el: &HtmlElement, name: &str, flag: bool) -> Result<(), JsValue> {
    if flag {
        el.set_attribute(name, "")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// `start` hook — wired so JS bundlers that call `init()` get a panic hook for
// free. Equivalent to `aphrody-wasm::start`, but defined locally so this crate
// does not pull `console_error_panic_hook` transitively for native builds.
// ---------------------------------------------------------------------------
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    // Intentionally minimal — consumer apps install their own panic hook.
}

// ---------------------------------------------------------------------------
// Public modules — one per shadcn primitive.
// ---------------------------------------------------------------------------
// v1: 12 shadcn primitives.
pub mod button;
pub mod input;
pub mod card;
pub mod dialog;
pub mod tabs;
pub mod toast;
pub mod select;
pub mod checkbox;
pub mod radio_group;
pub mod switch;
pub mod slider;
pub mod avatar;

// v2 batch A: 8 more M3 components (tick 37).
pub mod list;
pub mod navigation_bar;
pub mod navigation_drawer;
pub mod navigation_rail;
pub mod app_bar;
pub mod fab;
pub mod chip;
pub mod progress;

// v2 batch B: 8 more M3 components (tick 37).
pub mod badge;
pub mod tooltip;
pub mod icon_button;
pub mod segmented_button;
pub mod bottom_sheet;
pub mod date_picker;
pub mod time_picker;
pub mod search_bar;

// Gemini-specific composable atoms (tick 37 — Gemini clone pixel-perfect).
pub mod gemini;

// ---------------------------------------------------------------------------
// Native-host smoke tests — assert each Props struct is constructible with a
// deterministic field count (proxy for "this module compiles and the API
// surface didn't accidentally shrink"). One test per module, 12 total.
// ---------------------------------------------------------------------------
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    // Each test below asserts the module's Props struct exposes the expected
    // *minimum* number of fields by JSON-serialising a default instance and
    // counting the resulting object keys. The count tracks the shadcn-ui v0.8
    // prop surface (label/variant/disabled/...) and acts as a regression
    // guard against accidental API shrink.

    /// Counts the top-level keys in the JSON object produced by `value`.
    /// Panics if the value does not serialise to a JSON object — which is
    /// exactly what we want: any non-struct payload is an immediate test
    /// failure rather than a silent "0 fields".
    fn json_field_count<T: serde::Serialize>(value: &T) -> usize {
        let v = serde_json::to_value(value).expect("serde must serialise Props to JSON value");
        match v {
            serde_json::Value::Object(map) => map.len(),
            other => panic!("expected JSON object, got {other:?}"),
        }
    }

    #[test]
    fn button_module_smoke() {
        // label, variant, disabled, on_click_id = 4 fields
        let p = button::ButtonProps::default();
        assert_eq!(json_field_count(&p), 4);
        assert_eq!(button::ButtonProps::FIELD_COUNT, 4);
    }

    #[test]
    fn input_module_smoke() {
        // label, value, placeholder, variant, disabled, on_input_id = 6
        let p = input::InputProps::default();
        assert_eq!(json_field_count(&p), 6);
        assert_eq!(input::InputProps::FIELD_COUNT, 6);
    }

    #[test]
    fn card_module_smoke() {
        // title, body, elevation_dp = 3
        let p = card::CardProps::default();
        assert_eq!(json_field_count(&p), 3);
        assert_eq!(card::CardProps::FIELD_COUNT, 3);
    }

    #[test]
    fn dialog_module_smoke() {
        // open, headline, body, primary_action_id, secondary_action_id = 5
        let p = dialog::DialogProps::default();
        assert_eq!(json_field_count(&p), 5);
        assert_eq!(dialog::DialogProps::FIELD_COUNT, 5);
    }

    #[test]
    fn tabs_module_smoke() {
        // tabs, active_index, variant = 3
        let p = tabs::TabsProps::default();
        assert_eq!(json_field_count(&p), 3);
        assert_eq!(tabs::TabsProps::FIELD_COUNT, 3);
    }

    #[test]
    fn toast_module_smoke() {
        // message, action_label, on_action_id, open = 4
        let p = toast::ToastProps::default();
        assert_eq!(json_field_count(&p), 4);
        assert_eq!(toast::ToastProps::FIELD_COUNT, 4);
    }

    #[test]
    fn select_module_smoke() {
        // label, value, options, disabled = 4
        let p = select::SelectProps::default();
        assert_eq!(json_field_count(&p), 4);
        assert_eq!(select::SelectProps::FIELD_COUNT, 4);
    }

    #[test]
    fn checkbox_module_smoke() {
        // checked, indeterminate, disabled, on_change_id = 4
        let p = checkbox::CheckboxProps::default();
        assert_eq!(json_field_count(&p), 4);
        assert_eq!(checkbox::CheckboxProps::FIELD_COUNT, 4);
    }

    #[test]
    fn radio_group_module_smoke() {
        // name, value, options, on_change_id = 4
        let p = radio_group::RadioGroupProps::default();
        assert_eq!(json_field_count(&p), 4);
        assert_eq!(radio_group::RadioGroupProps::FIELD_COUNT, 4);
    }

    #[test]
    fn switch_module_smoke() {
        // checked, disabled, on_change_id = 3
        let p = switch::SwitchProps::default();
        assert_eq!(json_field_count(&p), 3);
        assert_eq!(switch::SwitchProps::FIELD_COUNT, 3);
    }

    #[test]
    fn slider_module_smoke() {
        // value, min, max, step, disabled, on_change_id = 6
        let p = slider::SliderProps::default();
        assert_eq!(json_field_count(&p), 6);
        assert_eq!(slider::SliderProps::FIELD_COUNT, 6);
    }

    #[test]
    fn avatar_module_smoke() {
        // src, alt, size_px, elevation_dp = 4
        let p = avatar::AvatarProps::default();
        assert_eq!(json_field_count(&p), 4);
        assert_eq!(avatar::AvatarProps::FIELD_COUNT, 4);
    }
}
