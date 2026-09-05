/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, nothing, PropertyValues, svg, SVGTemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";
import { styleMap } from "lit/directives/style-map.js";

import { ChartBase, colorAt, fmt, niceScale } from "./chart-base.js";

interface BarHover {
  x: number;
  y: number;
  value: number;
  seriesIndex: number;
  categoryIndex: number;
}

/** Memoised geometry computed once per update in `willUpdate`. */
interface BarLayout {
  w: number;
  h: number;
  plotW: number;
  plotH: number;
  n: number;
  min: number;
  max: number;
  step: number;
  ticks: number[];
  bandW: number;
  groupPad: number;
  barW: number;
  active: number[];
}

/**
 * A Material 3 bar chart. Supports grouped bars (default) and stacked bars
 * (set `stacked`, or give series a shared `stack` id), in vertical (default) or
 * horizontal orientation (`horizontal`). Renders SVG rects with X/Y axes, a
 * grid, a legend and a hover tooltip. Self-contained on the `--md-sys-*` tokens.
 */
export class BarChart extends ChartBase {
  /** Stack all series into a single column per category. */
  @property({ type: Boolean }) stacked = false;

  /** Render bars horizontally (categories on the Y axis, values on the X). */
  @property({ type: Boolean }) horizontal = false;

  /** Y-axis title. */
  @property({ attribute: "y-axis-label" }) yAxisLabel = "";

  @state() private hiddenSeries = new Set<number>();
  @state() private hover: BarHover | null = null;

  protected readonly margin = { top: 16, right: 16, bottom: 32, left: 44 };

  /** Memoised geometry; recomputed only in `willUpdate`. */
  private layout: BarLayout = {
    w: 0,
    h: 0,
    plotW: 0,
    plotH: 0,
    n: 0,
    min: 0,
    max: 1,
    step: 1,
    ticks: [],
    bandW: 0,
    groupPad: 0,
    barW: 0,
    active: [],
  };

  private get categoryCount(): number {
    let n = this.categories.length;
    for (const s of this.series) {
      n = Math.max(n, s.data.length);
    }
    return n;
  }

  private get activeSeries(): number[] {
    return this.series.map((_, i) => i).filter((i) => !this.hiddenSeries.has(i));
  }

  protected override willUpdate(changed: PropertyValues): void {
    super.willUpdate?.(changed);
    this.layout = this.computeLayout();
  }

  /** Compute the value-axis scale, accounting for stacking. */
  private computeScale(): { min: number; max: number; step: number } {
    let lo = 0;
    let hi = -Infinity;
    const n = this.categoryCount;
    if (this.stacked) {
      for (let ci = 0; ci < n; ci++) {
        let pos = 0;
        let neg = 0;
        this.activeSeries.forEach((si) => {
          const v = this.series[si].data[ci] ?? 0;
          if (v >= 0) pos += v;
          else neg += v;
        });
        hi = Math.max(hi, pos);
        lo = Math.min(lo, neg);
      }
    } else {
      this.activeSeries.forEach((si) => {
        for (const v of this.series[si].data) {
          hi = Math.max(hi, v);
          lo = Math.min(lo, v);
        }
      });
    }
    if (!isFinite(hi)) {
      hi = 1;
    }
    return niceScale(lo, hi);
  }

  private computeLayout(): BarLayout {
    const w = this.renderWidth;
    const h = this.height;
    const plotW = Math.max(0, w - this.margin.left - this.margin.right);
    const plotH = Math.max(0, h - this.margin.top - this.margin.bottom);
    const n = this.categoryCount;
    const { min, max, step } = this.computeScale();
    const ticks: number[] = [];
    for (let t = min; t <= max + step / 2; t += step) {
      ticks.push(Math.round(t * 1e6) / 1e6);
    }
    const active = this.activeSeries;
    // The "band" axis is X when vertical, Y when horizontal.
    const bandSpan = this.horizontal ? plotH : plotW;
    const bandW = n > 0 ? bandSpan / n : bandSpan;
    const groupPad = bandW * 0.18;
    const innerW = bandW - groupPad * 2;
    const barW = this.stacked ? innerW : active.length ? innerW / active.length : innerW;
    return {
      w,
      h,
      plotW,
      plotH,
      n,
      min,
      max,
      step,
      ticks,
      bandW,
      groupPad,
      barW,
      active,
    };
  }

