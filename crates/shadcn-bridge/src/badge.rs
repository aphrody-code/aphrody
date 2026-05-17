// SPDX-License-Identifier: Apache-2.0
//! `shadcn Badge` → bespoke `<span class="md-badge">`.
//!
//! Material Web Components 3 does not currently ship a first-class badge
//! custom element (the M3 spec describes badges as a navigation-bar /
//! navigation-rail decoration rendered via the host element's `<md-badge>`
//! slot stub, which is itself unshipped in `@material/web` as of the 2026-05
//! release train). The bridge therefore emits the official Material 3 badge
//! recipe directly: a `<span>` carrying the `md-badge` class plus a
//! variant-driven modifier (`md-badge--small` for the 6dp dot, default for
//! the 16dp number badge). The class names mirror the public design-tokens
//! used by `@material/web` consumers so downstream CSS can target a single
//! selector regardless of whether MWC3 starts shipping a `<md-badge>`
//! element later.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")] use crate::document;

/// Subset of shadcn `<Badge>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BadgeProps {
    /// Numeric or short textual label shown inside the badge (e.g. `"3"` or
    /// `"99+"`). When empty, the badge collapses to the M3 6dp "dot" form.
    pub label: String,
    /// shadcn variant — one of `"default"`, `"secondary"`, `"destructive"`,
    /// `"outline"`. Maps to a `md-badge--{variant}` modifier class.
    pub variant: String,
    /// When true, force the small-dot rendering regardless of `label`.
    /// Mirrors the M3 spec's "small badge" affordance for notification dots.
    pub small: bool,
}

impl BadgeProps {
    /// Number of declared public fields. Acts as a regression guard against
    /// accidental API shrink (mirrors the convention used by every other
    /// module in this crate).
    pub const FIELD_COUNT: usize = 3;
}

/// Builds the `<span class="md-badge">` element from `props`.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable (e.g. running inside a
/// Worker without a document).
#[cfg(target_arch = "wasm32")]
pub fn create_badge(props: &BadgeProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let root = doc.create_element("span")?.dyn_into::<HtmlElement>()?;

    let variant = match props.variant.as_str() {
        "secondary" | "destructive" | "outline" => props.variant.as_str(),
        _ => "default",
    };
    let small = props.small || props.label.is_empty();
    let class = if small {
        format!("md-badge md-badge--small md-badge--{variant}")
    } else {
        format!("md-badge md-badge--{variant}")
    };
    root.set_class_name(&class);

    // ARIA: badges convey a "status" landmark — see WAI-ARIA 1.2 §5.4.4.
    root.set_attribute("role", "status")?;

    if !small {
        root.set_text_content(Some(&props.label));
    }

    Ok(root)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_badge)]
pub fn create_badge_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: BadgeProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid BadgeProps JSON: {e}")))?;
    create_badge(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Smoke test: default Props serialises to a 3-key JSON object — the
    /// `FIELD_COUNT` invariant the lib-level test suite relies on.
    #[test]
    fn badge_props_default_field_count() {
        let p = BadgeProps::default();
        let v = serde_json::to_value(&p).expect("serde must serialise BadgeProps");
        let map = v.as_object().expect("BadgeProps must serialise as object");
        assert_eq!(map.len(), BadgeProps::FIELD_COUNT);
    }
}
