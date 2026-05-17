// SPDX-License-Identifier: Apache-2.0
//
// Gradient hero WGSL shader — animated Gemini brand spectrum-shift.
//
// Vertex stage: emits a full-screen triangle (3 vertices, no vertex buffer).
// Fragment stage: 4-colour animated gradient driven by a `time` uniform.
//
// Brand colours (sRGB linear-ish, sourced from crates/m3-tokens gemini_brand.rs):
//   Blue   #4285F4  →  (0.259, 0.522, 0.957)
//   Purple #9168C0  →  (0.569, 0.408, 0.753)
//   Pink   #EC4899  →  (0.925, 0.282, 0.600)
//   Cyan   #00BCD4  →  (0.000, 0.737, 0.831)
//
// The animation mimics the CSS `gemini-shimmer` keyframes: the gradient
// band sweeps horizontally at ~1.8 s period (angular frequency 3.49 rad/s)
// while a slow vertical morph blends in the fourth colour (Cyan) over ~6 s.

// ---------------------------------------------------------------------------
// Vertex stage
// ---------------------------------------------------------------------------
struct VertexOut {
    @builtin(position) position : vec4<f32>,
    @location(0)       uv       : vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx : u32) -> VertexOut {
    // Three vertices of a large triangle that covers the entire NDC viewport.
    // No vertex buffer required — indices 0-2 drive the fullscreen pass.
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let p = positions[idx];
    var out : VertexOut;
    out.position = vec4<f32>(p, 0.0, 1.0);
    // Map NDC [-1,1]×[-1,1] to UV [0,1]×[0,1] (Y-flip matches canvas origin).
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 1.0 - (p.y * 0.5 + 0.5));
    return out;
}

// ---------------------------------------------------------------------------
// Uniforms
// ---------------------------------------------------------------------------
struct Uniforms {
    // Elapsed time in seconds since the renderer was mounted.
    time_s : f32,
    // Three floats of padding to satisfy the WebGPU minimum uniform stride
    // (16 bytes / 4-component alignment, wgsl offset alignment rule §13.4.1).
    _pad0  : f32,
    _pad1  : f32,
    _pad2  : f32,
}

@group(0) @binding(0) var<uniform> u : Uniforms;

// ---------------------------------------------------------------------------
// Brand colour constants (linear-light sRGB, no gamma transfer)
// ---------------------------------------------------------------------------
const BLUE   : vec3<f32> = vec3<f32>(0.259, 0.522, 0.957); // #4285F4
const PURPLE : vec3<f32> = vec3<f32>(0.569, 0.408, 0.753); // #9168C0
const PINK   : vec3<f32> = vec3<f32>(0.925, 0.282, 0.600); // #EC4899
const CYAN   : vec3<f32> = vec3<f32>(0.000, 0.737, 0.831); // #00BCD4

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Smooth cubic Hermite: maps t ∈ [0,1] to a smooth S-curve.
fn smoothstep_v(t : f32) -> f32 {
    let c = clamp(t, 0.0, 1.0);
    return c * c * (3.0 - 2.0 * c);
}

// Linear interpolation between two vec3.
fn lerp3(a : vec3<f32>, b : vec3<f32>, t : f32) -> vec3<f32> {
    return a + (b - a) * clamp(t, 0.0, 1.0);
}

// ---------------------------------------------------------------------------
// Fragment stage
// ---------------------------------------------------------------------------
@fragment
fn fs_main(in : VertexOut) -> @location(0) vec4<f32> {
    // --- Horizontal shimmer sweep (1.8 s period, mirrors gemini-shimmer CSS) ---
    // shift ∈ [0, 1] oscillates at 2π/1.8 ≈ 3.491 rad/s.
    let shift = 0.5 + 0.5 * sin(u.time_s * 3.491);
    // Perturb UV.x by ±12.5 % of the viewport width.
    let x = clamp(in.uv.x + (shift - 0.5) * 0.25, 0.0, 1.0);

    // --- Vertical morph weight (6 s period) blends PINK ↔ CYAN at the top ---
    let morph = 0.5 + 0.5 * sin(u.time_s * 1.047); // 2π/6 ≈ 1.047 rad/s
    // At y=0 (bottom edge) no morph; at y=1 (top edge) full morph weight.
    let y_weight = smoothstep_v(in.uv.y);

    // --- Base gradient: BLUE → PURPLE → PINK across x ∈ [0, 1] ---
    var base : vec3<f32>;
    if x < 0.5 {
        base = lerp3(BLUE, PURPLE, x * 2.0);
    } else {
        base = lerp3(PURPLE, PINK, (x - 0.5) * 2.0);
    }

    // --- Inject CYAN tint at the top edge proportional to morph ---
    let top_accent = lerp3(base, CYAN, morph * y_weight * 0.55);

    // --- Diagonal vignette: soft radial darkening from centre edges ---
    let dist = length(in.uv - vec2<f32>(0.5, 0.5));
    let vignette = 1.0 - smoothstep_v(dist * 1.2);

    let colour = top_accent * (0.80 + 0.20 * vignette);
    return vec4<f32>(colour, 1.0);
}
