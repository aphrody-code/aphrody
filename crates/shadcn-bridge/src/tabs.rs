// SPDX-License-Identifier: Apache-2.0
//! `shadcn Tabs` → `<md-tabs>` + `<md-primary-tab>` / `<md-secondary-tab>`.
//!
//! `variant = "secondary"` selects `<md-secondary-tab>` children; any other
//! value (including default) selects `<md-primary-tab>`.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")] use crate::create_mwc_element;

/// Subset of shadcn `<Tabs>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TabsProps {
    /// Tab labels in display order.
    pub tabs: Vec<String>,
    /// Zero-based active tab index.
    pub active_index: usize,
    /// `"primary"` (default) or `"secondary"`.
    pub variant: String,
}

impl TabsProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::tabs_module_smoke`.
    pub const FIELD_COUNT: usize = 3;
}

/// Builds an `<md-tabs>` container with one child tab per label.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_tabs(props: &TabsProps) -> Result<HtmlElement, JsValue> {
    let container = create_mwc_element("md-tabs")?;
    container.set_attribute("active-tab-index", &props.active_index.to_string())?;

    let child_tag =
        if props.variant == "secondary" { "md-secondary-tab" } else { "md-primary-tab" };

    for (i, label) in props.tabs.iter().enumerate() {
        let tab = create_mwc_element(child_tag)?;
        tab.set_text_content(Some(label));
        if i == props.active_index {
            tab.set_attribute("active", "")?;
        }
        container.append_child(&tab)?;
    }

    Ok(container)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_tabs)]
pub fn create_tabs_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: TabsProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid TabsProps JSON: {e}")))?;
    create_tabs(&props)
}
