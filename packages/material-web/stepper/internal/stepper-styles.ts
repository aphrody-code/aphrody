/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 stepper styles. The header renders a row (horizontal) or column
 * (vertical) of numbered indicators connected by `outline-variant` lines. The
 * active/completed indicator uses the `primary` container; inactive uses
 * `surface-variant`. Consumes the `--md-sys-*` tokens; per-instance overrides
 * via `--md-stepper-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_active-indicator-color: var(
      --md-stepper-active-indicator-color,
      var(--md-sys-color-primary, #6750a4)
    );
    --_active-indicator-text-color: var(
      --md-stepper-active-indicator-text-color,
      var(--md-sys-color-on-primary, #ffffff)
    );
    --_inactive-indicator-color: var(
      --md-stepper-inactive-indicator-color,
      var(--md-sys-color-surface-variant, #e7e0ec)
    );
    --_inactive-indicator-text-color: var(
      --md-stepper-inactive-indicator-text-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_label-color: var(--md-stepper-label-color, var(--md-sys-color-on-surface, #1d1b20));
    --_optional-color: var(
      --md-stepper-optional-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_connector-color: var(
      --md-stepper-connector-color,
      var(--md-sys-color-outline-variant, #cac4d0)
    );
    font-family: var(
      --md-sys-typescale-title-small-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
  }

  .header {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0;
    padding: 8px 0;
  }

  .header.vertical {
    flex-direction: column;
    align-items: stretch;
  }

  .step-header {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--md-sys-shape-corner-medium, 12px);
    cursor: pointer;
    user-select: none;
    background: none;
    border: none;
    flex: 0 0 auto;
    position: relative;
  }

  .step-header:hover {
    background: color-mix(in srgb, var(--_label-color) 8%, transparent);
  }

  .step-header:focus-visible {
    outline: 2px solid var(--_active-indicator-color);
    outline-offset: -2px;
  }

  .step-header[aria-disabled="true"] {
    cursor: default;
    opacity: 0.55;
  }

  .step-header[aria-disabled="true"]:hover {
    background: none;
  }

  .indicator {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    background: var(--_inactive-indicator-color);
    color: var(--_inactive-indicator-text-color);
    flex: 0 0 auto;
    transition:
      background-color 150ms cubic-bezier(0.2, 0, 0, 1),
      color 150ms cubic-bezier(0.2, 0, 0, 1);
  }

  .state-active .indicator,
  .state-completed .indicator {
    background: var(--_active-indicator-color);
    color: var(--_active-indicator-text-color);
  }

  .number {
    font-size: var(--md-sys-typescale-label-medium-size, 12px);
    font-weight: var(--md-sys-typescale-label-medium-weight, 500);
    line-height: 1;
  }

  .check {
    width: 18px;
    height: 18px;
    fill: currentColor;
  }

  .labels {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    line-height: 1.1;
  }

  .label {
    font-size: var(--md-sys-typescale-title-small-size, 14px);
    font-weight: var(--md-sys-typescale-title-small-weight, 500);
    letter-spacing: var(--md-sys-typescale-title-small-tracking, 0.1px);
    color: var(--_label-color);
  }

  .state-inactive .label {
    color: var(--_optional-color);
  }

  .optional {
    font-size: var(--md-sys-typescale-label-small-size, 11px);
    color: var(--_optional-color);
  }

  /* Horizontal connector: a thin line between adjacent step headers. */
  .connector {
    flex: 1 1 auto;
    min-width: 16px;
    height: 1px;
    background: var(--_connector-color);
    margin: 0 4px;
  }

  /* Vertical connector: a short vertical line under the indicator column. */
  .header.vertical .connector {
    width: 1px;
    min-width: 0;
    height: 16px;
    min-height: 16px;
    margin: 0 0 0 24px;
    flex: 0 0 auto;
  }

  .content {
    padding: 8px 12px 16px;
    color: var(--md-sys-color-on-surface, #1d1b20);
    font-family: var(
      --md-sys-typescale-body-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
  }

  @media (prefers-reduced-motion: reduce) {
    .indicator {
      transition-duration: 1ms;
    }
  }

  @media (forced-colors: active) {
    .indicator {
      outline: 1px solid CanvasText;
    }
    .state-active .indicator,
    .state-completed .indicator {
      background: Highlight;
      color: HighlightText;
    }
    .connector {
      background: CanvasText;
    }
    .step-header:focus-visible {
      outline-color: Highlight;
    }
  }
`;
