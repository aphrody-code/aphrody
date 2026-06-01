/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 grid-list styles. A CSS grid with `var(--_cols)` equal columns and
 * a configurable gap. Row sizing is driven by custom properties the element
 * computes from its `row-height` attribute (`--_grid-auto-rows`,
 * `--_aspect-ratio`). Per-instance overrides via `--md-grid-list-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_cols: 4;
    --_gap: 8px;
    --_grid-auto-rows: 120px;
    --_aspect-ratio: auto;
    --_row-height: 120px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(var(--_cols), minmax(0, 1fr));
    grid-auto-rows: var(--_grid-auto-rows);
    gap: var(--_gap);
    width: 100%;
    box-sizing: border-box;
  }

  /* Propagate the computed aspect ratio to tiles when rows are ratio-sized. */
  ::slotted(md-grid-tile) {
    --md-grid-tile-aspect-ratio: var(--_aspect-ratio);
  }
`;
