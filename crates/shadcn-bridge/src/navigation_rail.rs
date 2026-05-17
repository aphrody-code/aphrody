// SPDX-License-Identifier: Apache-2.0
//! `shadcn NavigationRail` → `<md-navigation-rail>` + tabs.
//!
//! M3 navigation rail — a narrow vertical strip of destinations sized for
//! medium-width layouts (tablets, small desktops). Per the M3 spec at
//! <https://m3.material.io/components/navigation-rail/overview>, the rail
//! aligns icon + label vertically with optional leading FAB + trailing
//! actions. We expose the destinations subset (the dominant 90 % use case);
//! consumers slot custom leading / trailing children directly into the
//! returned element if needed.
//!
//! Each entry becomes a `<md-navigation-tab>` (M3 re-uses the same tab
//! primitive for both bar and rail). `active_index` decides which tab carries
//! the `active` attribute.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// One destination inside a [`NavigationRailProps`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationRailTabProps {
    /// Visible text label rendered under the icon.
    pub label: String,
    /// Opaque icon identifier; forwarded via `data-icon`.
    pub icon: Option<String>,
    /// Opaque identifier surfaced on the tab's `click` event.
    pub on_click_id: Option<String>,
}

impl NavigationRailTabProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 3;
}

/// Subset of shadcn `<NavigationRail>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationRailProps {
    /// Destinations rendered top-to-bottom along the rail.
    pub tabs: Vec<NavigationRailTabProps>,
    /// Zero-based index of the active destination.
    pub active_index: usize,
    /// When `true`, hide inactive tab labels (icon-only resting state).
    pub hide_inactive_labels: bool,
}

impl NavigationRailProps {
    /// Number of declared public fields. Verified by the smoke test below.
    pub const FIELD_COUNT: usize = 3;
}

/// Builds the `<md-navigation-rail>` element.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_navigation_rail(props: &NavigationRailProps) -> Result<HtmlElement, JsValue> {
    let container = create_mwc_element("md-navigation-rail")?;
    container.set_attribute("active-index", &props.active_index.to_string())?;
    set_attr_bool(&container, "hide-inactive-labels", props.hide_inactive_labels)?;

    for (i, tab) in props.tabs.iter().enumerate() {
        let el = create_mwc_element("md-navigation-tab")?;
        el.set_attribute("label", &tab.label)?;
        set_attr_opt(&el, "data-icon", tab.icon.as_deref())?;
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
#[wasm_bindgen(js_name = create_navigation_rail)]
pub fn create_navigation_rail_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: NavigationRailProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid NavigationRailProps JSON: {e}")))?;
    create_navigation_rail(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn navigation_rail_props_default_smoke() {
        let p = NavigationRailProps::default();
        assert_eq!(p.active_index, 0);
        assert!(!p.hide_inactive_labels);
        assert!(p.tabs.is_empty());
        assert_eq!(NavigationRailProps::FIELD_COUNT, 3);
        assert_eq!(NavigationRailTabProps::FIELD_COUNT, 3);
    }
}
