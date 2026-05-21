// SPDX-License-Identifier: Apache-2.0
//! Real text rendering for mui-rs: parley layout → vello glyph runs.
//!
//! This is the piece the components were missing (`Button` et al. had a
//! `// TODO: Label Text (requires parley)`). [`TextRenderer`] owns the parley
//! font + layout contexts (system fonts are loaded lazily by parley) and emits
//! fully-positioned glyphs into a vello [`Scene`]. No placeholders, no stubs:
//! it lays the string out and draws the actual glyph outlines.

use std::borrow::Cow;

use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily, FontWeight, Layout, LayoutContext,
    PositionedLayoutItem, StyleProperty,
};
use vello::Glyph;
use vello::kurbo::Affine;
use vello::peniko::{Color, Fill};
use vello::Scene;

/// A resolved text style: family (CSS source string), pixel size, weight, colour.
#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    /// CSS-style family list, e.g. `"Segoe UI, Roboto, sans-serif"`.
    pub family: &'static str,
    /// Font size in pixels.
    pub size: f32,
    /// OpenType weight (400 regular, 500 medium, 700 bold).
    pub weight: f32,
    /// Fill colour.
    pub color: Color,
}

impl TextStyle {
    #[must_use]
    pub const fn new(family: &'static str, size: f32, weight: f32, color: Color) -> Self {
        Self { family, size, weight, color }
    }
}

/// Owns the parley contexts and renders text into a vello scene.
pub struct TextRenderer {
    font_cx: FontContext,
    layout_cx: LayoutContext<[u8; 4]>,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self { font_cx: FontContext::new(), layout_cx: LayoutContext::new() }
    }

    /// Builds a parley [`Layout`] for `text` in `style`, line-broken at
    /// `max_advance` (None = single unbroken line).
    fn build(&mut self, text: &str, style: TextStyle, max_advance: Option<f32>) -> Layout<[u8; 4]> {
        let mut builder = self.layout_cx.ranged_builder(&mut self.font_cx, text, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(FontFamily::Source(Cow::Borrowed(
            style.family,
        ))));
        builder.push_default(StyleProperty::FontSize(style.size));
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(style.weight)));
        let mut layout: Layout<[u8; 4]> = builder.build(text);
        layout.break_all_lines(max_advance);
        layout.align(max_advance, Alignment::Start, AlignmentOptions::default());
        layout
    }

    /// Measures `text` without drawing. Returns `(width, height)` in px.
    pub fn measure(&mut self, text: &str, style: TextStyle) -> (f32, f32) {
        let layout = self.build(text, style, None);
        (layout.width(), layout.height())
    }

    /// Draws `text` at `transform` (applied to layout-space glyph positions,
    /// origin = top-left of the layout box). Returns `(width, height)` in px so
    /// callers can centre or right-align by offsetting the transform.
    pub fn draw(&mut self, scene: &mut Scene, text: &str, style: TextStyle, transform: Affine) -> (f32, f32) {
        let layout = self.build(text, style, None);
        let (w, h) = (layout.width(), layout.height());
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let run = glyph_run.run();
                let font = run.font();
                let font_size = run.font_size();
                scene
                    .draw_glyphs(font)
                    .font_size(font_size)
                    .brush(style.color)
                    .transform(transform)
                    .draw(
                        Fill::NonZero,
                        glyph_run.positioned_glyphs().map(|g| Glyph {
                            id: g.id,
                            x: g.x,
                            y: g.y,
                        }),
                    );
            }
        }
        (w, h)
    }

    /// Draws `text` centred horizontally within `[x0, x0 + box_width]`, with the
    /// text baseline area starting at `y0` (top of the layout box). Convenience
    /// wrapper around [`Self::draw`] + [`Self::measure`].
    pub fn draw_centered(
        &mut self,
        scene: &mut Scene,
        text: &str,
        style: TextStyle,
        x0: f64,
        box_width: f64,
        y0: f64,
    ) -> (f32, f32) {
        let (w, _h) = self.measure(text, style);
        let x = x0 + (box_width - w as f64) / 2.0;
        self.draw(scene, text, style, Affine::translate((x, y0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SANS: &str = "Roboto, Segoe UI, Arial, sans-serif";

    #[test]
    fn measure_nonempty_is_positive() {
        let mut tr = TextRenderer::new();
        let style = TextStyle::new(SANS, 16.0, 400.0, Color::BLACK);
        let (w, h) = tr.measure("Hello", style);
        assert!(w > 0.0, "width should be positive, got {w}");
        assert!(h > 0.0, "height should be positive, got {h}");
    }

    #[test]
    fn longer_text_is_wider() {
        let mut tr = TextRenderer::new();
        let style = TextStyle::new(SANS, 16.0, 400.0, Color::BLACK);
        let (short, _) = tr.measure("I", style);
        let (long, _) = tr.measure("Filled tonal button", style);
        assert!(long > short, "‘…button’ ({long}) should be wider than ‘I’ ({short})");
    }

    #[test]
    fn bigger_size_is_taller() {
        let mut tr = TextRenderer::new();
        let small = tr.measure("Ag", TextStyle::new(SANS, 12.0, 400.0, Color::BLACK)).1;
        let big = tr.measure("Ag", TextStyle::new(SANS, 32.0, 400.0, Color::BLACK)).1;
        assert!(big > small, "32px ({big}) should be taller than 12px ({small})");
    }

    #[test]
    fn draw_into_scene_does_not_panic() {
        let mut tr = TextRenderer::new();
        let mut scene = Scene::new();
        let style = TextStyle::new(SANS, 14.0, 500.0, Color::WHITE);
        let (w, h) = tr.draw(&mut scene, "aphrody", style, Affine::IDENTITY);
        assert!(w > 0.0 && h > 0.0);
    }
}
