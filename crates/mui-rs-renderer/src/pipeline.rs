//! Render pipeline — scene graph vello → wgpu draw calls.

use vello::{
    Scene,
    kurbo::Affine,
    peniko::Color,
};

use crate::surface::RenderSurface;
use crate::layout::LayoutEngine;

pub trait Widget {
    fn draw(&self, scene: &mut Scene, transform: Affine);
}

pub struct RenderPipeline {
    pub scene: Scene,
    pub base_color: Color,
    pub layout: LayoutEngine,
}

impl RenderPipeline {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            base_color: Color::BLACK,
            layout: LayoutEngine::new(),
        }
    }

    pub fn set_base_color(&mut self, color: Color) {
        self.base_color = color;
    }

    pub fn draw_widget(&mut self, widget: &dyn Widget, transform: Affine) {
        widget.draw(&mut self.scene, transform);
    }

    /// Draws a widget using its computed layout position.
    pub fn draw_at_node(&mut self, widget: &dyn Widget, node: taffy::NodeId, offset: Affine) {
        let transform = offset * self.layout.get_transform(node);
        widget.draw(&mut self.scene, transform);
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
