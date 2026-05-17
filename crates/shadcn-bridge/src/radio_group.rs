// SPDX-License-Identifier: Apache-2.0
//! `shadcn RadioGroup` → one `<md-radio>` per option, all sharing the same
//! `name` attribute (MWC3 groups radios by name, not by parent container).

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, document};

/// Option entry for the radio group (label + form value pair).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RadioOption {
    /// Underlying form value (submitted on change).
    pub value: String,
    /// User-visible label, rendered next to the radio.
    pub label: String,
}

/// Subset of shadcn `<RadioGroup>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RadioGroupProps {
    /// Shared `name` attribute — required for MWC3 to group the radios.
    pub name: String,
    /// Currently selected value.
    pub value: String,
    /// Option list in display order.
    pub options: Vec<RadioOption>,
    /// CustomEvent id dispatched on `change`.
    pub on_change_id: Option<String>,
}

impl RadioGroupProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::radio_group_module_smoke`.
    pub const FIELD_COUNT: usize = 4;
}

/// Builds a `<div class="md-radio-group">` containing one `<md-radio>` +
/// `<label>` pair per option.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_radio_group(props: &RadioGroupProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let group = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    group.set_class_name("md-radio-group");
    group.set_attribute("role", "radiogroup")?;
    group.set_attribute("data-name", &props.name)?;

    for opt in &props.options {
        let radio = create_mwc_element("md-radio")?;
        radio.set_attribute("name", &props.name)?;
        radio.set_attribute("value", &opt.value)?;
        if opt.value == props.value {
            radio.set_attribute("checked", "")?;
        }
        if let Some(id) = props.on_change_id.as_deref() {
            radio.set_attribute("data-on-change-id", id)?;
        }
        let label = doc.create_element("label")?.dyn_into::<HtmlElement>()?;
        label.set_text_content(Some(&opt.label));
        group.append_child(&radio)?;
        group.append_child(&label)?;
    }

    Ok(group)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_radio_group)]
pub fn create_radio_group_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: RadioGroupProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid RadioGroupProps JSON: {e}")))?;
    create_radio_group(&props)
}
