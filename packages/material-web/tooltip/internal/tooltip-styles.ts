/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 tooltip styles. The surface anchors to its trigger via CSS Anchor
 * Positioning (`position-anchor` + `inset-area`) where supported, with a
 * JS-positioned `position: fixed` fallback. Plain tooltips use the inverse
 * surface tokens; rich tooltips use a surface container with elevation level 2.
 * Per-instance overrides via `--md-tooltip-*`.
 */
export const styles = css`
  :host {
    display: contents;
    --_plain-container-color: var(
      --md-tooltip-plain-container-color,
      var(--md-sys-color-inverse-surface, #322f35)
    );
    --_plain-text-color: var(
      --md-tooltip-plain-text-color,
      var(--md-sys-color-inverse-on-surface, #f5eff7)
    );
    --_rich-container-color: var(
      --md-tooltip-rich-container-color,
      var(--md-sys-color-surface-container, #f3edf7)
    );
    --_rich-text-color: var(
      --md-tooltip-rich-text-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_rich-headline-color: var(
      --md-tooltip-rich-headline-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );
    --_rich-action-color: var(--md-tooltip-rich-action-color, var(--md-sys-color-primary, #6750a4));
  }

  .tooltip {
    position: fixed;
    z-index: 1000;
    box-sizing: border-box;
    width: max-content;
    pointer-events: none;
    opacity: 0;
    transform: scale(0.9);
    transition:
      opacity 150ms cubic-bezier(0.3, 0, 0.8, 0.15),
      transform 150ms cubic-bezier(0.3, 0, 0.8, 0.15);
  }

  :host([visible]) .tooltip {
    opacity: 1;
    transform: scale(1);
    transition:
      opacity 150ms cubic-bezier(0.05, 0.7, 0.1, 1),
      transform 150ms cubic-bezier(0.05, 0.7, 0.1, 1);
  }

  /* Plain variant: single line, inverse surface. */
  .tooltip.plain {
    max-width: 200px;
    padding: 4px 8px;
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: var(--_plain-container-color);
    color: var(--_plain-text-color);
    font-family: var(
      --md-sys-typescale-body-small-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-small-size, 12px);
    line-height: var(--md-sys-typescale-body-small-line-height, 16px);
    letter-spacing: var(--md-sys-typescale-body-small-tracking, 0.4px);
    font-weight: var(--md-sys-typescale-body-small-weight, 400);
    text-align: center;
  }

  /* Rich variant: headline + text + actions, surface container, elevation 2. */
  .tooltip.rich {
    pointer-events: auto;
    min-width: 200px;
    max-width: 320px;
    padding: 12px 16px;
    border-radius: var(--md-sys-shape-corner-medium, 12px);
    background: var(--_rich-container-color);
    color: var(--_rich-text-color);
    box-shadow: var(
      --md-sys-elevation-level2,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 2px 6px 2px rgba(0, 0, 0, 0.15)
    );
  }

  .rich .headline {
    color: var(--_rich-headline-color);
    font-family: var(
      --md-sys-typescale-title-small-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-title-small-size, 14px);
    line-height: var(--md-sys-typescale-title-small-line-height, 20px);
    font-weight: var(--md-sys-typescale-title-small-weight, 500);
  }

  .rich .headline:not(:empty) {
    margin-bottom: 4px;
  }

  .rich .supporting-text {
    font-family: var(
      --md-sys-typescale-body-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-medium-size, 14px);
    line-height: var(--md-sys-typescale-body-medium-line-height, 20px);
    letter-spacing: var(--md-sys-typescale-body-medium-tracking, 0.25px);
  }

  .rich .actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  .rich .actions:empty {
    display: none;
  }

  ::slotted([slot="action"]) {
    color: var(--_rich-action-color);
    --md-text-button-label-text-color: var(--_rich-action-color);
  }

  /* CSS Anchor Positioning: place below the anchor, centered. */
  @supports (anchor-name: --md-tooltip-anchor) {
    .tooltip {
      position: absolute;
      position-anchor: var(--md-tooltip-anchor-name, --md-tooltip-anchor);
      position-area: bottom;
      margin-top: 8px;
      justify-self: anchor-center;
      position-try-fallbacks: flip-block, flip-inline;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tooltip,
    :host([visible]) .tooltip {
      transform: none;
      transition-duration: 0ms;
    }
  }

  @media (forced-colors: active) {
    .tooltip {
      outline: 1px solid CanvasText;
    }
  }
`;
