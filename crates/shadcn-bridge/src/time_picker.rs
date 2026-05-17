// SPDX-License-Identifier: Apache-2.0
//! `shadcn TimePicker` → bespoke `<div class="md-time-picker">` rendering a
//! Material 3 clock face.
//!
//! Material Web Components 3 does not ship a time-picker custom element
//! (`@material/web` as of the 2026-05 release train still has
//! `<md-time-picker>` in draft). The bridge therefore emits the M3
//! clock-face recipe directly: a digital readout (hours + minutes + optional
//! AM/PM toggle) followed by a 12-position clock dial of `<button>` cells
//! arranged radially via CSS custom properties (`--md-time-picker-tick-{n}`
//! is set on each cell so the consumer stylesheet can position them).
//!
//! Selection logic is delegated to JS via the `data-on-select-id`
//! `CustomEvent` channel — keeping the wasm bundle minimal.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")] use crate::{document, set_attr_opt};

/// Subset of shadcn `<TimePicker>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimePickerProps {
    /// Selected hour 0..=23 (24-hour clock internally; AM/PM toggle is a
    /// display concern only).
    pub hour: u8,
    /// Selected minute 0..=59.
    pub minute: u8,
    /// When true, render the AM/PM toggle and a 12-tick dial. When false,
    /// render the 24-tick dial in M3 input-mode style.
    pub twelve_hour: bool,
    /// Mirrors HTML `disabled` — prevents all interaction.
    pub disabled: bool,
    /// CustomEvent id dispatched on tick click (`detail.hour`/`detail.minute`).
    pub on_select_id: Option<String>,
}

impl TimePickerProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 5;
}

/// Builds the `<div class="md-time-picker">` subtree from `props`.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_time_picker(props: &TimePickerProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let root = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    root.set_class_name("md-time-picker");
    root.set_attribute("role", "application")?;
    if props.disabled {
        root.set_attribute("aria-disabled", "true")?;
    }
    set_attr_opt(&root, "data-on-select-id", props.on_select_id.as_deref())?;

    let hour = props.hour.min(23);
    let minute = props.minute.min(59);

    // Digital readout — HH:MM, zero-padded.
    let readout = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    readout.set_class_name("md-time-picker__readout");
    let display_hour = if props.twelve_hour {
        let h12 = hour % 12;
        if h12 == 0 { 12 } else { h12 }
    } else {
        hour
    };
    readout.set_text_content(Some(&format!("{display_hour:02}:{minute:02}")));
    root.append_child(&readout)?;

    if props.twelve_hour {
        let toggle = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
        toggle.set_class_name("md-time-picker__ampm");
        toggle.set_attribute("data-meridiem", if hour < 12 { "AM" } else { "PM" })?;
        root.append_child(&toggle)?;
    }

    // Clock dial.
    let dial = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    dial.set_class_name("md-time-picker__dial");
    let tick_count: u8 = if props.twelve_hour { 12 } else { 24 };
    for tick in 0..tick_count {
        let cell = doc.create_element("button")?.dyn_into::<HtmlElement>()?;
        cell.set_class_name("md-time-picker__tick");
        cell.set_attribute("type", "button")?;
        let label_value = if props.twelve_hour { if tick == 0 { 12 } else { tick } } else { tick };
        cell.set_text_content(Some(&format!("{label_value:02}")));
        cell.set_attribute("data-tick", &tick.to_string())?;
        cell.set_attribute("style", &format!("--md-time-picker-tick-index: {tick};"))?;
        // Compare against the *normalised* selected tick so the dial-selected
        // state survives AM/PM round-trips.
        let selected_tick = if props.twelve_hour { hour % 12 } else { hour };
        if selected_tick == tick {
            cell.set_attribute("aria-selected", "true")?;
            cell.set_attribute("data-selected", "true")?;
        }
        if props.disabled {
            cell.set_attribute("disabled", "")?;
        }
        dial.append_child(&cell)?;
    }
    root.append_child(&dial)?;

    Ok(root)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_time_picker)]
pub fn create_time_picker_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: TimePickerProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid TimePickerProps JSON: {e}")))?;
    create_time_picker(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Smoke test: default Props serialises to a `FIELD_COUNT`-key JSON object.
    #[test]
    fn time_picker_props_default_field_count() {
        let p = TimePickerProps::default();
        let v = serde_json::to_value(&p).expect("serde must serialise TimePickerProps");
        let map = v.as_object().expect("TimePickerProps must serialise as object");
        assert_eq!(map.len(), TimePickerProps::FIELD_COUNT);
    }
}
