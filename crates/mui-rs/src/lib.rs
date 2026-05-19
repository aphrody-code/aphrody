//! m3-rs — Material Design 3 native UI library for Rust.
//! Cross-platform: desktop (wgpu/vello), web (WASM), Android, iOS.

pub use mui_rs_tokens as tokens;
pub use mui_rs_motion as motion;
pub use mui_rs_components as components;
pub use mui_rs_renderer as renderer;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
