//! Render pipeline — scene graph vello → wgpu draw calls.

use vello::{Scene, kurbo::Affine, peniko::Color};

use crate::surface::RenderSurface;
use crate::layout::LayoutEngine;
use crate::text::{TextRenderer, TextStyle};

/// Draw context handed to every [`Widget`]. Bundles the vello [`Scene`] (vector
/// geometry) and the [`TextRenderer`] (parley-backed glyph runs) so a widget can
/// paint both shapes and real text in one pass.
pub struct DrawCx<'a> {
    pub scene: &'a mut Scene,
    pub text: &'a mut TextRenderer,
}

impl DrawCx<'_> {
    /// Draws `s` at `transform`; returns the laid-out `(width, height)` in px.
    /// Borrows the two disjoint fields, so widgets can mix shapes and text.
    pub fn draw_text(&mut self, s: &str, style: TextStyle, transform: Affine) -> (f32, f32) {
        self.text.draw(self.scene, s, style, transform)
    }

    /// Measures `s` without painting.
    pub fn measure_text(&mut self, s: &str, style: TextStyle) -> (f32, f32) {
        self.text.measure(s, style)
    }
}

/// A paintable Material component: vector geometry + text via [`DrawCx`].
pub trait Widget {
    fn draw(&self, cx: &mut DrawCx, transform: Affine);
}

pub struct RenderPipeline {
    pub scene: Scene,
    pub base_color: Color,
    pub layout: LayoutEngine,
    pub text: TextRenderer,
}

impl RenderPipeline {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            base_color: Color::BLACK,
            layout: LayoutEngine::new(),
            text: TextRenderer::new(),
        }
    }

    pub fn set_base_color(&mut self, color: Color) {
        self.base_color = color;
    }

    pub fn draw_widget(&mut self, widget: &dyn Widget, transform: Affine) {
        let mut cx = DrawCx { scene: &mut self.scene, text: &mut self.text };
        widget.draw(&mut cx, transform);
    }

    /// Draws a widget using its computed layout position.
    pub fn draw_at_node(&mut self, widget: &dyn Widget, node: taffy::NodeId, offset: Affine) {
        let transform = offset * self.layout.get_transform(node);
        let mut cx = DrawCx { scene: &mut self.scene, text: &mut self.text };
        widget.draw(&mut cx, transform);
    }

    pub fn render_to_surface(&mut self, surface: &mut RenderSurface) -> anyhow::Result<()> {
        let surface_texture = match surface.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Timeout)
            | Err(wgpu::SurfaceError::Outdated)
            | Err(wgpu::SurfaceError::Lost) => return Ok(()),
            Err(e) => return Err(e.into()),
        };

        // 1) Vello renders into the intermediate Rgba8Unorm storage texture.
        surface
            .renderer
            .render_to_texture(
                &surface.device,
                &surface.queue,
                &self.scene,
                &surface.target_view,
                &vello::RenderParams {
                    base_color: self.base_color,
                    width: surface.config.width,
                    height: surface.config.height,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .map_err(|e| anyhow::anyhow!("vello render_to_texture failed: {e:?}"))?;

        // 2) Blit the intermediate onto the presentable (sRGB) surface view.
        let surface_view =
            surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = surface
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("mui-rs blit") });
        surface.blitter.copy(
            &surface.device,
            &mut encoder,
            &surface.target_view,
            &surface_view,
        );
        surface.queue.submit([encoder.finish()]);

        surface_texture.present();
        Ok(())
    }
}

impl Default for RenderPipeline {
    fn default() -> Self {
        Self::new()
    }
}
