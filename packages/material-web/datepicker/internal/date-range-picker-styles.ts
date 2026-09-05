/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 docked date *range* picker styles. Shares the container/header/grid
 * layout with the single date picker, adding a connected range highlight: the
 * `in-range` days draw a `secondary-container` band edge-to-edge, and the
 * `range-start`/`range-end` days draw a `primary` circle over a half-band so the
 * selection reads as one continuous pill. Consumes `--md-sys-*` tokens.
 */
export const styles = css`
  :host {
    --_container-color: var(
      --md-date-range-picker-container-color,
      var(--md-sys-color-surface-container-high, #ece6f0)
    );
    --_on-surface: var(
      --md-date-range-picker-on-surface-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );
    --_primary: var(--md-date-range-picker-selected-color, var(--md-sys-color-primary, #6750a4));
    --_on-primary: var(
      --md-date-range-picker-on-selected-color,
      var(--md-sys-color-on-primary, #ffffff)
    );
    --_range-band: var(
      --md-date-range-picker-range-color,
      var(--md-sys-color-secondary-container, #e8def8)
    );
    --_on-range-band: var(
      --md-date-range-picker-on-range-color,
      var(--md-sys-color-on-secondary-container, #1d192b)
    );
    --_container-shape: var(
      --md-date-range-picker-container-shape,
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

  .fields {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 4px 8px;
  }

  .field {
    flex: 1 1 0;
    box-sizing: border-box;
    padding: 8px 12px;
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: var(--md-sys-color-surface-container-highest, #e6e0e9);
    border-bottom: 1px solid var(--md-sys-color-on-surface-variant, #49454f);
  }

  .field-sep {
    opacity: 0.7;
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
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 40px;
    padding: 0;
    color: var(--_on-surface);
    cursor: pointer;
    font-family: var(
      --md-sys-typescale-body-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-large-size, 16px);
  }

  .day-num {
    position: relative;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
  }

  /* The connected range band behind in-range and edge days. */
  .day-bg {
    position: absolute;
    inset-block: 4px;
    inset-inline: 0;
    z-index: 0;
  }

  button.day.in-range .day-bg {
    background: var(--_range-band);
  }

  button.day.in-range {
    color: var(--_on-range-band);
  }

  /* Half-bands so the edge circles connect into the band. */
  button.day.range-start:not(.range-end) .day-bg {
    background: linear-gradient(to right, transparent 50%, var(--_range-band) 50%);
  }

  button.day.range-end:not(.range-start) .day-bg {
    background: linear-gradient(to left, transparent 50%, var(--_range-band) 50%);
  }

  button.day.range-start .day-num,
  button.day.range-end .day-num {
    background: var(--_primary);
    color: var(--_on-primary);
  }

  button.day:hover:not(.range-start):not(.range-end):not(:disabled) .day-num {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  button.day:focus-visible {
    outline: none;
  }

  button.day:focus-visible .day-num {
    outline: 2px solid var(--_primary);
    outline-offset: 2px;
  }

  button.day.today .day-num {
    border: 1px solid var(--_primary);
  }

  button.day:disabled {
    opacity: 0.38;
    cursor: default;
  }

  @media (forced-colors: active) {
    .picker {
      outline: 1px solid CanvasText;
    }
    button.day.range-start .day-num,
    button.day.range-end .day-num {
      outline: 2px solid CanvasText;
    }
  }
`;
