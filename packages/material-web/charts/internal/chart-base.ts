/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { isServer, LitElement } from "lit";
import { property, state } from "lit/decorators.js";

/** A single data series passed declaratively to a chart. */
export interface ChartSeries {
  /** Numeric data points for the series. */
  data: number[];
  /** Human-readable name shown in the legend / tooltip. */
  label?: string;
  /** Optional explicit colour (CSS value). Defaults to the M3 palette. */
  color?: string;
  /**
   * Stack id. Series sharing a stack id are stacked on top of each other
   * (bar / area charts). Omit for grouped / overlaid rendering.
   */
  stack?: string;
  /** Render the line as a smooth curve rather than straight segments. */
  curve?: boolean;
  /** Show point markers (line / area charts). */
  showMarkers?: boolean;
}

/** The default Material 3 tonal palette used when a series has no colour. */
export const M3_PALETTE = [
  "var(--md-sys-color-primary, #6750a4)",
  "var(--md-sys-color-secondary, #625b71)",
  "var(--md-sys-color-tertiary, #7d5260)",
  "var(--md-sys-color-error, #b3261e)",
  "var(--md-sys-color-primary-container, #eaddff)",
  "var(--md-sys-color-secondary-container, #e8def8)",
  "var(--md-sys-color-tertiary-container, #ffd8e4)",
  "var(--md-sys-color-inverse-primary, #d0bcff)",
];

/** Resolve a colour for series index `i`, honouring overrides. */
export function colorAt(
  colors: string[] | undefined,
  series: ChartSeries | undefined,
  i: number,
): string {
  if (series?.color) {
    return series.color;
  }
  const palette = colors && colors.length ? colors : M3_PALETTE;
  return palette[i % palette.length];
}

/** "Nice" axis bounds + tick count for a numeric range. */
export function niceScale(
  min: number,
  max: number,
  ticks = 5,
): { min: number; max: number; step: number } {
  if (min === max) {
    // Avoid a zero-height range.
    max = min + 1;
    min = min - 1;
  }
  const range = max - min;
  const rawStep = range / Math.max(1, ticks);
  const mag = Math.pow(10, Math.floor(Math.log10(rawStep)));
  const norm = rawStep / mag;
  let niceStep: number;
  if (norm < 1.5) {
    niceStep = 1;
  } else if (norm < 3) {
    niceStep = 2;
  } else if (norm < 7) {
    niceStep = 5;
  } else {
    niceStep = 10;
  }
  niceStep *= mag;
  const niceMin = Math.floor(min / niceStep) * niceStep;
  const niceMax = Math.ceil(max / niceStep) * niceStep;
  return { min: niceMin, max: niceMax, step: niceStep };
}

/** Build an SVG path `d` for a polyline / smooth curve through points. */
export function linePath(points: ReadonlyArray<readonly [number, number]>, smooth = false): string {
  if (!points.length) {
    return "";
  }
  if (!smooth || points.length < 3) {
    return points.map((p, i) => `${i === 0 ? "M" : "L"}${p[0]},${p[1]}`).join(" ");
  }
  // Catmull-Rom -> cubic Bézier smoothing.
  let d = `M${points[0][0]},${points[0][1]}`;
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[i === 0 ? 0 : i - 1];
    const p1 = points[i];
    const p2 = points[i + 1];
    const p3 = points[i + 2 < points.length ? i + 2 : i + 1];
    const cp1x = p1[0] + (p2[0] - p0[0]) / 6;
    const cp1y = p1[1] + (p2[1] - p0[1]) / 6;
    const cp2x = p2[0] - (p3[0] - p1[0]) / 6;
    const cp2y = p2[1] - (p3[1] - p1[1]) / 6;
    d += ` C${cp1x},${cp1y} ${cp2x},${cp2y} ${p2[0]},${p2[1]}`;
  }
  return d;
}

/** Format a number compactly for axis ticks / labels. */
export function fmt(n: number): string {
  if (!isFinite(n)) {
    return "";
  }
  if (Math.abs(n) >= 1000) {
    return n.toLocaleString(undefined, { maximumFractionDigits: 1 });
  }
  return String(Math.round(n * 100) / 100);
}

/**
 * Base class for all charts. Provides declarative sizing, a ResizeObserver for
 * responsive width, the shared `series`/`colors`/`legend`/`tooltip` API and an
 * a11y label. Subclasses implement `renderChart()`.
 */
export abstract class ChartBase extends LitElement {
  /** Data series. Set via the `.series` property (not an attribute). */
  @property({ attribute: false }) series: ChartSeries[] = [];

  /** Category labels for the X axis (bar / line / area). */
  @property({ attribute: false }) categories: string[] = [];

  /** Explicit colour palette (CSS values), overriding the M3 default. */
  @property({ attribute: false }) colors?: string[];

  /** Chart height in px. */
  @property({ type: Number }) height = 300;

  /**
   * Chart width in px. When unset (0), the chart fills its container width and
   * tracks it with a ResizeObserver.
   */
  @property({ type: Number }) width = 0;

  /** Show the legend. */
  @property({ type: Boolean }) legend = true;

  /** Enable the hover tooltip. */
  @property({ type: Boolean }) tooltip = true;

  /** Accessible label describing the chart. */
  @property({ attribute: "aria-label" }) ariaLabelText = "";

  /** Measured container width (px) used when `width` is 0. */
  @state() protected measuredWidth = 600;

  private resizeObserver?: ResizeObserver;

  /** Effective render width. */
  protected get renderWidth(): number {
    return this.width > 0 ? this.width : this.measuredWidth;
  }

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer && typeof ResizeObserver !== "undefined") {
      this.resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          const w = entry.contentRect.width;
          if (w > 0 && Math.abs(w - this.measuredWidth) > 1) {
            this.measuredWidth = w;
          }
        }
      });
      this.resizeObserver.observe(this);
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    this.resizeObserver?.disconnect();
    this.resizeObserver = undefined;
  }

  /** A11y description built from the series / categories. */
  protected describe(): string {
    if (this.ariaLabelText) {
      return this.ariaLabelText;
    }
    const names = this.series.map((s) => s.label).filter(Boolean);
    if (names.length) {
      return `Chart with series: ${names.join(", ")}.`;
    }
    return "Chart.";
  }
}
