/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 skeleton styles. The base surface uses the
 * `--md-sys-color-surface-variant` role (with a `surface-container` highlight
 * for the wave sweep) and baseline fallbacks. Shape is driven by `variant`;
 * animation by `animation`, timed with `--md-sys-motion-*` where it maps
 * cleanly. Per-instance overrides via `--md-skeleton-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_color: var(--md-skeleton-color, var(--md-sys-color-surface-variant, #e7e0ec));
    --_highlight-color: var(
      --md-skeleton-highlight-color,
      var(--md-sys-color-surface-container, #f3edf7)
    );
  }

  /* circular sizing collapses to content unless given dimensions. */
  :host([variant="circular"]),
  :host([variant="rectangular"]),
  :host([variant="rounded"]) {
    display: inline-block;
  }

  .skeleton {
    box-sizing: border-box;
    width: 100%;
    background-color: var(--_color);
  }

  /* text: line-shaped placeholder on the text rhythm. */
  :host([variant="text"]) .skeleton {
    height: 1.2em;
    margin-block: 0;
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    transform-origin: 0 55%;
    /* The default text line scales to ~the surrounding font. */
    transform: scale(1, 0.7);
  }

  :host([variant="circular"]) .skeleton {
    width: 40px;
    height: 40px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
  }

  :host([variant="rectangular"]) .skeleton {
    width: 100%;
    height: 100%;
    border-radius: 0;
  }

  :host([variant="rounded"]) .skeleton {
    width: 100%;
    height: 100%;
    border-radius: var(--md-sys-shape-corner-medium, 12px);
  }

  /* pulse: opacity breathes. */
  :host([animation="pulse"]) .skeleton {
    animation: md-skeleton-pulse var(--md-sys-motion-duration-long2, 1500ms)
      var(--md-sys-motion-easing-standard, cubic-bezier(0.2, 0, 0, 1)) 0.5s infinite;
  }

  /* wave: a highlight sweeps across. */
  :host([animation="wave"]) .skeleton {
    position: relative;
    overflow: hidden;
  }

  :host([animation="wave"]) .skeleton::after {
    content: "";
    position: absolute;
    inset: 0;
    transform: translateX(-100%);
    background: linear-gradient(90deg, transparent, var(--_highlight-color), transparent);
    animation: md-skeleton-wave 1600ms linear 0.5s infinite;
  }

  @keyframes md-skeleton-pulse {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
    100% {
      opacity: 1;
    }
  }

  @keyframes md-skeleton-wave {
    0% {
      transform: translateX(-100%);
    }
    60% {
      transform: translateX(100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    :host([animation="pulse"]) .skeleton,
    :host([animation="wave"]) .skeleton::after {
      animation: none;
    }
  }

  @media (forced-colors: active) {
    .skeleton {
      outline: 1px solid CanvasText;
    }
  }
`;
