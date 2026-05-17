// SPDX-License-Identifier: Apache-2.0
//! `shadcn Dialog` → `<md-dialog>`.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, document, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<Dialog>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogProps {
    /// Whether the dialog is initially open (mirrors `open` attribute).
    pub open: bool,
    /// Headline text rendered in the dialog's title slot.
    pub headline: String,
    /// Body text rendered in the dialog's content slot.
    pub body: String,
    /// CustomEvent id dispatched when the primary (confirm) action fires.
    pub primary_action_id: Option<String>,
    /// CustomEvent id dispatched when the secondary (dismiss) action fires.
    pub secondary_action_id: Option<String>,
}

impl DialogProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::dialog_module_smoke`.
    pub const FIELD_COUNT: usize = 5;
}

/// Builds an `<md-dialog>` populated with headline + body + slot anchors for
/// primary / secondary actions (so consumers can append their own buttons).
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_dialog(props: &DialogProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let el = create_mwc_element("md-dialog")?;
    set_attr_bool(&el, "open", props.open)?;
    set_attr_opt(&el, "data-primary-id", props.primary_action_id.as_deref())?;
    set_attr_opt(&el, "data-secondary-id", props.secondary_action_id.as_deref())?;

    let headline = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    headline.set_attribute("slot", "headline")?;
    headline.set_text_content(Some(&props.headline));
    el.append_child(&headline)?;

    let content = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    content.set_attribute("slot", "content")?;
    content.set_text_content(Some(&props.body));
    el.append_child(&content)?;

    Ok(el)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_dialog)]
pub fn create_dialog_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: DialogProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid DialogProps JSON: {e}")))?;
    create_dialog(&props)
}
