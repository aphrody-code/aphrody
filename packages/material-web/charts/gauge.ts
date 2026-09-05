/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Gauge } from "./internal/gauge.js";
import { styles } from "./internal/gauge-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-gauge": MdGauge;
  }
}

/**
 * @summary A Material 3 radial gauge (0–100 by default).
 *
 * @description
 * Renders `value` within `[min, max]` as an arc over a track, with an optional
 * centre readout, unit suffix and coloured threshold `bands`. The `sweep`
 * (degrees) controls the dial opening. Self-contained on the `--md-sys-*`
 * tokens and exposed with the `meter` ARIA semantics via `role="img"` plus
 * `aria-valuenow/min/max`.
 *
 * ```html
 * <md-gauge value="72" unit="%"></md-gauge>
 * ```
 *
 * @final
 */
@customElement("md-gauge")
export class MdGauge extends Gauge {
  static override styles: CSSResultOrNative[] = styles;
}
