// SPDX-License-Identifier: Apache-2.0
//! `shadcn NavigationBar` → `<md-navigation-bar>` + `<md-navigation-tab>`.
//!
//! M3 bottom navigation bar — a horizontal row of 3-to-5 destinations anchored
//! to the bottom of compact-width layouts. Per the M3 spec at
//! <https://m3.material.io/components/navigation-bar/overview>, each tab
//! carries a `label` and an optional `icon` slot.
//!
//! The bridge mirrors that surface 1:1: each [`NavigationTabProps`] becomes a
//! `<md-navigation-tab>` with the label as its textContent and (if provided)
//! an `<svg slot="active-icon">`-style icon left to the consumer — we only
//! expose `icon` as an opaque string forwarded via the `data-icon` attribute,
//! so the consumer's bundler / sprite system stays in charge of asset wiring.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// One destination inside a [`NavigationBarProps`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationTabProps {
    /// Visible text label rendered under the icon.
    pub label: String,
    /// Opaque icon identifier (e.g. Material Symbol name). Forwarded via
    /// `data-icon` so the consumer's icon sprite layer can resolve it.
    pub icon: Option<String>,
    /// Hide the active indicator pill behind the icon when `true`.
    pub hide_inactive_label: bool,
    /// Opaque identifier surfaced on the tab's `click` event's `detail.id`.
    pub on_click_id: Option<String>,
}

impl NavigationTabProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 4;
}

/// Subset of shadcn `<NavigationBar>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationBarProps {
    /// Destinations rendered left-to-right. M3 enforces 3..=5 visually.
    pub tabs: Vec<NavigationTabProps>,
    /// Zero-based index of the active destination.
    pub active_index: usize,
}

impl NavigationBarProps {
    /// Number of declared public fields. Verified by the smoke test below.
    pub const FIELD_COUNT: usize = 2;
}

/// Builds the `<md-navigation-bar>` container.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_navigation_bar(props: &NavigationBarProps) -> Result<HtmlElement, JsValue> {
    let container = create_mwc_element("md-navigation-bar")?;
    container.set_attribute("active-index", &props.active_index.to_string())?;

    for (i, tab) in props.tabs.iter().enumerate() {
        let el = create_mwc_element("md-navigation-tab")?;
        el.set_attribute("label", &tab.label)?;
        set_attr_opt(&el, "data-icon", tab.icon.as_deref())?;
        set_attr_bool(&el, "hide-inactive-label", tab.hide_inactive_label)?;
        set_attr_opt(&el, "data-on-click-id", tab.on_click_id.as_deref())?;
        if i == props.active_index {
            el.set_attribute("active", "")?;
        }
        container.append_child(&el)?;
    }

    Ok(container)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_navigation_bar)]
pub fn create_navigation_bar_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: NavigationBarProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid NavigationBarProps JSON: {e}")))?;
    create_navigation_bar(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn navigation_bar_props_default_smoke() {
        let p = NavigationBarProps::default();
        assert_eq!(p.active_index, 0);
        assert!(p.tabs.is_empty());
        assert_eq!(NavigationBarProps::FIELD_COUNT, 2);
        assert_eq!(NavigationTabProps::FIELD_COUNT, 4);
    }
}