  /** Map a value to a pixel coordinate along the value axis. */
  private vToPx(v: number): number {
    const { min, max, plotW, plotH } = this.layout;
    const frac = (v - min) / (max - min || 1);
    if (this.horizontal) {
      return this.margin.left + frac * plotW;
    }
    return this.margin.top + plotH - frac * plotH;
  }

  /** Start pixel of category band `ci` along the band axis. */
  private bandStart(ci: number): number {
    const { bandW } = this.layout;
    return this.horizontal ? this.margin.top + bandW * ci : this.margin.left + bandW * ci;
  }

  protected override render(): SVGTemplateResult {
    const { w, h } = this.layout;
    return html`
      <div
        class="chart"
        role="img"
        aria-label=${this.describe()}
        @pointerleave=${this.onPointerLeave}
      >
        <svg
          viewBox="0 0 ${w} ${h}"
          width=${w}
          height=${h}
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          ${this.renderGrid()} ${this.renderAxes()} ${this.renderBars()}
        </svg>
        ${this.renderTooltip()} ${this.legend ? this.renderLegend() : nothing} ${this.renderTable()}
      </div>
    ` as unknown as SVGTemplateResult;
  }

  private renderGrid(): SVGTemplateResult {
    const { ticks, plotW, plotH } = this.layout;
    return svg`<g class="grid">
      ${ticks.map((t) => {
        const p = this.vToPx(t);
        return this.horizontal
          ? svg`<line
              x1=${p}
              x2=${p}
              y1=${this.margin.top}
              y2=${this.margin.top + plotH}
            ></line>`
          : svg`<line
              x1=${this.margin.left}
              x2=${this.margin.left + plotW}
              y1=${p}
              y2=${p}
            ></line>`;
      })}
    </g>`;
  }

  private renderAxes(): SVGTemplateResult {
    const { ticks, plotW, plotH, n, bandW } = this.layout;
    const baseV = this.vToPx(0);
    const categoryLabels = svg`${Array.from({ length: n }, (_, i) => i).map((i) => {
      const center = this.bandStart(i) + bandW / 2;
      return this.horizontal
        ? svg`<text
              class="tick-label"
              x=${this.margin.left - 6}
              y=${center + 4}
              text-anchor="end"
            >${this.categories[i] ?? i + 1}</text>`
        : svg`<text
              class="tick-label"
              x=${center}
              y=${this.margin.top + plotH + 16}
              text-anchor="middle"
            >${this.categories[i] ?? i + 1}</text>`;
    })}`;
    const valueLabels = svg`${ticks.map((t) => {
      const p = this.vToPx(t);
      return this.horizontal
        ? svg`<text
              class="tick-label"
              x=${p}
              y=${this.margin.top + plotH + 16}
              text-anchor="middle"
            >${fmt(t)}</text>`
        : svg`<text
              class="tick-label"
              x=${this.margin.left - 6}
              y=${p + 4}
              text-anchor="end"
            >${fmt(t)}</text>`;
    })}`;
    return svg`<g class="axis">
      <line
        class="domain"
        x1=${this.margin.left}
        y1=${this.margin.top}
        x2=${this.margin.left}
        y2=${this.margin.top + plotH}
      ></line>
      <line
        class="domain"
        x1=${this.margin.left}
        y1=${this.horizontal ? this.margin.top + plotH : baseV}
        x2=${this.margin.left + plotW}
        y2=${this.horizontal ? this.margin.top + plotH : baseV}
      ></line>
      ${valueLabels}
      ${categoryLabels}
      ${
        this.yAxisLabel
          ? svg`<text
              class="axis-title"
              transform="translate(12 ${this.margin.top + plotH / 2}) rotate(-90)"
              text-anchor="middle"
            >${this.yAxisLabel}</text>`
          : nothing
      }
    </g>`;
  }

