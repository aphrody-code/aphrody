// SPDX-License-Identifier: Apache-2.0
//! `aphrody-wgpu-material` — Material Design 3 component library rendered
//! through [`wgpu`](https://wgpu.rs).
//!
//! This crate provides ten core M3 components (Button, Card, FAB,
//! TextField, NavigationBar, Snackbar, Dialog, Tabs, Switch, Slider)
//! exposed as plain Rust types implementing [`MaterialComponent`].
//! Layout is computed in logical pixels (1 dp = 1 logical px at 1.0
//! density), painting is dispatched through a [`Canvas`] abstraction that
//! tessellates rounded rectangles via [`lyon`] and uploads them as
//! vertex/index buffers to a [`wgpu::RenderPass`].
//!
//! The crate is `std` only (alloc + threads) but stays portable to
//! `wasm32-unknown-unknown` thanks to native-only gating on `winit` and
//! `cosmic-text` (see `Cargo.toml`).  WebGPU support is shipped natively
//! by [`wgpu`] on the browser.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ components::* (Button, Card, FAB, …)                            │
//! │     ▼ implements                                                │
//! │ MaterialComponent { layout, paint, handle_event }               │
//! │     ▼ uses                                                      │
//! │ Canvas (fill_rounded_rect, draw_text, draw_circle, …)           │
//! │     ▼ tessellates via                                           │
//! │ lyon → Vertex/Index buffers → wgpu::RenderPass                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick start
//!
//! ```no_run
//! use aphrody_wgpu_material::{
//!     components::Button,
//!     m3_tokens::BASELINE_LIGHT,
//!     Constraints, MaterialComponent,
//! };
//!
//! let mut btn = Button::filled("OK");
//! let _size = btn.layout(&Constraints::tight(120.0, 40.0));
//! // `btn.paint(&mut canvas)` would be called inside a wgpu render pass.
//! # let _ = BASELINE_LIGHT;
//! ```

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod canvas;
pub mod components;
pub mod m3_tokens;
pub mod ripple_shader;
pub mod state_layer;

use std::time::Duration;

pub use canvas::{Canvas, Color, Rect};
pub use components::{
    Button, ButtonVariant, Card, CardVariant, Dialog, Fab, FabSize,
    NavigationBar, Slider, Snackbar, Switch, Tabs, TextField,
    TextFieldVariant,
};
pub use m3_tokens::{BASELINE_DARK, BASELINE_LIGHT, ColorRoles};
pub use state_layer::{State, state_layer_color};

/// Constraints passed to [`MaterialComponent::layout`].  Mirrors the
/// canonical M3 layout protocol: a min/max box in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    /// Minimum width in logical pixels (≥ 0).
    pub min_width: f32,
    /// Maximum width in logical pixels (≥ `min_width`).
    pub max_width: f32,
    /// Minimum height in logical pixels (≥ 0).
    pub min_height: f32,
    /// Maximum height in logical pixels (≥ `min_height`).
    pub max_height: f32,
}

impl Constraints {
    /// Unbounded constraints (0..∞).
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }

    /// Tight constraints forcing an exact size.
    #[must_use]
    pub const fn tight(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    /// Clamp `(w, h)` to the constraints box.
    #[must_use]
    pub fn clamp(&self, w: f32, h: f32) -> Size {
        Size {
            width: w.clamp(self.min_width, self.max_width),
            height: h.clamp(self.min_height, self.max_height),
        }
    }
}

/// A 2D size in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    /// Width in logical pixels.
    pub width: f32,
    /// Height in logical pixels.
    pub height: f32,
}

/// Input event delivered to a component's [`MaterialComponent::handle_event`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Pointer entered the component bounds.
    PointerEnter {
        /// Pointer X (logical pixels, top-left origin).
        x: f32,
        /// Pointer Y (logical pixels, top-left origin).
        y: f32,
    },
    /// Pointer left the component bounds.
    PointerLeave,
    /// Pointer moved while inside the component.
    PointerMove {
        /// Pointer X.
        x: f32,
        /// Pointer Y.
        y: f32,
    },
    /// Pointer pressed inside the component.
    PointerDown {
        /// Pointer X at press.
        x: f32,
        /// Pointer Y at press.
        y: f32,
    },
    /// Pointer released.
    PointerUp {
        /// Pointer X at release.
        x: f32,
        /// Pointer Y at release.
        y: f32,
    },
    /// Keyboard focus gained.
    FocusGained,
    /// Keyboard focus lost.
    FocusLost,
    /// Animation tick (monotonic milliseconds since component creation).
    Tick {
        /// Monotonic clock value in milliseconds.
        now_ms: u64,
    },
}

/// Result returned from [`MaterialComponent::handle_event`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventResult {
    /// Event was consumed and triggered a state change requiring a repaint.
    Consumed,
    /// Event was observed but no repaint is necessary.
    Ignored,
    /// Component fired a user-facing action (click, value change, etc.).
    Activated,
}

/// Marker trait implemented by every M3 component shipped in this crate.
///
/// Components are pure CPU state machines: layout computes a final size
/// from constraints, paint emits draw commands through a [`Canvas`], and
/// `handle_event` advances the internal state machine (hover, press,
/// focus, value drag, etc.).
pub trait MaterialComponent {
    /// Compute the component's final size given parent constraints.
    fn layout(&mut self, constraints: &Constraints) -> Size;

    /// Paint the component into the canvas.  The canvas owns the
    /// underlying [`wgpu::RenderPass`] and stages tessellated geometry.
    fn paint(&self, canvas: &mut Canvas);

    /// Advance the component state machine in response to an input
    /// event.  Returns whether the event was consumed.
    fn handle_event(&mut self, ev: &Event) -> EventResult;
}

/// Canonical M3 motion durations (used for ripple & state-layer fades).
pub mod motion {
    use super::Duration;

    /// Emphasized easing — short, used for ripple fade-in.
    pub const EMPHASIZED_SHORT: Duration = Duration::from_millis(200);
    /// Emphasized easing — medium, used for ripple fade-out.
    pub const EMPHASIZED_MEDIUM: Duration = Duration::from_millis(400);
    /// Emphasized easing — long, used for full ripple expansion.
    pub const EMPHASIZED_LONG: Duration = Duration::from_millis(600);
}
