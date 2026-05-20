#![cfg(not(target_arch = "wasm32"))]
#![allow(warnings)]
#![forbid(unsafe_code)]
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use mui_rs_renderer::surface::RenderSurface;
use mui_rs_renderer::pipeline::RenderPipeline;
use mui_rs_components::actions::{Button, ButtonVariant};
use mui_rs_renderer::vello::kurbo::Affine;
use mui_rs_renderer::vello::peniko::Color;
use tracing::{info, error};
use gui::NextContext;

struct App<'a> {
    window: Option<Arc<Window>>,
    surface: Option<RenderSurface<'a>>,
    pipeline: RenderPipeline,
    next_ctx: Option<NextContext>,
    chat_history: Vec<String>,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self {
            window: None,
            surface: None,
            pipeline: RenderPipeline::new(),
            next_ctx: None,
            chat_history: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowSizeClass {
    Compact,     // < 600dp
    Medium,      // 600-839dp
    Expanded,    // 840-1199dp
    Large,       // 1200-1599dp
    ExtraLarge,  // >= 1600dp
}

impl WindowSizeClass {
    fn from_width(width: f32) -> Self {
        if width < 600.0 {
            Self::Compact
        } else if width < 840.0 {
            Self::Medium
        } else if width < 1200.0 {
            Self::Expanded
        } else if width < 1600.0 {
            Self::Large
        } else {
            Self::ExtraLarge
        }
    }

    fn margin(&self) -> f32 {
        match self {
            Self::Compact => 16.0,
            _ => 24.0,
        }
    }
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Aphrody Universal Engine")
                .with_decorations(false)
                .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            let size = window.inner_size();
            
            match pollster::block_on(RenderSurface::new(window.clone(), size.width, size.height)) {
                Ok(surface) => {
                    self.surface = Some(surface);
                    info!("WGPU/Vello Native surface initialized");
                }
                Err(e) => error!("Failed to initialize WGPU surface: {:?}", e),
            }

            // Async init of NextContext + Aphrody SDK
            let ctx = pollster::block_on(NextContext::new()).expect("Failed to init engine");
            self.next_ctx = Some(ctx);
            info!("Aphrody Universal Engine initialized (SDK v{})", aphrody_sdk::VERSION);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = self.window.as_ref().unwrap();
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    surface.resize(size.width, size.height);
                }
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let (Some(surface), Some(next_ctx)) = (&mut self.surface, &self.next_ctx) {
                    let window_size = window.inner_size();
                    let width = window_size.width as f32;
                    let height = window_size.height as f32;
                    let size_class = WindowSizeClass::from_width(width);

                    // Clear the scene and draw our native M3 widget tree
                    self.pipeline.scene.reset();
                    
                    // Material 3 Surface background color
                    let bg_color = Color::from_rgb8(28, 27, 31); // Surface color (Dark)
                    self.pipeline.set_base_color(bg_color);

                    use mui_rs_components::navigation::{TopAppBar, NavigationBar, NavigationRail, NavigationDrawer};
                    
                    // 1. Top App Bar (Custom Title Bar)
                    let header = TopAppBar {
                        title: "Aphrody Universal Native".to_owned(),
                        logo_id: "gemini".to_owned(),
                        width: width as f64,
                    };
                    self.pipeline.draw_widget(&header, Affine::IDENTITY);

                    // --- TAFFY ADAPTIVE GRID LAYOUT ---
                    use taffy::prelude::*;
                    
                    // Root Grid: Sidebar (Rail/Drawer), Main (Flexible), Engine Status (Bottom)
                    let root_style = Style {
                        display: Display::Grid,
                        size: Size { 
                            width: length(width), 
                            height: length(height) 
                        },
                        grid_template_columns: match size_class {
                            WindowSizeClass::Compact => vec![fr(1.0_f32)],
                            WindowSizeClass::Medium => vec![length(NavigationRail::WIDTH_DP as f32), fr(1.0_f32)],
                            _ => vec![length(NavigationDrawer::WIDTH_DP as f32), fr(1.0_f32)],
                        },
                        grid_template_rows: vec![
                            length(TopAppBar::HEIGHT_DP as f32), // Header
                            fr(1.0_f32),                        // Content
                            length(120.0_f32),                   // Footer/Engine Status
                        ],
                        ..Default::default()
                    };

                    // Sidebar Node
                    let sidebar_node = self.pipeline.layout.taffy.new_leaf(match size_class {
                        WindowSizeClass::Compact => Style { display: Display::None, ..Default::default() },
                        _ => Style { grid_column: line(1), grid_row: line(2), ..Default::default() },
                    }).unwrap();

                    // Main Node
                    let main_node = self.pipeline.layout.taffy.new_leaf(Style {
                        grid_column: if size_class == WindowSizeClass::Compact { line(1) } else { line(2) },
                        grid_row: line(2),
                        padding: Rect {
                            left: length(size_class.margin()),
                            right: length(size_class.margin()),
                            top: length(size_class.margin()),
                            bottom: length(size_class.margin()),
                        },
                        ..Default::default()
                    }).unwrap();

                    // Engine Status Node (Bottom)
                    let engine_node = self.pipeline.layout.taffy.new_leaf(Style {
                        grid_column: if size_class == WindowSizeClass::Compact { span(1) } else { span(2) },
                        grid_row: line(3),
                        padding: Rect { left: length(24.0_f32), right: length(24.0_f32), top: length(8.0_f32), bottom: length(8.0_f32) },
                        ..Default::default()
                    }).unwrap();

                    let root_node = self.pipeline.layout.taffy.new_with_children(root_style, &[sidebar_node, main_node, engine_node]).unwrap();

                    // Compute
                    self.pipeline.layout.compute_layout(
                        root_node, 
                        mui_rs_renderer::vello::kurbo::Size::new(width as f64, height as f64)
                    ).unwrap();

                    // --- DRAWING ---

                    // Draw Navigation based on size class
                    match size_class {
                        WindowSizeClass::Compact => {
                            let nav_bar = NavigationBar { active_item: 0, width: width as f64 };
                            self.pipeline.draw_widget(&nav_bar, Affine::translate((0.0, height as f64 - NavigationBar::HEIGHT_DP)));
                        }
                        WindowSizeClass::Medium => {
                            let nav_rail = NavigationRail { active_item: 0, height: height as f64 };
                            self.pipeline.draw_widget(&nav_rail, Affine::IDENTITY);
                        }
                        _ => {
                            let nav_drawer = NavigationDrawer { open: true, height: height as f64 };
                            self.pipeline.draw_widget(&nav_drawer, Affine::IDENTITY);
                        }
                    }

                    // Main Content Drawing
                    let main_layout = self.pipeline.layout.get_layout(main_node).unwrap();
                    let main_offset = Affine::translate((main_layout.location.x as f64, main_layout.location.y as f64));
                    
                    // Universal Engine Status
                    let status_label = format!("Agent: Active | Memory: LocalSqlite | Tools: {} loaded", next_ctx.sdk.tool_registry().len());
                    let chip = Button {
                        variant: ButtonVariant::Filled,
                        disabled: true,
                        icon: None,
                        label: status_label,
                        on_click_id: None,
                    };
                    self.pipeline.draw_widget(&chip, main_offset);

                    // Live Engine Transpilation (SWC)
                    let raw_code = "const engine = { active: true, unified: true };";
                    let transpiled = next_ctx.compiler.transpile_snippet(raw_code);
                    use mui_rs_components::inputs::{TextField, TextFieldVariant};
                    let swc_field = TextField {
                        label: "Integrated Compiler Backend (SWC)".to_owned(),
                        value: transpiled,
                        variant: TextFieldVariant::Filled,
                        disabled: true,
                        error: false,
                    };
                    self.pipeline.draw_at_node(&swc_field, engine_node, Affine::IDENTITY);

                    if let Err(e) = self.pipeline.render_to_surface(surface) {
                        error!("Render error: {:?}", e);
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting Aphrody M3 Native Desktop UI");

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
