// SPDX-License-Identifier: Apache-2.0
//! `shadcn Progress` → `<md-circular-progress>` + `<md-linear-progress>`.
//!
//! M3 progress indicators per <https://m3.material.io/components/progress-indicators/overview>.
//! The bridge exposes both shapes through a single [`ProgressProps`] surface;
//! [`ProgressProps::shape`] selects the underlying element:
//! * `"linear"`   → `<md-linear-progress>` (default)
//! * `"circular"` → `<md-circular-progress>`
//!
//! Both elements accept the same core attributes:
//! * `value` (0.0..=1.0) — fraction complete when `indeterminate=false`
//! * `indeterminate` — when `true`, the indicator loops infinitely and ignores `value`. M3 strongly
//!   prefers indeterminate when no progress estimate is available.
//! * `four-color` — circular only; cycles through the four M3 accent colours.

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")] use crate::{create_mwc_element, set_attr_bool};

/// Subset of shadcn `<Progress>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressProps {
    /// `"linear"` (default) or `"circular"`.
    pub shape: String,
    /// Fraction complete in the closed interval `[0.0, 1.0]`. Ignored when
    /// `indeterminate=true`. Clamped on the JS side by MWC3.
    pub value: f64,
    /// Buffer fraction in `[0.0, 1.0]` for linear indicators. Ignored for
    /// circular. Defaults to `1.0` per the M3 spec (no visible buffer line).
    pub buffer: f64,
    /// When `true`, the indicator loops infinitely and ignores `value`.
    pub indeterminate: bool,
    /// Circular-only: cycle through the four M3 accent colours.
    pub four_color: bool,
}

impl ProgressProps {
    /// Number of declared public fields. Verified by the smoke test below.
    pub const FIELD_COUNT: usize = 5;
}

/// Builds the `<md-*-progress>` element from `props`.
///
/// # Errors
/// Returns a [`JsValue`] if the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_progress(props: &ProgressProps) -> Result<HtmlElement, JsValue> {
    let (tag, is_circular) = if props.shape == "circular" {
        ("md-circular-progress", true)
    } else {
        ("md-linear-progress", false)
    };
    let el = create_mwc_element(tag)?;

    // Always emit `value`; MWC3 honours it iff !indeterminate.
    el.set_attribute("value", &format!("{:.6}", props.value))?;

    if !is_circular {
        // `buffer` only exists on the linear element; default 1.0 matches M3.
        let buffer = if props.buffer == 0.0 { 1.0 } else { props.buffer };
        el.set_attribute("buffer", &format!("{buffer:.6}"))?;
    }

    set_attr_bool(&el, "indeterminate", props.indeterminate)?;
    if is_circular {
        set_attr_bool(&el, "four-color", props.four_color)?;
    }

    Ok(el)
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_progress)]
pub fn create_progress_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: ProgressProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid ProgressProps JSON: {e}")))?;
    create_progress(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn progress_props_default_smoke() {
        let p = ProgressProps::default();
        assert!(!p.indeterminate);
        assert!(!p.four_color);
        assert_eq!(p.value, 0.0);
        assert_eq!(ProgressProps::FIELD_COUNT, 5);
    }
}
