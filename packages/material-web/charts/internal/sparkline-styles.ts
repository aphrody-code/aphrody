/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

import { sharedChartStyles } from "./chart-shared-styles.js";

/** Styles for `md-sparkline`. */
export const styles = [
  sharedChartStyles,
  css`
    :host {
      display: inline-block;
      inline-size: auto;
      line-height: 0;
    }
    svg {
      inline-size: revert;
      block-size: revert;
    }
  `,
];
