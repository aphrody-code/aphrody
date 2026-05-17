// SPDX-License-Identifier: Apache-2.0
//! `shadcn Button` → Material Web Components 3 button family.
//!
//! Maps `variant` to one of:
//! * `"filled"`   → `<md-filled-button>`
//! * `"outlined"` → `<md-outlined-button>`
//! * `"text"`     → `<md-text-button>`
//! * `"tonal"`    → `<md-tonal-button>`
//!
//! Any other variant string falls back to `<md-filled-button>` (the M3 default
//! emphasis level).

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<Button>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ButtonProps {
    /// Inner text label shown to the user. Empty string allowed (icon-only).
    pub label: String,
    /// shadcn variant string — see module docs for the supported set.
    pub variant: String,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
    /// Opaque identifier surfaced on a `click` `CustomEvent`'s `detail.id`.
    /// JS listeners route events to handlers by inspecting `event.detail.id`.
    pub on_click_id: Option<String>,
}

impl ButtonProps {
    /// Number of declared public fields. Verified by the smoke test in
    /// `lib.rs::tests::button_module_smoke`.
    pub const FIELD_COUNT: usize = 4;
}

/// Builds the `<md-*-button>` element from `props`.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable (e.g. running in a Worker
/// context without a document).
#[cfg(target_arch = "wasm32")]
pub fn create_button(props: &ButtonProps) -> Result<HtmlElement, JsValue> {
    let tag = match props.variant.as_str() {
        "outlined" => "md-outlined-button",
        "text" => "md-text-button",
        "tonal" => "md-tonal-button",
        // shadcn defaults map to filled emphasis on M3.
        _ => "md-filled-button",
    };
    let el = create_mwc_element(tag)?;
    el.set_text_content(Some(&props.label));
    set_attr_bool(&el, "disabled", props.disabled)?;
    set_attr_opt(&el, "data-on-click-id", props.on_click_id.as_deref())?;
    Ok(el)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string to
/// avoid wiring a serde-wasm-bindgen dep at this layer.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_button)]
pub fn create_button_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: ButtonProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid ButtonProps JSON: {e}")))?;
    create_button(&props)
}
