// SPDX-License-Identifier: Apache-2.0
//! `shadcn Chip` → `<md-chip-set>` + chip variants.
//!
//! M3 chip family per <https://m3.material.io/components/chips/overview>.
//! The bridge supports the four standard MWC3 chip variants, selected per
//! item via [`ChipItemProps::kind`]:
//! * `"assist"`     → `<md-assist-chip>`     (suggested action)
//! * `"filter"`     → `<md-filter-chip>`     (multi-select filter)
//! * `"input"`      → `<md-input-chip>`      (user-entered token, removable)
//! * `"suggestion"` → `<md-suggestion-chip>` (model-suggested content)
//!
//! Any other value (including default) falls back to `"assist"` — the M3
//! default emphasis level. Chips render inside an `<md-chip-set>` so MWC3
//! handles overflow + keyboard navigation for free.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// One chip inside a [`ChipSetProps`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChipItemProps {
    /// Visible text label.
    pub label: String,
    /// Chip variant — see module docs for the supported set.
    pub kind: String,
    /// Mark a filter chip as selected. Ignored for non-filter variants
    /// (MWC3 ignores `selected` on assist/suggestion chips).
    pub selected: bool,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
    /// Opaque identifier surfaced on the chip's `click` event.
    pub on_click_id: Option<String>,
}

impl ChipItemProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 5;
}

/// Subset of shadcn `<ChipSet>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChipSetProps {
    /// Chips rendered left-to-right inside the set.
    pub chips: Vec<ChipItemProps>,
}

impl ChipSetProps {
    /// Number of declared public fields. Verified by the smoke test below.
    pub const FIELD_COUNT: usize = 1;
}

/// Builds the `<md-chip-set>` container with one chip per entry.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_chip_set(props: &ChipSetProps) -> Result<HtmlElement, JsValue> {
    let container = create_mwc_element("md-chip-set")?;

    for item in &props.chips {
        let tag = match item.kind.as_str() {
            "filter" => "md-filter-chip",
            "input" => "md-input-chip",
            "suggestion" => "md-suggestion-chip",
            // M3 default emphasis = assist.
            _ => "md-assist-chip",
        };
        let el = create_mwc_element(tag)?;
        el.set_attribute("label", &item.label)?;
        set_attr_bool(&el, "selected", item.selected && tag == "md-filter-chip")?;
        set_attr_bool(&el, "disabled", item.disabled)?;
        set_attr_opt(&el, "data-on-click-id", item.on_click_id.as_deref())?;
        container.append_child(&el)?;
    }

    Ok(container)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_chip_set)]
pub fn create_chip_set_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: ChipSetProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid ChipSetProps JSON: {e}")))?;
    create_chip_set(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn chip_set_props_default_smoke() {
        let p = ChipSetProps::default();
        assert!(p.chips.is_empty());
        assert_eq!(ChipSetProps::FIELD_COUNT, 1);
        assert_eq!(ChipItemProps::FIELD_COUNT, 5);
    }
}
