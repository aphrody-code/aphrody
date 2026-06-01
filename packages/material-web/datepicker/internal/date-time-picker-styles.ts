/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

import { styles as dateStyles } from "./date-picker-styles.js";

/**
 * Material 3 date-time-picker styles. Reuses the docked date-picker panel and
 * appends a divided time row that hosts an editable `<md-time-picker>`.
 */
export const styles = [
  dateStyles,
  css`
    .date-time {
      box-sizing: border-box;
      width: 328px;
      border-radius: var(--_container-shape);
      background: var(--_container-color);
      color: var(--_on-surface);
      box-shadow: var(
        --md-sys-elevation-level3,
        0 1px 3px 0 rgba(0, 0, 0, 0.3),
        0 4px 8px 3px rgba(0, 0, 0, 0.15)
      );
      overflow: hidden;
    }

    /* The inherited .picker panel sits flush inside the composite container. */
    .date-time .picker {
      width: 328px;
      box-shadow: none;
      background: transparent;
    }

    .time-row {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 12px;
      border-top: 1px solid color-mix(in srgb, var(--_on-surface) 12%, transparent);
    }

    .time-label {
      font-family: var(
        --md-sys-typescale-title-small-font,
        "Google Sans Flex",
        Roboto,
        system-ui,
        sans-serif
      );
      font-size: var(--md-sys-typescale-title-small-size, 14px);
      font-weight: var(--md-sys-typescale-title-small-weight, 500);
      letter-spacing: 0.1px;
    }

    .time {
      flex: 1;
    }

    @media (forced-colors: active) {
      .date-time {
        outline: 1px solid CanvasText;
      }
    }
  `,
];