  private renderBars(): SVGTemplateResult {
    const { n, groupPad, barW, active } = this.layout;
    const baseV = this.vToPx(0);
    return svg`${Array.from({ length: n }, (_, ci) => ci).map((ci) => {
      const bandOrigin = this.bandStart(ci) + groupPad;
      let posAcc = 0;
      let negAcc = 0;
      return svg`${active.map((si, k) => {
        const v = this.series[si].data[ci] ?? 0;
        const color = colorAt(this.colors, this.series[si], si);
        const geom = this.barGeometry(v, bandOrigin, k, barW, baseV, posAcc, negAcc);
        if (this.stacked) {
          if (v >= 0) posAcc += v;
          else negAcc += v;
        }
        const hovered =
          this.hover && this.hover.seriesIndex === si && this.hover.categoryIndex === ci;
        return svg`<rect
          x=${geom.x}
          y=${geom.y}
          width=${Math.max(0, geom.width)}
          height=${Math.max(0, geom.height)}
          rx="2"
          fill=${color}
          fill-opacity=${hovered ? 1 : 0.92}
          @pointerenter=${() => {
            this.hover = {
              x: geom.tipX,
              y: geom.tipY,
              value: v,
              seriesIndex: si,
              categoryIndex: ci,
            };
          }}
        ></rect>`;
      })}`;
    })}`;
  }

  /** Rect geometry for a single bar, handling orientation + stacking. */
  private barGeometry(
    v: number,
    bandOrigin: number,
    k: number,
    barW: number,
    baseV: number,
    posAcc: number,
    negAcc: number,
  ): {
    x: number;
    y: number;
    width: number;
    height: number;
    tipX: number;
    tipY: number;
  } {
    if (this.horizontal) {
      const bandPos = this.stacked ? bandOrigin : bandOrigin + barW * k;
      if (this.stacked) {
        const start = this.vToPx(v >= 0 ? posAcc : negAcc + v);
        const endPx = this.vToPx(v >= 0 ? posAcc + v : negAcc);
        const x = Math.min(start, endPx);
        const width = Math.abs(endPx - start);
        return {
          x,
          y: bandPos,
          width,
          height: Math.max(0, barW - 1),
          tipX: x + width,
          tipY: bandPos + barW / 2,
        };
      }
      const end = this.vToPx(v);
      const x = Math.min(baseV, end);
      const width = Math.abs(end - baseV);
      return {
        x,
        y: bandPos,
        width,
        height: Math.max(0, barW - 1),
        tipX: x + width,
        tipY: bandPos + barW / 2,
      };
    }
    // Vertical.
    const bandPos = this.stacked ? bandOrigin : bandOrigin + barW * k;
    if (this.stacked) {
      if (v >= 0) {
        const top = this.vToPx(posAcc + v);
        return {
          x: bandPos,
          y: top,
          width: Math.max(0, barW - 1),
          height: this.vToPx(posAcc) - top,
          tipX: bandPos + barW / 2,
          tipY: top,
        };
      }
      const bottom = this.vToPx(negAcc + v);
      const y = this.vToPx(negAcc);
      return {
        x: bandPos,
        y,
        width: Math.max(0, barW - 1),
        height: bottom - y,
        tipX: bandPos + barW / 2,
        tipY: y,
      };
    }
    const top = this.vToPx(v);
    const y = Math.min(top, baseV);
    return {
      x: bandPos,
      y,
      width: Math.max(0, barW - 1),
      height: Math.abs(baseV - top),
      tipX: bandPos + barW / 2,
      tipY: y,
    };
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
            @click=${() => this.toggle(i)}
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
    const n = this.categoryCount;
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
          ${Array.from({ length: n }, (_, ci) => ci).map(
            (ci) => html`<tr>
              <th>${this.categories[ci] ?? ci + 1}</th>
              ${this.series.map((s) => html`<td>${s.data[ci] ?? ""}</td>`)}
            </tr>`,
          )}
        </tbody>
      </table>
    ` as unknown as SVGTemplateResult;
  }

  private onPointerLeave(): void {
    this.hover = null;
  }

  private toggle(i: number): void {
    const next = new Set(this.hiddenSeries);
    if (next.has(i)) {
      next.delete(i);
    } else {
      next.add(i);
    }
    this.hiddenSeries = next;
  }
}
