/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

import { sharedChartStyles } from "./chart-shared-styles.js";

/** Styles for `md-radar-chart`. */
export const styles = [
  sharedChartStyles,
  css`
    .chart {
      position: relative;
    }
    .radar-ring {
      fill: none;
      stroke: var(--_grid-color);
      stroke-width: 1;
    }
    .radar-area {
      stroke-width: 2;
      stroke-linejoin: round;
    }
    .radar-marker {
      stroke: var(--md-sys-color-surface, #fef7ff);
      stroke-width: 1.5;
      transition: r 100ms ease;
      cursor: default;
    }
  `,
];
