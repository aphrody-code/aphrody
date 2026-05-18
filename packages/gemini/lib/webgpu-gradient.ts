// SPDX-License-Identifier: Apache-2.0
//
// WebGPU spectrum-shift gradient renderer.
//
// Note: TypeScript's `lib.dom` (as of TS 5.6) ships the WebGPU types
// (GPU, GPUDevice, GPUBuffer, GPUTextureFormat, ...) gated behind the
// `"WebGPU"` lib entry — but we treat them as opaque structural types
// here so the file compiles against either lib set. The runtime uses
// `navigator.gpu` discovery so missing types are simply branch-skipped.
//
// We deliberately AVOID an `@webgpu/types` dev dep — it's superseded by
// the upstream TS lib in 5.6+ and would diverge from `lib.dom`.
//
// Renders the canonical blue -> purple -> pink Gemini brand gradient onto a
// `HTMLCanvasElement` via a real WGSL fragment shader. Used by the
// HeroEmptyState component for the animated empty-state backdrop.
//
// Falls back to a pure CSS `linear-gradient` overlay when WebGPU is
// unavailable (`!navigator.gpu`). The CSS fallback is the same gradient
// string exported from `m3-tokens.ts` so the visual diff between the two
// paths is zero except for shimmer animation parity.

import { SPECTRUM_SHIFT_CSS } from "./m3-tokens.ts";

// ── Structural WebGPU type declarations ─────────────────────────────────────
// We declare the slimmest subset we use so the file typechecks regardless
// of whether the consumer's tsconfig pulls in `lib.dom` with WebGPU types.

interface WebGpuBuffer {
  destroy(): void;
}
interface WebGpuBindGroup {}
interface WebGpuPipeline {
  getBindGroupLayout(index: number): unknown;
}
interface WebGpuTextureView {}
interface WebGpuTexture {
  createView(): WebGpuTextureView;
}
interface WebGpuRenderPass {
  setPipeline(p: WebGpuPipeline): void;
  setBindGroup(index: number, group: WebGpuBindGroup): void;
  draw(vertexCount: number, instanceCount: number, firstVertex: number, firstInstance: number): void;
  end(): void;
}
interface WebGpuEncoder {
  beginRenderPass(desc: unknown): WebGpuRenderPass;
  finish(): unknown;
}
interface WebGpuQueue {
  writeBuffer(buf: WebGpuBuffer, offset: number, data: ArrayBufferView): void;
  submit(commands: ReadonlyArray<unknown>): void;
}
interface WebGpuDevice {
  createShaderModule(desc: { code: string }): unknown;
  createRenderPipeline(desc: unknown): WebGpuPipeline;
  createBuffer(desc: { size: number; usage: number }): WebGpuBuffer;
  createBindGroup(desc: unknown): WebGpuBindGroup;
  createCommandEncoder(): WebGpuEncoder;
  queue: WebGpuQueue;
}
interface WebGpuAdapter {
  requestDevice(): Promise<WebGpuDevice>;
}
interface WebGpuContext {
  configure(desc: unknown): void;
  getCurrentTexture(): WebGpuTexture;
}
interface WebGpuRoot {
  requestAdapter(): Promise<WebGpuAdapter | null>;
  getPreferredCanvasFormat(): string;
}

// Buffer-usage flags — mirror the WebGPU spec constants without depending
// on the `GPUBufferUsage` global.
const BUF_USAGE_UNIFORM = 0x40;
const BUF_USAGE_COPY_DST = 0x8;

