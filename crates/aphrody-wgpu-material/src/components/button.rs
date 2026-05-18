// SPDX-License-Identifier: Apache-2.0
//! M3 Button — 5 variants (Filled, Outlined, Text, Elevated, Tonal).
//!
//! Canonical metrics: height 40 dp, corner radius 20 dp (full pill),
//! horizontal padding 24 dp, label uses `LABEL_LARGE`.  State layer at
//! hover 8% / focus 12% / press 12%.

use crate::canvas::{Canvas, Color, Rect, TextStyle};
use crate::m3_tokens::{self, BASELINE_LIGHT, LABEL_LARGE};
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// Visual variant for [`Button`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    /// High-emphasis filled button (primary).
    Filled,
    /// Medium-emphasis outlined button.
    Outlined,
    /// Low-emphasis text button.
    Text,
    /// Elevated button (surface fill + elevation level 1).
    Elevated,
    /// Tonal button (secondary_container fill).
    Tonal,
}

/// Material Design 3 Button.
#[derive(Clone, Debug)]
pub struct Button {
    /// Visual variant (changes fill, outline, elevation).
    pub variant: ButtonVariant,
    /// Button label.
    pub label: String,
    /// Layout-resolved bounds in logical pixels.
    pub bounds: Rect,
    /// Current interaction state.
    pub state: State,
    /// Disabled flag.
    pub disabled: bool,
    /// Number of activations observed since creation.
    pub activations: u32,
}

impl Button {
    /// Canonical M3 button height (dp).
    pub const HEIGHT: f32 = 40.0;
    /// Canonical M3 button corner radius (dp) — full pill at 40 dp.
    pub const RADIUS: f32 = 20.0;
    /// Canonical horizontal padding (dp).
    pub const H_PADDING: f32 = 24.0;

    /// Build a filled button with `label`.
    #[must_use]
    pub fn filled(label: impl Into<String>) -> Self {
        Self::new(ButtonVariant::Filled, label)
    }

    /// Build an outlined button with `label`.
    #[must_use]
    pub fn outlined(label: impl Into<String>) -> Self {
        Self::new(ButtonVariant::Outlined, label)
    }

    /// Build a text button with `label`.
    #[must_use]
    pub fn text(label: impl Into<String>) -> Self {
        Self::new(ButtonVariant::Text, label)
    }

    /// Build an elevated button with `label`.
    #[must_use]
    pub fn elevated(label: impl Into<String>) -> Self {
        Self::new(ButtonVariant::Elevated, label)
    }

    /// Build a tonal button with `label`.
    #[must_use]
    pub fn tonal(label: impl Into<String>) -> Self {
        Self::new(ButtonVariant::Tonal, label)
    }

    fn new(variant: ButtonVariant, label: impl Into<String>) -> Self {
        Self {
            variant,
            label: label.into(),
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            state: State::Enabled,
            disabled: false,
            activations: 0,
        }
    }

    fn fill_color(&self) -> Color {
        match self.variant {
            ButtonVariant::Filled => BASELINE_LIGHT.primary,
            ButtonVariant::Tonal => BASELINE_LIGHT.secondary_container,
            ButtonVariant::Elevated => BASELINE_LIGHT.surface,
            ButtonVariant::Outlined | ButtonVariant::Text => Color::TRANSPARENT,
        }
    }

    fn label_color(&self) -> Color {
        match self.variant {
            ButtonVariant::Filled => BASELINE_LIGHT.on_primary,
            ButtonVariant::Tonal => BASELINE_LIGHT.on_secondary_container,
            ButtonVariant::Elevated
            | ButtonVariant::Outlined
            | ButtonVariant::Text => BASELINE_LIGHT.primary,
        }
    }
}

impl MaterialComponent for Button {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        // Estimated label width: ≈ 8 dp per char (LABEL_LARGE = 14 sp medium).
        let est_label_w = self.label.chars().count() as f32 * 8.0;
        let min_w = (est_label_w + 2.0 * Self::H_PADDING).max(64.0);
        let size = constraints.clamp(min_w, Self::HEIGHT);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, size.width, size.height);
        size
    }

    fn paint(&self, canvas: &mut Canvas) {
        let r = self.bounds;
        // Elevation shadow for Elevated variant.
        if self.variant == ButtonVariant::Elevated && !self.disabled {
            canvas.draw_elevation_shadow(r, Self::RADIUS, 1);
        }
        // Background fill.
        let fill = self.fill_color();
        if fill.a > 0.0 {
            canvas.fill_rounded_rect(r, Self::RADIUS, fill);
        }
        // Outline ring for Outlined variant — drawn as outer rect minus inset rect.
        if self.variant == ButtonVariant::Outlined {
            canvas.fill_rounded_rect(r, Self::RADIUS, BASELINE_LIGHT.outline);
            canvas.fill_rounded_rect(
                r.inset(1.0, 1.0),
                (Self::RADIUS - 1.0).max(0.0),
                BASELINE_LIGHT.surface,
            );
        }
        // State layer overlay.
        let overlay = self.label_color();
        canvas.apply_state_layer(r, Self::RADIUS, overlay, self.state);
        // Label text.
        canvas.draw_text(
            &self.label,
            r,
            TextStyle {
                size_sp: LABEL_LARGE.size_sp,
                weight: LABEL_LARGE.weight,
                color: self.label_color(),
            },
        );
        // Mute unused: tokens module ref for compile coverage.
        let _ = m3_tokens::shape::SMALL;
    }

    fn handle_event(&mut self, ev: &Event) -> EventResult {
        if self.disabled {
            return EventResult::Ignored;
        }
        match *ev {
            Event::PointerEnter { .. } if self.state == State::Enabled => {
                self.state = State::Hover;
                EventResult::Consumed
            }
            Event::PointerLeave if matches!(self.state, State::Hover | State::Pressed) => {
                self.state = State::Enabled;
                EventResult::Consumed
            }
            Event::PointerDown { x, y } if self.bounds.contains(x, y) => {
                self.state = State::Pressed;
                EventResult::Consumed
            }
            Event::PointerUp { x, y } if self.state == State::Pressed => {
                self.state = if self.bounds.contains(x, y) {
                    self.activations = self.activations.saturating_add(1);
                    State::Hover
                } else {
                    State::Enabled
                };
                EventResult::Activated
            }
            Event::FocusGained => {
                self.state = State::Focus;
                EventResult::Consumed
            }
            Event::FocusLost if self.state == State::Focus => {
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
    fn button_size_filled() {
        let mut b = Button::filled("OK");
        let s = b.layout(&Constraints::unbounded());
        assert!((s.height - 40.0).abs() < f32::EPSILON);
        assert!(s.width >= 64.0);
    }

    #[test]
    fn button_press_activate() {
        let mut b = Button::filled("Go");
        b.bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        let r1 = b.handle_event(&Event::PointerDown { x: 50.0, y: 20.0 });
        assert_eq!(r1, EventResult::Consumed);
        assert_eq!(b.state, State::Pressed);
        let r2 = b.handle_event(&Event::PointerUp { x: 50.0, y: 20.0 });
        assert_eq!(r2, EventResult::Activated);
        assert_eq!(b.activations, 1);
    }

    #[test]
    fn button_disabled_ignores_events() {
        let mut b = Button::filled("Off");
        b.bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        b.disabled = true;
        let r = b.handle_event(&Event::PointerDown { x: 50.0, y: 20.0 });
        assert_eq!(r, EventResult::Ignored);
    }
}
