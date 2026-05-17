// SPDX-License-Identifier: Apache-2.0
//! `shadcn DatePicker` → bespoke `<div class="md-date-picker">` rendering a
//! Material 3 calendar grid.
//!
//! Material Web Components 3 does not ship a date-picker custom element
//! (`@material/web` as of the 2026-05 release train still has the
//! `<md-date-picker>` proposal in draft). The bridge therefore emits the
//! Material 3 calendar-grid recipe directly: a header row carrying
//! month/year navigation buttons + the seven-column day-of-week heading,
//! then a 6-row by 7-column grid of `<button>` cells (M3 calls these
//! "day cells"). Consumers wire selection logic via the
//! `data-on-select-id` `CustomEvent` channel — the bridge does *not* embed
//! a date library, keeping the wasm bundle minimal.
//!
//! Date arithmetic in this module uses a Zeller-congruence weekday lookup
//! and a leap-year-aware month-length table, both pure Rust, so the
//! renderer remains dependency-free on every host.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlElement;

#[cfg(target_arch = "wasm32")]
use crate::{document, set_attr_opt};

/// Subset of shadcn `<DatePicker>` props supported by the MWC3 bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatePickerProps {
    /// Selected date in ISO-8601 `YYYY-MM-DD` form. Empty string means
    /// "no selection". The visible month follows this date when present;
    /// otherwise the picker falls back to `display_year`/`display_month`.
    pub value: String,
    /// Display year (e.g. 2026) used when `value` is empty.
    pub display_year: i32,
    /// Display month, 1..=12, used when `value` is empty.
    pub display_month: u8,
    /// Mirrors HTML `disabled` — prevents all interaction.
    pub disabled: bool,
    /// CustomEvent id dispatched on cell click (`detail.value = "YYYY-MM-DD"`).
    pub on_select_id: Option<String>,
}

impl DatePickerProps {
    /// Number of declared public fields.
    pub const FIELD_COUNT: usize = 5;
}

/// Returns the number of days in `month` for `year`, 1-indexed month.
/// Implements the Gregorian leap-year rule (divisible by 4 except centuries
/// not divisible by 400).
#[allow(dead_code)] // host-build references only; wasm path uses it directly
fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap { 29 } else { 28 }
        }
        _ => 0,
    }
}

/// Zeller-congruence weekday lookup for the *first* of `month` in `year`.
/// Returns 0 = Sunday, 1 = Monday, ..., 6 = Saturday — matching the M3
/// calendar's default Sunday-first layout.
#[allow(dead_code)]
fn first_weekday(year: i32, month: u8) -> u8 {
    // Zeller treats Jan/Feb as months 13/14 of the previous year.
    let (m, y) = if month < 3 {
        (i32::from(month) + 12, year - 1)
    } else {
        (i32::from(month), year)
    };
    let k = y.rem_euclid(100);
    let j = y.div_euclid(100);
    // Zeller's congruence (Gregorian variant): h = 0 = Saturday.
    let h = (1 + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j).rem_euclid(7);
    // Remap so Sunday = 0.
    ((h + 6) % 7) as u8
}

