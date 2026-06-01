/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 scheduler styles. Self-contained on the `--md-sys-*` tokens with
 * `--md-scheduler-*` per-instance overrides. Lays out a toolbar plus either a
 * month grid (6×7 cells) or a day/week time grid (24 hour rows + absolutely
 * positioned event chips). Honours `prefers-reduced-motion`.
 */
export const styles = css`
  :host {
    display: block;
    box-sizing: border-box;
    height: 100%;
    --_surface-color: var(--md-scheduler-surface-color, var(--md-sys-color-surface, #fef7ff));
    --_on-surface-color: var(
      --md-scheduler-on-surface-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );
    --_outline-color: var(
      --md-scheduler-outline-color,
      var(--md-sys-color-outline-variant, #cac4d0)
    );
    --_primary-color: var(--md-scheduler-primary-color, var(--md-sys-color-primary, #6750a4));
    --_on-primary-color: var(
      --md-scheduler-on-primary-color,
      var(--md-sys-color-on-primary, #ffffff)
    );
    --_today-color: var(--md-scheduler-today-color, var(--md-sys-color-primary-container, #eaddff));
    --_on-today-color: var(
      --md-scheduler-on-today-color,
      var(--md-sys-color-on-primary-container, #21005d)
    );
    --_event-color: var(
      --md-scheduler-event-color,
      var(--md-sys-color-secondary-container, #e8def8)
    );
    --_on-event-color: var(
      --md-scheduler-on-event-color,
      var(--md-sys-color-on-secondary-container, #1d192b)
    );
    --_shape: var(--md-scheduler-container-shape, var(--md-sys-shape-corner-medium, 12px));
    --_font: var(
      --md-sys-typescale-body-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    color: var(--_on-surface-color);
    font-family: var(--_font);
  }

  *,
  *::before,
  *::after {
    box-sizing: border-box;
  }

  .scheduler {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--_surface-color);
    border-radius: var(--_shape);
  }

  /* Toolbar */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--_outline-color);
    flex-wrap: wrap;
  }

  .nav-group {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .title {
    flex: 1;
    text-align: center;
    font-size: var(--md-sys-typescale-title-medium-size, 16px);
    font-weight: var(--md-sys-typescale-title-medium-weight, 500);
  }

  .nav,
  .today,
  .view-tab {
    appearance: none;
    border: none;
    background: none;
    cursor: pointer;
    color: inherit;
    font-family: inherit;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    transition: background-color 150ms ease;
  }

  .nav {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    padding: 8px;
  }

  .nav svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  .today {
    height: 40px;
    padding: 0 16px;
    border: 1px solid var(--_outline-color);
    font-size: var(--md-sys-typescale-label-large-size, 14px);
    font-weight: 500;
  }

  .views {
    display: flex;
    gap: 4px;
  }

  .view-tab {
    height: 36px;
    padding: 0 14px;
    font-size: var(--md-sys-typescale-label-large-size, 14px);
    font-weight: 500;
  }

  .view-tab.selected {
    background: var(--_primary-color);
    color: var(--_on-primary-color);
  }

  .nav:hover,
  .today:hover,
  .view-tab:not(.selected):hover {
    background: color-mix(in srgb, var(--_on-surface-color) 8%, transparent);
  }

  .nav:focus-visible,
  .today:focus-visible,
  .view-tab:focus-visible {
    outline: 2px solid var(--_primary-color);
    outline-offset: 2px;
  }

  /* Month view */
  .month {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    border-bottom: 1px solid var(--_outline-color);
  }

  .weekday {
    padding: 8px 4px;
    text-align: center;
    font-size: var(--md-sys-typescale-label-medium-size, 12px);
    font-weight: 500;
    opacity: 0.7;
  }

  .month-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-auto-rows: minmax(80px, 1fr);
    flex: 1;
    min-height: 0;
  }

  .month-cell {
    border-right: 1px solid var(--_outline-color);
    border-bottom: 1px solid var(--_outline-color);
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    cursor: pointer;
    overflow: hidden;
  }

  .month-cell:nth-child(7n) {
    border-right: none;
  }

  .month-cell:hover {
    background: color-mix(in srgb, var(--_on-surface-color) 4%, transparent);
  }

  .month-cell:focus-visible {
    outline: 2px solid var(--_primary-color);
    outline-offset: -2px;
  }

  .month-cell.out-of-month {
    opacity: 0.45;
  }

  .month-day-number {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 6px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    font-size: var(--md-sys-typescale-label-large-size, 14px);
  }

  .month-cell.today .month-day-number {
    background: var(--_today-color);
    color: var(--_on-today-color);
    font-weight: 600;
  }

  .month-events {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }

  /* Time grid (day/week) */
  .time-grid {
    display: grid;
    grid-template-columns: 56px 1fr;
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  .time-axis {
    display: grid;
    grid-template-rows: 32px repeat(24, 48px);
    border-right: 1px solid var(--_outline-color);
  }

  .hour-label {
    grid-row: span 1;
    text-align: right;
    padding: 2px 6px 0 0;
    font-size: var(--md-sys-typescale-label-small-size, 11px);
    opacity: 0.7;
    transform: translateY(-6px);
  }

  /* offset the hour labels so they align with the slot border lines */
  .time-axis .hour-label:first-of-type {
    transform: none;
  }

  .columns {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    grid-template-rows: 32px 1fr;
  }

  .column-header {
    grid-row: 1;
    position: sticky;
    top: 0;
    z-index: 1;
    text-align: center;
    padding: 6px 4px;
    font-size: var(--md-sys-typescale-label-medium-size, 12px);
    font-weight: 500;
    background: var(--_surface-color);
    border-right: 1px solid var(--_outline-color);
    border-bottom: 1px solid var(--_outline-color);
  }

  .column-header.today {
    color: var(--_primary-color);
    font-weight: 700;
  }

  .column-body {
    grid-row: 2;
    position: relative;
    border-right: 1px solid var(--_outline-color);
    display: grid;
    grid-template-rows: repeat(24, 48px);
  }

  .hour-slot {
    border-bottom: 1px solid var(--_outline-color);
    cursor: pointer;
  }

  .hour-slot:hover {
    background: color-mix(in srgb, var(--_on-surface-color) 4%, transparent);
  }

  .hour-slot:focus-visible {
    outline: 2px solid var(--_primary-color);
    outline-offset: -2px;
  }

  .column-events {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  /* Events */
  .event {
    appearance: none;
    border: none;
    cursor: pointer;
    font-family: inherit;
    text-align: left;
    background: var(--_event-color);
    color: var(--_on-event-color);
    border-radius: var(--md-sys-shape-corner-small, 8px);
    overflow: hidden;
    transition: filter 150ms ease;
  }

  .event:hover {
    filter: brightness(0.95);
  }

  .event:focus-visible {
    outline: 2px solid var(--_primary-color);
    outline-offset: 1px;
  }

  .event .event-title {
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .event.chip {
    width: 100%;
    padding: 1px 6px;
    font-size: var(--md-sys-typescale-label-small-size, 11px);
    line-height: 1.4;
  }

  .event.positioned {
    position: absolute;
    left: 2px;
    right: 2px;
    pointer-events: auto;
    padding: 2px 6px;
    font-size: var(--md-sys-typescale-label-small-size, 11px);
    box-shadow: var(--md-sys-elevation-level1, 0 1px 2px rgba(0, 0, 0, 0.3));
  }

  @media (prefers-reduced-motion: reduce) {
    .nav,
    .today,
    .view-tab,
    .event {
      transition-duration: 0ms;
    }
  }

  @media (forced-colors: active) {
    .month-cell,
    .column-body,
    .event {
      outline: 1px solid CanvasText;
    }
  }
`;
