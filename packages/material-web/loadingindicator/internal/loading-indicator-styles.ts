/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 Expressive loading-indicator styles. The active shape rotates
 * continuously while its rounded-polygon form morphs via animated
 * `border-radius`. Consumes the `--md-sys-*` design tokens directly.
 * Component-level `--md-loading-indicator-*` custom properties allow
 * per-instance overrides.
 */
export const styles = css`
  :host {
    --_size: var(--md-loading-indicator-size, 48px);
    --_active-color: var(--md-loading-indicator-active-color, var(--md-sys-color-primary, #6750a4));

    display: inline-flex;
    box-sizing: border-box;
    width: var(--_size);
    height: var(--_size);
  }

  .indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
  }

  .shape {
    width: 75%;
    height: 75%;
    background: var(--_active-color);
    /* Rounded-polygon-ish form that the morph animation reshapes. */
    border-radius: 38% 62% 63% 37% / 41% 44% 56% 59%;
  }

  /* Continuous rotation + morph between rounded polygon forms. */
  .indeterminate .shape {
    animation:
      md-loading-rotate 2666ms linear infinite,
      md-loading-morph 1333ms cubic-bezier(0.2, 0, 0, 1) infinite alternate;
  }

  .determinate .shape {
    animation: md-loading-rotate 2666ms linear infinite;
    transition: transform 200ms cubic-bezier(0.2, 0, 0, 1);
  }

  @keyframes md-loading-rotate {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes md-loading-morph {
    0% {
      border-radius: 38% 62% 63% 37% / 41% 44% 56% 59%;
      transform: rotate(0deg) scale(1);
    }
    50% {
      border-radius: 62% 38% 41% 59% / 57% 63% 37% 43%;
      transform: rotate(180deg) scale(0.85);
    }
    100% {
      border-radius: 50% 50% 50% 50% / 50% 50% 50% 50%;
      transform: rotate(360deg) scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .indeterminate .shape,
    .determinate .shape {
      animation: none;
    }
  }

  @media (forced-colors: active) {
    .shape {
      background: CanvasText;
    }
  }
`;
