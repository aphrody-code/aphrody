// SPDX-License-Identifier: Apache-2.0
//! `shadcn Switch` → `<md-switch>`.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<Switch>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwitchProps {
    /// Mirrors HTML `checked` (i.e. MWC3 `selected`).
    pub checked: bool,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
    /// CustomEvent id dispatched on `change`.
    pub on_change_id: Option<String>,
}

impl SwitchProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::switch_module_smoke`.
    pub const FIELD_COUNT: usize = 3;
}

/// Builds an `<md-switch>` element.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_switch(props: &SwitchProps) -> Result<HtmlElement, JsValue> {
    let el = create_mwc_element("md-switch")?;
    // MWC3 uses `selected`, not `checked`, for the boolean visual state.
    set_attr_bool(&el, "selected", props.checked)?;
    set_attr_bool(&el, "disabled", props.disabled)?;
    set_attr_opt(&el, "data-on-change-id", props.on_change_id.as_deref())?;
    Ok(el)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_switch)]
pub fn create_switch_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: SwitchProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid SwitchProps JSON: {e}")))?;
    create_switch(&props)
}
