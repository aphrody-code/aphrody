// SPDX-License-Identifier: Apache-2.0
//! `shadcn BottomSheet` → bespoke `<div role="dialog" class="md-bottom-sheet">`.
//!
//! Material Web Components 3 does not ship a `<md-bottom-sheet>` custom
//! element. The bridge therefore emits the M3 bottom-sheet recipe as a
//! `<div role="dialog">` with the canonical `md-bottom-sheet` class plus a
//! variant modifier. Two M3 variants are supported:
//!
//! * **modal**    — full overlay, scrim-backed, focus trap expected
//!   (`aria-modal="true"`). Default.
//! * **standard** — inline / persistent, no scrim. `aria-modal="false"`.
//!
//! A drag-handle `<span>` and a content slot `<div>` are emitted as nested
//! children so consumers can style + populate them without re-creating the
//! DOM tree.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{document, set_attr_opt};

/// Subset of shadcn `<BottomSheet>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BottomSheetProps {
    /// Whether the sheet is initially open (mirrors `open` attribute).
    pub open: bool,
    /// shadcn variant — `"modal"` (default) or `"standard"`.
    pub variant: String,
    /// Optional headline rendered in the sheet's title row.
    pub headline: String,
    /// Body content (plain text — HTML escaping handled by `setTextContent`).
    pub body: String,
    /// CustomEvent id dispatched on dismiss (swipe / scrim click).
    pub on_dismiss_id: Option<String>,
}

impl BottomSheetProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 5;
}

/// Builds the `<div role="dialog">` bottom-sheet subtree from `props`.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_bottom_sheet(props: &BottomSheetProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let root = doc.create_element("div")?.dyn_into::<HtmlElement>()?;

    let variant = match props.variant.as_str() {
        "standard" => "standard",
        _ => "modal",
    };
    root.set_class_name(&format!("md-bottom-sheet md-bottom-sheet--{variant}"));
    root.set_attribute("role", "dialog")?;
    root.set_attribute("aria-modal", if variant == "modal" { "true" } else { "false" })?;
    root.set_attribute("data-open", if props.open { "true" } else { "false" })?;
    set_attr_opt(&root, "data-on-dismiss-id", props.on_dismiss_id.as_deref())?;

    // Drag handle — 32dp pill per the M3 bottom-sheet spec.
    let handle = doc.create_element("span")?.dyn_into::<HtmlElement>()?;
    handle.set_class_name("md-bottom-sheet__handle");
    handle.set_attribute("aria-hidden", "true")?;
    root.append_child(&handle)?;

    if !props.headline.is_empty() {
        let headline = doc.create_element("h2")?.dyn_into::<HtmlElement>()?;
        headline.set_class_name("md-bottom-sheet__headline");
        headline.set_text_content(Some(&props.headline));
        root.append_child(&headline)?;
    }

    let content = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    content.set_class_name("md-bottom-sheet__content");
    content.set_text_content(Some(&props.body));
    root.append_child(&content)?;

    Ok(root)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_bottom_sheet)]
pub fn create_bottom_sheet_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: BottomSheetProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid BottomSheetProps JSON: {e}")))?;
    create_bottom_sheet(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Smoke test: default Props serialises to a `FIELD_COUNT`-key JSON object.
    #[test]
    fn bottom_sheet_props_default_field_count() {
        let p = BottomSheetProps::default();
        let v = serde_json::to_value(&p).expect("serde must serialise BottomSheetProps");
        let map = v
            .as_object()
            .expect("BottomSheetProps must serialise as object");
        assert_eq!(map.len(), BottomSheetProps::FIELD_COUNT);
    }
}
