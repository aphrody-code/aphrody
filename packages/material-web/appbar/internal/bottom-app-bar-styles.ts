/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/** Material 3 bottom-app-bar styles. */
export const styles = css`
  :host {
    --_container-color: var(
      --md-bottom-app-bar-container-color,
      var(--md-sys-color-surface-container, #f3edf7)
    );

    display: block;
    position: sticky;
    inset-block-end: 0;
    z-index: 4;
  }

  .bar {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    width: 100%;
    height: 80px;
    padding-inline: 4px 16px;
    background: var(--_container-color);
    color: var(--md-sys-color-on-surface-variant, #49454f);
  }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .fab {
    margin-inline-start: auto;
    display: inline-flex;
    align-items: center;
  }

  .fab ::slotted(*) {
    --md-fab-container-color: var(--md-sys-color-primary-container, #eaddff);
  }
`;
