// SPDX-License-Identifier: Apache-2.0
//! M3 TextField — Filled and Outlined variants.
//!
//! Canonical metrics: height 56 dp, label floats from baseline 16 dp
//! down→up 8 dp on focus, corner radius 4 dp top corners (Filled) or
//! all corners (Outlined).

use crate::canvas::{Canvas, Color, Rect, TextStyle};
use crate::m3_tokens::{BASELINE_LIGHT, BODY_LARGE, BODY_SMALL};
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// Visual variant for [`TextField`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFieldVariant {
    /// Filled text field — surface_variant fill, bottom indicator.
    Filled,
    /// Outlined text field — 1 dp stroke around full rect.
    Outlined,
}

/// Material Design 3 TextField.
#[derive(Clone, Debug)]
pub struct TextField {
    /// Variant.
    pub variant: TextFieldVariant,
    /// Floating label text.
    pub label: String,
    /// Current value (user text).
    pub value: String,
    /// Layout bounds.
    pub bounds: Rect,
    /// Interaction state.
    pub state: State,
    /// Whether the floating label is in the "raised" position (focused or filled).
    pub label_raised: bool,
}

impl TextField {
    /// Canonical M3 TextField height (dp).
    pub const HEIGHT: f32 = 56.0;
    /// Top-corner radius (dp) for Filled variant.
    pub const RADIUS_FILLED: f32 = 4.0;
    /// All-corner radius (dp) for Outlined variant.
    pub const RADIUS_OUTLINED: f32 = 4.0;

    /// Build a Filled text field.
    #[must_use]
    pub fn filled(label: impl Into<String>) -> Self {
        Self::new(TextFieldVariant::Filled, label)
    }

    /// Build an Outlined text field.
    #[must_use]
    pub fn outlined(label: impl Into<String>) -> Self {
        Self::new(TextFieldVariant::Outlined, label)
    }

    fn new(variant: TextFieldVariant, label: impl Into<String>) -> Self {
        Self {
            variant,
            label: label.into(),
            value: String::new(),
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            state: State::Enabled,
            label_raised: false,
        }
    }

    fn recompute_label(&mut self) {
        self.label_raised = self.state == State::Focus || !self.value.is_empty();
    }
}

impl MaterialComponent for TextField {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        let w = constraints.max_width.min(280.0).max(constraints.min_width.max(120.0));
        let s = constraints.clamp(w, Self::HEIGHT);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        let r = self.bounds;
        match self.variant {
            TextFieldVariant::Filled => {
                canvas.fill_rounded_rect(r, Self::RADIUS_FILLED, BASELINE_LIGHT.surface_variant);
                // Bottom active indicator: 1 dp default / 2 dp when focused.
                let indicator_h = if self.state == State::Focus { 2.0 } else { 1.0 };
                let indicator_color = if self.state == State::Focus {
                    BASELINE_LIGHT.primary
                } else {
                    BASELINE_LIGHT.on_surface_variant
                };
                canvas.fill_rounded_rect(
                    Rect::new(r.x, r.y + r.h - indicator_h, r.w, indicator_h),
                    0.0,
                    indicator_color,
                );
            }
            TextFieldVariant::Outlined => {
                let stroke = if self.state == State::Focus { 2.0 } else { 1.0 };
                let stroke_color = if self.state == State::Focus {
                    BASELINE_LIGHT.primary
                } else {
                    BASELINE_LIGHT.outline
                };
                canvas.fill_rounded_rect(r, Self::RADIUS_OUTLINED, stroke_color);
                canvas.fill_rounded_rect(
                    r.inset(stroke, stroke),
                    (Self::RADIUS_OUTLINED - stroke).max(0.0),
                    BASELINE_LIGHT.surface,
                );
            }
        }
        canvas.apply_state_layer(r, Self::RADIUS_FILLED, BASELINE_LIGHT.on_surface, self.state);
        // Floating label: baseline at 28 dp when resting, 8 dp when raised.
        let label_y_offset = if self.label_raised { 8.0 } else { 28.0 };
        let label_style = if self.label_raised {
            TextStyle {
                size_sp: BODY_SMALL.size_sp,
                weight: BODY_SMALL.weight,
                color: if self.state == State::Focus {
                    BASELINE_LIGHT.primary
                } else {
                    BASELINE_LIGHT.on_surface_variant
                },
            }
        } else {
            TextStyle {
                size_sp: BODY_LARGE.size_sp,
                weight: BODY_LARGE.weight,
                color: BASELINE_LIGHT.on_surface_variant,
            }
        };
        canvas.draw_text(
            &self.label,
            Rect::new(r.x + 16.0, r.y + label_y_offset - 8.0, r.w - 32.0, 24.0),
            label_style,
        );
        // Value text — only rendered when present.
        if !self.value.is_empty() {
            canvas.draw_text(
                &self.value,
                Rect::new(r.x + 16.0, r.y + 28.0, r.w - 32.0, 24.0),
                TextStyle {
                    size_sp: BODY_LARGE.size_sp,
                    weight: BODY_LARGE.weight,
                    color: BASELINE_LIGHT.on_surface,
                },
            );
        }
        let _ = Color::TRANSPARENT;
    }

    fn handle_event(&mut self, ev: &Event) -> EventResult {
        match *ev {
            Event::FocusGained => {
                self.state = State::Focus;
                self.recompute_label();
                EventResult::Consumed
            }
            Event::FocusLost => {
                self.state = State::Enabled;
                self.recompute_label();
                EventResult::Consumed
            }
            Event::PointerEnter { .. } if self.state != State::Focus => {
                self.state = State::Hover;
                EventResult::Consumed
            }
            Event::PointerLeave if self.state == State::Hover => {
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
    fn text_field_height() {
        let mut t = TextField::filled("Name");
        let s = t.layout(&Constraints::unbounded());
        assert!((s.height - 56.0).abs() < f32::EPSILON);
    }

    #[test]
    fn text_field_focus_raises_label() {
        let mut t = TextField::filled("Email");
        t.handle_event(&Event::FocusGained);
        assert!(t.label_raised);
        t.handle_event(&Event::FocusLost);
        assert!(!t.label_raised);
    }
}
