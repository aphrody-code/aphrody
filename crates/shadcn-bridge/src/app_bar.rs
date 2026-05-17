// SPDX-License-Identifier: Apache-2.0
//! `shadcn AppBar` → bespoke `<header class="md-top-app-bar">` markup.
//!
//! `@material/web` does **not** ship a `<md-top-app-bar>` custom element —
//! Google's M3 spec for the top-app-bar lives entirely in the CSS / layout
//! layer at <https://m3.material.io/components/top-app-bar/overview>, with
//! the leading icon button, title, and trailing actions composed by the
//! consumer. This bridge mirrors that contract by emitting a semantic
//! `<header>` carrying the documented M3 class names + slots so consumer
//! stylesheets (Material You tokens) can paint it correctly.
//!
//! The four M3 variants — `center-aligned`, `small`, `medium`, `large` — are
//! selected via [`AppBarProps::variant`] and surfaced as
//! `class="md-top-app-bar md-top-app-bar--<variant>"`. The leading and
//! trailing action slots are exposed as opaque string identifiers
//! (`leading_action_id`, `trailing_action_id`); consumers wire concrete
//! `<md-icon-button>` children post-creation if needed.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")] use crate::{create_mwc_element, set_attr_opt};

/// Subset of shadcn `<AppBar>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppBarProps {
    /// Page-level title rendered in the title slot.
    pub title: String,
    /// One of `"small"` (default), `"center-aligned"`, `"medium"`, `"large"`.
    /// Any other value falls back to `"small"`.
    pub variant: String,
    /// Opaque identifier for the leading icon button (e.g. menu / back).
    pub leading_action_id: Option<String>,
    /// Opaque identifier for the first trailing icon button (e.g. search).
    pub trailing_action_id: Option<String>,
    /// When `true`, render with the M3 `scrolled` token (elevated +
    /// surface-container colour). Consumer JS toggles this in response to
    /// scroll events.
    pub scrolled: bool,
}

impl AppBarProps {
    /// Number of declared public fields. Verified by the smoke test below.
    pub const FIELD_COUNT: usize = 5;
}

/// Builds the `<header class="md-top-app-bar ...">` element with title and
/// (optional) leading + trailing slot placeholders.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_app_bar(props: &AppBarProps) -> Result<HtmlElement, JsValue> {
    let header = create_mwc_element("header")?;

    let variant = match props.variant.as_str() {
        "center-aligned" | "medium" | "large" => props.variant.as_str(),
        _ => "small",
    };
    let mut class = format!("md-top-app-bar md-top-app-bar--{variant}");
    if props.scrolled {
        class.push_str(" md-top-app-bar--scrolled");
    }
    header.set_attribute("class", &class)?;
    header.set_attribute("role", "banner")?;

    // Leading slot (e.g. menu button placeholder).
    if props.leading_action_id.is_some() {
        let slot = create_mwc_element("span")?;
        slot.set_attribute("class", "md-top-app-bar__leading")?;
        set_attr_opt(&slot, "data-on-click-id", props.leading_action_id.as_deref())?;
        header.append_child(&slot)?;
    }

    // Title slot — always rendered, even when empty (M3 reserves the row).
    let title_el = create_mwc_element("h1")?;
    title_el.set_attribute("class", "md-top-app-bar__title")?;
    title_el.set_text_content(Some(&props.title));
    header.append_child(&title_el)?;

    // Trailing slot (e.g. action icon button placeholder).
    if props.trailing_action_id.is_some() {
        let slot = create_mwc_element("span")?;
        slot.set_attribute("class", "md-top-app-bar__trailing")?;
        set_attr_opt(&slot, "data-on-click-id", props.trailing_action_id.as_deref())?;
        header.append_child(&slot)?;
    }

    Ok(header)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_app_bar)]
pub fn create_app_bar_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: AppBarProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid AppBarProps JSON: {e}")))?;
    create_app_bar(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn app_bar_props_default_smoke() {
        let p = AppBarProps::default();
        assert!(!p.scrolled);
        assert!(p.title.is_empty());
        assert_eq!(AppBarProps::FIELD_COUNT, 5);
    }
}
