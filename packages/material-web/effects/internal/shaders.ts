/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview WGSL shaders for the `<md-webgpu-canvas>` brand effects.
 *
 * All effects share a fullscreen-triangle vertex stage and a uniform block
 * (resolution, time, three brand colors). The fragment stages reproduce the
 * design.google motion language: the Gemini blue→purple→pink "spectrum-shift"
 * flow, a four-color radial "sparkle", and a transparent-surface "glimmer".
 */

/** Shared uniform block + fullscreen-triangle vertex stage. */
const COMMON_WGSL = /* wgsl */ `
struct Uniforms {
  resolution : vec2<f32>,
  time : f32,
  intensity : f32,
  colorA : vec4<f32>,
  colorB : vec4<f32>,
  colorC : vec4<f32>,
};
@group(0) @binding(0) var<uniform> u : Uniforms;

struct VsOut {
  @builtin(position) pos : vec4<f32>,
  @location(0) uv : vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i : u32) -> VsOut {
  // Oversized triangle covering the viewport.
  var p = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -3.0),
    vec2<f32>(-1.0,  1.0),
    vec2<f32>( 3.0,  1.0),
  );
  var out : VsOut;
  let xy = p[i];
  out.pos = vec4<f32>(xy, 0.0, 1.0);
  out.uv = (xy + vec2<f32>(1.0, 1.0)) * 0.5;
  return out;
}
`;

/** Spectrum-shift: flowing blue→purple→pink aurora bands. */
export const SPECTRUM_WGSL =
  COMMON_WGSL +
  /* wgsl */ `
@fragment
fn fs(in : VsOut) -> @location(0) vec4<f32> {
  let uv = in.uv;
  let t = u.time * 0.12;
  // Superposed sine flow gives an organic, non-repeating drift.
  var f = 0.5
    + 0.30 * sin(uv.x * 3.0 + t * 2.0)
    + 0.25 * sin((uv.x + uv.y) * 2.0 - t * 1.3)
    + 0.20 * sin(uv.y * 4.0 + t * 0.9);
  f = clamp(f, 0.0, 1.0);
  // Two-segment blend across the three brand stops.
  var col : vec3<f32>;
  if (f < 0.5) {
    col = mix(u.colorA.rgb, u.colorB.rgb, f * 2.0);
  } else {
    col = mix(u.colorB.rgb, u.colorC.rgb, (f - 0.5) * 2.0);
  }
  // Subtle vertical luminance lift toward the top.
  col += (1.0 - uv.y) * 0.05;
  return vec4<f32>(col * u.intensity, 1.0);
}
`;

/** Sparkle: four-color radial twinkle echoing the Google dots. */
export const SPARKLE_WGSL =
  COMMON_WGSL +
  /* wgsl */ `
fn hash(p : vec2<f32>) -> f32 {
  return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fs(in : VsOut) -> @location(0) vec4<f32> {
  let aspect = u.resolution.x / max(u.resolution.y, 1.0);
  var uv = in.uv;
  uv.x = uv.x * aspect;
  let center = vec2<f32>(0.5 * aspect, 0.5);
  let d = distance(uv, center);
  // Radial brand sweep.
  let ang = atan2(uv.y - center.y, uv.x - center.x);
  let band = (ang + u.time * 0.4) / 6.2831853;
  let bf = fract(band);
  var col : vec3<f32>;
  if (bf < 0.5) {
    col = mix(u.colorA.rgb, u.colorB.rgb, bf * 2.0);
  } else {
    col = mix(u.colorB.rgb, u.colorC.rgb, (bf - 0.5) * 2.0);
  }
  // Twinkling specks.
  let cell = floor(uv * 40.0);
  let tw = hash(cell);
  let spark = smoothstep(0.96, 1.0, sin(tw * 6.2831 + u.time * (1.0 + tw * 3.0)) * 0.5 + 0.5);
  let glow = smoothstep(0.55, 0.0, d);
  let rgb = col * glow + vec3<f32>(spark) * 0.6 * glow;
  return vec4<f32>(rgb * u.intensity, glow);
}
`;

/** Glimmer: faint moving sheen for dark/transparent surfaces. */
export const GLIMMER_WGSL =
  COMMON_WGSL +
  /* wgsl */ `
@fragment
fn fs(in : VsOut) -> @location(0) vec4<f32> {
  let uv = in.uv;
  // Diagonal travelling band.
  let band = sin((uv.x + uv.y) * 3.0 - u.time * 1.2) * 0.5 + 0.5;
  let sheen = pow(band, 6.0);
  let col = mix(u.colorA.rgb, u.colorC.rgb, uv.x);
  let a = sheen * 0.35 * u.intensity;
  return vec4<f32>(col, a);
}
`;

/** Available fragment effects. */
export const EFFECT_WGSL = {
  spectrum: SPECTRUM_WGSL,
  sparkle: SPARKLE_WGSL,
  glimmer: GLIMMER_WGSL,
} as const;

/** Name of a WebGPU brand effect. */
export type EffectName = keyof typeof EFFECT_WGSL;
