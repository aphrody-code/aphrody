/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, nothing, PropertyValues, svg, SVGTemplateResult } from "lit";
import { property } from "lit/decorators.js";

import { ChartBase, fmt } from "./chart-base.js";

/** A coloured threshold band on the gauge track. */
export interface GaugeBand {
  /** Upper bound of the band (in value units). */
  to: number;
  /** Band colour (CSS value). */
  color: string;
}

const DEG = Math.PI / 180;

function polar(cx: number, cy: number, r: number, deg: number): [number, number] {
  const a = deg * DEG;
  return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
}

function arc(cx: number, cy: number, r: number, startDeg: number, endDeg: number): string {
  const [x0, y0] = polar(cx, cy, r, startDeg);
  const [x1, y1] = polar(cx, cy, r, endDeg);
  const large = Math.abs(endDeg - startDeg) > 180 ? 1 : 0;
  const sweep = endDeg > startDeg ? 1 : 0;
  return `M${x0},${y0} A${r},${r} 0 ${large} ${sweep} ${x1},${y1}`;
}

/**
 * A Material 3 radial gauge. Renders a 0–100 (configurable) value as an arc
 * with a track, a coloured value arc, optional threshold bands and a centre
 * readout. Self-contained on the `--md-sys-*` tokens.
 */
export class Gauge extends ChartBase {
  /** The current value. */
  @property({ type: Number }) value = 0;

  /** Minimum of the range. */
  @property({ type: Number }) min = 0;

  /** Maximum of the range. */
  @property({ type: Number }) max = 100;

  /** Total sweep of the gauge in degrees (e.g. 270 for a 3/4 dial). */
  @property({ type: Number }) sweep = 270;

  /** Arc thickness in px. */
  @property({ type: Number }) thickness = 16;

  /** Optional unit suffix shown after the value (e.g. "%"). */
  @property({ attribute: "unit" }) unit = "";

  /** Hide the centre numeric readout. */
  @property({ type: Boolean, attribute: "hide-value" }) hideValue = false;

  /** Optional coloured threshold bands along the track. */
  @property({ attribute: false }) bands?: GaugeBand[];

  override height = 200;
  override legend = false;
  override tooltip = false;

  /** Memoised geometry; recomputed only in `willUpdate`. */
  private layout: {
    w: number;
    h: number;
    size: number;
    cx: number;
    cy: number;
    r: number;
    start: number;
    end: number;
    valueEnd: number;
  } = {
    w: 0,
    h: 0,
    size: 0,
    cx: 0,
    cy: 0,
    r: 0,
    start: 0,
    end: 0,
    valueEnd: 0,
  };

  private get fraction(): number {
    const f = (this.value - this.min) / (this.max - this.min || 1);
    return Math.max(0, Math.min(1, f));
  }

  protected override willUpdate(changed: PropertyValues): void {
    super.willUpdate?.(changed);
    const w = this.renderWidth || 200;
    const h = this.height;
    const size = Math.min(w, h);
    // Centre the arc, opening downward, symmetric around the vertical axis.
    const start = 90 + (360 - this.sweep) / 2;
    this.layout = {
      w,
      h,
      size,
      cx: w / 2,
      cy: h / 2 + (this.sweep < 360 ? size * 0.08 : 0),
      r: size / 2 - this.thickness / 2 - 4,
      start,
      end: start + this.sweep,
      valueEnd: start + this.sweep * this.fraction,
    };
  }

  protected override render(): SVGTemplateResult {
    const { w, h, size, cx, cy, r, start, end, valueEnd } = this.layout;
    const valueColor = this.bandColor() ?? "var(--md-sys-color-primary, #6750a4)";

    return html`
      <div
        role="img"
        aria-label=${this.describe()}
        aria-valuenow=${this.value}
        aria-valuemin=${this.min}
        aria-valuemax=${this.max}
      >
        <svg viewBox="0 0 ${w} ${h}" width=${w} height=${h} aria-hidden="true">
          <path
            d=${arc(cx, cy, r, start, end)}
            fill="none"
            stroke="var(--md-sys-color-surface-container-highest, #e6e0e9)"
            stroke-width=${this.thickness}
            stroke-linecap="round"
          ></path>
          ${this.renderBands(cx, cy, r, start)}
          <path
            d=${arc(cx, cy, r, start, Math.max(start + 0.01, valueEnd))}
            fill="none"
            stroke=${valueColor}
            stroke-width=${this.thickness}
            stroke-linecap="round"
          ></path>
          ${this.hideValue
            ? nothing
            : svg`
                <text
                  x=${cx}
                  y=${cy}
                  text-anchor="middle"
                  dominant-baseline="central"
                  fill="var(--md-sys-color-on-surface, #1d1b20)"
                  font-size=${Math.round(size * 0.18)}
                  font-weight="500"
                >${fmt(this.value)}${this.unit}</text>`}
        </svg>
      </div>
    ` as unknown as SVGTemplateResult;
  }

  private renderBands(
    cx: number,
    cy: number,
    r: number,
    start: number,
  ): SVGTemplateResult | typeof nothing {
    if (!this.bands?.length) {
      return nothing;
    }
    const range = this.max - this.min || 1;
    let from = this.min;
    return svg`${this.bands.map((b) => {
      const f0 = Math.max(0, Math.min(1, (from - this.min) / range));
      const f1 = Math.max(0, Math.min(1, (b.to - this.min) / range));
      from = b.to;
      const a0 = start + this.sweep * f0;
      const a1 = start + this.sweep * f1;
      if (a1 <= a0) {
        return nothing;
      }
      return svg`<path
        d=${arc(cx, cy, r, a0, a1)}
        fill="none"
        stroke=${b.color}
        stroke-width=${this.thickness}
        stroke-opacity="0.35"
      ></path>`;
    })}`;
  }

  private bandColor(): string | undefined {
    if (!this.bands?.length) {
      return undefined;
    }
    for (const b of this.bands) {
      if (this.value <= b.to) {
        return b.color;
      }
    }
    return this.bands[this.bands.length - 1]?.color;
  }

  protected override describe(): string {
    if (this.ariaLabelText) {
      return this.ariaLabelText;
    }
    return `Gauge: ${fmt(this.value)}${this.unit} of ${fmt(this.max)}.`;
  }
}