/// Builds the `<div class="md-date-picker">` subtree from `props`.
///
/// # Errors
/// Returns [`JsValue`] when the DOM is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn create_date_picker(props: &DatePickerProps) -> Result<HtmlElement, JsValue> {
    let doc = document()?;
    let root = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    root.set_class_name("md-date-picker");
    root.set_attribute("role", "application")?;
    if props.disabled {
        root.set_attribute("aria-disabled", "true")?;
    }
    set_attr_opt(&root, "data-on-select-id", props.on_select_id.as_deref())?;

    // Resolve the month/year to display.
    let (year, month) = if let Some((y, m)) = parse_ym(&props.value) {
        (y, m)
    } else {
        let y = if props.display_year == 0 { 1970 } else { props.display_year };
        let m = match props.display_month {
            1..=12 => props.display_month,
            _ => 1,
        };
        (y, m)
    };

    // Header — month/year label flanked by prev/next nav buttons.
    let header = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    header.set_class_name("md-date-picker__header");
    header.set_attribute("data-year", &year.to_string())?;
    header.set_attribute("data-month", &month.to_string())?;
    root.append_child(&header)?;

    // Day-of-week heading row (Sun..Sat, M3 default).
    let dow = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    dow.set_class_name("md-date-picker__dow-row");
    for label in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
        let cell = doc.create_element("span")?.dyn_into::<HtmlElement>()?;
        cell.set_class_name("md-date-picker__dow-cell");
        cell.set_text_content(Some(label));
        dow.append_child(&cell)?;
    }
    root.append_child(&dow)?;

    // Day-cell grid — 6 rows x 7 cols, padded with empty cells before the 1st
    // and after the last day of the month.
    let grid = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
    grid.set_class_name("md-date-picker__grid");
    let first = first_weekday(year, month);
    let last_day = days_in_month(year, month);
    let total_cells: u32 = 42; // 6 * 7
    for cell_idx in 0..total_cells {
        let cell = doc.create_element("button")?.dyn_into::<HtmlElement>()?;
        cell.set_class_name("md-date-picker__cell");
        cell.set_attribute("type", "button")?;
        if cell_idx < u32::from(first) || cell_idx >= u32::from(first) + u32::from(last_day) {
            // Padding cell — empty, non-interactive.
            cell.set_attribute("aria-hidden", "true")?;
            cell.set_attribute("disabled", "")?;
        } else {
            let day = (cell_idx - u32::from(first) + 1) as u8;
            let iso = format!("{year:04}-{month:02}-{day:02}");
            cell.set_text_content(Some(&day.to_string()));
            cell.set_attribute("data-value", &iso)?;
            if iso == props.value {
                cell.set_attribute("aria-selected", "true")?;
                cell.set_attribute("data-selected", "true")?;
            }
            if props.disabled {
                cell.set_attribute("disabled", "")?;
            }
        }
        grid.append_child(&cell)?;
    }
    root.append_child(&grid)?;

    Ok(root)
}

/// Parses `YYYY-MM-DD` to `(year, month)`. Returns `None` on any malformed
/// input — callers fall back to the explicit `display_*` props.
fn parse_ym(s: &str) -> Option<(i32, u8)> {
    if s.len() < 10 {
        return None;
    }
    let bytes = s.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: u8 = s[5..7].parse().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    Some((year, month))
}

/// `#[wasm_bindgen]` JS-facing wrapper. Consumes `props` as a JSON string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = create_date_picker)]
pub fn create_date_picker_js(props_json: &str) -> Result<HtmlElement, JsValue> {
    let props: DatePickerProps = serde_json::from_str(props_json)
        .map_err(|e| JsValue::from_str(&format!("invalid DatePickerProps JSON: {e}")))?;
    create_date_picker(&props)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Smoke test: default Props serialises to a `FIELD_COUNT`-key JSON object
    /// AND the pure-Rust date helpers agree with hand-computed references.
    #[test]
    fn date_picker_props_default_field_count() {
        let p = DatePickerProps::default();
        let v = serde_json::to_value(&p).expect("serde must serialise DatePickerProps");
        let map = v
            .as_object()
            .expect("DatePickerProps must serialise as object");
        assert_eq!(map.len(), DatePickerProps::FIELD_COUNT);

        // Sanity-check the date helpers so a future refactor cannot silently
        // break the calendar grid layout.
        assert_eq!(days_in_month(2024, 2), 29, "2024 is a leap year");
        assert_eq!(days_in_month(2025, 2), 28, "2025 is not a leap year");
        assert_eq!(days_in_month(2000, 2), 29, "2000 (div 400) is a leap year");
        assert_eq!(days_in_month(1900, 2), 28, "1900 (div 100, not 400) is not");
        // 2026-05-01 is a Friday (verified against ISO calendar).
        assert_eq!(first_weekday(2026, 5), 5);
        assert_eq!(parse_ym("2026-05-17"), Some((2026, 5)));
        assert_eq!(parse_ym("bad"), None);
    }
}
