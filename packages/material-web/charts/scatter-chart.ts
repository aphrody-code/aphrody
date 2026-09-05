/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { ScatterChart } from "./internal/scatter-chart.js";
import { styles } from "./internal/scatter-chart-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-scatter-chart": MdScatterChart;
  }
}

/**
 * @summary A Material 3 scatter chart for one or more {x, y} series.
 *
 * @description
 * Renders each series of `{x, y}` points as SVG circles against numeric X/Y
 * axes, with a grid, a clickable legend and a hover tooltip. Set data via the
 * `.scatterSeries` property (`{data: {x, y, size?}[], label, color}[]`). Point
 * radius is configurable per chart (`point-size`), per series (`size`) or per
 * point (`size`). Colours default to the M3 tonal palette; override with
 * `.colors`. Responsive via a ResizeObserver, accessible via `role="img"` plus
 * a hidden data table.
 *
 * ```html
 * <md-scatter-chart point-size="5"></md-scatter-chart>
 * <script>
 *   chart.scatterSeries = [
 *     {label: 'A', data: [{x: 1, y: 2}, {x: 3, y: 5}, {x: 4, y: 4}]},
 *     {label: 'B', data: [{x: 2, y: 1}, {x: 3, y: 3}, {x: 5, y: 6}]},
 *   ];
 * </script>
 * ```
 *
 * @final
 */
@customElement("md-scatter-chart")
export class MdScatterChart extends ScatterChart {
  static override styles: CSSResultOrNative[] = styles;
}
