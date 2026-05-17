// SPDX-License-Identifier: Apache-2.0
//! `shadcn Card` → bespoke `<div class="md-elevated-card">` with M3 elevation.
//!
//! MWC3 does not currently ship a first-class card element (deprecated mid-2024
//! in favour of design-system-supplied surfaces), so this bridge emits the
//! Material 3 elevated-card recipe: an outer `<div>` carrying the
//! `--md-sys-elevation` CSS custom property + the standard `md-elevated-card`
//! class for consumer styling.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::document;

use crate::M3_ELEVATION_LEVEL_1_DP;

/// Subset of shadcn `<Card>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardProps {
    /// Header title text (rendered inside a nested `<h3>`).
    pub title: String,
    /// Body content (plain text — HTML escaping handled by `setTextContent`).
    pub body: String,
    /// M3 elevation token in `dp`. Defaults to level 1.
    pub elevation_dp: u8,
}

impl Default for CardProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            elevation_dp: M3_ELEVATION_LEVEL_1_DP,
        }
    }
}

impl CardProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::card_module_smoke`.
    pub const FIELD_COUNT: usize = 3;
}

/// Builds an elevated-card `<div>` subtree from `props`.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_card(props: &CardProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let root = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    root.set_class_name("md-elevated-card");
    root.set_attribute(
        "style",
        &format!(
            "--md-sys-elevation: {}; --md-elevation-level: {};",
            props.elevation_dp, props.elevation_dp
        ),
    )?;

    let title = doc.create_element("h3")?.dyn_into::<HtmlElement>()?;
    title.set_class_name("md-elevated-card__title");
    title.set_text_content(Some(&props.title));
    root.append_child(&title)?;

    let body = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    body.set_class_name("md-elevated-card__body");
    body.set_text_content(Some(&props.body));
    root.append_child(&body)?;

    Ok(root)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_card)]
pub fn create_card_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: CardProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid CardProps JSON: {e}")))?;
    create_card(&props)
}
