/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 docked date-picker styles. A `surface-container-high` panel with a
 * 16dp radius, a header with prev/next, a 7-column day grid; the selected day
 * is a `primary` circle and today is outlined with `primary`.
 */
export const styles = css`
  :host {
    --_container-color: var(
      --md-date-picker-container-color,
      var(--md-sys-color-surface-container-high, #ece6f0)
    );
    --_on-surface: var(--md-date-picker-on-surface-color, var(--md-sys-color-on-surface, #1d1b20));
    --_primary: var(--md-date-picker-selected-color, var(--md-sys-color-primary, #6750a4));
    --_on-primary: var(--md-date-picker-on-selected-color, var(--md-sys-color-on-primary, #ffffff));
    --_container-shape: var(
      --md-date-picker-container-shape,
      var(--md-sys-shape-corner-large, 16px)
    );

    display: inline-block;
  }

  .picker {
    box-sizing: border-box;
    width: 328px;
    padding: 12px;
    border-radius: var(--_container-shape);
    background: var(--_container-color);
    color: var(--_on-surface);
    box-shadow: var(
      --md-sys-elevation-level3,
      0 1px 3px 0 rgba(0, 0, 0, 0.3),
      0 4px 8px 3px rgba(0, 0, 0, 0.15)
    );
  }

  .field {
    box-sizing: border-box;
    margin: 4px 4px 8px;
    padding: 8px 12px;
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: var(--md-sys-color-surface-container-highest, #e6e0e9);
    border-bottom: 1px solid var(--md-sys-color-on-surface-variant, #49454f);
  }

  .field.invalid {
    border-bottom-color: var(--md-sys-color-error, #b3261e);
  }

  .field-input {
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

  .field.invalid .field-input {
    color: var(--md-sys-color-error, #b3261e);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-inline: 4px;
    height: 56px;
  }

  .month-label {
    font-family: var(
      --md-sys-typescale-title-small-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-title-small-size, 14px);
    font-weight: var(--md-sys-typescale-title-small-weight, 500);
    letter-spacing: 0.1px;
  }

  .nav {
    appearance: none;
    border: none;
    background: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    color: var(--_on-surface);
    cursor: pointer;
  }

  .nav:hover {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  .nav:focus-visible {
    outline: 2px solid var(--_primary);
    outline-offset: 2px;
  }

  .nav svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  .weekdays,
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
  }

  .weekday {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 40px;
    font-family: var(
      --md-sys-typescale-body-small-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-small-size, 12px);
    font-weight: 500;
    opacity: 0.7;
  }

  .cell {
    box-sizing: border-box;
    height: 40px;
  }

  button.day {
    appearance: none;
    border: none;
    background: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    margin: 0 auto;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    color: var(--_on-surface);
    cursor: pointer;
    /* M3 body-large */
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

  button.day:hover:not(.selected):not(:disabled) {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  button.day:focus-visible {
    outline: 2px solid var(--_primary);
    outline-offset: 2px;
  }

  button.day.today {
    border: 1px solid var(--_primary);
  }

  button.day.selected {
    background: var(--_primary);
    color: var(--_on-primary);
  }

  button.day:disabled {
    opacity: 0.38;
    cursor: default;
  }

  @media (forced-colors: active) {
    .picker {
      outline: 1px solid CanvasText;
    }
    button.day.selected {
      outline: 2px solid CanvasText;
    }
  }
`;
