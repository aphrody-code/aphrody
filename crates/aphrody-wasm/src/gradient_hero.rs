// SPDX-License-Identifier: Apache-2.0
//! WebGPU animated gradient hero renderer.
//!
//! Renders the Gemini brand spectrum-shift gradient (Blue → Purple → Pink, with
//! a slow Cyan morph at the top edge) onto a `<canvas>` element via a real WGSL
//! fragment shader.  The fragment shader implements the same animation cadence as
//! the CSS `gemini-shimmer` keyframes used in `packages/gemini-app-aphrody` so
//! the visual result is pixel-compatible between the two render paths.
//!
//! This module is **`wasm32`-only** — the WebGPU surface acquisition
//! (`wgpu::Instance`, `wgpu::Surface::create_surface_from_canvas`) and the
//! async device handshake are intrinsically browser-runtime operations and
//! cannot be mocked on a native Rust test runner.
//!
//! # Usage (JavaScript/WASM glue)
//!
//! ```js
//! import init, { GradientHero } from "@aphrody-code/aphrody-wasm";
//! await init();
//! const hero = await GradientHero.mount("hero-canvas");
//! // In your rAF loop or Rust tick:
//! hero.render(performance.now());
//! // On cleanup:
//! hero.resize(newW, newH);
//! hero.destroy();
//! ```
//!
//! The `<canvas id="hero-canvas">` must already be present in the DOM before
//! `GradientHero::mount` is called.

#![cfg(target_arch = "wasm32")]

use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlCanvasElement;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can arise while initialising or running the gradient renderer.
#[derive(Debug)]
pub enum GradientError {
    /// The canvas element was not found in the document.
    CanvasNotFound(String),
    /// WebGPU is not available in this browser / context.
    WebGpuUnavailable,
    /// `requestAdapter()` returned `null` (no compatible GPU adapter).
    NoAdapter,
    /// Device or surface creation failed.
    DeviceCreation(String),
    /// Pipeline compilation failed.
    PipelineCompile(String),
    /// A JS promise rejected with an unexpected value.
    JsError(String),
}

impl core::fmt::Display for GradientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CanvasNotFound(id) => write!(f, "canvas element #{id} not found in DOM"),
            Self::WebGpuUnavailable => write!(f, "WebGPU is not available in this browser"),
            Self::NoAdapter => write!(f, "requestAdapter() returned null — no compatible GPU"),
            Self::DeviceCreation(msg) => write!(f, "device/surface creation failed: {msg}"),
            Self::PipelineCompile(msg) => write!(f, "pipeline compile error: {msg}"),
            Self::JsError(msg) => write!(f, "JS error: {msg}"),
        }
    }
}

impl From<GradientError> for JsValue {
    fn from(e: GradientError) -> Self {
        JsValue::from_str(&e.to_string())
    }
}

// ---------------------------------------------------------------------------
// WGSL shader source
// ---------------------------------------------------------------------------

const SHADER_SRC: &str = include_str!("gradient_hero.wgsl");

// ---------------------------------------------------------------------------
// Uniform layout (must match gradient_hero.wgsl struct Uniforms)
// ---------------------------------------------------------------------------

// Total size: 4 × f32 = 16 bytes (satisfies WebGPU minimum uniform stride).
const UNIFORM_BUFFER_BYTES: u64 = 16;

// Byte offset of `time_s` within the uniform buffer (it is the first field).
const UNIFORM_OFFSET_TIME: u64 = 0;

// ---------------------------------------------------------------------------
// GradientHero
// ---------------------------------------------------------------------------

/// WebGPU-backed animated gradient hero renderer, bound to a browser canvas.
///
/// Obtain an instance via [`GradientHero::mount`]; call [`GradientHero::render`]
/// on every animation frame; call [`GradientHero::resize`] when the canvas
/// dimensions change; call [`GradientHero::destroy`] to release GPU resources.
///
/// The struct is exposed to JS via `#[wasm_bindgen]` so the JS glue layer can
/// drive the render loop without allocating additional WASM closures.
#[wasm_bindgen]
pub struct GradientHero {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// Canvas width in physical pixels.
    width: u32,
    /// Canvas height in physical pixels.
    height: u32,
}

#[wasm_bindgen]
impl GradientHero {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Mount a `GradientHero` onto the `<canvas id="{canvas_id}">` element.
    ///
    /// This is an `async` function because WebGPU adapter + device acquisition
    /// is asynchronous in the browser.  The returned promise resolves to the
    /// renderer on success, or rejects with a descriptive error string.
    ///
    /// # Errors
    /// Rejects if the canvas is not found, WebGPU is unavailable, or the
    /// GPU adapter / device cannot be obtained.
    pub async fn mount(canvas_id: &str) -> Result<GradientHero, JsValue> {
        Self::new(canvas_id).await.map_err(JsValue::from)
    }

    // -----------------------------------------------------------------------
    // Frame rendering
    // -----------------------------------------------------------------------

    /// Draw one frame of the animated gradient.
    ///
    /// `time_ms` is the timestamp in **milliseconds** since the renderer was
    /// mounted (e.g. the value from `performance.now()` minus the value
    /// recorded at mount time).  Internally this is converted to seconds and
    /// written into the `time_s` uniform before dispatching the draw call.
    ///
    /// Silently no-ops if the surface texture cannot be acquired (e.g. while
    /// the canvas is hidden or the GPU is temporarily lost).
    pub fn render(&self, time_ms: f32) {
        self.render_frame(time_ms);
    }

