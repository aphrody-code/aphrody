// SPDX-License-Identifier: Apache-2.0
//! M3 Slider — continuous and discrete variants.
//!
//! Canonical metrics: track height 16 dp (active) / 4 dp (inactive),
//! thumb width 4 dp × height 44 dp (M3 expressive thumb), stop dot
//! diameter 2 dp.

use crate::canvas::{Canvas, Color, Rect};
use crate::m3_tokens::BASELINE_LIGHT;
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// Material Design 3 Slider.
#[derive(Clone, Debug)]
pub struct Slider {
    /// Bounds.
    pub bounds: Rect,
    /// Current value, normalized to `[min, max]`.
    pub value: f32,
    /// Minimum value (inclusive).
    pub min: f32,
    /// Maximum value (inclusive).
    pub max: f32,
    /// Discrete stops (≥ 0).  When `> 0` the slider snaps to `stops + 1` evenly-spaced positions.
    pub stops: u32,
    /// Interaction state.
    pub state: State,
    /// Whether the thumb is currently being dragged.
    pub dragging: bool,
    /// Activation counter (drag commits).
    pub activations: u32,
}

impl Slider {
    /// Canonical slider height (overall) in dp.
    pub const HEIGHT: f32 = 44.0;
    /// Active track height (dp).
    pub const TRACK_HEIGHT_ACTIVE: f32 = 16.0;
    /// Inactive track height (dp).
    pub const TRACK_HEIGHT_INACTIVE: f32 = 4.0;
    /// Thumb width (dp).
    pub const THUMB_WIDTH: f32 = 4.0;
    /// Thumb height (dp).
    pub const THUMB_HEIGHT: f32 = 44.0;

    /// Continuous slider over `[0.0, 1.0]`.
    #[must_use]
    pub fn continuous() -> Self {
        Self::new(0.0, 1.0, 0)
    }

    /// Discrete slider with `stops` interior stops between `min` and `max`.
    #[must_use]
    pub fn discrete(min: f32, max: f32, stops: u32) -> Self {
        Self::new(min, max, stops)
    }

    fn new(min: f32, max: f32, stops: u32) -> Self {
        assert!(max > min, "slider max ({}) must be > min ({})", max, min);
        Self {
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            value: min,
            min,
            max,
            stops,
            state: State::Enabled,
            dragging: false,
            activations: 0,
        }
    }

    /// Clamp + snap `v` to `[min, max]` honoring discrete stops.
    #[must_use]
    pub fn clamp_value(&self, v: f32) -> f32 {
        let clamped = v.clamp(self.min, self.max);
        if self.stops == 0 {
            return clamped;
        }
        let step = (self.max - self.min) / (self.stops as f32 + 1.0);
        let n = ((clamped - self.min) / step).round();
        (self.min + n * step).clamp(self.min, self.max)
    }

    /// Set value with clamping/snapping.
    pub fn set_value(&mut self, v: f32) {
        self.value = self.clamp_value(v);
    }

    /// Normalized value in `[0.0, 1.0]`.
    fn normalized(&self) -> f32 {
        (self.value - self.min) / (self.max - self.min)
    }
}

impl MaterialComponent for Slider {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        let w = constraints.max_width.max(constraints.min_width.max(120.0));
        let s = constraints.clamp(w, Self::HEIGHT);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        let r = self.bounds;
        let cy = r.y + r.h * 0.5;
        let t = self.normalized();
        let thumb_x = r.x + 8.0 + (r.w - 16.0) * t;
        // Inactive track (after thumb).
        canvas.fill_rounded_rect(
            Rect::new(thumb_x, cy - Self::TRACK_HEIGHT_INACTIVE * 0.5, r.x + r.w - thumb_x, Self::TRACK_HEIGHT_INACTIVE),
            Self::TRACK_HEIGHT_INACTIVE * 0.5,
            BASELINE_LIGHT.surface_variant,
        );
        // Active track (before thumb).
        canvas.fill_rounded_rect(
            Rect::new(r.x, cy - Self::TRACK_HEIGHT_ACTIVE * 0.5, thumb_x - r.x, Self::TRACK_HEIGHT_ACTIVE),
            Self::TRACK_HEIGHT_ACTIVE * 0.5,
            BASELINE_LIGHT.primary,
        );
        // Tick marks (discrete only).
        if self.stops > 0 {
            for i in 1..=self.stops {
                let f = i as f32 / (self.stops as f32 + 1.0);
                let tx = r.x + 8.0 + (r.w - 16.0) * f;
                canvas.draw_circle(tx, cy, 1.5, BASELINE_LIGHT.on_primary);
            }
        }
        // Thumb (M3 expressive: tall rounded rect).
        let thumb_rect = Rect::new(
            thumb_x - Self::THUMB_WIDTH * 0.5,
            cy - Self::THUMB_HEIGHT * 0.5,
            Self::THUMB_WIDTH,
            Self::THUMB_HEIGHT,
        );
        canvas.fill_rounded_rect(thumb_rect, Self::THUMB_WIDTH * 0.5, BASELINE_LIGHT.primary);
        // State layer overlay (circle behind the thumb).
        let alpha = crate::state_layer::state_alpha(self.state);
        if alpha > 0.0 {
            canvas.draw_circle(thumb_x, cy, 20.0, Color {
                r: BASELINE_LIGHT.primary.r,
                g: BASELINE_LIGHT.primary.g,
                b: BASELINE_LIGHT.primary.b,
                a: alpha,
            });
        }
    }

    fn handle_event(&mut self, ev: &Event) -> EventResult {
        match *ev {
            Event::PointerDown { x, y } if self.bounds.contains(x, y) => {
                self.state = State::Pressed;
                self.dragging = true;
                let t = ((x - self.bounds.x - 8.0) / (self.bounds.w - 16.0)).clamp(0.0, 1.0);
                self.set_value(self.min + (self.max - self.min) * t);
                EventResult::Consumed
            }
            Event::PointerMove { x, .. } if self.dragging => {
                let t = ((x - self.bounds.x - 8.0) / (self.bounds.w - 16.0)).clamp(0.0, 1.0);
                self.set_value(self.min + (self.max - self.min) * t);
                self.state = State::Dragged;
                EventResult::Consumed
            }
            Event::PointerUp { .. } if self.dragging => {
                self.dragging = false;
                self.state = State::Hover;
                self.activations = self.activations.saturating_add(1);
                EventResult::Activated
            }
            _ => EventResult::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_value_clamp() {
        let s = Slider::continuous();
        assert_eq!(s.clamp_value(-1.0), 0.0);
        assert_eq!(s.clamp_value(2.0), 1.0);
        assert!((s.clamp_value(0.5) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn slider_discrete_snaps() {
        let s = Slider::discrete(0.0, 10.0, 4); // 5 steps of 2.0
        assert!((s.clamp_value(3.3) - 4.0).abs() < f32::EPSILON);
        assert!((s.clamp_value(7.4) - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn slider_drag_updates_value() {
        let mut s = Slider::continuous();
        s.bounds = Rect::new(0.0, 0.0, 216.0, 44.0); // 200 dp usable track.
        s.handle_event(&Event::PointerDown { x: 108.0, y: 22.0 });
        assert!((s.value - 0.5).abs() < 0.05);
    }
}
