// SPDX-License-Identifier: Apache-2.0
//! `Canvas` — the CPU-side geometry buffer that every component paints
//! into.
//!
//! `Canvas` collects flat-color triangles (rounded rectangles, circles,
//! text glyphs, state-layer overlays) into [`Vertex`] / index buffers in
//! NDC space.  The owner of the canvas (typically the app event loop) is
//! responsible for uploading the resulting buffers to wgpu and issuing
//! the draw call against the shipped [`crate::ripple_shader::UI_SHADER_WGSL`].
//!
//! The canvas is intentionally CPU-only: it does **not** own a
//! `wgpu::Queue` or a `RenderPass`.  This keeps the crate trivially
//! unit-testable (no GPU required) and keeps the public surface portable
//! across native + WASM without `cfg` branches.

use bytemuck::{Pod, Zeroable};
use lyon::math::{Box2D, Point, point};
use lyon::path::Winding;
use lyon::path::builder::BorderRadii;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

// ─── Primitives ─────────────────────────────────────────────────────────────

/// Linear-RGBA color.  Channel range 0.0..=1.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Alpha.
    pub a: f32,
}

impl Color {
    /// Fully transparent.
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    /// Solid white.
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    /// Solid black.
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

    /// Build from a packed `0xAARRGGBB` literal.
    #[must_use]
    pub const fn from_argb(p: u32) -> Self {
        let a = ((p >> 24) & 0xFF) as u8;
        let r = ((p >> 16) & 0xFF) as u8;
        let g = ((p >> 8) & 0xFF) as u8;
        let b = (p & 0xFF) as u8;
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Returns a copy of self with the given alpha.
    #[must_use]
    pub const fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }
}

/// A logical-pixel axis-aligned rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// X of top-left in logical pixels.
    pub x: f32,
    /// Y of top-left in logical pixels.
    pub y: f32,
    /// Width in logical pixels.
    pub w: f32,
    /// Height in logical pixels.
    pub h: f32,
}

impl Rect {
    /// Construct from x/y/w/h.
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Returns true if `(px, py)` lies inside the rectangle.
    #[must_use]
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    /// Inset by `dx` / `dy` on each side.
    #[must_use]
    pub const fn inset(&self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            w: self.w - 2.0 * dx,
            h: self.h - 2.0 * dy,
        }
    }

    /// Center point.
    #[must_use]
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

/// Text styling info used by [`Canvas::draw_text`].
#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    /// Font size in scalable pixels (1 sp = 1 dp at default density).
    pub size_sp: f32,
    /// CSS-style weight (400=regular, 500=medium).
    pub weight: u16,
    /// Fill color.
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self { size_sp: 14.0, weight: 500, color: Color::BLACK }
    }
}

// ─── Vertex ────────────────────────────────────────────────────────────────

/// GPU vertex layout: 2 floats position + 4 floats color.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// Position in NDC space (-1..=1 on each axis).
    pub position: [f32; 2],
    /// Linear-RGBA color in 0..=1 range.
    pub color: [f32; 4],
}

impl Vertex {
    /// `wgpu::VertexBufferLayout` descriptor for [`Vertex`].
    #[must_use]
    pub fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Recorded glyph run staged by [`Canvas::draw_text`].
///
/// The canvas does not rasterize text itself: it stores the request and
/// the host upgrades it to an actual glyph atlas pass using
/// `cosmic-text` + `glyphon` (native) or the WebGPU canvas text path on
/// the web.  This keeps the crate testable without a real font system.
#[derive(Clone, Debug)]
pub struct GlyphRun {
    /// UTF-8 string to render.
    pub text: String,
    /// Bounding rect (in logical pixels) the text should be aligned in.
    pub rect: Rect,
    /// Style descriptor.
    pub style: TextStyle,
}

// ─── Canvas ────────────────────────────────────────────────────────────────

/// CPU-side draw buffer used by every M3 component.
///
/// Components emit geometry by calling the high-level draw helpers
/// ([`Canvas::fill_rounded_rect`], [`Canvas::draw_circle`], …) which
/// internally tessellate via [`lyon`] into the shared
/// [`VertexBuffers<Vertex, u32>`].
///
/// `Canvas` does not implement `Debug` because lyon's `FillTessellator`
/// does not.  Use `Canvas::vertex_count()` / `Canvas::index_count()` for
/// inspection.
pub struct Canvas {
    /// Logical viewport size in pixels (width, height).
    viewport: (f32, f32),
    /// Tessellated geometry, ready to upload as wgpu buffers.
    pub buffers: VertexBuffers<Vertex, u32>,
    /// Recorded text runs awaiting glyph-atlas resolution.
    pub glyphs: Vec<GlyphRun>,
    /// Shared lyon tessellator (re-used across calls to avoid alloc).
    tess: FillTessellator,
}

impl Canvas {
    /// Create an empty canvas sized for a `viewport` in logical pixels.
    #[must_use]
    pub fn new(viewport: (f32, f32)) -> Self {
        Self {
            viewport,
            buffers: VertexBuffers::new(),
            glyphs: Vec::new(),
            tess: FillTessellator::new(),
        }
    }

    /// Clear all staged geometry (call once per frame before painting).
    pub fn clear(&mut self) {
        self.buffers.vertices.clear();
        self.buffers.indices.clear();
        self.glyphs.clear();
    }

    /// Tessellate and stage a rounded rectangle.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let r_max = (rect.w.min(rect.h)) * 0.5;
        let r = if radius.is_infinite() { r_max } else { radius.min(r_max) };

