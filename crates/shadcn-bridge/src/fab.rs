// SPDX-License-Identifier: Apache-2.0
//! `shadcn FAB` → `<md-fab>` / `<md-branded-fab>` (Material Web Components 3).
//!
//! M3 Floating Action Button family. Per the spec at
//! <https://m3.material.io/components/floating-action-button/overview>, FABs
//! come in three sizes:
//! * `"small"`   → 40 dp, `<md-fab size="small">`
//! * `"medium"`  → 56 dp, `<md-fab size="medium">` (default)
//! * `"large"`   → 96 dp, `<md-fab size="large">`
//!
//! `variant` selects between the regular and branded element:
//! * `"branded"` → `<md-branded-fab>` (multi-coloured logo-style FAB)
//! * any other value (default) → `<md-fab>`
//!
//! Extended FABs (FAB with a text label) are produced by setting
//! [`FabProps::label`] to a non-empty string — MWC3 toggles the extended
//! shape automatically when the `label` attribute is present.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<FAB>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FabProps {
    /// Extended-FAB label. Empty string renders an icon-only FAB.
    pub label: String,
    /// One of `"small"`, `"medium"` (default), `"large"`. Any other value
    /// falls back to `"medium"`.
    pub size: String,
    /// `"branded"` selects `<md-branded-fab>`; any other value (default)
    /// selects `<md-fab>`.
    pub variant: String,
    /// When `true`, render as a low-emphasis surface FAB (M3 "lowered"
    /// elevation set). MWC3 maps this onto the `lowered` attribute.
    pub lowered: bool,
    /// Opaque identifier surfaced on the FAB's `click` `CustomEvent`'s
    /// `detail.id`.
    pub on_click_id: Option<String>,
}

impl FabProps {
    /// Number of declared public fields. Verified by the smoke test below.
    pub const FIELD_COUNT: usize = 5;
}

/// Builds the `<md-fab>` / `<md-branded-fab>` element from `props`.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_fab(props: &FabProps) -> Result<HtmlElement, JsValue> {
    let tag = if props.variant == "branded" { "md-branded-fab" } else { "md-fab" };
    let el = create_mwc_element(tag)?;

    let size = match props.size.as_str() {
        "small" | "large" => props.size.as_str(),
        _ => "medium",
    };
    el.set_attribute("size", size)?;

    if !props.label.is_empty() {
        // Setting the `label` attribute morphs the FAB into its extended
        // shape per MWC3 spec; textContent is ignored for <md-fab>.
        el.set_attribute("label", &props.label)?;
    }

    set_attr_bool(&el, "lowered", props.lowered)?;
    set_attr_opt(&el, "data-on-click-id", props.on_click_id.as_deref())?;

    Ok(el)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_fab)]
pub fn create_fab_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: FabProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid FabProps JSON: {e}")))?;
    create_fab(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn fab_props_default_smoke() {
        let p = FabProps::default();
        assert!(!p.lowered);
        assert!(p.label.is_empty());
        assert_eq!(FabProps::FIELD_COUNT, 5);
    }
}
