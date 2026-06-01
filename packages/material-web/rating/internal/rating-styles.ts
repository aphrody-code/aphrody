/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 rating styles. The control is a row of star icons. The active
 * (selected/hover) portion uses `--md-sys-color-primary`; the inactive portion
 * uses `--md-sys-color-outline`. Half-star precision is achieved by clipping a
 * second, overlaid active icon to 50% width. Consumes the `--md-sys-*` tokens;
 * per-instance overrides via `--md-rating-*`.
 */
export const styles = css`
  :host {
    display: inline-flex;
    vertical-align: middle;
    --_active-color: var(--md-rating-active-color, var(--md-sys-color-primary, #6750a4));
    --_inactive-color: var(--md-rating-inactive-color, var(--md-sys-color-outline, #79747e));
    --_icon-size: var(--md-rating-icon-size, 24px);
    --_gap: var(--md-rating-gap, 2px);
    outline: none;
  }

  :host([disabled]) {
    pointer-events: none;
    opacity: 0.38;
  }

  .rating {
    display: inline-flex;
    align-items: center;
    gap: var(--_gap);
  }

  .item {
    appearance: none;
    border: none;
    background: none;
    margin: 0;
    padding: 2px;
    display: inline-flex;
    position: relative;
    width: var(--_icon-size);
    height: var(--_icon-size);
    box-sizing: content-box;
    cursor: pointer;
    color: var(--_inactive-color);
    border-radius: var(--md-sys-shape-corner-small, 8px);
    transition: transform var(--md-sys-motion-duration-short3, 150ms)
      var(--md-sys-motion-easing-emphasized, cubic-bezier(0.2, 0, 0, 1));
  }

  :host([readonly]) .item,
  :host([disabled]) .item {
    cursor: default;
  }

  .item:not([data-readonly]):hover {
    transform: scale(1.1);
  }

  .item:focus-visible {
    outline: 2px solid var(--_active-color);
    outline-offset: 2px;
  }

  /* The base (inactive) icon layer. */
  .icon {
    width: 100%;
    height: 100%;
    display: block;
    pointer-events: none;
  }

  .icon svg,
  .icon ::slotted(*) {
    width: 100%;
    height: 100%;
    display: block;
    fill: currentColor;
  }

  /* The active overlay, clipped to --_fill (0 to 100%). */
  .active {
    position: absolute;
    inset: 2px;
    overflow: hidden;
    width: var(--_fill, 0%);
    color: var(--_active-color);
    pointer-events: none;
  }

  /* Keep the overlaid icon at the full item width while the wrapper clips it. */
  .active .icon {
    width: var(--_icon-size);
  }

  @media (prefers-reduced-motion: reduce) {
    .item {
      transition: none;
    }
    .item:not([data-readonly]):hover {
      transform: none;
    }
  }

  @media (forced-colors: active) {
    .active {
      color: Highlight;
    }
    .item {
      color: CanvasText;
    }
  }
`;