        let mut builder = lyon::path::Path::builder();
        builder.add_rounded_rectangle(
            &Box2D::new(point(rect.x, rect.y), point(rect.x + rect.w, rect.y + rect.h)),
            &BorderRadii::new(r),
            Winding::Positive,
        );
        let path = builder.build();

        let viewport = self.viewport;
        let mut output: BuffersBuilder<'_, Vertex, u32, _> =
            BuffersBuilder::new(&mut self.buffers, |v: FillVertex| {
                let p: Point = v.position();
                let nx = (p.x / viewport.0) * 2.0 - 1.0;
                let ny = 1.0 - (p.y / viewport.1) * 2.0;
                Vertex { position: [nx, ny], color: [color.r, color.g, color.b, color.a] }
            });

        let _ = self
            .tess
            .tessellate_path(&path, &FillOptions::default(), &mut output);
    }

    /// Tessellate and stage a circle.
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        let mut builder = lyon::path::Path::builder();
        builder.add_circle(point(cx, cy), radius, Winding::Positive);
        let path = builder.build();

        let viewport = self.viewport;
        let mut output: BuffersBuilder<'_, Vertex, u32, _> =
            BuffersBuilder::new(&mut self.buffers, |v: FillVertex| {
                let p: Point = v.position();
                let nx = (p.x / viewport.0) * 2.0 - 1.0;
                let ny = 1.0 - (p.y / viewport.1) * 2.0;
                Vertex { position: [nx, ny], color: [color.r, color.g, color.b, color.a] }
            });

        let _ = self
            .tess
            .tessellate_path(&path, &FillOptions::default().with_tolerance(0.25), &mut output);
    }

    /// Stage a text run.  Actual glyph rasterization is deferred to the
    /// host (cosmic-text on native, browser fonts on WASM).
    pub fn draw_text(&mut self, text: &str, rect: Rect, style: TextStyle) {
        self.glyphs.push(GlyphRun { text: text.into(), rect, style });
    }

    /// Apply an M3 state-layer overlay to `rect` for `state`, mixing
    /// `overlay_color` (typically the on-* foreground role) into the
    /// surface.  The overlay is rendered as a translucent rounded rect
    /// matching the host shape.
    pub fn apply_state_layer(
        &mut self,
        rect: Rect,
        radius: f32,
        overlay: Color,
        state: crate::state_layer::State,
    ) {
        let a = crate::state_layer::state_alpha(state);
        if a == 0.0 {
            return;
        }
        self.fill_rounded_rect(rect, radius, overlay.with_alpha(a));
    }

    /// Stage an M3 elevation shadow for `rect` at `level` (0..=5).
    ///
    /// Implemented as a soft offset rounded rect using the shadow
    /// descriptor from [`crate::m3_tokens::elevation::shadow_for_level`].
    /// True Gaussian blur belongs in a downstream post-process pass.
    pub fn draw_elevation_shadow(&mut self, rect: Rect, radius: f32, level: u8) {
        let s = crate::m3_tokens::elevation::shadow_for_level(level);
        if s.opacity == 0.0 {
            return;
        }
        let shadow_rect = Rect::new(
            rect.x + s.offset_x,
            rect.y + s.offset_y + s.blur * 0.25,
            rect.w,
            rect.h,
        );
        self.fill_rounded_rect(
            shadow_rect,
            radius,
            Color { r: 0.0, g: 0.0, b: 0.0, a: s.opacity },
        );
    }

    /// Number of vertices staged in this frame.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.buffers.vertices.len()
    }

    /// Number of indices staged in this frame.
    #[must_use]
    pub fn index_count(&self) -> usize {
        self.buffers.indices.len()
    }

    /// Returns the (vertex, index) byte slices ready for `wgpu::Queue::write_buffer`.
    #[must_use]
    pub fn raw_buffers(&self) -> (&[u8], &[u8]) {
        (
            bytemuck::cast_slice(&self.buffers.vertices),
            bytemuck::cast_slice(&self.buffers.indices),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_rect_tessellation() {
        let mut c = Canvas::new((800.0, 600.0));
        c.fill_rounded_rect(Rect::new(100.0, 100.0, 200.0, 40.0), 20.0, Color::WHITE);
        assert!(c.vertex_count() > 0, "rounded rect must produce vertices");
        assert!(c.index_count() > 0, "rounded rect must produce indices");
        // Index count must be a multiple of 3 (triangle list).
        assert_eq!(c.index_count() % 3, 0);
    }

    #[test]
    fn rect_contains_works() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(r.contains(50.0, 40.0));
        assert!(!r.contains(5.0, 40.0));
        assert!(!r.contains(50.0, 100.0));
    }

    #[test]
    fn color_from_argb() {
        let c = Color::from_argb(0xFF6750A4);
        assert!((c.a - 1.0).abs() < 1e-6);
        assert!((c.r - (0x67 as f32 / 255.0)).abs() < 1e-6);
    }

    #[test]
    fn canvas_clear_resets_buffers() {
        let mut c = Canvas::new((100.0, 100.0));
        c.fill_rounded_rect(Rect::new(0.0, 0.0, 10.0, 10.0), 2.0, Color::WHITE);
        assert!(c.vertex_count() > 0);
        c.clear();
        assert_eq!(c.vertex_count(), 0);
        assert_eq!(c.index_count(), 0);
    }
}
