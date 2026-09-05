/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, nothing, PropertyValues, svg, SVGTemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";
import { styleMap } from "lit/directives/style-map.js";

import type { ChartSeries } from "./chart-base.js";
import { ChartBase, colorAt, fmt, linePath, niceScale } from "./chart-base.js";

interface PlotPoint {
  x: number;
  y: number;
  value: number;
  seriesIndex: number;
  categoryIndex: number;
}

/** Memoised geometry computed once per update in `willUpdate`. */
interface LineLayout {
  w: number;
  h: number;
  plotW: number;
  plotH: number;
  n: number;
  min: number;
  max: number;
  step: number;
  ticks: number[];
}

/**
 * A Material 3 line chart. Renders multiple series as SVG polylines (straight
 * or smooth), with X/Y axes, a grid, optional point markers, a legend and a
 * hover tooltip. Subclassed by the area chart.
 *
 * The chart is self-contained (no external chart library) and draws everything
 * with inline SVG so it can live inside a shadow root and theme from the
 * `--md-sys-*` tokens.
 */
export class LineChart extends ChartBase {
  /** Render every series as a smooth curve unless the series overrides it. */
  @property({ type: Boolean }) smooth = false;

  /** Show point markers on every series unless the series overrides it. */
  @property({ type: Boolean, attribute: "show-markers" }) showMarkers = false;

  /** Fill the area beneath each line (area-chart behaviour). */
  @property({ type: Boolean }) area = false;

  /** Y-axis title. */
  @property({ attribute: "y-axis-label" }) yAxisLabel = "";

  /** Hidden series indices (toggled via the legend). */
  @state() private hiddenSeries = new Set<number>();

  @state() private hover: PlotPoint | null = null;

  /** Layout margins. */
  protected readonly margin = { top: 16, right: 16, bottom: 32, left: 44 };

  /**
   * Memoised geometry. Recomputed in `willUpdate` whenever inputs change so
   * `render()` stays pure and free of scale math.
   */
  protected layout: LineLayout = {
    w: 0,
    h: 0,
    plotW: 0,
    plotH: 0,
    n: 0,
    min: 0,
    max: 1,
    step: 1,
    ticks: [],
  };

  /**
   * The series actually drawn. Subclasses (area chart) override this to supply
   * transformed data (e.g. cumulative stacks) without mutating `series`.
   */
  protected get renderSeriesData(): ChartSeries[] {
    return this.series;
  }

  protected override willUpdate(changed: PropertyValues): void {
    super.willUpdate?.(changed);
    this.layout = this.computeLayout();
  }

  /** Map a value to a Y pixel using the current layout. */
  protected yToPx(v: number): number {
    const { min, max, plotH } = this.layout;
    return this.margin.top + plotH - ((v - min) / (max - min || 1)) * plotH;
  }

  /** Map a category index to an X pixel using the current layout. */
  protected xToPx(i: number): number {
    const { n, plotW } = this.layout;
    return this.margin.left + (n <= 1 ? plotW / 2 : (i / (n - 1)) * plotW);
  }

  private computeLayout(): LineLayout {
    const w = this.renderWidth;
    const h = this.height;
    const plotW = Math.max(0, w - this.margin.left - this.margin.right);
    const plotH = Math.max(0, h - this.margin.top - this.margin.bottom);
    const n = this.maxCategoryCount();
    const { min, max, step } = this.computeYScale();
    const ticks: number[] = [];
    for (let t = min; t <= max + step / 2; t += step) {
      ticks.push(Math.round(t * 1e6) / 1e6);
    }
    return { w, h, plotW, plotH, n, min, max, step, ticks };
  }

  protected override render(): SVGTemplateResult {
    const { w, h } = this.layout;
    return html`
      <div
        class="chart"
        role="img"
        aria-label=${this.describe()}
        @pointermove=${this.onPointerMove}
        @pointerleave=${this.onPointerLeave}
      >
        <svg
          viewBox="0 0 ${w} ${h}"
          width=${w}
          height=${h}
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          ${this.renderGrid()} ${this.renderAxes()} ${this.renderSeries()} ${this.renderMarkers()}
        </svg>
        ${this.renderTooltip()} ${this.legend ? this.renderLegend() : nothing} ${this.renderTable()}
      </div>
    ` as unknown as SVGTemplateResult;
  }

  protected maxCategoryCount(): number {
    let n = this.categories.length;
    for (const s of this.renderSeriesData) {
      n = Math.max(n, s.data.length);
    }
    return n;
  }

