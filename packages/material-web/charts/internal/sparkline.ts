/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, nothing, PropertyValues, svg, SVGTemplateResult } from "lit";
import { property } from "lit/decorators.js";

import { ChartBase, colorAt, linePath } from "./chart-base.js";

/**
 * A Material 3 sparkline: a compact, axis-free inline trend line for a single
 * data set. Renders an SVG polyline (optionally smooth / filled) with an
 * optional end-point marker. Inherits `data` from the first series or the
 * `values` property. Self-contained on the `--md-sys-*` tokens.
 */
export class Sparkline extends ChartBase {
  /** Data values (alternative to `series`). */
  @property({ attribute: false }) values: number[] = [];

  /** Render as a smooth curve. */
  @property({ type: Boolean }) smooth = false;

  /** Fill the area beneath the line. */
  @property({ type: Boolean }) area = false;

  /** Render as bars rather than a line. */
  @property({ type: Boolean }) bars = false;

  /** Show a marker on the last point. */
  @property({ type: Boolean, attribute: "show-endpoint" }) showEndpoint = true;

  /** Sparklines are compact by default. */
  override height = 40;
  override width = 120;
  override legend = false;
  override tooltip = false;

  /** Memoised geometry; recomputed only in `willUpdate`. */
  private layout: {
    w: number;
    h: number;
    pad: number;
    data: number[];
    innerW: number;
    innerH: number;
    lo: number;
    range: number;
  } = { w: 0, h: 0, pad: 3, data: [], innerW: 1, innerH: 1, lo: 0, range: 1 };

  private get points(): number[] {
    if (this.values.length) {
      return this.values;
    }
    return this.series[0]?.data ?? [];
  }

  protected override willUpdate(changed: PropertyValues): void {
    super.willUpdate?.(changed);
    const w = this.renderWidth || 120;
    const h = this.height;
    const pad = 3;
    const data = this.points;
    const lo = data.length ? Math.min(...data) : 0;
    const hi = data.length ? Math.max(...data) : 1;
    this.layout = {
      w,
      h,
      pad,
      data,
      innerW: Math.max(1, w - pad * 2),
      innerH: Math.max(1, h - pad * 2),
      lo,
      range: hi - lo || 1,
    };
  }

  /** Map a data index to an X pixel. */
  private xAt(i: number): number {
    const { pad, data, innerW } = this.layout;
    return pad + (data.length <= 1 ? innerW / 2 : (i / (data.length - 1)) * innerW);
  }

  /** Map a value to a Y pixel. */
  private yAt(v: number): number {
    const { pad, innerH, lo, range } = this.layout;
    return pad + innerH - ((v - lo) / range) * innerH;
  }

  protected override render(): SVGTemplateResult {
    const { w, h, pad, data } = this.layout;
    const color = colorAt(this.colors, this.series[0], 0);

    if (!data.length) {
      return html`<div role="img" aria-label=${this.describe()}>
        <svg viewBox="0 0 ${w} ${h}" width=${w} height=${h} aria-hidden="true"></svg>
      </div>` as unknown as SVGTemplateResult;
    }

    return html`
      <div role="img" aria-label=${this.describe()}>
        <svg
          viewBox="0 0 ${w} ${h}"
          width=${w}
          height=${h}
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          ${this.bars
            ? this.renderBars(data, h - pad, color)
            : this.renderLine(data, h - pad, color)}
        </svg>
      </div>
    ` as unknown as SVGTemplateResult;
  }

  private renderLine(data: number[], baseline: number, color: string): SVGTemplateResult {
    const pts = data.map((v, i) => [this.xAt(i), this.yAt(v)] as [number, number]);
    const d = linePath(pts, this.smooth);
    const last = pts[pts.length - 1];
    return svg`
      ${
        this.area
          ? svg`<path
              d=${d + ` L${last[0]},${baseline} L${pts[0][0]},${baseline} Z`}
              fill=${color}
              fill-opacity="0.2"
              stroke="none"
            ></path>`
          : nothing
      }
      <path class="series-line" d=${d} stroke=${color}></path>
      ${
        this.showEndpoint
          ? svg`<circle
              class="series-marker"
              cx=${last[0]}
              cy=${last[1]}
              r="2.5"
              fill=${color}
            ></circle>`
          : nothing
      }
    `;
  }

  private renderBars(data: number[], baseline: number, color: string): SVGTemplateResult {
    const bw = Math.max(1, (this.layout.innerW / data.length) * 0.7);
    return svg`${data.map((v, i) => {
      const y = this.yAt(v);
      return svg`<rect
        x=${this.xAt(i) - bw / 2}
        y=${y}
        width=${bw}
        height=${Math.max(0, baseline - y)}
        rx="1"
        fill=${color}
      ></rect>`;
    })}`;
  }

  protected override describe(): string {
    if (this.ariaLabelText) {
      return this.ariaLabelText;
    }
    const data = this.points;
    if (!data.length) {
      return "Sparkline.";
    }
    return `Sparkline trend from ${data[0]} to ${data[data.length - 1]}.`;
  }
}
