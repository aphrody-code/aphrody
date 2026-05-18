// SPDX-License-Identifier: Apache-2.0
//! M3 component showcase — winit window 1280×800 rendering every shipped
//! component on a grid with real interaction.
//!
//! Runs on native targets only.  On `wasm32-unknown-unknown` the binary
//! degrades to a trivial `main` so the showcase still compiles (the real
//! WASM showcase lives in `aphrody-wgpu-demo` / `aphrody-wasm`).

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    native::run();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // No-op stub for WASM target compilation.
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::sync::Arc;

    use aphrody_wgpu_material::{
        Button, Card, Dialog, Event as MEvent, Fab, MaterialComponent,
        NavigationBar, Slider, Snackbar, Switch, Tabs, TextField,
        canvas::{Canvas, Rect},
        m3_tokens::BASELINE_LIGHT,
        ripple_shader::UI_SHADER_WGSL,
    };
    use pollster::block_on;
    use tracing::info;
    use wgpu::util::DeviceExt as _;
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::{Window, WindowId};

    const W: u32 = 1280;
    const H: u32 = 800;

    /// Shared component grid laid out across the viewport.
    struct Showcase {
        buttons: Vec<Button>,
        cards: Vec<Card>,
        fabs: Vec<Fab>,
        text_fields: Vec<TextField>,
        nav_bar: NavigationBar,
        snackbar: Snackbar,
        dialog: Dialog,
        tabs: Tabs,
        switches: Vec<Switch>,
        sliders: Vec<Slider>,
        cursor: (f32, f32),
    }

    impl Showcase {
        fn new() -> Self {
            let mut buttons = vec![
                Button::filled("Filled"),
                Button::outlined("Outlined"),
                Button::text("Text"),
                Button::elevated("Elevated"),
                Button::tonal("Tonal"),
            ];
            for (i, b) in buttons.iter_mut().enumerate() {
                b.bounds = Rect::new(24.0 + i as f32 * 140.0, 24.0, 120.0, 40.0);
            }
            let mut cards = vec![Card::elevated(), Card::filled(), Card::outlined()];
            for (i, c) in cards.iter_mut().enumerate() {
                c.bounds = Rect::new(24.0 + i as f32 * 260.0, 100.0, 240.0, 120.0);
            }
            let mut fabs = vec![Fab::small(), Fab::regular(), Fab::large(), Fab::extended("Create")];
            let mut x = 24.0;
            for f in &mut fabs {
                let dim = f.size.dimension();
                let w = if matches!(f.size, aphrody_wgpu_material::FabSize::Extended) {
                    144.0
                } else {
                    dim
                };
                f.bounds = Rect::new(x, 252.0, w, dim);
                x += w + 16.0;
            }
            let mut text_fields = vec![
                TextField::filled("Name"),
                TextField::outlined("Email"),
            ];
            for (i, t) in text_fields.iter_mut().enumerate() {
                t.bounds = Rect::new(24.0 + i as f32 * 300.0, 380.0, 280.0, 56.0);
            }
            let mut nav_bar = NavigationBar::new(&["Home", "Search", "Library", "Profile"]);
            nav_bar.bounds = Rect::new(0.0, (H as f32) - 80.0, W as f32, 80.0);
            let mut snackbar = Snackbar::with_action("File saved", "Undo");
            snackbar.bounds = Rect::new(24.0, 470.0, 400.0, 48.0);
            snackbar.auto_dismiss_ms = 60_000; // long-lived for showcase.
            let mut dialog = Dialog::new("Discard changes?", "Your edits will be lost.");
            dialog.set_viewport(W as f32, H as f32);
            dialog.open = false; // off by default; press D to open.
            let mut tabs = Tabs::primary(&["Overview", "Specs", "Reviews"]);
            tabs.bounds = Rect::new(450.0, 470.0, 480.0, 48.0);
            let mut switches = vec![Switch::new(), Switch::new()];
            switches[1].checked = true;
            for (i, s) in switches.iter_mut().enumerate() {
                s.bounds = Rect::new(24.0 + i as f32 * 80.0, 540.0, Switch::WIDTH, Switch::HEIGHT);
            }
            let mut sliders = vec![Slider::continuous(), Slider::discrete(0.0, 100.0, 4)];
            for (i, s) in sliders.iter_mut().enumerate() {
                s.bounds = Rect::new(200.0 + i as f32 * 380.0, 540.0, 360.0, Slider::HEIGHT);
            }
            Self {
                buttons,
                cards,
                fabs,
                text_fields,
                nav_bar,
                snackbar,
                dialog,
                tabs,
                switches,
                sliders,
                cursor: (0.0, 0.0),
            }
        }

        fn dispatch(&mut self, ev: MEvent) {
            macro_rules! relay {
                ($coll:expr) => {
                    for c in $coll.iter_mut() {
                        let _ = c.handle_event(&ev);
                    }
                };
            }
            relay!(self.buttons);
            relay!(self.cards);
            relay!(self.fabs);
            relay!(self.text_fields);
            self.nav_bar.handle_event(&ev);
            self.snackbar.handle_event(&ev);
            self.dialog.handle_event(&ev);
            self.tabs.handle_event(&ev);
            relay!(self.switches);
            relay!(self.sliders);
        }

        fn paint(&self, canvas: &mut Canvas) {
            // Background.
            canvas.fill_rounded_rect(Rect::new(0.0, 0.0, W as f32, H as f32), 0.0, BASELINE_LIGHT.background);
            for b in &self.buttons { b.paint(canvas); }
            for c in &self.cards { c.paint(canvas); }
            for f in &self.fabs { f.paint(canvas); }
            for t in &self.text_fields { t.paint(canvas); }
            self.tabs.paint(canvas);
            for s in &self.switches { s.paint(canvas); }
            for s in &self.sliders { s.paint(canvas); }
            self.snackbar.paint(canvas);
            self.nav_bar.paint(canvas);
            // Dialog last (overlays scrim).
            self.dialog.paint(canvas);
        }
    }

    struct App {
        window: Option<Arc<Window>>,
        gpu: Option<Gpu>,
        showcase: Showcase,
    }

    struct Gpu {
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        pipeline: wgpu::RenderPipeline,
        canvas: Canvas,
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let win_attrs = Window::default_attributes()
                .with_title("aphrody-wgpu-material — M3 Showcase")
                .with_inner_size(winit::dpi::LogicalSize::new(W as f64, H as f64));
            let window = Arc::new(event_loop.create_window(win_attrs).expect("create window"));
            self.window = Some(window.clone());

            let instance =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let surface = instance.create_surface(window.clone()).expect("create surface");
            let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .expect("adapter");
            let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("aphrody-wgpu-material device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            }))
            .expect("device");
            let size = window.inner_size();
            let surface_caps = surface.get_capabilities(&adapter);
            let format = surface_caps.formats[0];
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ui_shader"),
                source: wgpu::ShaderSource::Wgsl(UI_SHADER_WGSL.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ui_layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ui_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[aphrody_wgpu_material::canvas::Vertex::buffer_layout()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
            let canvas = Canvas::new((W as f32, H as f32));
            self.gpu = Some(Gpu { device, queue, surface, config, pipeline, canvas });
            info!("aphrody-wgpu-material showcase ready");
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::CursorMoved { position, .. } => {
                    self.showcase.cursor = (position.x as f32, position.y as f32);
                    self.showcase.dispatch(MEvent::PointerMove {
                        x: position.x as f32,
                        y: position.y as f32,
                    });
                }
                WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                    let (x, y) = self.showcase.cursor;
                    let ev = if matches!(state, ElementState::Pressed) {
                        MEvent::PointerDown { x, y }
                    } else {
                        MEvent::PointerUp { x, y }
                    };
                    self.showcase.dispatch(ev);
                }
                WindowEvent::Resized(new_size) => {
                    if let Some(g) = self.gpu.as_mut() {
                        g.config.width = new_size.width.max(1);
                        g.config.height = new_size.height.max(1);
                        g.surface.configure(&g.device, &g.config);
                    }
                }
                WindowEvent::RedrawRequested => self.render(),
                _ => {}
            }
            if let Some(w) = self.window.as_ref() {
                w.request_redraw();
            }
        }
    }

    impl App {
        fn render(&mut self) {
            let Some(g) = self.gpu.as_mut() else { return; };
            g.canvas.clear();
            self.showcase.paint(&mut g.canvas);
            if g.canvas.index_count() == 0 {
                return;
            }
            let (vbytes, ibytes) = g.canvas.raw_buffers();
            let vbuf = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vbuf"),
                contents: vbytes,
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ibuf = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ibuf"),
                contents: ibytes,
                usage: wgpu::BufferUsages::INDEX,
            });
            let frame = match g.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                _ => return,
            };
            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut enc =
                g.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 1.0,
                                g: 0.985,
                                b: 0.996,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                rp.set_pipeline(&g.pipeline);
                rp.set_vertex_buffer(0, vbuf.slice(..));
                rp.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..g.canvas.index_count() as u32, 0, 0..1);
            }
            g.queue.submit(Some(enc.finish()));
            frame.present();
        }
    }

    pub(super) fn run() {
        let event_loop = EventLoop::new().expect("event loop");
        let mut app = App { window: None, gpu: None, showcase: Showcase::new() };
        let _ = event_loop.run_app(&mut app);
    }
}
