//! GPU surface — wraps wgpu device/queue/surface + vello renderer.
//!
//! Vello renders through a compute shader that writes to a *storage* texture,
//! which must be `Rgba8Unorm`. A presentable swapchain surface is almost always
//! an sRGB format (e.g. `Bgra8UnormSrgb`) and cannot be bound as a storage
//! texture. We therefore render into an intermediate `Rgba8Unorm` texture and
//! blit it onto the surface with [`wgpu::util::TextureBlitter`] — the same
//! pattern vello's own `util` module uses.

use anyhow::{Context, Result};
use vello::{Renderer, RendererOptions};
use wgpu::{
    Device, Queue, Surface, SurfaceConfiguration, Texture, TextureView,
    util::TextureBlitter,
};

pub struct RenderSurface<'a> {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'a>,
    pub config: SurfaceConfiguration,
    pub renderer: Renderer,
    /// Intermediate `Rgba8Unorm` storage texture vello renders into.
    pub target_texture: Texture,
    pub target_view: TextureView,
    /// Blits `target_view` (Rgba8Unorm) onto the presentable surface view.
    pub blitter: TextureBlitter,
}

/// Creates the intermediate render target. Vello's compute shader needs
/// `STORAGE_BINDING`; the blitter samples it, hence `TEXTURE_BINDING`.
fn create_target(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mui-rs vello intermediate target"),
        size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        format: wgpu::TextureFormat::Rgba8Unorm,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

impl<'a> RenderSurface<'a> {
    pub async fn new(
        target: impl Into<wgpu::SurfaceTarget<'a>>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(target).context("Failed to create surface")?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Failed to find wgpu adapter")?;

        let (device, queue) = adapter
            .request_device(&Default::default())
            .await
            .context("Failed to request wgpu device")?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, RendererOptions {
            use_cpu: false,
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: None,
            pipeline_cache: None,
        })
        .context("Failed to create Vello renderer")?;

        let (target_texture, target_view) = create_target(&device, width, height);
        // Blitter target = the presentable surface, so build it for `format`.
        let blitter = TextureBlitter::new(&device, format);

        Ok(Self { device, queue, surface, config, renderer, target_texture, target_view, blitter })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        let (texture, view) = create_target(&self.device, width, height);
        self.target_texture = texture;
        self.target_view = view;
    }
}
