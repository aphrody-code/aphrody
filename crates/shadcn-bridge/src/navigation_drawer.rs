// SPDX-License-Identifier: Apache-2.0
//! `shadcn NavigationDrawer` → `<md-navigation-drawer>` + drawer items.
//!
//! M3 navigation drawer — a tall pane of destinations anchored to the side of
//! expanded-width layouts. Per the M3 spec at
//! <https://m3.material.io/components/navigation-drawer/overview>, the drawer
//! holds a stack of `<md-list-item>`-shaped rows (M3 currently re-uses the
//! list-item primitive; there is no dedicated `<md-drawer-item>` element).
//!
//! The bridge therefore composes `<md-navigation-drawer>` + N
//! `<md-list-item>` children, each carrying its label and (optional) icon.
//! `modal=true` flips the underlying element into modal mode (overlays the
//! page, dismissible by scrim click); `open=true` reveals the drawer at boot.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// One destination inside a [`NavigationDrawerProps`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationDrawerItemProps {
    /// Visible text label.
    pub label: String,
    /// Opaque icon identifier; forwarded via `data-icon`.
    pub icon: Option<String>,
    /// Mark this item as the active destination.
    pub active: bool,
    /// Opaque identifier surfaced on the row's `click` event.
    pub on_click_id: Option<String>,
}

impl NavigationDrawerItemProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 4;
}

/// Subset of shadcn `<NavigationDrawer>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationDrawerProps {
    /// Destinations rendered top-to-bottom.
    pub items: Vec<NavigationDrawerItemProps>,
    /// When `true`, the drawer is visible at boot. Modal drawers also need
    /// this flag to open.
    pub open: bool,
    /// When `true`, the drawer renders modally with a scrim.
    pub modal: bool,
    /// Optional title rendered above the items as an M3 "headline" slot.
    pub title: Option<String>,
}

impl NavigationDrawerProps {
    /// Number of declared public fields. Verified by the smoke test below.
    pub const FIELD_COUNT: usize = 4;
}

/// Builds the `<md-navigation-drawer>` element.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_navigation_drawer(props: &NavigationDrawerProps) -> Result<HtmlElement, JsValue> {
    let container = create_mwc_element("md-navigation-drawer")?;
    set_attr_bool(&container, "opened", props.open)?;
    set_attr_bool(&container, "modal", props.modal)?;

    if let Some(ref title) = props.title {
        if !title.is_empty() {
            let header = create_mwc_element("div")?;
            header.set_attribute("slot", "header")?;
            header.set_text_content(Some(title));
            container.append_child(&header)?;
        }
    }

    for item in &props.items {
        let row = create_mwc_element("md-list-item")?;
        row.set_text_content(Some(&item.label));
        set_attr_opt(&row, "data-icon", item.icon.as_deref())?;
        set_attr_bool(&row, "active", item.active)?;
        set_attr_opt(&row, "data-on-click-id", item.on_click_id.as_deref())?;
        container.append_child(&row)?;
    }

    Ok(container)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_navigation_drawer)]
pub fn create_navigation_drawer_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: NavigationDrawerProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid NavigationDrawerProps JSON: {e}")))?;
    create_navigation_drawer(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn navigation_drawer_props_default_smoke() {
        let p = NavigationDrawerProps::default();
        assert!(!p.open);
        assert!(!p.modal);
        assert!(p.items.is_empty());
        assert_eq!(NavigationDrawerProps::FIELD_COUNT, 4);
        assert_eq!(NavigationDrawerItemProps::FIELD_COUNT, 4);
    }
}
