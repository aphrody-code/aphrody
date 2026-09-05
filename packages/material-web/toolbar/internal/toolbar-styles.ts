/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 toolbar styles (docked + floating). Consumes the `--md-sys-*`
 * design tokens directly and paints elevation itself. Component-level
 * `--md-toolbar-*` custom properties allow per-instance overrides.
 */
export const styles = css`
  :host {
    --_container-color: var(
      --md-toolbar-container-color,
      var(--md-sys-color-surface-container, #f3edf7)
    );
    --_container-shape: var(--md-toolbar-container-shape, var(--md-sys-shape-corner-full, 9999px));

    display: block;
    box-sizing: border-box;
    color: var(--md-sys-color-on-surface, #1d1b20);
  }

  .toolbar {
    box-sizing: border-box;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    background: var(--_container-color);
  }

  /* Docked: full width, fixed 64px height, no rounding. */
  :host([variant="docked"]) {
    width: 100%;
  }

  :host([variant="docked"]) .toolbar {
    width: 100%;
    height: 64px;
    padding-inline: 16px;
    border-radius: 0;
  }

  /* Floating: rounded, elevated pill with inline content. */
  :host([variant="floating"]) {
    display: inline-block;
    margin: 16px;
  }

  :host([variant="floating"]) .toolbar {
    height: 64px;
    padding-inline: 12px;
    border-radius: var(--_container-shape);
    /* M3 elevation level 2 */
    box-shadow: var(
      --md-sys-elevation-level2,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 2px 6px 2px rgba(0, 0, 0, 0.15)
    );
  }

  @media (forced-colors: active) {
    .toolbar {
      outline: 1px solid CanvasText;
    }
  }
`;
