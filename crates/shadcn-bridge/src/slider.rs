// SPDX-License-Identifier: Apache-2.0
//! `shadcn Slider` → `<md-slider>`.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<Slider>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliderProps {
    /// Current value.
    pub value: f64,
    /// Minimum value (inclusive).
    pub min: f64,
    /// Maximum value (inclusive).
    pub max: f64,
    /// Step increment.
    pub step: f64,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
    /// CustomEvent id dispatched on `input`.
    pub on_change_id: Option<String>,
}

impl Default for SliderProps {
    fn default() -> Self {
        Self { value: 0.0, min: 0.0, max: 100.0, step: 1.0, disabled: false, on_change_id: None }
    }
}

impl SliderProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::slider_module_smoke`.
    pub const FIELD_COUNT: usize = 6;
}

/// Builds an `<md-slider>` element.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_slider(props: &SliderProps) -> Result<HtmlElement, JsValue> {
    let el = create_mwc_element("md-slider")?;
    el.set_attribute("value", &props.value.to_string())?;
    el.set_attribute("min", &props.min.to_string())?;
    el.set_attribute("max", &props.max.to_string())?;
    el.set_attribute("step", &props.step.to_string())?;
    set_attr_bool(&el, "disabled", props.disabled)?;
    set_attr_opt(&el, "data-on-change-id", props.on_change_id.as_deref())?;
    Ok(el)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_slider)]
pub fn create_slider_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: SliderProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid SliderProps JSON: {e}")))?;
    create_slider(&props)
}
