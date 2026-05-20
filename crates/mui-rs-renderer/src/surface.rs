//! GPU surface — wraps wgpu device/queue/surface + vello renderer.


use anyhow::{Context, Result};
use vello::{Renderer, RendererOptions};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};

pub struct RenderSurface<'a> {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'a>,
    pub config: SurfaceConfiguration,
    pub renderer: Renderer,
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

        Ok(Self { device, queue, surface, config, renderer })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }
}
