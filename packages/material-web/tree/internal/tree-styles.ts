/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 tree container styles. A simple `surface` block that hosts the
 * slotted `<md-tree-item>` rows; per-item visuals live in the tree-item styles.
 * Consumes the `--md-sys-*` tokens; per-instance overrides via `--md-tree-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_container-color: var(--md-tree-container-color, var(--md-sys-color-surface, #fef7ff));
    background: var(--_container-color);
    padding: 8px;
    box-sizing: border-box;
  }

  .tree {
    display: flex;
    flex-direction: column;
  }

  @media (forced-colors: active) {
    :host {
      outline: 1px solid CanvasText;
    }
  }
`;