  protected computeYScale(): { min: number; max: number; step: number } {
    let lo = Infinity;
    let hi = -Infinity;
    this.renderSeriesData.forEach((s, i) => {
      if (this.hiddenSeries.has(i)) {
        return;
      }
      for (const v of s.data) {
        if (v < lo) lo = v;
        if (v > hi) hi = v;
      }
    });
    if (!isFinite(lo) || !isFinite(hi)) {
      lo = 0;
      hi = 1;
    }
    // Always include the zero baseline for line/area charts.
    lo = Math.min(lo, 0);
    hi = Math.max(hi, 0);
    return niceScale(lo, hi);
  }

  private renderGrid(): SVGTemplateResult {
    const { ticks, plotW } = this.layout;
    return svg`
      <g class="grid">
        ${ticks.map(
          (t) => svg`
          <line
            x1=${this.margin.left}
            x2=${this.margin.left + plotW}
            y1=${this.yToPx(t)}
            y2=${this.yToPx(t)}
          ></line>`,
        )}
      </g>
    `;
  }

  private renderAxes(): SVGTemplateResult {
    const { ticks, plotH, n } = this.layout;
    const baseY = this.margin.top + plotH;
    return svg`
      <g class="axis">
        <line
          class="domain"
          x1=${this.margin.left}
          y1=${this.margin.top}
          x2=${this.margin.left}
          y2=${baseY}
        ></line>
        <line
          class="domain"
          x1=${this.margin.left}
          y1=${baseY}
          x2=${this.renderWidth - this.margin.right}
          y2=${baseY}
        ></line>
        ${ticks.map(
          (t) => svg`
          <text
            class="tick-label"
            x=${this.margin.left - 6}
            y=${this.yToPx(t) + 4}
            text-anchor="end"
          >${fmt(t)}</text>`,
        )}
        ${this.renderXLabels(baseY, n)}
        ${
          this.yAxisLabel
            ? svg`<text
                class="axis-title"
                transform="translate(12 ${this.margin.top + plotH / 2}) rotate(-90)"
                text-anchor="middle"
              >${this.yAxisLabel}</text>`
            : nothing
        }
      </g>
    `;
  }

  private renderXLabels(baseY: number, n: number): SVGTemplateResult {
    const labels = this.categories;
    const step = Math.max(1, Math.ceil(n / 12));
    return svg`${Array.from({ length: n }, (_, i) => i)
      .filter((i) => i % step === 0)
      .map(
        (i) => svg`
          <text
            class="tick-label"
            x=${this.xToPx(i)}
            y=${baseY + 16}
            text-anchor="middle"
          >${labels[i] ?? i + 1}</text>`,
      )}`;
  }

  private renderSeries(): SVGTemplateResult {
    const baseY = this.margin.top + this.layout.plotH;
    return svg`${this.renderSeriesData.map((s, si) => {
      if (this.hiddenSeries.has(si)) {
        return nothing;
      }
      const color = colorAt(this.colors, s, si);
      const pts = s.data.map((v, ci) => [this.xToPx(ci), this.yToPx(v)] as [number, number]);
      const smooth = s.curve ?? this.smooth;
      const d = linePath(pts, smooth);
      let areaTpl: SVGTemplateResult | typeof nothing = nothing;
      if (this.area && pts.length) {
        const areaD = d + ` L${pts[pts.length - 1][0]},${baseY} L${pts[0][0]},${baseY} Z`;
        areaTpl = svg`<path
          d=${areaD}
          fill=${color}
          fill-opacity="0.18"
          stroke="none"
        ></path>`;
      }
      return svg`
        ${areaTpl}
        <path class="series-line" d=${d} stroke=${color}></path>
      `;
    })}`;
  }

  private renderMarkers(): SVGTemplateResult {
    return svg`${this.renderSeriesData.map((s, si) => {
      if (this.hiddenSeries.has(si)) {
        return nothing;
      }
      const show = s.showMarkers ?? this.showMarkers;
      if (!show && !(this.hover && this.hover.seriesIndex === si)) {
        return nothing;
      }
      const color = colorAt(this.colors, s, si);
      return svg`${s.data.map((v, ci) => {
        const isHover =
          this.hover && this.hover.seriesIndex === si && this.hover.categoryIndex === ci;
        if (!show && !isHover) {
          return nothing;
        }
        return svg`<circle
          class="series-marker"
          cx=${this.xToPx(ci)}
          cy=${this.yToPx(v)}
          r=${isHover ? 5 : 3}
          fill=${color}
        ></circle>`;
      })}`;
    })}`;
  }

