// SPDX-License-Identifier: Apache-2.0
//! M3 Snackbar — transient surface with optional action.
//!
//! Canonical metrics: single-line height 48 dp, two-line height 68 dp,
//! min width 344 dp, max width 568 dp, corner radius 4 dp.  Auto-dismiss
//! defaults to 4 s (`SHORT`) or 7 s (`LONG`).

use crate::canvas::{Canvas, Rect, TextStyle};
use crate::m3_tokens::{BASELINE_LIGHT, BODY_MEDIUM, LABEL_LARGE};
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// Material Design 3 Snackbar.
#[derive(Clone, Debug)]
pub struct Snackbar {
    /// Message text.
    pub message: String,
    /// Optional action label.
    pub action: Option<String>,
    /// Auto-dismiss duration in milliseconds.
    pub auto_dismiss_ms: u64,
    /// Number of message lines (1 or 2).
    pub lines: u8,
    /// Layout bounds.
    pub bounds: Rect,
    /// Action-button interaction state.
    pub action_state: State,
    /// Whether the snackbar is currently visible.
    pub visible: bool,
    /// Timestamp the snackbar was shown (ms).
    pub shown_at_ms: u64,
    /// Activation counter (action button clicks).
    pub activations: u32,
}

impl Snackbar {
    /// Canonical single-line height (dp).
    pub const HEIGHT_SINGLE: f32 = 48.0;
    /// Canonical two-line height (dp).
    pub const HEIGHT_DOUBLE: f32 = 68.0;
    /// Canonical max width (dp).
    pub const MAX_WIDTH: f32 = 568.0;
    /// Canonical min width (dp).
    pub const MIN_WIDTH: f32 = 344.0;
    /// Short auto-dismiss duration (ms).
    pub const DURATION_SHORT: u64 = 4_000;
    /// Long auto-dismiss duration (ms).
    pub const DURATION_LONG: u64 = 7_000;

    /// Single-line snackbar.
    #[must_use]
    pub fn single_line(message: impl Into<String>) -> Self {
        Self::new(message, None, 1)
    }

    /// Two-line snackbar.
    #[must_use]
    pub fn two_line(message: impl Into<String>) -> Self {
        Self::new(message, None, 2)
    }

    /// Snackbar with action button.
    #[must_use]
    pub fn with_action(message: impl Into<String>, action: impl Into<String>) -> Self {
        Self::new(message, Some(action.into()), 1)
    }

    fn new(message: impl Into<String>, action: Option<String>, lines: u8) -> Self {
        Self {
            message: message.into(),
            action,
            auto_dismiss_ms: Self::DURATION_SHORT,
            lines,
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            action_state: State::Enabled,
            visible: true,
            shown_at_ms: 0,
            activations: 0,
        }
    }
}

impl MaterialComponent for Snackbar {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        let h = if self.lines >= 2 { Self::HEIGHT_DOUBLE } else { Self::HEIGHT_SINGLE };
        let w = constraints
            .max_width
            .min(Self::MAX_WIDTH)
            .max(constraints.min_width.max(Self::MIN_WIDTH));
        let s = constraints.clamp(w, h);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        if !self.visible {
            return;
        }
        let r = self.bounds;
        // Inverse surface fill (canonical M3 snackbar background).
        canvas.draw_elevation_shadow(r, 4.0, 3);
        canvas.fill_rounded_rect(r, 4.0, BASELINE_LIGHT.inverse_surface);
        canvas.draw_text(
            &self.message,
            Rect::new(r.x + 16.0, r.y, r.w - 32.0 - 96.0, r.h),
            TextStyle {
                size_sp: BODY_MEDIUM.size_sp,
                weight: BODY_MEDIUM.weight,
                color: BASELINE_LIGHT.inverse_on_surface,
            },
        );
        if let Some(action) = &self.action {
            let action_rect = Rect::new(r.x + r.w - 96.0, r.y + 4.0, 88.0, r.h - 8.0);
            canvas.apply_state_layer(
                action_rect,
                (r.h - 8.0) * 0.5,
                BASELINE_LIGHT.inverse_primary,
                self.action_state,
            );
            canvas.draw_text(
                action,
                action_rect,
                TextStyle {
                    size_sp: LABEL_LARGE.size_sp,
                    weight: LABEL_LARGE.weight,
                    color: BASELINE_LIGHT.inverse_primary,
                },
            );
        }
    }

    fn handle_event(&mut self, ev: &Event) -> EventResult {
        match *ev {
            Event::Tick { now_ms } => {
                if self.shown_at_ms == 0 {
                    self.shown_at_ms = now_ms;
                }
                if now_ms.saturating_sub(self.shown_at_ms) >= self.auto_dismiss_ms {
                    self.visible = false;
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }
            Event::PointerDown { x, y } if self.action.is_some() => {
                let r = self.bounds;
                let action_rect = Rect::new(r.x + r.w - 96.0, r.y + 4.0, 88.0, r.h - 8.0);
                if action_rect.contains(x, y) {
                    self.action_state = State::Pressed;
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }
            Event::PointerUp { x, y } if self.action_state == State::Pressed => {
                let r = self.bounds;
                let action_rect = Rect::new(r.x + r.w - 96.0, r.y + 4.0, 88.0, r.h - 8.0);
                self.action_state = State::Enabled;
                if action_rect.contains(x, y) {
                    self.activations = self.activations.saturating_add(1);
                    self.visible = false;
                    return EventResult::Activated;
                }
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
    fn snackbar_auto_dismiss() {
        let mut s = Snackbar::single_line("Saved");
        s.auto_dismiss_ms = 100;
        s.layout(&Constraints::tight(400.0, 48.0));
        s.handle_event(&Event::Tick { now_ms: 50 });
        assert!(s.visible);
        s.handle_event(&Event::Tick { now_ms: 200 });
        assert!(!s.visible);
    }

    #[test]
    fn snackbar_min_width_enforced() {
        let mut s = Snackbar::single_line("hi");
        let sz = s.layout(&Constraints::unbounded());
        assert!(sz.width >= Snackbar::MIN_WIDTH);
        assert!(sz.width <= Snackbar::MAX_WIDTH);
    }
}
