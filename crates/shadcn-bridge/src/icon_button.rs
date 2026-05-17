// SPDX-License-Identifier: Apache-2.0
//! `shadcn IconButton` → Material Web Components 3 icon-button family.
//!
//! Maps `variant` to one of:
//! * `"filled"`        → `<md-filled-icon-button>`
//! * `"outlined"`      → `<md-outlined-icon-button>`
//! * `"filled-tonal"`  → `<md-filled-tonal-icon-button>`
//! * default / `"standard"` → `<md-icon-button>` (standard / "text" emphasis)
//!
//! The icon glyph is supplied as text content inside a nested `<md-icon>`
//! element, matching the MWC3 documented usage for both icon-fonts and
//! ligature-based glyphs (e.g. Material Symbols).

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<IconButton>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IconButtonProps {
    /// Material Symbols ligature (e.g. `"settings"`, `"close"`).
    pub icon: String,
    /// shadcn variant — see module docs for the supported set.
    pub variant: String,
    /// When true, renders the toggle variant (`toggle` attribute on the
    /// underlying MWC3 element) for stateful icon buttons.
    pub toggle: bool,
    /// When `toggle` is true, the initial selected state.
    pub selected: bool,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
    /// Accessibility label (`aria-label`) — mandatory for icon-only buttons.
    pub aria_label: String,
    /// Opaque identifier surfaced on a `click` `CustomEvent`'s `detail.id`.
    pub on_click_id: Option<String>,
}

impl IconButtonProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 7;
}

/// Builds the `<md-*-icon-button>` element from `props`.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_icon_button(props: &IconButtonProps) -> Result<HtmlElement, JsValue> {
    let tag = match props.variant.as_str() {
        "filled" => "md-filled-icon-button",
        "outlined" => "md-outlined-icon-button",
        "filled-tonal" | "tonal" => "md-filled-tonal-icon-button",
        // shadcn defaults map to the standard (low-emphasis) variant on M3.
        _ => "md-icon-button",
    };
    let el = create_mwc_element(tag)?;
    set_attr_bool(&el, "toggle", props.toggle)?;
    set_attr_bool(&el, "selected", props.toggle && props.selected)?;
    set_attr_bool(&el, "disabled", props.disabled)?;
    el.set_attribute("aria-label", &props.aria_label)?;
    set_attr_opt(&el, "data-on-click-id", props.on_click_id.as_deref())?;

    // Nested <md-icon> hosts the glyph (icon-font ligature). Matches the
    // documented MWC3 pattern for all icon-button variants.
    let icon = create_mwc_element("md-icon")?;
    icon.set_text_content(Some(&props.icon));
    el.append_child(&icon)?;

    Ok(el)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_icon_button)]
pub fn create_icon_button_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: IconButtonProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid IconButtonProps JSON: {e}")))?;
    create_icon_button(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Smoke test: default Props serialises to a `FIELD_COUNT`-key JSON object.
    #[test]
    fn icon_button_props_default_field_count() {
        let p = IconButtonProps::default();
        let v = serde_json::to_value(&p).expect("serde must serialise IconButtonProps");
        let map = v.as_object().expect("IconButtonProps must serialise as object");
        assert_eq!(map.len(), IconButtonProps::FIELD_COUNT);
    }
}
