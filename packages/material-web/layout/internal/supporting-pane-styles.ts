/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 supporting-pane layout styles. Autonomous (token + hex fallbacks).
 * Side-by-side mode (expanded and up) is a flex row with the 24dp pane spacer;
 * otherwise the panes stack as a column (or the supporting pane is hidden when
 * `collapsed`).
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
    flex-direction: column;
    /* PANE_SPACER_DP = 24 (adaptive.rs). */
    gap: 24px;
  }

  .main {
    flex: 1 1 auto;
    min-inline-size: 0;
    min-block-size: 0;
    box-sizing: border-box;
  }

  .supporting {
    flex: 0 0 auto;
    min-inline-size: 0;
    box-sizing: border-box;
  }

  /* Side-by-side from the expanded breakpoint up. */
  :host([size-class="expanded"]) .container,
  :host([size-class="large"]) .container,
  :host([size-class="extra-large"]) .container {
    flex-direction: row;
  }

  :host([size-class="expanded"]) .main,
  :host([size-class="large"]) .main,
  :host([size-class="extra-large"]) .main {
    block-size: 100%;
  }

  /* Collapsed: hide the supporting pane when stacked (compact / medium). */
  :host(
      [collapsed]:not([size-class="expanded"]):not([size-class="large"]):not(
          [size-class="extra-large"]
        )
    )
    .supporting {
    display: none;
  }

  @media (forced-colors: active) {
    .main,
    .supporting {
      outline: 1px solid CanvasText;
    }
  }
`;
