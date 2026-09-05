/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 FAB-menu styles. A 56px primary-container trigger with a 16dp
 * radius; the items stack above it and the trigger icon rotates 45° on open.
 */
export const styles = css`
  :host {
    --_container-color: var(
      --md-fab-menu-container-color,
      var(--md-sys-color-primary-container, #eaddff)
    );
    --_icon-color: var(--md-fab-menu-icon-color, var(--md-sys-color-on-primary-container, #21005d));
    --_container-shape: var(--md-fab-menu-container-shape, var(--md-sys-shape-corner-large, 16px));

    position: relative;
    display: inline-flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 16px;
  }

  .scrim {
    display: none;
    position: fixed;
    inset: 0;
    z-index: -1;
    background: color-mix(in srgb, var(--md-sys-color-scrim, #000000) 32%, transparent);
  }

  :host([open]) .scrim {
    display: block;
  }

  .items {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 12px;
  }

  :host(:not([open])) .items {
    display: none;
  }

  .trigger {
    anchor-name: --fab-trigger;
    appearance: none;
    border: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 56px;
    height: 56px;
    border-radius: var(--_container-shape);
    background: var(--_container-color);
    color: var(--_icon-color);
    cursor: pointer;
    box-shadow: var(
      --md-sys-elevation-level3,
      0 1px 3px 0 rgba(0, 0, 0, 0.3),
      0 4px 8px 3px rgba(0, 0, 0, 0.15)
    );
  }

  @supports (anchor-name: --x) {
    .items {
      position: absolute;
      position-anchor: --fab-trigger;
      bottom: anchor(top);
      right: anchor(right);
      margin-bottom: 16px;
    }
  }

  .trigger:hover {
    background: color-mix(in srgb, var(--_icon-color) 8%, var(--_container-color));
  }

  .trigger:focus-visible {
    outline: 2px solid var(--_icon-color);
    outline-offset: 2px;
  }

  .icon {
    width: 24px;
    height: 24px;
    fill: currentColor;
    transition: transform 200ms cubic-bezier(0.2, 0, 0, 1);
  }

  :host([open]) .icon {
    transform: rotate(45deg);
  }

  @media (prefers-reduced-motion: reduce) {
    .icon {
      transition: none;
    }
  }

  @media (forced-colors: active) {
    .trigger {
      outline: 1px solid CanvasText;
    }
  }
`;