  private renderLegend(): SVGTemplateResult | typeof nothing {
    if (!this.series.some((s) => s.label)) {
      return nothing;
    }
    return html`
      <div class="legend" role="list">
        ${this.series.map((s, i) => {
          const color = colorAt(this.colors, s, i);
          const off = this.hiddenSeries.has(i);
          return html`<span
            class=${classMap({ "legend-item": true, "legend-item--off": off })}
            role="listitem"
            @click=${() => this.toggleSeries(i)}
          >
            <span class="legend-swatch" style=${styleMap({ background: color })}></span>
            ${s.label ?? `Series ${i + 1}`}
          </span>`;
        })}
      </div>
    ` as unknown as SVGTemplateResult;
  }

  private renderTooltip(): SVGTemplateResult | typeof nothing {
    if (!this.tooltip || !this.hover) {
      return nothing;
    }
    const { w } = this.layout;
    const p = this.hover;
    const s = this.series[p.seriesIndex];
    const color = colorAt(this.colors, s, p.seriesIndex);
    const cat = this.categories[p.categoryIndex] ?? `#${p.categoryIndex + 1}`;
    const left = Math.min(Math.max(p.x, 8), w - 8);
    const top = Math.max(p.y - 12, 8);
    const flip = left > w * 0.7;
    return html`
      <div
        class="tooltip"
        data-visible
        style=${styleMap({
          left: `${left}px`,
          top: `${top}px`,
          transform: `translate(${flip ? "-100%" : "0"}, -100%)`,
        })}
      >
        <div class="tooltip-title">${cat}</div>
        <div class="tooltip-row">
          <span class="tooltip-swatch" style=${styleMap({ background: color })}></span>
          ${s.label ?? `Series ${p.seriesIndex + 1}`}: ${fmt(p.value)}
        </div>
      </div>
    ` as unknown as SVGTemplateResult;
  }

  private renderTable(): SVGTemplateResult {
    return html`
      <table class="sr-only">
        <caption>
          ${this.describe()}
        </caption>
        <thead>
          <tr>
            <th>Category</th>
            ${this.series.map((s, i) => html`<th>${s.label ?? `Series ${i + 1}`}</th>`)}
          </tr>
        </thead>
        <tbody>
          ${Array.from({ length: this.maxCategoryCount() }, (_, ci) => ci).map(
            (ci) => html`<tr>
              <th>${this.categories[ci] ?? ci + 1}</th>
              ${this.series.map((s) => html`<td>${s.data[ci] ?? ""}</td>`)}
            </tr>`,
          )}
        </tbody>
      </table>
    ` as unknown as SVGTemplateResult;
  }

  private onPointerMove(event: PointerEvent): void {
    if (!this.tooltip || !this.series.length) {
      return;
    }
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const px = event.clientX - rect.left;
    const py = event.clientY - rect.top;
    const { n, plotW } = this.layout;

    // Nearest category index by X.
    let ci = 0;
    if (n > 1) {
      ci = Math.round(((px - this.margin.left) / plotW) * (n - 1));
      ci = Math.max(0, Math.min(n - 1, ci));
    }
    // Nearest series by Y at that category. Use the rendered (possibly
    // stacked) data for the marker position but report the raw series value.
    const drawn = this.renderSeriesData;
    let best: PlotPoint | null = null;
    let bestDist = Infinity;
    drawn.forEach((s, si) => {
      if (this.hiddenSeries.has(si) || s.data[ci] === undefined) {
        return;
      }
      const sy = this.yToPx(s.data[ci]);
      const dist = Math.abs(sy - py);
      if (dist < bestDist) {
        bestDist = dist;
        best = {
          x: this.xToPx(ci),
          y: sy,
          value: this.series[si]?.data[ci] ?? s.data[ci],
          seriesIndex: si,
          categoryIndex: ci,
        };
      }
    });
    this.hover = best;
  }

  private onPointerLeave(): void {
    this.hover = null;
  }

  private toggleSeries(i: number): void {
    const next = new Set(this.hiddenSeries);
    if (next.has(i)) {
      next.delete(i);
    } else {
      next.add(i);
    }
    this.hiddenSeries = next;
  }
}
