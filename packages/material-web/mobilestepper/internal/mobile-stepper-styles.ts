/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 mobile stepper styles. Lays out app-supplied back/next controls on
 * the edges with the progress affordance (dots / text / progress bar) centred
 * between them. Consumes the `--md-sys-*` tokens; per-instance overrides via
 * `--md-mobile-stepper-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_active-dot-color: var(
      --md-mobile-stepper-active-dot-color,
      var(--md-sys-color-primary, #6750a4)
    );
    --_inactive-dot-color: var(
      --md-mobile-stepper-inactive-dot-color,
      var(--md-sys-color-surface-variant, #e7e0ec)
    );
    --_container-color: var(
      --md-mobile-stepper-container-color,
      var(--md-sys-color-surface, #fef7ff)
    );
    --_text-color: var(--md-mobile-stepper-text-color, var(--md-sys-color-on-surface, #1d1b20));
    --_track-color: var(
      --md-mobile-stepper-track-color,
      var(--md-sys-color-surface-variant, #e7e0ec)
    );
    --_indicator-color: var(
      --md-mobile-stepper-indicator-color,
      var(--md-sys-color-primary, #6750a4)
    );
  }

  :host([position="bottom"]) {
    position: fixed;
    inset-block-end: 0;
    inset-inline: 0;
    z-index: 2;
  }

  :host([position="top"]) {
    position: fixed;
    inset-block-start: 0;
    inset-inline: 0;
    z-index: 2;
  }

  .stepper {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 48px;
    padding-block: 4px;
    padding-inline: 8px;
    background: var(--_container-color);
    color: var(--_text-color);
  }

  .control {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
  }

  .control.next {
    justify-content: flex-end;
  }

  .progress {
    flex: 1 1 auto;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
  }

  /* Dots variant. */
  .dots {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .dot {
    inline-size: 8px;
    block-size: 8px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    background: var(--_inactive-dot-color);
    transition: background 150ms cubic-bezier(0.2, 0, 0, 1);
  }

  .dot.active {
    background: var(--_active-dot-color);
  }

  /* Text variant. */
  .text {
    font-family: var(
      --md-sys-typescale-label-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-label-medium-size, 12px);
    line-height: var(--md-sys-typescale-label-medium-line-height, 16px);
    letter-spacing: var(--md-sys-typescale-label-medium-tracking, 0.5px);
    font-weight: var(--md-sys-typescale-label-medium-weight, 500);
    color: var(--_text-color);
    white-space: nowrap;
  }

  /* Progress variant. */
  .bar {
    position: relative;
    flex: 1 1 auto;
    block-size: 4px;
    inline-size: 100%;
    max-inline-size: 200px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    overflow: hidden;
  }

  .track {
    position: absolute;
    inset: 0;
    background: var(--_track-color);
    border-radius: inherit;
  }

  .indicator {
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    background: var(--_indicator-color);
    border-radius: inherit;
    transition: inline-size 250ms cubic-bezier(0.2, 0, 0, 1);
  }

  @media (prefers-reduced-motion: reduce) {
    .dot,
    .indicator {
      transition-duration: 0ms;
    }
  }

  @media (forced-colors: active) {
    .dot {
      background: GrayText;
    }
    .dot.active {
      background: CanvasText;
    }
    .indicator {
      background: CanvasText;
    }
    .track {
      background: GrayText;
    }
  }
`;
