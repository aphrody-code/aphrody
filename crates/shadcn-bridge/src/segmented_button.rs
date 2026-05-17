// SPDX-License-Identifier: Apache-2.0
//! `shadcn SegmentedButton` → `<md-outlined-segmented-button-set>` +
//! `<md-outlined-segmented-button>` (one per segment).
//!
//! Material Web Components 3 only ships the outlined emphasis level for
//! segmented buttons (per the M3 spec — segmented buttons are always
//! outlined). The set element handles single-/multi-select semantics via
//! the `multiselect` attribute; each segment carries a `value`, `label`,
//! and optional leading icon glyph (Material Symbols ligature).

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{create_mwc_element, set_attr_bool};

/// One segment within a segmented-button set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SegmentedButtonSegment {
    /// Underlying form value (submitted on selection change).
    pub value: String,
    /// User-visible label.
    pub label: String,
    /// Optional leading Material Symbols ligature (e.g. `"check"`).
    pub icon: Option<String>,
    /// Mirrors HTML `disabled` on this individual segment.
    pub disabled: bool,
    /// Initial selection state — honoured at element-creation time.
    pub selected: bool,
}

/// Subset of shadcn `<SegmentedButton>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SegmentedButtonProps {
    /// Segment list in display order.
    pub segments: Vec<SegmentedButtonSegment>,
    /// When true, the set allows multiple concurrent selections (`multiselect`
    /// attribute on the wrapper element).
    pub multiselect: bool,
    /// Mirrors HTML `disabled` on the set wrapper (disables every segment).
    pub disabled: bool,
    /// Opaque identifier surfaced on a `change` `CustomEvent`'s `detail.id`.
    pub on_change_id: Option<String>,
}

impl SegmentedButtonProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 4;
}

/// Builds an `<md-outlined-segmented-button-set>` with one
/// `<md-outlined-segmented-button>` per [`SegmentedButtonSegment`].
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_segmented_button(
    props: &SegmentedButtonProps,
) -> Result<HtmlElement, JsValue> {
    let set = create_mwc_element("md-outlined-segmented-button-set")?;
    set_attr_bool(&set, "multiselect", props.multiselect)?;
    set_attr_bool(&set, "disabled", props.disabled)?;
    if let Some(id) = props.on_change_id.as_deref() {
        set.set_attribute("data-on-change-id", id)?;
    }

    for seg in &props.segments {
        let el = create_mwc_element("md-outlined-segmented-button")?;
        el.set_attribute("value", &seg.value)?;
        el.set_attribute("label", &seg.label)?;
        set_attr_bool(&el, "selected", seg.selected)?;
        set_attr_bool(&el, "disabled", seg.disabled || props.disabled)?;

        if let Some(icon) = seg.icon.as_deref() {
            // Nested <md-icon> in the segment's `icon` slot, per the MWC3
            // documented composition pattern.
            let icon_el = create_mwc_element("md-icon")?;
            icon_el.set_attribute("slot", "icon")?;
            icon_el.set_text_content(Some(icon));
            el.append_child(&icon_el)?;
        }

        set.append_child(&el)?;
    }

    Ok(set)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_segmented_button)]
pub fn create_segmented_button_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: SegmentedButtonProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid SegmentedButtonProps JSON: {e}")))?;
    create_segmented_button(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Smoke test: default Props serialises to a `FIELD_COUNT`-key JSON object.
    #[test]
    fn segmented_button_props_default_field_count() {
        let p = SegmentedButtonProps::default();
        let v = serde_json::to_value(&p).expect("serde must serialise SegmentedButtonProps");
        let map = v
            .as_object()
            .expect("SegmentedButtonProps must serialise as object");
        assert_eq!(map.len(), SegmentedButtonProps::FIELD_COUNT);
    }
}
