/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Styles shared by every Material 3 chart: host layout, the SVG canvas, axes /
 * grid on `outline-variant`, text on the `on-surface` roles using the
 * `label-small` / `body-small` typescale, the legend, and the hover tooltip.
 * Self-contained on the `--md-sys-*` tokens with sensible literal fallbacks.
 */
export const sharedChartStyles = css`
  :host {
    display: block;
    position: relative;
    box-sizing: border-box;
    inline-size: 100%;
    font-family: var(--md-sys-typescale-body-small-font, system-ui, sans-serif);
    color: var(--md-sys-color-on-surface, #1d1b20);
    --_grid-color: var(--md-sys-color-outline-variant, #cac4d0);
    --_axis-color: var(--md-sys-color-outline, #79747e);
    --_label-color: var(--md-sys-color-on-surface-variant, #49454f);
    --_surface-color: var(--md-sys-color-surface-container, #f3edf7);
    --_on-surface-color: var(--md-sys-color-on-surface, #1d1b20);
  }

  svg {
    display: block;
    inline-size: 100%;
    block-size: auto;
    overflow: visible;
  }

  .grid line,
  .axis line {
    stroke: var(--_grid-color);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }

  .axis .domain {
    stroke: var(--_axis-color);
    stroke-width: 1;
    fill: none;
    shape-rendering: crispEdges;
  }

  .tick-label {
    fill: var(--_label-color);
    font-size: var(--md-sys-typescale-label-small-size, 11px);
    font-family: var(--md-sys-typescale-label-small-font, system-ui, sans-serif);
  }

  .axis-title {
    fill: var(--_label-color);
    font-size: var(--md-sys-typescale-label-small-size, 11px);
  }

  .value-label {
    fill: var(--_on-surface-color);
    font-size: var(--md-sys-typescale-label-small-size, 11px);
    paint-order: stroke;
  }

  /* Legend */
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    margin-block-start: 8px;
    padding-inline: 8px;
    font-size: var(--md-sys-typescale-label-small-size, 11px);
    color: var(--_label-color);
  }

  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: default;
    user-select: none;
  }

  .legend-item--off {
    opacity: 0.4;
  }

  .legend-swatch {
    inline-size: 12px;
    block-size: 12px;
    border-radius: 3px;
    flex: none;
  }

  /* Tooltip */
  .tooltip {
    position: absolute;
    pointer-events: none;
    z-index: 2;
    padding: 6px 10px;
    border-radius: var(--md-sys-shape-corner-small, 8px);
    background: var(--md-sys-color-inverse-surface, #322f35);
    color: var(--md-sys-color-inverse-on-surface, #f5eff7);
    font-size: var(--md-sys-typescale-body-small-size, 12px);
    box-shadow: var(
      --md-sys-elevation-level2,
      0 1px 2px rgba(0, 0, 0, 0.3),
      0 2px 6px rgba(0, 0, 0, 0.15)
    );
    white-space: nowrap;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .tooltip[data-visible] {
    opacity: 1;
  }

  .tooltip-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .tooltip-swatch {
    inline-size: 8px;
    block-size: 8px;
    border-radius: 2px;
    flex: none;
  }

  .tooltip-title {
    font-weight: 500;
    margin-block-end: 2px;
  }

  /* Visually-hidden a11y data table. */
  .sr-only {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .series-line {
    fill: none;
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }

  .series-marker {
    stroke: var(--md-sys-color-surface, #fef7ff);
    stroke-width: 1.5;
  }
`;
