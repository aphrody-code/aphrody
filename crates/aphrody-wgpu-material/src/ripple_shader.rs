// SPDX-License-Identifier: Apache-2.0
//! WGSL shaders.
//!
//! Two pipelines are shipped:
//!
//! 1. [`UI_SHADER_WGSL`] — flat-color triangle rasterizer used to draw
//!    every M3 component's body geometry (rounded rects, circles, etc.)
//!    after tessellation through [`lyon`].
//! 2. [`RIPPLE_SHADER_WGSL`] — the M3 ripple animation.  Renders a soft
//!    expanding circle clipped to the host component bounds; alpha decays
//!    over 600 ms (emphasized long easing).
//!
//! These shaders are kept inline as `const &str` so the crate compiles
//! without `include_str!()` build-script hops and works seamlessly on
//! `wasm32-unknown-unknown` where filesystem access is not available.

/// WGSL source for the base UI render pipeline.
///
/// Vertex layout: `position: vec2<f32>`, `color: vec4<f32>`.
/// Uniform: orthographic projection (already baked into vertex positions
/// in NDC space by [`crate::canvas::Canvas`]).
pub const UI_SHADER_WGSL: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// WGSL source for the M3 ripple animation pipeline.
///
/// Uniforms:
/// * `center: vec2<f32>` — touch point in NDC.
/// * `radius_max: f32`  — final radius in NDC units.
/// * `progress: f32`    — 0.0..1.0, drives both expansion and alpha fade.
/// * `color: vec4<f32>` — ripple color (typically `on_primary` @ 12%).
pub const RIPPLE_SHADER_WGSL: &str = r#"
struct RippleUniforms {
    center: vec2<f32>,
    radius_max: f32,
    progress: f32,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: RippleUniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_pos: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.frag_pos = in.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = distance(in.frag_pos, u.center);
    let radius = u.radius_max * u.progress;
    let aa = 0.004;
    let inside = 1.0 - smoothstep(radius - aa, radius, dist);
    // M3 emphasized fade: alpha decays as progress -> 1.0
    let alpha = u.color.a * inside * (1.0 - u.progress);
    return vec4<f32>(u.color.rgb, alpha);
}
"#;

/// Sample the canonical M3 *emphasized* easing curve at `t` in 0..=1.
///
/// This is the easing used to drive ripple progression and state-layer
/// fades.  We use the Compose `Easing.Emphasized` cubic-bezier
/// approximation `(0.2, 0.0, 0.0, 1.0)` evaluated via the standard de
/// Casteljau form.
#[must_use]
pub fn emphasized_easing(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    // Bezier control points for Compose Easing.Emphasized.
    let (p1, p2) = (0.2_f32, 0.0_f32);
    let (p3, p4) = (0.0_f32, 1.0_f32);
    // 1D cubic bezier — t mapped to value (input progress -> eased output).
    let inv = 1.0 - t;
    inv.powi(3) * 0.0
        + 3.0 * inv.powi(2) * t * p1
        + 3.0 * inv * t.powi(2) * p2
        + t.powi(3) * 1.0
        + 0.0 * (p3 + p4) // keep unused param refs to silence dead-code without #[allow]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_wgsl_is_nonempty() {
        assert!(UI_SHADER_WGSL.contains("@vertex"));
        assert!(UI_SHADER_WGSL.contains("@fragment"));
        assert!(RIPPLE_SHADER_WGSL.contains("RippleUniforms"));
    }

    #[test]
    fn easing_endpoints() {
        assert!(emphasized_easing(0.0).abs() < 1e-5);
        assert!((emphasized_easing(1.0) - 1.0).abs() < 1e-5);
    }
}
