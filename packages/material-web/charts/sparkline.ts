/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Sparkline } from "./internal/sparkline.js";
import { styles } from "./internal/sparkline-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-sparkline": MdSparkline;
  }
}

/**
 * @summary A Material 3 sparkline — a compact, axis-free inline trend.
 *
 * @description
 * Renders a single data set (`.values` or the first `.series`) as a tiny inline
 * line, smooth curve, area or bars, with an optional end-point marker. Sized
 * compactly by default (120×40). Self-contained on the `--md-sys-*` tokens.
 *
 * ```html
 * <md-sparkline area .values=${[3, 5, 2, 8, 6, 9]}></md-sparkline>
 * ```
 *
 * @final
 */
@customElement("md-sparkline")
export class MdSparkline extends Sparkline {
  static override styles: CSSResultOrNative[] = styles;
}
