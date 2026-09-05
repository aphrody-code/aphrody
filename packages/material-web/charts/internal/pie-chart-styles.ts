/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

import { sharedChartStyles } from "./chart-shared-styles.js";

/** Styles for `md-pie-chart`. */
export const styles = [
  sharedChartStyles,
  css`
    .chart {
      position: relative;
    }
    path {
      transition: d 120ms ease;
      cursor: default;
    }
  `,
];
