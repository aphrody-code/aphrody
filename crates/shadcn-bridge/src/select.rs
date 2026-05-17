// SPDX-License-Identifier: Apache-2.0
//! `shadcn Select` → `<md-outlined-select>` + `<md-select-option>`.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")] use crate::{create_mwc_element, set_attr_bool};

/// Option entry mirroring shadcn's `{ value, label }` pair.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelectOption {
    /// Underlying form value (submitted on change).
    pub value: String,
    /// User-visible label.
    pub label: String,
}

/// Subset of shadcn `<Select>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelectProps {
    /// Floating label text.
    pub label: String,
    /// Currently selected value (matched against `SelectOption::value`).
    pub value: String,
    /// Option list in display order.
    pub options: Vec<SelectOption>,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
}

impl SelectProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::select_module_smoke`.
    pub const FIELD_COUNT: usize = 4;
}

/// Builds an `<md-outlined-select>` with one `<md-select-option>` per entry.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_select(props: &SelectProps) -> Result<HtmlElement, JsValue> {
    let el = create_mwc_element("md-outlined-select")?;
    el.set_attribute("label", &props.label)?;
    el.set_attribute("value", &props.value)?;
    set_attr_bool(&el, "disabled", props.disabled)?;

    for opt in &props.options {
        let option = create_mwc_element("md-select-option")?;
        option.set_attribute("value", &opt.value)?;
        if opt.value == props.value {
            option.set_attribute("selected", "")?;
        }
        option.set_text_content(Some(&opt.label));
        el.append_child(&option)?;
    }

    Ok(el)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_select)]
pub fn create_select_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: SelectProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid SelectProps JSON: {e}")))?;
    create_select(&props)
}
