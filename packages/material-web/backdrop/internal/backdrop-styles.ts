/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 backdrop (scrim) styles. The host is a fixed, full-viewport
 * overlay tinted with `--md-sys-color-scrim` at a configurable alpha. It is
 * hidden (and non-interactive) until `[open]`, then fades in. `[invisible]`
 * drops the tint while still catching clicks; `[blur]` frosts the content
 * behind it. Per-instance overrides via `--md-backdrop-*`.
 */
export const styles = css`
  :host {
    position: fixed;
    inset: 0;
    z-index: var(--md-backdrop-z-index, 1);
    display: flex;
    align-items: center;
    justify-content: center;
    /* scrim token (neutral0) at the M3 scrim opacity. */
    background: color-mix(
      in srgb,
      var(--md-backdrop-scrim-color, var(--md-sys-color-scrim, #000)) 32%,
      transparent
    );
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
    transition:
      opacity var(--md-backdrop-transition-duration, 200ms)
        var(--md-backdrop-transition-easing, cubic-bezier(0.2, 0, 0, 1)),
      visibility var(--md-backdrop-transition-duration, 200ms)
        var(--md-backdrop-transition-easing, cubic-bezier(0.2, 0, 0, 1));
  }

  :host([open]) {
    opacity: 1;
    visibility: visible;
    pointer-events: auto;
  }

  :host([invisible]) {
    background: transparent;
  }

  :host([blur]) {
    backdrop-filter: blur(var(--md-backdrop-blur-amount, 4px));
    -webkit-backdrop-filter: blur(var(--md-backdrop-blur-amount, 4px));
  }

  @media (prefers-reduced-motion: reduce) {
    :host {
      transition-duration: 0ms;
    }
  }

  @media (forced-colors: active) {
    :host {
      background: transparent;
    }
  }
`;
