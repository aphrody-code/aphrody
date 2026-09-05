/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 alert styles. Each severity resolves a triplet of `--md-sys-color-*`
 * roles (`--_main` / `--_container` / `--_on-container`) with baseline
 * fallbacks; the `variant` then selects how those roles paint the surface,
 * icon, and outline. Per-instance overrides via `--md-alert-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_container-shape: var(--md-alert-container-shape, var(--md-sys-shape-corner-small, 8px));
    --_main: var(--md-sys-color-primary, #6750a4);
    --_container: var(--md-sys-color-primary-container, #eaddff);
    --_on-container: var(--md-sys-color-on-primary-container, #21005d);
    --_on-main: var(--md-sys-color-on-primary, #fff);
  }

  :host([severity="success"]) {
    --_main: var(--md-sys-color-tertiary, #7d5260);
    --_container: var(--md-sys-color-tertiary-container, #ffd8e4);
    --_on-container: var(--md-sys-color-on-tertiary-container, #31111d);
    --_on-main: var(--md-sys-color-on-tertiary, #fff);
  }

  :host([severity="info"]) {
    --_main: var(--md-sys-color-primary, #6750a4);
    --_container: var(--md-sys-color-primary-container, #eaddff);
    --_on-container: var(--md-sys-color-on-primary-container, #21005d);
    --_on-main: var(--md-sys-color-on-primary, #fff);
  }

  :host([severity="warning"]) {
    --_main: var(--md-sys-color-secondary, #625b71);
    --_container: var(--md-sys-color-secondary-container, #e8def8);
    --_on-container: var(--md-sys-color-on-secondary-container, #1d192b);
    --_on-main: var(--md-sys-color-on-secondary, #fff);
  }

  :host([severity="error"]) {
    --_main: var(--md-sys-color-error, #b3261e);
    --_container: var(--md-sys-color-error-container, #f9dedc);
    --_on-container: var(--md-sys-color-on-error-container, #410e0b);
    --_on-main: var(--md-sys-color-on-error, #fff);
  }

  .alert {
    box-sizing: border-box;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 16px;
    border-radius: var(--_container-shape);
    /* standard variant defaults */
    background: var(--_container);
    color: var(--_on-container);
  }

  /* filled: solid severity color, on-color content. */
  :host([variant="filled"]) .alert {
    background: var(--_main);
    color: var(--_on-main);
  }

  /* outlined: transparent surface, severity outline + text. */
  :host([variant="outlined"]) .alert {
    background: var(--md-sys-color-surface, #fff);
    color: var(--md-sys-color-on-surface, #1d1b20);
    outline: 1px solid var(--_main);
    outline-offset: -1px;
  }

  .icon {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding-block: 2px;
    box-sizing: content-box;
    color: var(--_main);
  }

  /* The default-icon glyph follows the severity main color, except on filled
     where it matches the on-color content. */
  :host([variant="filled"]) .icon {
    color: var(--_on-main);
  }

  .icon svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  .content {
    flex: 1;
    min-width: 0;
    padding-block: 1px;
  }

  .title {
    font-family: var(
      --md-sys-typescale-title-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-title-medium-size, 16px);
    line-height: var(--md-sys-typescale-title-medium-line-height, 24px);
    font-weight: var(--md-sys-typescale-title-medium-weight, 500);
    letter-spacing: var(--md-sys-typescale-title-medium-tracking, 0.15px);
  }

  /* Collapse the title row when nothing is slotted into it. */
  .title:not(:has(::slotted(*))) {
    display: none;
  }

  .message {
    font-family: var(
      --md-sys-typescale-body-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-medium-size, 14px);
    line-height: var(--md-sys-typescale-body-medium-line-height, 20px);
    font-weight: var(--md-sys-typescale-body-medium-weight, 400);
    letter-spacing: var(--md-sys-typescale-body-medium-tracking, 0.25px);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
    align-self: center;
  }

  /* Hide the actions row when empty so it adds no gap. */
  .actions:not(:has(::slotted(*))) {
    display: none;
  }

  ::slotted([slot="action"]) {
    --md-text-button-label-text-color: currentColor;
    color: inherit;
  }

  .close {
    appearance: none;
    border: none;
    background: none;
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 4px;
    margin: -4px -4px -4px 0;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    color: inherit;
    cursor: pointer;
  }

  .close:hover {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  .close:focus-visible {
    outline: 2px solid currentColor;
    outline-offset: 2px;
  }

  .close svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  @media (forced-colors: active) {
    .alert {
      outline: 1px solid CanvasText;
    }
  }
`;
