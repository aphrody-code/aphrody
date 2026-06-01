/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

import { sharedChartStyles } from "./chart-shared-styles.js";

/** Styles for `md-gauge`. */
export const styles = [
  sharedChartStyles,
  css`
    path {
      transition:
        stroke 200ms ease,
        d 300ms ease;
    }
  `,
];
