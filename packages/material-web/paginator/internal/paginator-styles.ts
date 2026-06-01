/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 paginator styles. The bar is a `surface` row, 56px tall, using the
 * `body-small` typescale on `on-surface-variant`. The page-size selector is a
 * native `<select>` themed with tokens; navigation uses autonomous icon buttons
 * (40px target) with an `on-surface` 8 % hover state layer and a disabled state
 * at the bounds.
 */
export const styles = css`
  :host {
    display: block;
    --_container-color: var(--md-paginator-container-color, var(--md-sys-color-surface, #fef7ff));
    --_text-color: var(--md-paginator-text-color, var(--md-sys-color-on-surface-variant, #49454f));
    --_icon-color: var(--md-paginator-icon-color, var(--md-sys-color-on-surface-variant, #49454f));
    --_accent-color: var(--md-paginator-accent-color, var(--md-sys-color-primary, #6750a4));
    --_disabled-color: var(
      --md-paginator-disabled-color,
      color-mix(in srgb, var(--md-sys-color-on-surface, #1d1b20) 38%, transparent)
    );
  }

  .paginator {
    box-sizing: border-box;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 16px;
    min-height: 56px;
    padding: 0 8px;
    background: var(--_container-color);
    color: var(--_text-color);
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
  }

  .page-size {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .select-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .select {
    appearance: none;
    -webkit-appearance: none;
    box-sizing: border-box;
    margin: 0;
    padding: 4px 28px 4px 8px;
    border: 1px solid var(--md-sys-color-outline, #79747e);
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: transparent;
    color: var(--_text-color);
    font: inherit;
    cursor: pointer;
    min-width: 56px;
  }

  .select:hover {
    background: color-mix(in srgb, var(--_text-color) 8%, transparent);
  }

  .select:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: 1px;
  }

  .select-arrow {
    position: absolute;
    inset-inline-end: 6px;
    display: inline-flex;
    pointer-events: none;
    color: var(--_icon-color);
  }

  .select-arrow svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
  }

  .range-label {
    white-space: nowrap;
  }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .icon-button {
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
    color: var(--_icon-color);
    cursor: pointer;
    transition: background 80ms linear;
  }

  .icon-button:hover:not(:disabled) {
    background: color-mix(in srgb, var(--_icon-color) 8%, transparent);
  }

  .icon-button:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: 2px;
  }

  .icon-button:disabled {
    color: var(--_disabled-color);
    cursor: default;
  }

  .icon-button svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  @media (prefers-reduced-motion: reduce) {
    .icon-button {
      transition: none;
    }
  }

  @media (forced-colors: active) {
    .select {
      border-color: CanvasText;
    }

    .icon-button:disabled {
      color: GrayText;
    }

    .icon-button:focus-visible,
    .select:focus-visible {
      outline-color: Highlight;
    }
  }
`;
