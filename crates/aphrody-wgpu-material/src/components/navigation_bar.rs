// SPDX-License-Identifier: Apache-2.0
//! M3 Navigation Bar — 3 to 5 destinations.
//!
//! Canonical metrics: height 80 dp, indicator pill 32 dp × 64 dp,
//! corner radius (pill) 16 dp.

use crate::canvas::{Canvas, Rect, TextStyle};
use crate::m3_tokens::{BASELINE_LIGHT, LABEL_MEDIUM};
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// A single navigation destination (icon + label).
#[derive(Clone, Debug)]
pub struct NavigationDestination {
    /// Text label.
    pub label: String,
    /// Whether this destination is currently selected.
    pub selected: bool,
    /// Hover/press state.
    pub state: State,
}

impl NavigationDestination {
    /// Build a new destination with the given label.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), selected: false, state: State::Enabled }
    }
}

/// Material Design 3 Navigation Bar.
#[derive(Clone, Debug)]
pub struct NavigationBar {
    /// Destinations (3..=5).
    pub destinations: Vec<NavigationDestination>,
    /// Bounds.
    pub bounds: Rect,
}

impl NavigationBar {
    /// Canonical M3 nav bar height (dp).
    pub const HEIGHT: f32 = 80.0;
    /// Active indicator pill height (dp).
    pub const INDICATOR_HEIGHT: f32 = 32.0;
    /// Active indicator pill width (dp).
    pub const INDICATOR_WIDTH: f32 = 64.0;

    /// Build a navigation bar from a list of labels.
    ///
    /// # Panics
    /// Panics if `labels.len()` is outside the canonical M3 range 3..=5.
    #[must_use]
    pub fn new(labels: &[&str]) -> Self {
        assert!(
            (3..=5).contains(&labels.len()),
            "M3 NavigationBar requires 3..=5 destinations, got {}",
            labels.len()
        );
        let mut destinations: Vec<NavigationDestination> =
            labels.iter().map(|l| NavigationDestination::new(*l)).collect();
        if let Some(first) = destinations.first_mut() {
            first.selected = true;
        }
        Self { destinations, bounds: Rect::new(0.0, 0.0, 0.0, 0.0) }
    }

    /// Index of the currently selected destination.
    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        self.destinations.iter().position(|d| d.selected)
    }

    /// Select destination at `index`.
    pub fn select(&mut self, index: usize) {
        for (i, d) in self.destinations.iter_mut().enumerate() {
            d.selected = i == index;
        }
    }
}

impl MaterialComponent for NavigationBar {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        let w = constraints.max_width.max(constraints.min_width);
        let s = constraints.clamp(w, Self::HEIGHT);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        let r = self.bounds;
        canvas.fill_rounded_rect(r, 0.0, BASELINE_LIGHT.surface);
        let n = self.destinations.len() as f32;
        let slot_w = r.w / n;
        for (i, dest) in self.destinations.iter().enumerate() {
            let slot_x = r.x + i as f32 * slot_w;
            let slot = Rect::new(slot_x, r.y, slot_w, r.h);
            // Active indicator pill — drawn above icon when selected.
            if dest.selected {
                let pill_x = slot_x + (slot_w - Self::INDICATOR_WIDTH) * 0.5;
                let pill_y = r.y + 12.0;
                canvas.fill_rounded_rect(
                    Rect::new(pill_x, pill_y, Self::INDICATOR_WIDTH, Self::INDICATOR_HEIGHT),
                    Self::INDICATOR_HEIGHT * 0.5,
                    BASELINE_LIGHT.secondary_container,
                );
            }
            // State layer overlay on the indicator area.
            let overlay_rect = Rect::new(
                slot_x + (slot_w - Self::INDICATOR_WIDTH) * 0.5,
                r.y + 12.0,
                Self::INDICATOR_WIDTH,
                Self::INDICATOR_HEIGHT,
            );
            canvas.apply_state_layer(
                overlay_rect,
                Self::INDICATOR_HEIGHT * 0.5,
                BASELINE_LIGHT.on_surface,
                dest.state,
            );
            // Icon glyph placeholder — small filled circle centered in the pill.
            let (cx, cy) = (slot_x + slot_w * 0.5, r.y + 12.0 + Self::INDICATOR_HEIGHT * 0.5);
            canvas.draw_circle(
                cx,
                cy,
                10.0,
                if dest.selected {
                    BASELINE_LIGHT.on_secondary_container
                } else {
                    BASELINE_LIGHT.on_surface_variant
                },
            );
            // Label.
            let label_rect = Rect::new(slot_x, r.y + 12.0 + Self::INDICATOR_HEIGHT + 4.0, slot_w, 20.0);
            canvas.draw_text(
                &dest.label,
                label_rect,
                TextStyle {
                    size_sp: LABEL_MEDIUM.size_sp,
                    weight: LABEL_MEDIUM.weight,
                    color: if dest.selected {
                        BASELINE_LIGHT.on_surface
                    } else {
                        BASELINE_LIGHT.on_surface_variant
                    },
                },
            );
            let _ = slot; // silence
        }
    }

    fn handle_event(&mut self, ev: &Event) -> EventResult {
        match *ev {
            Event::PointerUp { x, y } if self.bounds.contains(x, y) => {
                let n = self.destinations.len() as f32;
                let slot_w = self.bounds.w / n;
                let idx = ((x - self.bounds.x) / slot_w).floor() as usize;
                if idx < self.destinations.len() {
                    self.select(idx);
                    return EventResult::Activated;
                }
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_bar_first_destination_selected() {
        let n = NavigationBar::new(&["Home", "Search", "Library"]);
        assert_eq!(n.selected_index(), Some(0));
    }

    #[test]
    fn nav_bar_select_changes_index() {
        let mut n = NavigationBar::new(&["A", "B", "C", "D"]);
        n.select(2);
        assert_eq!(n.selected_index(), Some(2));
    }

    #[test]
    #[should_panic]
    fn nav_bar_rejects_two_destinations() {
        let _ = NavigationBar::new(&["A", "B"]);
    }
}
