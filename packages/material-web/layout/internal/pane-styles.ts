/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 pane styles. Autonomous: tokens are consumed with hex fallbacks.
 * The pane is a surface with internal scrolling and standard padding; an
 * optional large corner radius applies when `rounded` is set.
 */
export const styles = css`
  :host {
    --_surface: var(--md-sys-color-surface, #fef7ff);
    --_on-surface: var(--md-sys-color-on-surface, #1d1b20);
    --_shape: var(--md-sys-shape-corner-large, 16px);

    display: block;
    box-sizing: border-box;
    min-block-size: 0;
    min-inline-size: 0;
    block-size: 100%;
    color: var(--_on-surface);
  }

  .pane {
    box-sizing: border-box;
    block-size: 100%;
    inline-size: 100%;
    overflow: auto;
    padding: 16px;
    background: var(--_surface);
  }

  :host([rounded]) .pane {
    border-radius: var(--_shape);
  }

  @media (forced-colors: active) {
    .pane {
      outline: 1px solid CanvasText;
    }
  }
`;
