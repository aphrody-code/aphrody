// SPDX-License-Identifier: Apache-2.0
//! M3 Tabs — Primary and Secondary variants.
//!
//! Canonical metrics: tab height 48 dp (primary, icon + label) / 48 dp
//! (secondary, label only), indicator 3 dp height, divider 1 dp.

use crate::canvas::{Canvas, Rect, TextStyle};
use crate::m3_tokens::{BASELINE_LIGHT, TITLE_SMALL};
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// Tabs variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabsVariant {
    /// Primary tabs — top-level navigation; indicator pill above text.
    Primary,
    /// Secondary tabs — nested within a primary section.
    Secondary,
}

/// A single tab.
#[derive(Clone, Debug)]
pub struct Tab {
    /// Label text.
    pub label: String,
    /// Selected flag.
    pub selected: bool,
    /// Interaction state.
    pub state: State,
}

impl Tab {
    /// Construct a new tab.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), selected: false, state: State::Enabled }
    }
}

/// Material Design 3 Tabs strip.
#[derive(Clone, Debug)]
pub struct Tabs {
    /// Variant.
    pub variant: TabsVariant,
    /// Tabs (≥ 2).
    pub tabs: Vec<Tab>,
    /// Bounds.
    pub bounds: Rect,
}

impl Tabs {
    /// Canonical M3 tabs height (dp).
    pub const HEIGHT: f32 = 48.0;
    /// Active indicator height (dp).
    pub const INDICATOR_HEIGHT: f32 = 3.0;

    /// Build a primary tab strip.
    #[must_use]
    pub fn primary(labels: &[&str]) -> Self {
        Self::new(TabsVariant::Primary, labels)
    }

    /// Build a secondary tab strip.
    #[must_use]
    pub fn secondary(labels: &[&str]) -> Self {
        Self::new(TabsVariant::Secondary, labels)
    }

    fn new(variant: TabsVariant, labels: &[&str]) -> Self {
        assert!(labels.len() >= 2, "M3 Tabs requires ≥ 2 tabs");
        let mut tabs: Vec<Tab> = labels.iter().map(|l| Tab::new(*l)).collect();
        if let Some(first) = tabs.first_mut() {
            first.selected = true;
        }
        Self { variant, tabs, bounds: Rect::new(0.0, 0.0, 0.0, 0.0) }
    }

    /// Currently selected tab index.
    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        self.tabs.iter().position(|t| t.selected)
    }

    /// Select tab at `index`.
    pub fn select(&mut self, index: usize) {
        for (i, t) in self.tabs.iter_mut().enumerate() {
            t.selected = i == index;
        }
    }
}

impl MaterialComponent for Tabs {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        let w = constraints.max_width.max(constraints.min_width);
        let s = constraints.clamp(w, Self::HEIGHT);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        let r = self.bounds;
        // Background.
        canvas.fill_rounded_rect(r, 0.0, BASELINE_LIGHT.surface);
        // Divider line at the bottom.
        canvas.fill_rounded_rect(
            Rect::new(r.x, r.y + r.h - 1.0, r.w, 1.0),
            0.0,
            BASELINE_LIGHT.outline_variant,
        );
        let n = self.tabs.len() as f32;
        let slot_w = r.w / n;
        for (i, tab) in self.tabs.iter().enumerate() {
            let slot_x = r.x + i as f32 * slot_w;
            let slot = Rect::new(slot_x, r.y, slot_w, r.h);
            canvas.apply_state_layer(slot, 0.0, BASELINE_LIGHT.on_surface, tab.state);
            // Label centered vertically.
            canvas.draw_text(
                &tab.label,
                Rect::new(slot_x, r.y + 12.0, slot_w, 24.0),
                TextStyle {
                    size_sp: TITLE_SMALL.size_sp,
                    weight: TITLE_SMALL.weight,
                    color: if tab.selected {
                        BASELINE_LIGHT.primary
                    } else {
                        BASELINE_LIGHT.on_surface_variant
                    },
                },
            );
            // Active indicator.
            if tab.selected {
                let ind_w = match self.variant {
                    TabsVariant::Primary => slot_w * 0.5,
                    TabsVariant::Secondary => slot_w,
                };
                let ind_x = slot_x + (slot_w - ind_w) * 0.5;
                canvas.fill_rounded_rect(
                    Rect::new(ind_x, r.y + r.h - Self::INDICATOR_HEIGHT, ind_w, Self::INDICATOR_HEIGHT),
                    if matches!(self.variant, TabsVariant::Primary) { Self::INDICATOR_HEIGHT * 0.5 } else { 0.0 },
                    BASELINE_LIGHT.primary,
                );
            }
        }
    }

    fn handle_event(&mut self, ev: &Event) -> EventResult {
        match *ev {
            Event::PointerUp { x, y } if self.bounds.contains(x, y) => {
                let n = self.tabs.len() as f32;
                let slot_w = self.bounds.w / n;
                let idx = ((x - self.bounds.x) / slot_w).floor() as usize;
                if idx < self.tabs.len() {
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
    fn tabs_first_selected() {
        let t = Tabs::primary(&["One", "Two"]);
        assert_eq!(t.selected_index(), Some(0));
    }

    #[test]
    fn tabs_select_idx() {
        let mut t = Tabs::secondary(&["A", "B", "C"]);
        t.select(2);
        assert_eq!(t.selected_index(), Some(2));
    }
}
