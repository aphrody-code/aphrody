// SPDX-License-Identifier: Apache-2.0
//! Feedback: Badge, Snackbar, ProgressIndicator.

use mui_rs_renderer::pipeline::{DrawCx, Widget};
use mui_rs_renderer::shadow;
use mui_rs_renderer::vello::kurbo::{Affine, Circle, Rect, RoundedRect};
use mui_rs_renderer::vello::peniko::{Color, Fill};
use mui_rs_renderer::TextStyle;

const FAMILY: &str = "Roboto, Segoe UI, Arial, sans-serif";

/// M3 badge — a small error-coloured dot, or a pill carrying a count/label.
#[derive(Debug, Clone)]
pub struct Badge {
    pub value: String,
}

impl Widget for Badge {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let error = Color::from_rgb8(179, 38, 30); // error
        if self.value.is_empty() {
            // Small badge: 6dp dot.
            cx.scene.fill(Fill::NonZero, transform, error, None, &Circle::new((3.0, 3.0), 3.0));
            return;
        }
        // Large badge: 16dp pill with the value centred.
        let style = TextStyle::new(FAMILY, 11.0, 500.0, Color::WHITE); // on-error
        let (tw, th) = cx.measure_text(&self.value, style);
        let h = 16.0;
        let w = (f64::from(tw) + 12.0).max(h);
        let pill = RoundedRect::new(0.0, 0.0, w, h, h / 2.0);
        cx.scene.fill(Fill::NonZero, transform, error, None, &pill);
        let tx = (w - f64::from(tw)) / 2.0;
        let ty = (h - f64::from(th)) / 2.0;
        cx.draw_text(&self.value, style, transform * Affine::translate((tx, ty)));
    }
}

/// M3 snackbar — a single-line message on the inverse surface.
#[derive(Debug, Clone)]
pub struct Snackbar {
    pub message: String,
}

impl Snackbar {
    pub const HEIGHT_DP: f64 = 48.0;
    pub const WIDTH_DP: f64 = 344.0;
}

impl Widget for Snackbar {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let (w, h) = (Snackbar::WIDTH_DP, Snackbar::HEIGHT_DP);
        let rect = RoundedRect::new(0.0, 0.0, w, h, 4.0);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, w, h), 4.0, 3);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(50, 47, 53), None, &rect); // inverse-surface
        let style = TextStyle::new(FAMILY, 14.0, 400.0, Color::from_rgb8(245, 239, 247)); // inverse-on-surface
        let (_tw, th) = cx.measure_text(&self.message, style);
        let ty = (h - f64::from(th)) / 2.0;
        cx.draw_text(&self.message, style, transform * Affine::translate((16.0, ty)));
    }
}

/// M3 linear progress indicator. `progress` = `Some(0..=1)` (determinate) or
/// `None` (indeterminate — rendered as a partial moving bar at 35%).
#[derive(Debug, Clone)]
pub struct ProgressIndicator {
    pub progress: Option<f32>,
}

impl ProgressIndicator {
    pub const WIDTH_DP: f64 = 240.0;
    const HEIGHT_DP: f64 = 4.0;
}

impl Widget for ProgressIndicator {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let (w, h) = (ProgressIndicator::WIDTH_DP, ProgressIndicator::HEIGHT_DP);
        // Track (surface-container-highest) + active (primary), both fully rounded.
        let track = RoundedRect::new(0.0, 0.0, w, h, h / 2.0);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(230, 224, 233), None, &track);
        let frac = f64::from(self.progress.unwrap_or(0.35).clamp(0.0, 1.0));
        let active = RoundedRect::new(0.0, 0.0, w * frac, h, h / 2.0);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(103, 80, 164), None, &active);
    }
}
