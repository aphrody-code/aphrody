// SPDX-License-Identifier: Apache-2.0
//! `shadcn Tooltip` → bespoke `<div role="tooltip" class="md-tooltip">`.
//!
//! Material Web Components 3 does not ship a `<md-tooltip>` custom element in
//! `@material/web` as of the 2026-05 release train. The bridge therefore
//! emits the official Material 3 tooltip recipe directly. Two M3 variants
//! are supported:
//!
//! * **plain** — single-line label, surface-inverse background. The default.
//! * **rich**  — multi-line surface tooltip with optional subtitle + action
//!   slot. Used for opt-in walkthroughs and help affordances.
//!
//! Class names mirror the public design-token surface used by Google's first
//! party Material 3 implementations, so the same CSS rules apply whether or
//! not MWC3 eventually ships a `<md-tooltip>` element.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{document, set_attr_opt};

/// Subset of shadcn `<Tooltip>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TooltipProps {
    /// Tooltip body text. Required.
    pub label: String,
    /// shadcn `variant` — `"plain"` (default) or `"rich"`.
    pub variant: String,
    /// Subtitle line, only honoured by the `"rich"` variant.
    pub subtitle: Option<String>,
    /// id of the element the tooltip targets (sets `aria-describedby`
    /// equivalent via a `data-target` attribute consumers can wire up).
    pub anchor_id: Option<String>,
    /// Whether the tooltip is initially visible (mirrors `open` attribute).
    pub open: bool,
}

impl TooltipProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 5;
}

/// Builds the `<div role="tooltip">` subtree from `props`.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_tooltip(props: &TooltipProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let root = doc.create_element("div")?.dyn_into::<HtmlElement>()?;

    let variant = match props.variant.as_str() {
        "rich" => "rich",
        _ => "plain",
    };
    root.set_class_name(&format!("md-tooltip md-tooltip--{variant}"));
    root.set_attribute("role", "tooltip")?;
    set_attr_opt(&root, "data-target", props.anchor_id.as_deref())?;
    if props.open {
        root.set_attribute("data-open", "true")?;
    } else {
        root.set_attribute("data-open", "false")?;
    }

    if variant == "rich" {
        // Rich tooltips render label + optional subtitle in stacked spans so
        // CSS can size them per the M3 surface-tooltip recipe (12dp/14sp).
        let label = doc.create_element("span")?.dyn_into::<HtmlElement>()?;
        label.set_class_name("md-tooltip__label");
        label.set_text_content(Some(&props.label));
        root.append_child(&label)?;

        if let Some(sub) = props.subtitle.as_deref() {
            let subtitle = doc.create_element("span")?.dyn_into::<HtmlElement>()?;
            subtitle.set_class_name("md-tooltip__subtitle");
            subtitle.set_text_content(Some(sub));
            root.append_child(&subtitle)?;
        }
    } else {
        // Plain tooltip — single text node, no inner structure.
        root.set_text_content(Some(&props.label));
    }

    Ok(root)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_tooltip)]
pub fn create_tooltip_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: TooltipProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid TooltipProps JSON: {e}")))?;
    create_tooltip(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Smoke test: default Props serialises to a `FIELD_COUNT`-key JSON object.
    #[test]
    fn tooltip_props_default_field_count() {
        let p = TooltipProps::default();
        let v = serde_json::to_value(&p).expect("serde must serialise TooltipProps");
        let map = v.as_object().expect("TooltipProps must serialise as object");
        assert_eq!(map.len(), TooltipProps::FIELD_COUNT);
    }
}
