// SPDX-License-Identifier: Apache-2.0
//! `shadcn Checkbox` → `<md-checkbox>`.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<Checkbox>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckboxProps {
    /// Mirrors HTML `checked`.
    pub checked: bool,
    /// Mirrors MWC3's `indeterminate` tri-state.
    pub indeterminate: bool,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
    /// CustomEvent id dispatched on `change`.
    pub on_change_id: Option<String>,
}

impl CheckboxProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::checkbox_module_smoke`.
    pub const FIELD_COUNT: usize = 4;
}

/// Builds an `<md-checkbox>` element.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_checkbox(props: &CheckboxProps) -> Result<HtmlElement, JsValue> {
    let el = create_mwc_element("md-checkbox")?;
    set_attr_bool(&el, "checked", props.checked)?;
    set_attr_bool(&el, "indeterminate", props.indeterminate)?;
    set_attr_bool(&el, "disabled", props.disabled)?;
    set_attr_opt(&el, "data-on-change-id", props.on_change_id.as_deref())?;
    Ok(el)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_checkbox)]
pub fn create_checkbox_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: CheckboxProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid CheckboxProps JSON: {e}")))?;
    create_checkbox(&props)
}
