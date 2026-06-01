/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, nothing, PropertyValues, svg, SVGTemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";
import { styleMap } from "lit/directives/style-map.js";

import { ChartBase, colorAt, fmt } from "./chart-base.js";

/** A single pie slice descriptor. */
export interface PieDatum {
  value: number;
  label?: string;
  color?: string;
}

interface Arc {
  index: number;
  value: number;
  pct: number;
  startAngle: number;
  endAngle: number;
  midAngle: number;
  label: string;
  color: string;
}

const TAU = Math.PI * 2;

/** Cartesian point on a circle for an angle measured clockwise from 12 o'clock. */
function polar(cx: number, cy: number, r: number, angle: number): [number, number] {
  const a = angle - Math.PI / 2;
  return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
}

/** SVG arc/donut segment path between two angles. */
function arcPath(
  cx: number,
  cy: number,
  rOuter: number,
  rInner: number,
  start: number,
  end: number,
): string {
  const large = end - start > Math.PI ? 1 : 0;
  const [x0, y0] = polar(cx, cy, rOuter, start);
  const [x1, y1] = polar(cx, cy, rOuter, end);
  if (rInner <= 0) {
    return `M${cx},${cy} L${x0},${y0} A${rOuter},${rOuter} 0 ${large} 1 ${x1},${y1} Z`;
  }
  const [ix1, iy1] = polar(cx, cy, rInner, end);
  const [ix0, iy0] = polar(cx, cy, rInner, start);
  return (
    `M${x0},${y0} A${rOuter},${rOuter} 0 ${large} 1 ${x1},${y1} ` +
    `L${ix1},${iy1} A${rInner},${rInner} 0 ${large} 0 ${ix0},${iy0} Z`
  );
}

/**
 * A Material 3 pie / donut chart. Provide slices via the `data` property (or
 * a single-series `series`). Set `inner-radius` (0–1) for a donut, `show-labels`
 * for inline percentage labels, plus a legend and hover tooltip. Self-contained
 * on the `--md-sys-*` tokens.
 */
export class PieChart extends ChartBase {
  /** Slice data. Alternatively the first entry of `series` is used. */
  @property({ attribute: false }) data: PieDatum[] = [];

  /** Inner radius as a fraction of the outer radius (0 = pie, >0 = donut). */
  @property({ type: Number, attribute: "inner-radius" }) innerRadius = 0;

  /** Show percentage labels on each slice. */
  @property({ type: Boolean, attribute: "show-labels" }) showLabels = false;

  /** Padding angle between slices, in degrees. */
  @property({ type: Number, attribute: "pad-angle" }) padAngle = 0;

  @state() private hiddenSeries = new Set<number>();
  @state() private hover: Arc | null = null;

  override height = 300;

  /** Memoised geometry; recomputed only in `willUpdate`. */
  private layout: {
    cx: number;
    cy: number;
    rOuter: number;
    rInner: number;
    arcs: Arc[];
  } = { cx: 0, cy: 0, rOuter: 0, rInner: 0, arcs: [] };

  protected override willUpdate(changed: PropertyValues): void {
    super.willUpdate?.(changed);
    const w = this.renderWidth;
    const h = this.height;
    const rOuter = Math.max(0, Math.min(w, h) / 2 - 8);
    this.layout = {
      cx: w / 2,
      cy: h / 2,
      rOuter,
      rInner: Math.max(0, Math.min(0.95, this.innerRadius)) * rOuter,
      arcs: this.buildArcs(),
    };
  }

  private get slices(): PieDatum[] {
    if (this.data.length) {
      return this.data;
    }
    // Fall back to the first series, one slice per value.
    const s = this.series[0];
    if (s) {
      return s.data.map((v, i) => ({
        value: v,
        label: this.categories[i],
      }));
    }
    return [];
  }

  private buildArcs(): Arc[] {
    const slices = this.slices;
    const total = slices.reduce(
      (acc, d, i) => (this.hiddenSeries.has(i) ? acc : acc + Math.max(0, d.value)),
      0,
    );
    const arcs: Arc[] = [];
    if (total <= 0) {
      return arcs;
    }
    const pad = (this.padAngle * Math.PI) / 180;
    let angle = 0;
    slices.forEach((d, i) => {
      if (this.hiddenSeries.has(i)) {
        return;
      }
      const frac = Math.max(0, d.value) / total;
      const sweep = frac * TAU;
      const start = angle + pad / 2;
      const end = angle + sweep - pad / 2;
      arcs.push({
        index: i,
        value: d.value,
        pct: frac,
        startAngle: start,
        endAngle: Math.max(start, end),
        midAngle: (start + end) / 2,
        label: d.label ?? `Slice ${i + 1}`,
        color: d.color ?? colorAt(this.colors, undefined, i),
      });
      angle += sweep;
    });
    return arcs;
  }

