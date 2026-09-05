/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 list-detail layout styles. Autonomous (token + hex fallbacks).
 * Dual-pane mode (expanded and up) lays the list and detail in a flex row with
 * the 24dp pane spacer; single-pane mode shows only the `showing` pane. The use
 * of logical `flex` and `gap` keeps the leading list correct under RTL.
 */
export const styles = css`
  :host {
    display: block;
    box-sizing: border-box;
    block-size: 100%;
    inline-size: 100%;
    min-block-size: 0;
  }

  .container {
    box-sizing: border-box;
    block-size: 100%;
    inline-size: 100%;
    display: flex;
    /* PANE_SPACER_DP = 24 (adaptive.rs). */
    gap: 24px;
  }

  .list,
  .detail {
    box-sizing: border-box;
    min-inline-size: 0;
    min-block-size: 0;
    block-size: 100%;
  }

  .detail {
    flex: 1 1 0%;
  }

  /* Single-pane mode: each pane takes the full width; show only one. */
  :host(:not([size-class="expanded"]):not([size-class="large"]):not([size-class="extra-large"]))
    .list,
  :host(:not([size-class="expanded"]):not([size-class="large"]):not([size-class="extra-large"]))
    .detail {
    flex: 1 1 100%;
    inline-size: 100%;
  }

  :host(
      [showing="detail"]:not([size-class="expanded"]):not([size-class="large"]):not(
          [size-class="extra-large"]
        )
    )
    .list {
    display: none;
  }

  :host(
      [showing="list"]:not([size-class="expanded"]):not([size-class="large"]):not(
          [size-class="extra-large"]
        )
    )
    .detail {
    display: none;
  }

  @media (forced-colors: active) {
    .list,
    .detail {
      outline: 1px solid CanvasText;
    }
  }
`;
