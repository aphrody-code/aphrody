// SPDX-License-Identifier: Apache-2.0
//! Brand gradients, distilled from Google's Gemini visual-design language.
//!
//! The Gemini illustration system treats gradients as *directional energy*: a
//! sharp, near-opaque leading edge that diffuses toward a transparent tail,
//! used as a visual pointer that guides attention. This module exposes the
//! signature brand colours (captured from the live app) and two builders —
//! [`brand_linear`] (the blue→purple→cyan brand sweep) and [`directional`]
//! (the leading-edge→diffuse energy gradient) — returning vello [`Gradient`]
//! brushes that any `Scene::fill` can use.

use vello::kurbo::Point;
use vello::peniko::{Color, Gradient};

/// Gemini brand blue (`#4285F4`) — the gradient lead.
pub const BRAND_BLUE: Color = Color::from_rgb8(0x42, 0x85, 0xf4);
/// Gemini brand purple (`#9B72CB`) — the gradient midpoint.
pub const BRAND_PURPLE: Color = Color::from_rgb8(0x9b, 0x72, 0xcb);
/// Gemini brand cyan (`#1BA1E3`) — the gradient tail.
pub const BRAND_CYAN: Color = Color::from_rgb8(0x1b, 0xa1, 0xe3);
/// Gemini accent red (`#FF4641`).
pub const BRAND_RED: Color = Color::from_rgb8(0xff, 0x46, 0x41);
/// Gemini accent green (`#0EBC5F`).
pub const BRAND_GREEN: Color = Color::from_rgb8(0x0e, 0xbc, 0x5f);
/// Gemini accent yellow (`#FFCC00`).
pub const BRAND_YELLOW: Color = Color::from_rgb8(0xff, 0xcc, 0x00);

/// The signature Gemini brand gradient: blue → purple → cyan along the line
/// `start`→`end`. Use it to fill text masks, hero surfaces or the sparkle.
#[must_use]
pub fn brand_linear(start: impl Into<Point>, end: impl Into<Point>) -> Gradient {
    Gradient::new_linear(start, end)
        .with_stops([(0.0, BRAND_BLUE), (0.5, BRAND_PURPLE), (1.0, BRAND_CYAN)])
}

/// A directional "energy" gradient in `color`: opaque at `start`, half at 70%,
/// fully transparent at `end` — the sharp-edge→diffuse pointer Gemini uses to
/// steer the eye. The colour's own alpha is taken as the lead opacity.
#[must_use]
pub fn directional(start: impl Into<Point>, end: impl Into<Point>, color: Color) -> Gradient {
    Gradient::new_linear(start, end).with_stops([
        (0.0, color),
        (0.7, color.with_alpha(0.5)),
        (1.0, color.with_alpha(0.0)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use vello::peniko::GradientKind;

    #[test]
    fn brand_linear_has_three_stops_and_is_linear() {
        let g = brand_linear((0.0, 0.0), (100.0, 0.0));
        assert!(matches!(g.kind, GradientKind::Linear { .. }));
        assert_eq!(g.stops.len(), 3);
    }

    #[test]
    fn directional_fades_to_transparent() {
        let g = directional((0.0, 0.0), (100.0, 0.0), BRAND_BLUE);
        assert_eq!(g.stops.len(), 3);
        // Lead opaque, tail transparent.
        assert!(g.stops[0].color.components[3] > 0.99);
        assert!(g.stops[2].color.components[3] < 0.01);
    }
}
