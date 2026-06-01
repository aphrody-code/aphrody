/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 data table styles. The container is a `surface` with rounded
 * corners, hidden overflow and elevation level 1. Header cells use the
 * `title-small` typescale on `on-surface`; data cells use `body-medium`. Rows
 * are separated by `outline-variant` hairlines and gain an `on-surface` 8 %
 * state layer on hover. Numeric columns are end-aligned.
 */
export const styles = css`
  :host {
    display: block;
    --_container-color: var(--md-table-container-color, var(--md-sys-color-surface, #fef7ff));
    --_headline-color: var(--md-table-headline-color, var(--md-sys-color-on-surface, #1d1b20));
    --_body-color: var(--md-table-body-color, var(--md-sys-color-on-surface, #1d1b20));
    --_divider-color: var(--md-table-divider-color, var(--md-sys-color-outline-variant, #cac4d0));
    --_container-shape: var(--md-table-container-shape, var(--md-sys-shape-corner-medium, 12px));
    --_accent-color: var(--md-table-accent-color, var(--md-sys-color-primary, #6750a4));
  }

  .container {
    box-sizing: border-box;
    width: 100%;
    overflow: hidden;
    border-radius: var(--_container-shape);
    background: var(--_container-color);
    color: var(--_body-color);
    /* M3 elevation level 1. */
    box-shadow: var(
      --md-sys-elevation-level1,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15)
    );
  }

  .table-scroll {
    width: 100%;
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    table-layout: auto;
  }

  thead tr {
    height: 56px;
  }

  th {
    box-sizing: border-box;
    height: 56px;
    padding: 0 16px;
    text-align: start;
    vertical-align: middle;
    color: var(--_headline-color);
    border-bottom: 1px solid var(--_divider-color);
    font-family: var(
      --md-sys-typescale-title-small-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-title-small-size, 14px);
    line-height: var(--md-sys-typescale-title-small-line-height, 20px);
    letter-spacing: var(--md-sys-typescale-title-small-tracking, 0.1px);
    font-weight: var(--md-sys-typescale-title-small-weight, 500);
    -webkit-user-select: none;
    user-select: none;
  }

  th.numeric {
    text-align: end;
  }

  th.sortable {
    cursor: pointer;
  }

  th.sortable:hover {
    background: color-mix(in srgb, var(--_headline-color) 8%, transparent);
  }

  th.sortable:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: -2px;
  }

  th.sorted {
    color: var(--_accent-color);
  }

  .header-content {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  th.numeric .header-content {
    flex-direction: row-reverse;
  }

  .sort-arrow {
    font-size: 12px;
    line-height: 1;
    color: var(--_accent-color);
  }

  tbody tr {
    height: 52px;
    transition: background 80ms linear;
  }

  tbody tr:not(:last-child) td {
    border-bottom: 1px solid var(--_divider-color);
  }

  tbody tr:hover {
    background: color-mix(in srgb, var(--_body-color) 8%, transparent);
  }

  tbody tr.selected {
    background: color-mix(in srgb, var(--_accent-color) 8%, transparent);
  }

  tbody tr.selected:hover {
    background: color-mix(in srgb, var(--_accent-color) 12%, transparent);
  }

  td {
    box-sizing: border-box;
    height: 52px;
    padding: 0 16px;
    text-align: start;
    vertical-align: middle;
    color: var(--_body-color);
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

  td.numeric {
    text-align: end;
    font-variant-numeric: tabular-nums;
  }

  .checkbox-cell {
    width: 56px;
    padding: 0 16px;
    text-align: center;
  }

  .empty-cell {
    text-align: center;
    color: var(--md-sys-color-on-surface-variant, #49454f);
    padding: 24px 16px;
  }

  /* Autonomous checkbox — no md-checkbox dependency. */
  .md-checkbox {
    appearance: none;
    -webkit-appearance: none;
    box-sizing: border-box;
    margin: 0;
    width: 18px;
    height: 18px;
    border: 2px solid var(--md-sys-color-on-surface-variant, #49454f);
    border-radius: 2px;
    background: transparent;
    cursor: pointer;
    position: relative;
    vertical-align: middle;
    transition:
      background 80ms linear,
      border-color 80ms linear;
  }

  .md-checkbox:checked,
  .md-checkbox:indeterminate {
    background: var(--_accent-color);
    border-color: var(--_accent-color);
  }

  .md-checkbox:checked::after {
    content: "";
    position: absolute;
    inset: 0;
    margin: auto;
    width: 5px;
    height: 9px;
    border: solid var(--md-sys-color-on-primary, #ffffff);
    border-width: 0 2px 2px 0;
    transform: translateY(-1px) rotate(45deg);
  }

  .md-checkbox:indeterminate::after {
    content: "";
    position: absolute;
    inset: 0;
    margin: auto;
    width: 10px;
    height: 2px;
    background: var(--md-sys-color-on-primary, #ffffff);
  }

  .md-checkbox:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: 2px;
  }

  /* ----------------------------------------------------------- Cell editing */

  td.editable {
    cursor: text;
  }

  td.editable:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: -2px;
  }

  td.editing {
    padding: 0 8px;
  }

  .cell-editor {
    box-sizing: border-box;
    width: 100%;
    margin: 0;
    padding: 6px 8px;
    border: 1px solid var(--_accent-color);
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: var(--_container-color);
    color: var(--_body-color);
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

  td.numeric .cell-editor {
    text-align: end;
    font-variant-numeric: tabular-nums;
  }

  .cell-editor:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: 1px;
  }

  @media (forced-colors: active) {
    .cell-editor {
      border-color: CanvasText;
    }

    .cell-editor:focus-visible {
      outline-color: Highlight;
    }
  }

  /* ---------------------------------------------------------- Sort ordinal */

  .sort-ord {
    font-size: 9px;
    line-height: 1;
    vertical-align: super;
    margin-inline-start: 1px;
  }

  /* ----------------------------------------------------------- Toolbar */

  .toolbar {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    min-height: 56px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--_divider-color);
  }

  .search-field {
    position: relative;
    display: inline-flex;
    align-items: center;
    flex: 0 1 280px;
  }

  .search-icon {
    position: absolute;
    inset-inline-start: 8px;
    width: 18px;
    height: 18px;
    fill: var(--md-sys-color-on-surface-variant, #49454f);
    pointer-events: none;
  }

  .search-input {
    box-sizing: border-box;
    width: 100%;
    padding: 6px 10px 6px 32px;
    border: 1px solid var(--md-sys-color-outline, #79747e);
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: transparent;
    color: var(--_body-color);
    font-family: var(
      --md-sys-typescale-body-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-medium-size, 14px);
  }

  .search-input:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: -1px;
  }

  /* ----------------------------------------------------------- Filter row */

  .filter-row th {
    height: auto;
    padding: 6px 16px;
    border-bottom: 1px solid var(--_divider-color);
  }

  .filter-control {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    width: 100%;
  }

  .filter-input,
  .filter-op {
    box-sizing: border-box;
    padding: 4px 6px;
    border: 1px solid var(--md-sys-color-outline, #79747e);
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: transparent;
    color: var(--_body-color);
    font-family: inherit;
    font-size: var(--md-sys-typescale-body-small-size, 12px);
    font-weight: 400;
  }

  .filter-input {
    flex: 1 1 auto;
    min-width: 0;
  }

  .filter-op {
    flex: 0 0 auto;
    cursor: pointer;
  }

  .filter-input:focus-visible,
  .filter-op:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: -1px;
  }

  /* ----------------------------------------------------------- Resize handle */

  th {
    position: relative;
  }

  .resize-handle {
    position: absolute;
    inset-block: 0;
    inset-inline-end: 0;
    width: 8px;
    cursor: col-resize;
    -webkit-user-select: none;
    user-select: none;
    touch-action: none;
  }

  .resize-handle::after {
    content: "";
    position: absolute;
    inset-block: 25%;
    inset-inline-end: 3px;
    width: 1px;
    background: var(--_divider-color);
  }

  .resize-handle:hover::after {
    background: var(--_accent-color);
  }

  th[draggable="true"] {
    cursor: grab;
  }

  /* ----------------------------------------------------------- Paginator */

  .paginator {
    box-sizing: border-box;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 16px;
    min-height: 56px;
    padding: 0 8px;
    border-top: 1px solid var(--_divider-color);
    color: var(--md-sys-color-on-surface-variant, #49454f);
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
    color: inherit;
    font: inherit;
    cursor: pointer;
    min-width: 56px;
  }

  .select:hover {
    background: color-mix(in srgb, currentColor 8%, transparent);
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
    color: var(--md-sys-color-on-surface-variant, #49454f);
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
    color: var(--md-sys-color-on-surface-variant, #49454f);
    cursor: pointer;
    transition: background 80ms linear;
  }

  .icon-button:hover:not(:disabled) {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  .icon-button:focus-visible {
    outline: 2px solid var(--_accent-color);
    outline-offset: 2px;
  }

  .icon-button:disabled {
    color: color-mix(in srgb, var(--md-sys-color-on-surface, #1d1b20) 38%, transparent);
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
    .container {
      outline: 1px solid CanvasText;
    }

    th,
    tbody tr:not(:last-child) td {
      border-bottom-color: CanvasText;
    }

    .md-checkbox {
      border-color: CanvasText;
    }

    .md-checkbox:checked,
    .md-checkbox:indeterminate {
      background: Highlight;
      border-color: Highlight;
    }

    .search-input,
    .filter-input,
    .filter-op,
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