const WGSL_SHADER = /* wgsl */ `
struct VertexOut {
  @builtin(position) position : vec4<f32>,
  @location(0) uv : vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx : u32) -> VertexOut {
  // Fullscreen triangle (three vertices covering NDC viewport).
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 3.0, -1.0),
    vec2<f32>(-1.0,  3.0),
  );
  let p = positions[idx];
  var out : VertexOut;
  out.position = vec4<f32>(p, 0.0, 1.0);
  out.uv = (p * 0.5) + vec2<f32>(0.5, 0.5);
  return out;
}

struct Uniforms {
  time_s : f32,
  _pad0  : f32,
  _pad1  : f32,
  _pad2  : f32,
};
@group(0) @binding(0) var<uniform> u : Uniforms;

const BLUE   : vec3<f32> = vec3<f32>(0.259, 0.522, 0.957); // #4285F4
const PURPLE : vec3<f32> = vec3<f32>(0.569, 0.408, 0.753); // #9168C0
const PINK   : vec3<f32> = vec3<f32>(0.925, 0.282, 0.600); // #EC4899

fn lerp3(a : vec3<f32>, b : vec3<f32>, t : f32) -> vec3<f32> {
  return a + (b - a) * t;
}

@fragment
fn fs_main(in : VertexOut) -> @location(0) vec4<f32> {
  // Shift the gradient horizontally as a slow standing wave to mimic the
  // gemini-shimmer keyframes (1.8s linear infinite cadence).
  let shift = 0.5 + 0.5 * sin(u.time_s * 3.49); // 2*pi / 1.8 ~= 3.49
  let x = clamp(in.uv.x + (shift - 0.5) * 0.25, 0.0, 1.0);
  var c : vec3<f32>;
  if (x < 0.5) {
    c = lerp3(BLUE, PURPLE, x * 2.0);
  } else {
    c = lerp3(PURPLE, PINK, (x - 0.5) * 2.0);
  }
  return vec4<f32>(c, 1.0);
}
`;

export interface SpectrumShiftHandle {
  /** Stop the render loop and release GPU resources. */
  stop(): void;
}

/**
 * Mount a WebGPU spectrum-shift renderer on `canvas`. Returns a handle the
 * caller MUST stop on unmount. If WebGPU is unavailable the function
 * returns `null` and the caller should render the CSS fallback instead.
 */
export async function mountSpectrumShift(
  canvas: HTMLCanvasElement,
): Promise<SpectrumShiftHandle | null> {
  if (typeof navigator === "undefined") return null;
  const gpu = (navigator as unknown as { gpu?: WebGpuRoot }).gpu;
  if (!gpu) return null;

  const adapter = await gpu.requestAdapter();
  if (!adapter) return null;
  const device = await adapter.requestDevice();
  const context = canvas.getContext("webgpu") as unknown as WebGpuContext | null;
  if (!context) return null;

  const format = gpu.getPreferredCanvasFormat();
  context.configure({ device, format, alphaMode: "premultiplied" });

  const shader = device.createShaderModule({ code: WGSL_SHADER });
  const pipeline = device.createRenderPipeline({
    layout: "auto",
    vertex: { module: shader, entryPoint: "vs_main" },
    fragment: {
      module: shader,
      entryPoint: "fs_main",
      targets: [{ format }],
    },
    primitive: { topology: "triangle-list" },
  });

  // 16 bytes (4 floats) — std140 minimum uniform buffer.
  const uniformBuffer = device.createBuffer({
    size: 16,
    usage: BUF_USAGE_UNIFORM | BUF_USAGE_COPY_DST,
  });
  const bindGroup = device.createBindGroup({
    layout: pipeline.getBindGroupLayout(0),
    entries: [{ binding: 0, resource: { buffer: uniformBuffer } }],
  });

  let running = true;
  const start = performance.now();
  const tick = (): void => {
    if (!running) return;
    const t = (performance.now() - start) / 1000;
    device.queue.writeBuffer(uniformBuffer, 0, new Float32Array([t, 0, 0, 0]));

    const encoder = device.createCommandEncoder();
    const view = context.getCurrentTexture().createView();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view,
          clearValue: { r: 0, g: 0, b: 0, a: 0 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.draw(3, 1, 0, 0);
    pass.end();
    device.queue.submit([encoder.finish()]);
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);

  return {
    stop(): void {
      running = false;
      uniformBuffer.destroy();
    },
  };
}

/** CSS fallback string for non-WebGPU contexts. */
export const SPECTRUM_SHIFT_FALLBACK_CSS = SPECTRUM_SHIFT_CSS;