  protected override render(): SVGTemplateResult {
    const w = this.renderWidth;
    const h = this.height;
    const { cx, cy, rOuter, rInner, arcs } = this.layout;

    return html`
      <div
        class="chart"
        role="img"
        aria-label=${this.describe()}
        @pointerleave=${this.onPointerLeave}
      >
        <svg viewBox="0 0 ${w} ${h}" width=${w} height=${h} aria-hidden="true">
          ${arcs.map((a) => {
            const hovered = this.hover?.index === a.index;
            const r = hovered ? rOuter + 4 : rOuter;
            return svg`<path
              d=${arcPath(cx, cy, r, rInner, a.startAngle, a.endAngle)}
              fill=${a.color}
              stroke="var(--md-sys-color-surface, #fef7ff)"
              stroke-width="2"
              @pointerenter=${() => {
                this.hover = a;
              }}
            ></path>`;
          })}
          ${this.showLabels ? this.renderSliceLabels(arcs, cx, cy, rOuter, rInner) : nothing}
        </svg>
        ${this.renderTooltip(w, cx, cy, rOuter)} ${this.legend ? this.renderLegend() : nothing}
        ${this.renderTable()}
      </div>
    ` as unknown as SVGTemplateResult;
  }

  private onPointerLeave(): void {
    this.hover = null;
  }

  private renderSliceLabels(
    arcs: Arc[],
    cx: number,
    cy: number,
    rOuter: number,
    rInner: number,
  ): SVGTemplateResult {
    const rLabel = rInner > 0 ? (rOuter + rInner) / 2 : rOuter * 0.62;
    return svg`${arcs.map((a) => {
      if (a.pct < 0.04) {
        return nothing;
      }
      const [x, y] = polar(cx, cy, rLabel, a.midAngle);
      return svg`<text
        class="value-label"
        x=${x}
        y=${y + 4}
        text-anchor="middle"
        fill="var(--md-sys-color-on-primary, #fff)"
      >${Math.round(a.pct * 100)}%</text>`;
    })}`;
  }

  private renderTooltip(
    w: number,
    cx: number,
    cy: number,
    rOuter: number,
  ): SVGTemplateResult | typeof nothing {
    if (!this.tooltip || !this.hover) {
      return nothing;
    }
    const a = this.hover;
    const [px, py] = polar(cx, cy, rOuter * 0.8, a.midAngle);
    const left = Math.min(Math.max(px, 8), w - 8);
    const top = Math.max(py - 12, 8);
    const flip = left > w * 0.6;
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
        <div class="tooltip-row">
          <span class="tooltip-swatch" style=${styleMap({ background: a.color })}></span>
          ${a.label}: ${fmt(a.value)} (${Math.round(a.pct * 100)}%)
        </div>
      </div>
    ` as unknown as SVGTemplateResult;
  }

  private renderLegend(): SVGTemplateResult | typeof nothing {
    const slices = this.slices;
    if (!slices.some((d) => d.label) && !this.categories.length) {
      return nothing;
    }
    return html`
      <div class="legend" role="list">
        ${slices.map((d, i) => {
          const color = d.color ?? colorAt(this.colors, undefined, i);
          const off = this.hiddenSeries.has(i);
          return html`<span
            class=${classMap({ "legend-item": true, "legend-item--off": off })}
            role="listitem"
            @click=${() => this.toggle(i)}
          >
            <span class="legend-swatch" style=${styleMap({ background: color })}></span>
            ${d.label ?? `Slice ${i + 1}`}
          </span>`;
        })}
      </div>
    ` as unknown as SVGTemplateResult;
  }

  private renderTable(): SVGTemplateResult {
    const slices = this.slices;
    return html`
      <table class="sr-only">
        <caption>
          ${this.describe()}
        </caption>
        <thead>
          <tr>
            <th>Slice</th>
            <th>Value</th>
          </tr>
        </thead>
        <tbody>
          ${slices.map(
            (d, i) => html`<tr>
              <th>${d.label ?? `Slice ${i + 1}`}</th>
              <td>${d.value}</td>
            </tr>`,
          )}
        </tbody>
      </table>
    ` as unknown as SVGTemplateResult;
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
