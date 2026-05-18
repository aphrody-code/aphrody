// SPDX-License-Identifier: Apache-2.0
//! M3 Switch — toggle component with track and thumb.
//!
//! Canonical metrics: width 52 dp, height 32 dp, track radius 16 dp,
//! thumb radius (unselected) 8 dp, thumb radius (selected) 12 dp, thumb
//! radius (pressed) 14 dp.

use crate::canvas::{Canvas, Color, Rect};
use crate::m3_tokens::BASELINE_LIGHT;
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// Material Design 3 Switch.
#[derive(Clone, Debug)]
pub struct Switch {
    /// Bounds.
    pub bounds: Rect,
    /// Current on/off state.
    pub checked: bool,
    /// Interaction state.
    pub state: State,
    /// Activation counter.
    pub activations: u32,
}

impl Switch {
    /// Canonical M3 switch width (dp).
    pub const WIDTH: f32 = 52.0;
    /// Canonical M3 switch height (dp).
    pub const HEIGHT: f32 = 32.0;
    /// Track corner radius (dp).
    pub const TRACK_RADIUS: f32 = 16.0;

    /// Build a Switch (default off).
    #[must_use]
    pub fn new() -> Self {
        Self {
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            checked: false,
            state: State::Enabled,
            activations: 0,
        }
    }

    fn thumb_radius(&self) -> f32 {
        if self.state == State::Pressed {
            14.0
        } else if self.checked {
            12.0
        } else {
            8.0
        }
    }
}

impl Default for Switch {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialComponent for Switch {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        let s = constraints.clamp(Self::WIDTH, Self::HEIGHT);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        let r = self.bounds;
        // Track.
        let track_color = if self.checked {
            BASELINE_LIGHT.primary
        } else {
            BASELINE_LIGHT.surface_variant
        };
        canvas.fill_rounded_rect(r, Self::TRACK_RADIUS, track_color);
        // Outline for off-state track.
        if !self.checked {
            canvas.fill_rounded_rect(
                r.inset(1.0, 1.0),
                Self::TRACK_RADIUS - 1.0,
                BASELINE_LIGHT.surface_variant,
            );
            // 2 dp outline ring.
            canvas.fill_rounded_rect(r, Self::TRACK_RADIUS, BASELINE_LIGHT.outline);
            canvas.fill_rounded_rect(
                r.inset(2.0, 2.0),
                Self::TRACK_RADIUS - 2.0,
                BASELINE_LIGHT.surface_variant,
            );
        }
        // Thumb position.
        let radius = self.thumb_radius();
        let (cy_track,) = (r.y + r.h * 0.5,);
        let cx_thumb = if self.checked {
            r.x + r.w - Self::HEIGHT * 0.5
        } else {
            r.x + Self::HEIGHT * 0.5
        };
        let thumb_color = if self.checked {
            BASELINE_LIGHT.on_primary
        } else {
            BASELINE_LIGHT.outline
        };
        // State layer behind the thumb (slightly larger circle).
        let overlay = if self.checked {
            BASELINE_LIGHT.primary
        } else {
            BASELINE_LIGHT.on_surface
        };
        let alpha = crate::state_layer::state_alpha(self.state);
        if alpha > 0.0 {
            canvas.draw_circle(cx_thumb, cy_track, radius + 8.0, Color {
                r: overlay.r,
                g: overlay.g,
                b: overlay.b,
                a: alpha,
            });
        }
        canvas.draw_circle(cx_thumb, cy_track, radius, thumb_color);
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
            Event::PointerDown { x, y } if self.bounds.contains(x, y) => {
                self.state = State::Pressed;
                EventResult::Consumed
            }
            Event::PointerUp { x, y } if self.state == State::Pressed => {
                if self.bounds.contains(x, y) {
                    self.checked = !self.checked;
                    self.activations = self.activations.saturating_add(1);
                }
                self.state = State::Hover;
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
    fn switch_toggle() {
        let mut s = Switch::new();
        s.bounds = Rect::new(0.0, 0.0, Switch::WIDTH, Switch::HEIGHT);
        assert!(!s.checked);
        s.handle_event(&Event::PointerDown { x: 26.0, y: 16.0 });
        s.handle_event(&Event::PointerUp { x: 26.0, y: 16.0 });
        assert!(s.checked);
        assert_eq!(s.activations, 1);
    }
}
