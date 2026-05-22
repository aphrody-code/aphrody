// SPDX-License-Identifier: Apache-2.0
//! mui-rs M3 showcase — a self-contained winit + wgpu/vello window that renders
//! native Material Design 3 component shapes (no webview, no JS).
//!
//! `mui-rs` and its render stack are heavy (wgpu/vello) and are kept OUT of the
//! default workspace for fast `nextest` runs (see root `Cargo.toml` `exclude`).
//! To run this demo, temporarily add `crates/mui-rs`, `crates/mui-rs-renderer`,
//! `crates/mui-rs-components` and `crates/mui-rs-motion` to `[workspace] members`
//! (move them out of `exclude`), then:
//!   cargo run -p mui-rs --example m3_showcase
//!
//! Demonstrates the mui-rs render path: RenderSurface (wgpu+vello, intermediate
//! Rgba8Unorm target + TextureBlitter) + RenderPipeline (vello Scene) + the
//! components' `Widget::draw`, on the aphrody dark theme (m3-tokens APHRODY_DARK).

#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use mui_rs::components::actions::{Button, ButtonVariant};
use mui_rs::components::containers::{Card, CardVariant};
use mui_rs::components::display::{Chip, Tab};
use mui_rs::components::feedback::{ProgressIndicator, Snackbar};
use mui_rs::components::inputs::{SearchBar, Slider, Switch};
use mui_rs::components::navigation::TopAppBar;
use mui_rs::renderer::pipeline::RenderPipeline;
use mui_rs::renderer::surface::RenderSurface;
use mui_rs::renderer::vello::kurbo::Affine;
use mui_rs::renderer::vello::peniko::Color;
use mui_rs::tokens::color::APHRODY_DARK;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

fn argb_to_color(argb: u32) -> Color {
    Color::from_rgb8((argb >> 16) as u8, (argb >> 8) as u8, argb as u8)
}

struct App<'a> {
    window: Option<Arc<Window>>,
    surface: Option<RenderSurface<'a>>,
    pipeline: RenderPipeline,
}

impl App<'_> {
    fn new() -> Self {
        Self { window: None, surface: None, pipeline: RenderPipeline::new() }
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("aphrody · mui-rs M3 showcase")
            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 600.0));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let size = window.inner_size();
        match pollster::block_on(RenderSurface::new(window.clone(), size.width, size.height)) {
            Ok(s) => self.surface = Some(s),
            Err(e) => eprintln!("surface init failed: {e:?}"),
        }
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(s) = &mut self.surface {
                    s.resize(size.width, size.height);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(surface) = &mut self.surface else { return };
                self.pipeline.scene.reset();
                self.pipeline.set_base_color(argb_to_color(APHRODY_DARK.surface));

                // Top app bar
                self.pipeline.draw_widget(
                    &TopAppBar {
                        title: "aphrody — native Material 3 (wgpu + vello)".to_owned(),
                        logo_id: "aphrody".to_owned(),
                        width: 960.0,
                    },
                    Affine::IDENTITY,
                );

                // A row of buttons, one per M3 variant
                let variants = [
                    ButtonVariant::Filled,
                    ButtonVariant::FilledTonal,
                    ButtonVariant::Elevated,
                    ButtonVariant::Outlined,
                    ButtonVariant::Text,
                ];
                for (i, variant) in variants.into_iter().enumerate() {
                    let btn = Button {
                        variant,
                        label: format!("{variant:?}"),
                        disabled: false,
                        icon: None,
                        on_click_id: None,
                    };
                    self.pipeline
                        .draw_widget(&btn, Affine::translate((40.0 + i as f64 * 150.0, 110.0)));
                }

                // A row of cards, one per variant
                let cards = [CardVariant::Elevated, CardVariant::Filled, CardVariant::Outlined];
                for (i, variant) in cards.into_iter().enumerate() {
                    let card = Card { variant, interactive: false, disabled: false };
                    self.pipeline
                        .draw_widget(&card, Affine::translate((40.0 + i as f64 * 300.0, 200.0)));
                }

                // Chips (filter, unselected + selected).
                self.pipeline.draw_widget(
                    &Chip { label: "All".to_owned(), selected: true },
                    Affine::translate((40.0, 380.0)),
                );
                self.pipeline.draw_widget(
                    &Chip { label: "Starred".to_owned(), selected: false },
                    Affine::translate((110.0, 380.0)),
                );

                // Switch on/off + a slider.
                self.pipeline
                    .draw_widget(&Switch { checked: true, disabled: false }, Affine::translate((240.0, 376.0)));
                self.pipeline
                    .draw_widget(&Switch { checked: false, disabled: false }, Affine::translate((310.0, 376.0)));
                self.pipeline
                    .draw_widget(&Slider { value: 0.6, disabled: false }, Affine::translate((400.0, 372.0)));
                self.pipeline.draw_widget(
                    &ProgressIndicator { progress: Some(0.45) },
                    Affine::translate((640.0, 392.0)),
                );

                // Primary tabs.
                self.pipeline.draw_widget(
                    &Tab {
                        labels: vec!["Overview".to_owned(), "Specs".to_owned(), "Code".to_owned()],
                        active: 0,
                    },
                    Affine::translate((40.0, 430.0)),
                );

                // Search bar + snackbar.
                self.pipeline.draw_widget(
                    &SearchBar { query: String::new() },
                    Affine::translate((40.0, 500.0)),
                );
                self.pipeline.draw_widget(
                    &Snackbar { message: "Saved to library".to_owned() },
                    Affine::translate((420.0, 504.0)),
                );

                if let Err(e) = self.pipeline.render_to_surface(surface) {
                    eprintln!("render error: {e:?}");
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run app");
}
