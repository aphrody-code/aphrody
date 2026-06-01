/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview Minimal WebGPU type surface — only the members used by
 * `<md-webgpu-canvas>`. Avoids a hard dependency on `@webgpu/types` while
 * keeping the rendering code typed. Complex descriptor objects are accepted as
 * `unknown` since their shape is validated by the browser at runtime.
 */

/* eslint-disable @typescript-eslint/no-explicit-any */

/** Buffer usage bit flags (from the WebGPU spec). */
export const BUFFER_USAGE_UNIFORM = 0x40;
/** Allows `queue.writeBuffer` into the buffer. */
export const BUFFER_USAGE_COPY_DST = 0x08;

/** `navigator.gpu` entry point. */
export interface GPU {
  requestAdapter(options?: unknown): Promise<GPUAdapter | null>;
  getPreferredCanvasFormat(): string;
}

export interface GPUAdapter {
  requestDevice(descriptor?: unknown): Promise<GPUDevice>;
}

export interface GPUQueue {
  writeBuffer(buffer: GPUBuffer, offset: number, data: BufferSource): void;
  submit(buffers: GPUCommandBuffer[]): void;
}

export interface GPUDevice {
  readonly queue: GPUQueue;
  createShaderModule(descriptor: { code: string }): GPUShaderModule;
  createRenderPipeline(descriptor: unknown): GPURenderPipeline;
  createBuffer(descriptor: { size: number; usage: number }): GPUBuffer;
  createBindGroup(descriptor: unknown): GPUBindGroup;
  createCommandEncoder(): GPUCommandEncoder;
  destroy(): void;
}

export interface GPUShaderModule {
  readonly __brand?: "GPUShaderModule";
}

export interface GPURenderPipeline {
  getBindGroupLayout(index: number): unknown;
}

export interface GPUBuffer {
  destroy(): void;
}

export interface GPUBindGroup {
  readonly __brand?: "GPUBindGroup";
}

export interface GPUTextureView {
  readonly __brand?: "GPUTextureView";
}

export interface GPUTexture {
  createView(): GPUTextureView;
}

export interface GPURenderPassEncoder {
  setPipeline(pipeline: GPURenderPipeline): void;
  setBindGroup(index: number, group: GPUBindGroup): void;
  draw(vertexCount: number): void;
  end(): void;
}

export interface GPUCommandBuffer {
  readonly __brand?: "GPUCommandBuffer";
}

export interface GPUCommandEncoder {
  beginRenderPass(descriptor: unknown): GPURenderPassEncoder;
  finish(): GPUCommandBuffer;
}

export interface GPUCanvasContext {
  configure(descriptor: unknown): void;
  getCurrentTexture(): GPUTexture;
}

/** Read `navigator.gpu` in a typed way, returning `undefined` when absent. */
export function getGPU(): GPU | undefined {
  if (typeof navigator === "undefined") {
    return undefined;
  }
  return (navigator as unknown as { gpu?: GPU }).gpu;
}

/** Get the WebGPU canvas context, or `null` if unsupported. */
export function getWebGPUContext(canvas: HTMLCanvasElement): GPUCanvasContext | null {
  return canvas.getContext("webgpu") as unknown as GPUCanvasContext | null;
}
