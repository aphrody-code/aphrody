// SPDX-License-Identifier: Apache-2.0
//! Material 3 elevation shadows, rendered with vello's gaussian-blurred rounded
//! rect primitive. M3 defines six elevation levels (0–5); this maps each to a
//! key-shadow blur radius + vertical offset + opacity, approximating the
//! ambient+key shadow pair with a single tuned blur (good enough at UI scale
//! and far cheaper than two passes).

use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::Color;

/// Per-level (blur std-dev, y-offset, alpha) tuned to the M3 elevation tokens
/// (dp 0/1/3/6/8/12). Level 0 paints nothing.
const LEVELS: [(f64, f64, u8); 6] = [
    (0.0, 0.0, 0),
    (3.0, 1.0, 60),
    (6.0, 2.0, 64),
    (8.0, 3.0, 68),
    (10.0, 4.0, 72),
    (14.0, 6.0, 76),
];

/// Draws the M3 elevation shadow for a rounded rect of size `rect` (in the
/// widget's local space) at the given `transform`. `level` is clamped to 0..=5.
/// Call this *before* painting the surface fill so the shadow sits behind it.
pub fn draw_elevation(scene: &mut Scene, transform: Affine, rect: Rect, radius: f64, level: usize) {
    let (std_dev, dy, alpha) = LEVELS[level.min(5)];
    if alpha == 0 {
        return;
    }
    let shadow_rect = rect + vello::kurbo::Vec2::new(0.0, dy);
    scene.draw_blurred_rounded_rect(
        transform,
        shadow_rect,
        Color::from_rgba8(0, 0, 0, alpha),
        radius,
        std_dev,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_levels_paint_without_panic() {
        // Levels 0..=6 (6 clamps to 5); a fresh scene needs no GPU.
        for level in 0..=6 {
            let mut scene = Scene::new();
            draw_elevation(&mut scene, Affine::IDENTITY, Rect::new(0.0, 0.0, 80.0, 40.0), 12.0, level);
        }
    }

    #[test]
    fn level_zero_is_a_noop() {
        // Level 0 must not enqueue any draw (alpha 0). We can't introspect the
        // scene's encoding directly, but the function must return without panic
        // and `LEVELS[0]` must carry zero alpha by contract.
        assert_eq!(LEVELS[0].2, 0);
        let mut scene = Scene::new();
        draw_elevation(&mut scene, Affine::IDENTITY, Rect::new(0.0, 0.0, 10.0, 10.0), 4.0, 0);
    }
}
