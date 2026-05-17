// SPDX-License-Identifier: Apache-2.0
//! `shadcn Avatar` → bespoke `<div class="md-avatar">` wrapping `<img>` and
//! carrying an M3 elevation token. MWC3 does not ship a first-class avatar
//! element, so the bridge emits the Material 3 avatar recipe directly.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

use crate::M3_ELEVATION_LEVEL_1_DP;
#[cfg(target_arch = "wasm32")] use crate::document;

/// Subset of shadcn `<Avatar>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarProps {
    /// Image URL (must be CORS-allowed for the consumer origin).
    pub src: String,
    /// Accessibility alt text.
    pub alt: String,
    /// Diameter in CSS pixels (square crop, circular clip via CSS).
    pub size_px: u32,
    /// M3 elevation token in `dp`. Defaults to level 1.
    pub elevation_dp: u8,
}

impl Default for AvatarProps {
    fn default() -> Self {
        Self {
            src: String::new(),
            alt: String::new(),
            size_px: 40,
            elevation_dp: M3_ELEVATION_LEVEL_1_DP,
        }
    }
}

impl AvatarProps {
    /// Number of declared public fields. Verified in `lib.rs::tests::avatar_module_smoke`.
    pub const FIELD_COUNT: usize = 4;
}

/// Builds a circular avatar `<div>` wrapping an `<img>` element.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_avatar(props: &AvatarProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let root = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    root.set_class_name("md-avatar");
    root.set_attribute(
        "style",
        &format!(
            "--md-sys-elevation: {}; width: {}px; height: {}px; border-radius: 50%;",
            props.elevation_dp, props.size_px, props.size_px
        ),
    )?;

    let img = doc.create_element("img")?.dyn_into::<HtmlElement>()?;
    img.set_attribute("src", &props.src)?;
    img.set_attribute("alt", &props.alt)?;
    img.set_attribute(
        "style",
        "width: 100%; height: 100%; object-fit: cover; border-radius: inherit;",
    )?;
    root.append_child(&img)?;

    Ok(root)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_avatar)]
pub fn create_avatar_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: AvatarProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid AvatarProps JSON: {e}")))?;
    create_avatar(&props)
}
