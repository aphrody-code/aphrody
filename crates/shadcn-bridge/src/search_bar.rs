// SPDX-License-Identifier: Apache-2.0
//! `shadcn SearchBar` → bespoke `<input class="md-search-bar">` wrapper.
//!
//! Material Web Components 3 does not currently ship a `<md-search>` custom
//! element (`@material/web` as of the 2026-05 release train scopes its
//! text-field surface to `<md-outlined-text-field>` / `<md-filled-text-field>`
//! only; the `<md-search>` proposal remains in draft). The bridge therefore
//! emits the M3 search-bar recipe directly: an outer
//! `<div class="md-search-bar">` carrying a leading icon slot, a native
//! `<input type="search">` element, and an optional trailing clear-action
//! `<button>`. The class names mirror the public design-tokens used by
//! Google's first-party Material 3 implementations.
//!
//! Should `<md-search>` later ship in `@material/web`, the bridge will be
//! updated to delegate to it (gated behind a runtime feature-detection
//! check) without changing the [`SearchBarProps`] surface.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")] use crate::{document, set_attr_opt};

/// Subset of shadcn `<SearchBar>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchBarProps {
    /// Initial input value.
    pub value: String,
    /// Placeholder shown when the input is empty.
    pub placeholder: String,
    /// Optional leading Material Symbols ligature (defaults to `"search"`).
    pub leading_icon: Option<String>,
    /// When true, render a trailing clear-action button. The button is
    /// labelled `"Clear"` and dispatches a `CustomEvent` on click; consumers
    /// are responsible for resetting the input value.
    pub clearable: bool,
    /// Mirrors HTML `disabled`.
    pub disabled: bool,
    /// CustomEvent id dispatched on `input` events (`detail.value`).
    pub on_input_id: Option<String>,
    /// CustomEvent id dispatched on the trailing clear-button click.
    pub on_clear_id: Option<String>,
}

impl SearchBarProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 7;
}

/// Builds the `<div class="md-search-bar">` subtree from `props`.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_search_bar(props: &SearchBarProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let root = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    root.set_class_name("md-search-bar");
    root.set_attribute("role", "search")?;
    if props.disabled {
        root.set_attribute("aria-disabled", "true")?;
    }

    // Leading icon — default "search" glyph (Material Symbols ligature).
    let icon = doc.create_element("span")?.dyn_into::<HtmlElement>()?;
    icon.set_class_name("md-search-bar__leading-icon material-symbols-outlined");
    icon.set_attribute("aria-hidden", "true")?;
    let icon_glyph = props.leading_icon.as_deref().unwrap_or("search");
    icon.set_text_content(Some(icon_glyph));
    root.append_child(&icon)?;

    // Native <input type="search"> — accessible by default, supports the
    // browser's built-in clear UI on most desktop chromium/firefox builds.
    let input = doc.create_element("input")?.dyn_into::<HtmlElement>()?;
    input.set_class_name("md-search-bar__input");
    input.set_attribute("type", "search")?;
    input.set_attribute("value", &props.value)?;
    input.set_attribute("placeholder", &props.placeholder)?;
    if props.disabled {
        input.set_attribute("disabled", "")?;
    }
    set_attr_opt(&input, "data-on-input-id", props.on_input_id.as_deref())?;
    root.append_child(&input)?;

    if props.clearable {
        let clear = doc.create_element("button")?.dyn_into::<HtmlElement>()?;
        clear.set_class_name("md-search-bar__clear");
        clear.set_attribute("type", "button")?;
        clear.set_attribute("aria-label", "Clear")?;
        if props.disabled {
            clear.set_attribute("disabled", "")?;
        }
        set_attr_opt(&clear, "data-on-clear-id", props.on_clear_id.as_deref())?;
        // Trailing "close" glyph (Material Symbols ligature).
        clear.set_text_content(Some("close"));
        root.append_child(&clear)?;
    }

    Ok(root)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_search_bar)]
pub fn create_search_bar_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: SearchBarProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid SearchBarProps JSON: {e}")))?;
    create_search_bar(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Smoke test: default Props serialises to a `FIELD_COUNT`-key JSON object.
    #[test]
    fn search_bar_props_default_field_count() {
        let p = SearchBarProps::default();
        let v = serde_json::to_value(&p).expect("serde must serialise SearchBarProps");
        let map = v.as_object().expect("SearchBarProps must serialise as object");
        assert_eq!(map.len(), SearchBarProps::FIELD_COUNT);
    }
}
