// SPDX-License-Identifier: Apache-2.0
//! `shadcn Toast` → `<md-snackbar>` (with a `<div class="md-snackbar">`
//! fallback for browsers that do not register the MWC3 snackbar custom
//! element).
//!
//! As of 2026-05, `@material/web` ships `<md-snackbar>` as an experimental
//! element; we emit it unconditionally and let the consumer's design-system
//! polyfill or fallback CSS handle non-supporting environments.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, document, set_attr_bool, set_attr_opt};

/// Subset of shadcn `<Toast>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToastProps {
    /// User-visible message body.
    pub message: String,
    /// Optional action button label (e.g. `"Undo"`).
    pub action_label: Option<String>,
    /// CustomEvent id dispatched when the action button is clicked.
    pub on_action_id: Option<String>,
    /// Whether the snackbar is initially visible.
    pub open: bool,
}

impl ToastProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::toast_module_smoke`.
    pub const FIELD_COUNT: usize = 4;
}

/// Builds an `<md-snackbar>` element with message + optional action.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_toast(props: &ToastProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let el = create_mwc_element("md-snackbar")?;
    set_attr_bool(&el, "open", props.open)?;
    el.set_text_content(Some(&props.message));

    if let Some(label) = props.action_label.as_deref() {
        let action = doc.create_element("md-text-button")?.dyn_into::<HtmlElement>()?;
        action.set_attribute("slot", "action")?;
        action.set_text_content(Some(label));
        set_attr_opt(&action, "data-on-action-id", props.on_action_id.as_deref())?;
        el.append_child(&action)?;
    }

    Ok(el)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_toast)]
pub fn create_toast_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: ToastProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid ToastProps JSON: {e}")))?;
    create_toast(&props)
}