    // -----------------------------------------------------------------------
    // Resize
    // -----------------------------------------------------------------------

    /// Reconfigure the swap-chain surface when the canvas dimensions change.
    ///
    /// Call this whenever the canvas `width`/`height` attributes are updated
    /// (e.g. on `ResizeObserver` callbacks).  The arguments are in **physical
    /// pixels** (i.e. already multiplied by `devicePixelRatio` if needed).
    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.width = w;
        self.height = h;
        self.surface_config.width = w;
        self.surface_config.height = h;
        self.surface.configure(&self.device, &self.surface_config);
    }

    // -----------------------------------------------------------------------
    // Teardown
    // -----------------------------------------------------------------------

    /// Release GPU resources (uniform buffer, pipeline, surface).
    ///
    /// After calling `destroy()` the renderer must not be used again.
    /// The JS garbage-collector will eventually drop the WASM binding as well,
    /// but calling `destroy()` eagerly is strongly recommended to free VRAM.
    pub fn destroy(self) {
        // `self` is moved in; drop order (Rust) is field-declaration order:
        // bind_group → uniform_buf → pipeline → surface → queue → device.
        // All wgpu objects implement Drop internally so no explicit call needed.
        drop(self);
    }
}

// ---------------------------------------------------------------------------
// Private implementation (not exposed to wasm_bindgen)
// ---------------------------------------------------------------------------

impl GradientHero {
    /// Core async constructor — acquires GPU adapter + device and builds the
    /// render pipeline.
    async fn new(canvas_id: &str) -> Result<Self, GradientError> {
        // 1. Retrieve the canvas element from the DOM.
        let canvas = canvas_by_id(canvas_id)?;
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        // 2. Create a wgpu Instance configured for WebGPU.
        let instance = wgpu::Instance::default();

        // 3. Create a surface from the canvas element. `create_surface_from_canvas` is synchronous
        //    and returns immediately.
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| GradientError::DeviceCreation(e.to_string()))?;

        // 4. Request an adapter (asynchronous in the browser).
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| GradientError::NoAdapter)?;

        // 5. Request a device + queue (asynchronous).
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("aphrody-gradient-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    ..Default::default()
                }
            )
            .await
            .map_err(|e| GradientError::DeviceCreation(e.to_string()))?;

        let device = Rc::new(device);
        let queue = Rc::new(queue);

        // 6. Configure the swap chain surface.
        let caps = surface.get_capabilities(&adapter);
        // Prefer `Bgra8Unorm` (standard WebGPU canvas format); fall back to
        // whatever the adapter supports as its first preferred format.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm)
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // 7. Compile the WGSL shader module.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gradient_hero_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        // 8. Create the uniform buffer (16 bytes: time_s + 3 × padding).
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gradient_hero_uniforms"),
            size: UNIFORM_BUFFER_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 9. Build bind group layout + bind group.
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gradient_hero_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gradient_hero_bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        // 10. Build the render pipeline.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gradient_hero_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gradient_hero_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // fullscreen triangle: no vertex buffer
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            pipeline,
            uniform_buf,
            bind_group,
            width,
            height,
        })
    }

    /// Internal synchronous frame draw.
    fn render_frame(&self, time_ms: f32) {
        // Convert ms → s and pack into a 16-byte uniform buffer [time, 0, 0, 0].
        let time_s = time_ms / 1000.0;
        let uniform_data = [time_s, 0.0_f32, 0.0_f32, 0.0_f32];
        self.queue.write_buffer(
            &self.uniform_buf,
            UNIFORM_OFFSET_TIME,
            bytemuck::cast_slice(&uniform_data),
        );

        // Acquire the next swap-chain texture.  On failure (e.g. lost device,
        // canvas hidden) we skip this frame silently.
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return,
        };
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gradient_hero_frame"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gradient_hero_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    // depth_slice: None for a non-3D texture view (standard 2D canvas).
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            // 3 vertices, 1 instance: fullscreen triangle from WGSL vertex shader.
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(core::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

// ---------------------------------------------------------------------------
// DOM helpers
// ---------------------------------------------------------------------------

/// Retrieve an `HtmlCanvasElement` by its `id` attribute.
///
/// Returns `GradientError::CanvasNotFound` if the element does not exist or
/// is not an `HTMLCanvasElement`.
fn canvas_by_id(id: &str) -> Result<HtmlCanvasElement, GradientError> {
    let window =
        web_sys::window().ok_or_else(|| GradientError::JsError("no window object".to_owned()))?;
    let document =
        window.document().ok_or_else(|| GradientError::JsError("no document".to_owned()))?;
    let element = document
        .get_element_by_id(id)
        .ok_or_else(|| GradientError::CanvasNotFound(id.to_owned()))?;
    element
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| GradientError::CanvasNotFound(id.to_owned()))
}

// ---------------------------------------------------------------------------
// JS promise bridge helper (kept for potential future use in this module)
// ---------------------------------------------------------------------------

/// Awaits a JavaScript `Promise` and maps its resolved `JsValue` with `f`.
///
/// Used internally for any async JS API that returns `Promise<T>`.
#[allow(dead_code)]
async fn await_promise<T, F>(promise: js_sys::Promise, f: F) -> Result<T, GradientError>
where
    F: FnOnce(JsValue) -> Result<T, GradientError>,
{
    let value =
        JsFuture::from(promise).await.map_err(|e| GradientError::JsError(format!("{e:?}")))?;
    f(value)
}
