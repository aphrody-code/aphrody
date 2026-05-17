// SPDX-License-Identifier: Apache-2.0
//! `shadcn Input` → `<md-outlined-text-field>` / `<md-filled-text-field>`.
//!
//! `variant = "filled"` yields `<md-filled-text-field>`. Any other value
//! (including the empty string) yields `<md-outlined-text-field>`, matching
//! the shadcn default of an outlined input.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<Input>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputProps {
    /// Floating label text.
    pub label: String,
    /// Current value (controlled component).
    pub value: String,
    /// Placeholder text (visible when value is empty).
    pub placeholder: Option<String>,
    /// `"filled"` or `"outlined"` (default).
    pub variant: String,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
    /// Opaque identifier surfaced on `input` `CustomEvent`'s `detail.id`.
    pub on_input_id: Option<String>,
}

impl InputProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::input_module_smoke`.
    pub const FIELD_COUNT: usize = 6;
}

/// Builds the `<md-*-text-field>` element.
///
/// # Errors
/// Returns [`JsValue`] when the DOM cannot be resolved.
#[cfg(target_arch = "wasm32")]
pub fn create_input(props: &InputProps) -> Result<HtmlElement, JsValue> {
    let tag =
        if props.variant == "filled" { "md-filled-text-field" } else { "md-outlined-text-field" };
    let el = create_mwc_element(tag)?;
    el.set_attribute("label", &props.label)?;
    el.set_attribute("value", &props.value)?;
    set_attr_opt(&el, "placeholder", props.placeholder.as_deref())?;
    set_attr_bool(&el, "disabled", props.disabled)?;
    set_attr_opt(&el, "data-on-input-id", props.on_input_id.as_deref())?;
    Ok(el)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_input)]
pub fn create_input_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: InputProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid InputProps JSON: {e}")))?;
    create_input(&props)
}
