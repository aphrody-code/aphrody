/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

import { sharedChartStyles } from "./chart-shared-styles.js";

/** Styles for `md-scatter-chart`. */
export const styles = [
  sharedChartStyles,
  css`
    .chart {
      position: relative;
    }
    .scatter-point {
      stroke: var(--md-sys-color-surface, #fef7ff);
      stroke-width: 1;
      transition:
        r 100ms ease,
        fill-opacity 100ms ease;
      cursor: default;
    }
  `,
];
