// SPDX-License-Identifier: Apache-2.0
//! `shadcn List` → `<md-list>` + `<md-list-item>` (Material Web Components 3).
//!
//! shadcn `<List>` / `<ListItem>` composes a flat vertical surface of clickable
//! rows; MWC3 exposes the same model via the `<md-list>` container and one
//! `<md-list-item>` per row. Each item supports a `headline` (primary text), an
//! optional `supporting_text` (secondary slot), and a `disabled` flag mapping
//! 1:1 onto the M3 spec at <https://m3.material.io/components/lists/overview>.
//!
//! The bridge intentionally stays markup-only: leading / trailing icons are
//! left to the consumer (they vary per design-system), but `data-on-click-id`
//! is propagated onto every `<md-list-item>` so JS event listeners can route
//! activation callbacks the same way the other bridge modules do.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool, set_attr_opt};

/// One row inside a [`ListProps`] list. Mirrors the shadcn `<ListItem>` slot
/// surface, trimmed to the fields the MWC3 element actually honours.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListItemProps {
    /// Primary line of text (M3 "headline" slot).
    pub headline: String,
    /// Optional secondary line of text (M3 "supporting-text" slot). Empty
    /// string and `None` both render as a single-line list item.
    pub supporting_text: Option<String>,
    /// Mirrors HTML `disabled`. Disabled items are still rendered but cannot
    /// be activated; M3 dims them via the surface tint token automatically.
    pub disabled: bool,
    /// Opaque identifier surfaced on the item's `click` `CustomEvent`'s
    /// `detail.id`. JS listeners route events to handlers by inspecting it.
    pub on_click_id: Option<String>,
}

impl ListItemProps {
    /// Number of declared public fields. Mirrored on [`ListProps::FIELD_COUNT`]
    /// indirectly via `items` (a `Vec<ListItemProps>`).
    pub const FIELD_COUNT: usize = 4;
}

/// Subset of shadcn `<List>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListProps {
    /// Rows to render, top-to-bottom.
    pub items: Vec<ListItemProps>,
}

impl ListProps {
    /// Number of declared public fields. Verified by the smoke test below.
    pub const FIELD_COUNT: usize = 1;
}

/// Builds the `<md-list>` container with one `<md-list-item>` per entry in
/// `props.items`.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable (e.g. running inside a
/// Worker context without a document).
#[cfg(target_arch = "wasm32")]
pub fn create_list(props: &ListProps) -> Result<HtmlElement, JsValue> {
    let container = create_mwc_element("md-list")?;
    for item in &props.items {
        let row = create_mwc_element("md-list-item")?;
        // Headline slot is the default slot for <md-list-item>; setting
        // textContent puts the string in the right place per MWC3 spec.
        row.set_text_content(Some(&item.headline));
        if let Some(ref supporting) = item.supporting_text {
            if !supporting.is_empty() {
                // M3 uses a named "supporting-text" slot; surface it as a
                // <div slot="supporting-text"> child to honour the spec.
                let supporting_el = create_mwc_element("div")?;
                supporting_el.set_attribute("slot", "supporting-text")?;
                supporting_el.set_text_content(Some(supporting));
                row.append_child(&supporting_el)?;
            }
        }
        set_attr_bool(&row, "disabled", item.disabled)?;
        set_attr_opt(&row, "data-on-click-id", item.on_click_id.as_deref())?;
        container.append_child(&row)?;
    }
    Ok(container)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string to
/// avoid wiring a `serde-wasm-bindgen` dep at this layer.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_list)]
pub fn create_list_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: ListProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid ListProps JSON: {e}")))?;
    create_list(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn list_props_default_smoke() {
        let p = ListProps::default();
        // Default list has zero items but the struct still declares 1 field.
        assert!(p.items.is_empty());
        assert_eq!(ListProps::FIELD_COUNT, 1);
        // And ListItemProps must keep its 4 declared fields.
        assert_eq!(ListItemProps::FIELD_COUNT, 4);
    }
}
