// SPDX-License-Identifier: Apache-2.0
//! M3 Card — Elevated, Filled, Outlined variants.
//!
//! Canonical metrics: corner radius 12 dp (`MEDIUM`), elevation level 1
//! for elevated / level 0 for filled+outlined.

use crate::canvas::{Canvas, Color, Rect};
use crate::m3_tokens::{BASELINE_LIGHT, shape};
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// Visual variant for [`Card`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardVariant {
    /// Elevated card — surface fill, elevation 1.
    Elevated,
    /// Filled card — surface_variant fill, no elevation.
    Filled,
    /// Outlined card — surface fill, 1 dp outline.
    Outlined,
}

/// Material Design 3 Card surface.
#[derive(Clone, Debug)]
pub struct Card {
    /// Variant.
    pub variant: CardVariant,
    /// Resting elevation level (0..=5), applied to Elevated variant.
    pub elevation_level: u8,
    /// Layout bounds.
    pub bounds: Rect,
    /// Interaction state.
    pub state: State,
}

impl Card {
    /// Canonical M3 card corner radius (dp).
    pub const RADIUS: f32 = shape::MEDIUM;

    /// Build an elevated card.
    #[must_use]
    pub fn elevated() -> Self {
        Self::new(CardVariant::Elevated)
    }

    /// Build a filled card.
    #[must_use]
    pub fn filled() -> Self {
        Self::new(CardVariant::Filled)
    }

    /// Build an outlined card.
    #[must_use]
    pub fn outlined() -> Self {
        Self::new(CardVariant::Outlined)
    }

    fn new(variant: CardVariant) -> Self {
        Self {
            variant,
            elevation_level: if matches!(variant, CardVariant::Elevated) { 1 } else { 0 },
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            state: State::Enabled,
        }
    }

    fn fill_color(&self) -> Color {
        match self.variant {
            CardVariant::Elevated => BASELINE_LIGHT.surface,
            CardVariant::Filled => BASELINE_LIGHT.surface_variant,
            CardVariant::Outlined => BASELINE_LIGHT.surface,
        }
    }
}

impl MaterialComponent for Card {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        // Cards take their parent's max size — no intrinsic minimum.
        let w = constraints.max_width.min(360.0).max(constraints.min_width);
        let h = constraints.max_height.min(200.0).max(constraints.min_height);
        let s = constraints.clamp(w, h);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        let r = self.bounds;
        // Shadow for elevated.
        if matches!(self.variant, CardVariant::Elevated) {
            canvas.draw_elevation_shadow(r, Self::RADIUS, self.elevation_level);
        }
        // Outline ring for Outlined.
        if matches!(self.variant, CardVariant::Outlined) {
            canvas.fill_rounded_rect(r, Self::RADIUS, BASELINE_LIGHT.outline_variant);
            canvas.fill_rounded_rect(
                r.inset(1.0, 1.0),
                (Self::RADIUS - 1.0).max(0.0),
                self.fill_color(),
            );
        } else {
            canvas.fill_rounded_rect(r, Self::RADIUS, self.fill_color());
        }
        // Optional state overlay (cards are sometimes interactive).
        canvas.apply_state_layer(r, Self::RADIUS, BASELINE_LIGHT.on_surface, self.state);
    }

    fn handle_event(&mut self, ev: &Event) -> EventResult {
        match *ev {
            Event::PointerEnter { .. } => {
                self.state = State::Hover;
                EventResult::Consumed
            }
            Event::PointerLeave => {
                self.state = State::Enabled;
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_default_elevation() {
        let c = Card::elevated();
        assert_eq!(c.elevation_level, 1);
        assert_eq!(c.variant, CardVariant::Elevated);
    }

    #[test]
    fn card_layout_respects_max() {
        let mut c = Card::filled();
        let s = c.layout(&Constraints {
            min_width: 100.0,
            max_width: 500.0,
            min_height: 50.0,
            max_height: 300.0,
        });
        assert!(s.width >= 100.0 && s.width <= 500.0);
        assert!(s.height >= 50.0 && s.height <= 300.0);
    }
}
