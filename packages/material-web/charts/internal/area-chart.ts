/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { property } from "lit/decorators.js";

import type { ChartSeries } from "./chart-base.js";
import { LineChart } from "./line-chart.js";

/**
 * A Material 3 area chart — a {@link LineChart} variant that fills the region
 * beneath each series. Set `stacked` to stack the series cumulatively (each
 * series sits on top of the previous one). Inherits the axes, grid, legend,
 * tooltip and a11y table from the line chart.
 */
export class AreaChart extends LineChart {
  /** Stack the series cumulatively rather than overlaying them. */
  @property({ type: Boolean }) stacked = false;

  /** Area charts fill by default. */
  override area = true;

  /**
   * Series with cumulative running totals applied when `stacked`. Derived in
   * `effectiveSeries()`; never mutates the public `series` property (which the
   * old implementation did, double-stacking on every re-render).
   */
  private get stackedSeries(): ChartSeries[] {
    const n = Math.max(0, ...this.series.map((s) => s.data.length));
    const totals: number[] = Array.from({ length: n }, () => 0);
    return this.series.map((s) => {
      const data = s.data.map((v, i) => {
        totals[i] += v;
        return totals[i];
      });
      return { ...s, data };
    });
  }

  /**
   * The series the line-chart renderer should draw — cumulative when stacked,
   * otherwise the raw series.
   */
  protected override get renderSeriesData(): ChartSeries[] {
    return this.stacked ? this.stackedSeries : this.series;
  }
}
