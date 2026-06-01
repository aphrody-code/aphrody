/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { WebGpuCanvas } from "./internal/webgpu-canvas.js";
import { styles } from "./internal/webgpu-canvas-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-webgpu-canvas": MdWebgpuCanvas;
  }
}

/**
 * @summary An animated Material/Gemini brand surface rendered on WebGPU, with a
 * CSS-gradient fallback.
 *
 * @description
 * Reproduces the design.google motion language. Use it as a hero backdrop,
 * avatar ring, or streaming shimmer.
 *
 * ```html
 * <md-webgpu-canvas effect="spectrum" style="height:240px"></md-webgpu-canvas>
 * <md-webgpu-canvas effect="sparkle" intensity="0.8"></md-webgpu-canvas>
 * <md-webgpu-canvas effect="glimmer" colors="#4285F4,#9168C0,#EC4899"></md-webgpu-canvas>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-webgpu-canvas")
export class MdWebgpuCanvas extends WebGpuCanvas {
  static override styles: CSSResultOrNative[] = [styles];
}
