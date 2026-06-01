/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/** Material 3 FAB-menu item styles: a label chip + a 40px mini FAB. */
export const styles = css`
  :host {
    --_chip-color: var(
      --md-fab-menu-item-chip-color,
      var(--md-sys-color-surface-container-high, #ece6f0)
    );
    --_chip-label-color: var(
      --md-fab-menu-item-label-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );
    --_fab-color: var(--md-fab-menu-item-fab-color, var(--md-sys-color-primary-container, #eaddff));
    --_fab-icon-color: var(
      --md-fab-menu-item-fab-icon-color,
      var(--md-sys-color-on-primary-container, #21005d)
    );

    display: block;
  }

  .item {
    appearance: none;
    border: none;
    background: none;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    width: 100%;
    padding: 0;
    cursor: pointer;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    height: 40px;
    padding-inline: 16px;
    border-radius: var(--md-sys-shape-corner-small, 8px);
    background: var(--_chip-color);
    color: var(--_chip-label-color);
    font-family: var(
      --md-sys-typescale-label-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-label-large-size, 14px);
    font-weight: var(--md-sys-typescale-label-large-weight, 500);
    letter-spacing: 0.1px;
    white-space: nowrap;
    box-shadow: var(
      --md-sys-elevation-level1,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15)
    );
  }

  .mini-fab {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--md-sys-shape-corner-medium, 12px);
    background: var(--_fab-color);
    color: var(--_fab-icon-color);
    box-shadow: var(
      --md-sys-elevation-level2,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 2px 6px 2px rgba(0, 0, 0, 0.15)
    );
  }

  .item:hover .mini-fab {
    background: color-mix(in srgb, var(--_fab-icon-color) 8%, var(--_fab-color));
  }

  .item:focus-visible {
    outline: none;
  }

  .item:focus-visible .mini-fab {
    outline: 2px solid var(--_fab-icon-color);
    outline-offset: 2px;
  }

  ::slotted(*) {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  @media (forced-colors: active) {
    .chip,
    .mini-fab {
      outline: 1px solid CanvasText;
    }
  }
`;
