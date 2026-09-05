/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 time-picker (input variant) styles. Large numeric fields on a
 * `surface-container-highest` container with an 8dp radius and a `primary`
 * 2px focus border; a vertical AM/PM segmented toggle in 12-hour mode.
 */
export const styles = css`
  :host {
    --_field-color: var(
      --md-time-picker-field-color,
      var(--md-sys-color-surface-container-highest, #e6e0e9)
    );
    --_on-surface: var(--md-time-picker-on-surface-color, var(--md-sys-color-on-surface, #1d1b20));
    --_primary: var(--md-time-picker-focus-color, var(--md-sys-color-primary, #6750a4));
    --_selected-container: var(
      --md-time-picker-selected-container-color,
      var(--md-sys-color-tertiary-container, #ffd8e4)
    );
    --_selected-label: var(
      --md-time-picker-selected-label-color,
      var(--md-sys-color-on-tertiary-container, #31111d)
    );
    --_field-shape: var(--md-time-picker-field-shape, var(--md-sys-shape-corner-small, 8px));

    display: inline-block;
    color: var(--_on-surface);
  }

  .picker {
    display: inline-flex;
    align-items: center;
    gap: 12px;
  }

  .text-field {
    box-sizing: border-box;
    display: inline-block;
    min-width: 160px;
    padding: 8px 12px;
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: var(--_field-color);
    border-bottom: 1px solid var(--md-sys-color-on-surface-variant, #49454f);
  }

  .text-field.invalid {
    border-bottom-color: var(--md-sys-color-error, #b3261e);
  }

  .text-field-input {
    width: 100%;
    box-sizing: border-box;
    appearance: none;
    border: none;
    background: none;
    outline: none;
    color: var(--_on-surface);
    font-family: var(
      --md-sys-typescale-body-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-large-size, 16px);
    line-height: var(--md-sys-typescale-body-large-line-height, 24px);
    letter-spacing: var(--md-sys-typescale-body-large-tracking, 0.5px);
  }

  .text-field.invalid .text-field-input {
    color: var(--md-sys-color-error, #b3261e);
  }

  .fields {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .field {
    box-sizing: border-box;
    width: 96px;
    height: 80px;
    border: 2px solid transparent;
    border-radius: var(--_field-shape);
    background: var(--_field-color);
    color: var(--_on-surface);
    text-align: center;
    /* M3 display-medium-ish — large numerals. */
    font-family: var(
      --md-sys-typescale-display-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-display-medium-size, 45px);
    line-height: var(--md-sys-typescale-display-medium-line-height, 52px);
    font-weight: 400;
    caret-color: var(--_primary);
  }

  .field:focus {
    outline: none;
    border-color: var(--_primary);
  }

  .separator {
    font-size: 45px;
    line-height: 1;
    font-weight: 400;
  }

  .ampm {
    display: inline-flex;
    flex-direction: column;
    border: 1px solid color-mix(in srgb, var(--_on-surface) 30%, transparent);
    border-radius: var(--md-sys-shape-corner-small, 8px);
    overflow: hidden;
  }

  .ampm-button {
    appearance: none;
    border: none;
    background: none;
    width: 52px;
    height: 38px;
    color: var(--_on-surface);
    cursor: pointer;
    font-family: var(
      --md-sys-typescale-title-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-title-medium-size, 16px);
    font-weight: var(--md-sys-typescale-title-medium-weight, 500);
  }

  .ampm-button.top {
    border-bottom: 1px solid color-mix(in srgb, var(--_on-surface) 30%, transparent);
  }

  .ampm-button:hover:not(.on) {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  .ampm-button:focus-visible {
    outline: 2px solid var(--_primary);
    outline-offset: -2px;
  }

  .ampm-button.on {
    background: var(--_selected-container);
    color: var(--_selected-label);
  }

  @media (forced-colors: active) {
    .field {
      border-color: CanvasText;
    }
    .ampm-button.on {
      outline: 2px solid CanvasText;
      outline-offset: -2px;
    }
  }
`;
