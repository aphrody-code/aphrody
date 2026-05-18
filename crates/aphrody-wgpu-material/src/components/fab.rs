// SPDX-License-Identifier: Apache-2.0
//! M3 Floating Action Button (FAB).
//!
//! Sizes: Small (40 dp), Regular (56 dp), Large (96 dp), Extended
//! (height 56 dp, label + leading icon, min width 80 dp).  Elevation
//! resting = level 3, pressed = level 5.  Corner radius: 12 dp for
//! Small / Regular, 16 dp for Large, 16 dp for Extended.

use crate::canvas::{Canvas, Color, Rect, TextStyle};
use crate::m3_tokens::{BASELINE_LIGHT, LABEL_LARGE};
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// FAB size variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FabSize {
    /// 40 dp square.
    Small,
    /// 56 dp square (default).
    Regular,
    /// 96 dp square.
    Large,
    /// 56 dp height, variable width with label.
    Extended,
}

impl FabSize {
    /// Canonical side length / height in dp.
    #[must_use]
    pub const fn dimension(self) -> f32 {
        match self {
            FabSize::Small => 40.0,
            FabSize::Regular | FabSize::Extended => 56.0,
            FabSize::Large => 96.0,
        }
    }

    /// Canonical corner radius in dp.
    #[must_use]
    pub const fn radius(self) -> f32 {
        match self {
            FabSize::Small | FabSize::Regular => 12.0,
            FabSize::Large | FabSize::Extended => 16.0,
        }
    }
}

/// Material Design 3 FAB.
#[derive(Clone, Debug)]
pub struct Fab {
    /// Size variant.
    pub size: FabSize,
    /// Label (only rendered when `size == Extended`).
    pub label: String,
    /// Layout bounds.
    pub bounds: Rect,
    /// Interaction state.
    pub state: State,
    /// Activation counter.
    pub activations: u32,
}

impl Fab {
    /// Construct a small FAB.
    #[must_use]
    pub fn small() -> Self {
        Self::new(FabSize::Small, "")
    }

    /// Construct a regular FAB.
    #[must_use]
    pub fn regular() -> Self {
        Self::new(FabSize::Regular, "")
    }

    /// Construct a large FAB.
    #[must_use]
    pub fn large() -> Self {
        Self::new(FabSize::Large, "")
    }

    /// Construct an extended FAB with a label.
    #[must_use]
    pub fn extended(label: impl Into<String>) -> Self {
        Self::new(FabSize::Extended, label)
    }

    fn new(size: FabSize, label: impl Into<String>) -> Self {
        Self {
            size,
            label: label.into(),
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            state: State::Enabled,
            activations: 0,
        }
    }

    fn elevation_level(&self) -> u8 {
        if self.state == State::Pressed { 5 } else { 3 }
    }
}

impl MaterialComponent for Fab {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        let h = self.size.dimension();
        let w = match self.size {
            FabSize::Extended => {
                // 16 dp leading + 24 dp icon + 12 dp gap + label (8 dp/char) + 20 dp trailing.
                let label_w = self.label.chars().count() as f32 * 8.0;
                (16.0 + 24.0 + 12.0 + label_w + 20.0).max(80.0)
            }
            _ => h,
        };
        let s = constraints.clamp(w, h);
        self.bounds = Rect::new(self.bounds.x, self.bounds.y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        let r = self.bounds;
        let radius = self.size.radius();
        canvas.draw_elevation_shadow(r, radius, self.elevation_level());
        canvas.fill_rounded_rect(r, radius, BASELINE_LIGHT.primary_container);
        canvas.apply_state_layer(r, radius, BASELINE_LIGHT.on_primary_container, self.state);
        // Icon glyph slot — drawn as a filled inner circle placeholder.
        let (cx, cy) = r.center();
        let icon_r = (self.size.dimension() * 0.18).max(8.0);
        canvas.draw_circle(
            if matches!(self.size, FabSize::Extended) {
                r.x + 16.0 + 12.0
            } else {
                cx
            },
            cy,
            icon_r,
            BASELINE_LIGHT.on_primary_container,
        );
        if matches!(self.size, FabSize::Extended) {
            canvas.draw_text(
                &self.label,
                Rect::new(r.x + 16.0 + 24.0 + 12.0, r.y, r.w - (16.0 + 24.0 + 12.0 + 20.0), r.h),
                TextStyle {
                    size_sp: LABEL_LARGE.size_sp,
                    weight: LABEL_LARGE.weight,
                    color: BASELINE_LIGHT.on_primary_container,
                },
            );
        }
        // Silence dead-code on the lighter Color helper.
        let _ = Color::TRANSPARENT;
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
            Event::PointerUp { .. } if self.state == State::Pressed => {
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
    fn fab_dimensions() {
        assert_eq!(FabSize::Small.dimension(), 40.0);
        assert_eq!(FabSize::Regular.dimension(), 56.0);
        assert_eq!(FabSize::Large.dimension(), 96.0);
    }

    #[test]
    fn fab_extended_layout() {
        let mut f = Fab::extended("Create");
        let s = f.layout(&Constraints::unbounded());
        assert!(s.width >= 80.0);
        assert!((s.height - 56.0).abs() < f32::EPSILON);
    }
}
