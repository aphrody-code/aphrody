// SPDX-License-Identifier: Apache-2.0
//! M3 Dialog — basic modal with scrim.
//!
//! Canonical metrics: min width 280 dp, max width 560 dp, corner radius
//! 28 dp (`EXTRA_LARGE`), padding 24 dp.  A full-viewport scrim is
//! painted behind the dialog at 32% black opacity.

use crate::canvas::{Canvas, Color, Rect, TextStyle};
use crate::m3_tokens::{BASELINE_LIGHT, BODY_MEDIUM, HEADLINE_SMALL, shape};
use crate::state_layer::State;
use crate::{Constraints, Event, EventResult, MaterialComponent, Size};

/// Material Design 3 basic Dialog.
#[derive(Clone, Debug)]
pub struct Dialog {
    /// Headline / title.
    pub headline: String,
    /// Supporting body text.
    pub body: String,
    /// Optional confirm button label.
    pub confirm: Option<String>,
    /// Optional dismiss button label.
    pub dismiss: Option<String>,
    /// Dialog content bounds (not the scrim).
    pub bounds: Rect,
    /// Viewport (used for scrim painting).
    pub viewport: (f32, f32),
    /// Visibility flag.
    pub open: bool,
    /// Last activated button (`true` = confirm, `false` = dismiss).
    pub last_confirm: Option<bool>,
}

impl Dialog {
    /// Canonical M3 dialog corner radius (dp).
    pub const RADIUS: f32 = shape::EXTRA_LARGE;
    /// Canonical min width (dp).
    pub const MIN_WIDTH: f32 = 280.0;
    /// Canonical max width (dp).
    pub const MAX_WIDTH: f32 = 560.0;
    /// Canonical content padding (dp).
    pub const PADDING: f32 = 24.0;
    /// Scrim opacity (M3 canonical 0.32).
    pub const SCRIM_OPACITY: f32 = 0.32;

    /// Construct a dialog with title + body.
    #[must_use]
    pub fn new(headline: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            body: body.into(),
            confirm: Some("OK".into()),
            dismiss: Some("Cancel".into()),
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            viewport: (0.0, 0.0),
            open: true,
            last_confirm: None,
        }
    }

    /// Set the viewport size (used to position the dialog + scrim).
    pub fn set_viewport(&mut self, w: f32, h: f32) {
        self.viewport = (w, h);
    }
}

impl MaterialComponent for Dialog {
    fn layout(&mut self, constraints: &Constraints) -> Size {
        let w = constraints
            .max_width
            .min(Self::MAX_WIDTH)
            .max(Self::MIN_WIDTH.min(constraints.max_width));
        // Height = padding + headline line + 16 gap + body lines (~24/line) + 16 + button row (40) + padding.
        let body_lines = (self.body.chars().count() as f32 / 40.0).ceil().max(1.0);
        let h = Self::PADDING
            + 36.0
            + 16.0
            + body_lines * 24.0
            + 24.0
            + 40.0
            + Self::PADDING;
        let s = constraints.clamp(w, h);
        // Center within viewport if available.
        let (vw, vh) = self.viewport;
        let x = if vw > 0.0 { (vw - s.width) * 0.5 } else { self.bounds.x };
        let y = if vh > 0.0 { (vh - s.height) * 0.5 } else { self.bounds.y };
        self.bounds = Rect::new(x, y, s.width, s.height);
        s
    }

    fn paint(&self, canvas: &mut Canvas) {
        if !self.open {
            return;
        }
        // Scrim — fullscreen translucent black.
        let (vw, vh) = self.viewport;
        if vw > 0.0 && vh > 0.0 {
            canvas.fill_rounded_rect(
                Rect::new(0.0, 0.0, vw, vh),
                0.0,
                Color { r: 0.0, g: 0.0, b: 0.0, a: Self::SCRIM_OPACITY },
            );
        }
        let r = self.bounds;
        canvas.draw_elevation_shadow(r, Self::RADIUS, 3);
        canvas.fill_rounded_rect(r, Self::RADIUS, BASELINE_LIGHT.surface);
        // Headline.
        canvas.draw_text(
            &self.headline,
            Rect::new(r.x + Self::PADDING, r.y + Self::PADDING, r.w - 2.0 * Self::PADDING, 36.0),
            TextStyle {
                size_sp: HEADLINE_SMALL.size_sp,
                weight: HEADLINE_SMALL.weight,
                color: BASELINE_LIGHT.on_surface,
            },
        );
        // Body.
        canvas.draw_text(
            &self.body,
            Rect::new(
                r.x + Self::PADDING,
                r.y + Self::PADDING + 36.0 + 16.0,
                r.w - 2.0 * Self::PADDING,
                r.h - Self::PADDING * 2.0 - 36.0 - 16.0 - 24.0 - 40.0,
            ),
            TextStyle {
                size_sp: BODY_MEDIUM.size_sp,
                weight: BODY_MEDIUM.weight,
                color: BASELINE_LIGHT.on_surface_variant,
            },
        );
        // Button row — bottom-right aligned, dismiss then confirm (text buttons).
        let mut bx = r.x + r.w - Self::PADDING;
        let by = r.y + r.h - Self::PADDING - 40.0;
        if let Some(label) = &self.confirm {
            let w = label.chars().count() as f32 * 8.0 + 32.0;
            bx -= w;
            canvas.draw_text(
                label,
                Rect::new(bx, by, w, 40.0),
                TextStyle { size_sp: 14.0, weight: 500, color: BASELINE_LIGHT.primary },
            );
        }
        if let Some(label) = &self.dismiss {
            let w = label.chars().count() as f32 * 8.0 + 32.0;
            bx -= w + 8.0;
            canvas.draw_text(
                label,
                Rect::new(bx, by, w, 40.0),
                TextStyle { size_sp: 14.0, weight: 500, color: BASELINE_LIGHT.primary },
            );
        }
        // Touch the unused `State` import to keep the API surface coverable.
        let _ = State::Enabled;
    }

    fn handle_event(&mut self, ev: &Event) -> EventResult {
        match *ev {
            Event::PointerUp { x, y } if self.open => {
                // Hit-test buttons. We approximate the labels at the bottom-right of bounds.
                let r = self.bounds;
                let by = r.y + r.h - Self::PADDING - 40.0;
                if y >= by && y <= by + 40.0 {
                    // Right cluster is "confirm", left of it is "dismiss".
                    if self.confirm.is_some() && x >= r.x + r.w - Self::PADDING - 80.0 {
                        self.last_confirm = Some(true);
                        self.open = false;
                        return EventResult::Activated;
                    } else if self.dismiss.is_some() && x >= r.x + r.w - Self::PADDING - 160.0 {
                        self.last_confirm = Some(false);
                        self.open = false;
                        return EventResult::Activated;
                    }
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
    fn dialog_layout_centers_in_viewport() {
        let mut d = Dialog::new("Title", "Body");
        d.set_viewport(1000.0, 800.0);
        d.layout(&Constraints {
            min_width: 280.0,
            max_width: 560.0,
            min_height: 0.0,
            max_height: 800.0,
        });
        let cx = d.bounds.x + d.bounds.w * 0.5;
        let cy = d.bounds.y + d.bounds.h * 0.5;
        assert!((cx - 500.0).abs() < 1.0);
        assert!((cy - 400.0).abs() < 1.0);
    }
}
