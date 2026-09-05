/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { RadarChart } from "./internal/radar-chart.js";
import { styles } from "./internal/radar-chart-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-radar-chart": MdRadarChart;
  }
}

/**
 * @summary A Material 3 radar / spider chart for one or more series.
 *
 * @description
 * Lays out each `.categories` entry as a spoke evenly around a circle and draws
 * each series as a closed polygon over the spokes, with a concentric grid,
 * value tick rings, vertex markers, a clickable legend and a hover tooltip. Set
 * data via the `.radarSeries` property (`{data: number[], label, color}[]`) and
 * axes via `.categories`. Colours default to the M3 tonal palette; override
 * with `.colors`. Responsive via a ResizeObserver, accessible via `role="img"`
 * plus a hidden data table.
 *
 * ```html
 * <md-radar-chart></md-radar-chart>
 * <script>
 *   chart.categories = ['Speed', 'Power', 'Range', 'Agility', 'Defence'];
 *   chart.radarSeries = [
 *     {label: 'Model A', data: [80, 60, 70, 50, 90]},
 *     {label: 'Model B', data: [50, 90, 60, 80, 40]},
 *   ];
 * </script>
 * ```
 *
 * @final
 */
@customElement("md-radar-chart")
export class MdRadarChart extends RadarChart {
  static override styles: CSSResultOrNative[] = styles;
}
