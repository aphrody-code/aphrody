/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 snackbar styles. The surface is a top-layer popover, positioned at
 * the bottom centre and animated in/out with `@starting-style` +
 * `transition-behavior: allow-discrete` (transitioning `overlay` so it stays in
 * the top layer through the exit). Consumes the `--md-sys-*` tokens; per-
 * instance overrides via `--md-snackbar-*`.
 */
export const styles = css`
  :host {
    display: contents;
    --_container-color: var(
      --md-snackbar-container-color,
      var(--md-sys-color-inverse-surface, #322f35)
    );
    --_supporting-text-color: var(
      --md-snackbar-supporting-text-color,
      var(--md-sys-color-inverse-on-surface, #f5eff7)
    );
    --_action-color: var(--md-snackbar-action-color, var(--md-sys-color-inverse-primary, #d0bcff));
    --_container-shape: var(
      --md-snackbar-container-shape,
      var(--md-sys-shape-corner-extra-small, 4px)
    );
  }

  /* The popover surface (top layer). */
  .snackbar {
    position: fixed;
    inset: auto;
    inset-block-end: 16px;
    inset-inline: 0;
    margin: 0 auto;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 48px;
    min-width: 344px;
    width: max-content;
    max-width: min(672px, calc(100vw - 32px));
    padding-block: 6px;
    padding-inline: 16px 8px;
    border: none;
    border-radius: var(--_container-shape);
    background: var(--_container-color);
    color: var(--_supporting-text-color);
    /* M3 elevation level 3 */
    box-shadow: var(
      --md-sys-elevation-level3,
      0 1px 3px 0 rgba(0, 0, 0, 0.3),
      0 4px 8px 3px rgba(0, 0, 0, 0.15)
    );
    /* Closed / exit state. */
    opacity: 0;
    transform: translateY(16px) scale(0.95);
    transition:
      opacity 200ms cubic-bezier(0.3, 0, 0.8, 0.15),
      transform 200ms cubic-bezier(0.3, 0, 0.8, 0.15),
      display 200ms allow-discrete,
      overlay 200ms allow-discrete;
  }

  /* Open state. */
  .snackbar:popover-open {
    opacity: 1;
    transform: translateY(0) scale(1);
    transition:
      opacity 300ms cubic-bezier(0.05, 0.7, 0.1, 1),
      transform 300ms cubic-bezier(0.05, 0.7, 0.1, 1),
      display 300ms allow-discrete,
      overlay 300ms allow-discrete;
  }

  /* Entry origin (must follow the open state). */
  @starting-style {
    .snackbar:popover-open {
      opacity: 0;
      transform: translateY(16px) scale(0.95);
    }
  }

  .snackbar.stacked {
    flex-direction: column;
    align-items: flex-start;
    padding-block: 12px;
    padding-inline: 16px;
  }

  .text {
    flex: 1;
    padding-block: 8px;
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
    font-weight: var(--md-sys-typescale-body-medium-weight, 400);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .stacked .actions {
    align-self: flex-end;
  }

  ::slotted([slot="action"]) {
    color: var(--_action-color);
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
    --md-text-button-label-text-color: var(--_action-color);
  }

  ::slotted(button[slot="action"]),
  ::slotted(a[slot="action"]) {
    appearance: none;
    border: none;
    background: none;
    cursor: pointer;
    padding: 10px 12px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    text-decoration: none;
  }

  .close {
    appearance: none;
    border: none;
    background: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    padding: 8px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    color: var(--_supporting-text-color);
    cursor: pointer;
  }

  .close:hover {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  .close:focus-visible {
    outline: 2px solid var(--_action-color);
    outline-offset: 2px;
  }

  .close svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  @media (prefers-reduced-motion: reduce) {
    .snackbar,
    .snackbar:popover-open {
      transform: none;
      transition-duration: 100ms;
    }
    @starting-style {
      .snackbar:popover-open {
        transform: none;
      }
    }
  }

  @media (forced-colors: active) {
    .snackbar {
      outline: 1px solid CanvasText;
    }
  }
`;
