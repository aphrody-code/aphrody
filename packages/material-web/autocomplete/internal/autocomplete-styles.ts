/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 autocomplete styles. The trigger is an outlined text field (border
 * `outline`, focus `primary` 2px, floating `body-large` label). The suggestions
 * panel is a `surface-container` surface at elevation level 2 with an
 * extra-small top radius, scrolling beyond 280px. Consumes the `--md-sys-*`
 * tokens; per-instance overrides via `--md-autocomplete-*`.
 */
export const styles = css`
  :host {
    display: inline-block;
    position: relative;
    min-width: 210px;
    --_field-text-color: var(--md-autocomplete-text-color, var(--md-sys-color-on-surface, #1d1b20));
    --_field-label-color: var(
      --md-autocomplete-label-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_outline-color: var(--md-autocomplete-outline-color, var(--md-sys-color-outline, #79747e));
    --_focus-color: var(--md-autocomplete-focus-color, var(--md-sys-color-primary, #6750a4));
    --_panel-color: var(
      --md-autocomplete-panel-color,
      var(--md-sys-color-surface-container, #f3edf7)
    );
    --_panel-text-color: var(
      --md-autocomplete-panel-text-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );
    --_panel-shape: var(--md-autocomplete-panel-shape, var(--md-sys-shape-corner-extra-small, 4px));
    font-family: var(
      --md-sys-typescale-body-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
  }

  .field {
    position: relative;
    display: block;
    height: 56px;
  }

  input {
    appearance: none;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 16px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--_field-text-color);
    font-family: inherit;
    font-size: var(--md-sys-typescale-body-large-size, 16px);
    line-height: var(--md-sys-typescale-body-large-line-height, 24px);
    letter-spacing: var(--md-sys-typescale-body-large-tracking, 0.5px);
  }

  .field.disabled input {
    color: color-mix(in srgb, var(--_field-text-color) 38%, transparent);
  }

  .outline {
    position: absolute;
    inset: 0;
    pointer-events: none;
    border: 1px solid var(--_outline-color);
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    transition:
      border-color 150ms cubic-bezier(0.2, 0, 0, 1),
      border-width 150ms cubic-bezier(0.2, 0, 0, 1);
  }

  .label {
    position: absolute;
    inset-inline-start: 12px;
    inset-block-start: 50%;
    transform: translateY(-50%);
    padding: 0 4px;
    background: var(--md-sys-color-surface, #fef7ff);
    color: var(--_field-label-color);
    pointer-events: none;
    font-size: var(--md-sys-typescale-body-large-size, 16px);
    line-height: 1;
    transition:
      transform 150ms cubic-bezier(0.2, 0, 0, 1),
      font-size 150ms cubic-bezier(0.2, 0, 0, 1),
      color 150ms cubic-bezier(0.2, 0, 0, 1);
  }

  /* Float the label when populated or focused. */
  .field.populated .label,
  .field:focus-within .label {
    transform: translateY(-150%);
    font-size: var(--md-sys-typescale-body-small-size, 12px);
  }

  .field:focus-within .label {
    color: var(--_focus-color);
  }

  .field:focus-within .outline {
    border-width: 2px;
    border-color: var(--_focus-color);
  }

  .field.disabled .outline {
    border-color: color-mix(in srgb, var(--_outline-color) 38%, transparent);
  }

  /* The floating suggestions panel. */
  .panel {
    position: absolute;
    inset-inline: 0;
    inset-block-start: calc(100% + 4px);
    z-index: 8;
    margin: 0;
    padding: 8px 0;
    list-style: none;
    max-height: 280px;
    overflow-y: auto;
    box-sizing: border-box;
    background: var(--_panel-color);
    color: var(--_panel-text-color);
    border-radius: var(--_panel-shape) var(--_panel-shape) var(--md-sys-shape-corner-medium, 12px)
      var(--md-sys-shape-corner-medium, 12px);
    /* M3 elevation level 2 */
    box-shadow: var(
      --md-sys-elevation-level2,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 2px 6px 2px rgba(0, 0, 0, 0.15)
    );
  }

  .panel[hidden] {
    display: none;
  }

  .option {
    display: flex;
    align-items: center;
    min-height: 48px;
    padding: 8px 16px;
    cursor: pointer;
    font-size: var(--md-sys-typescale-body-large-size, 16px);
    line-height: var(--md-sys-typescale-body-large-line-height, 24px);
    letter-spacing: var(--md-sys-typescale-body-large-tracking, 0.5px);
    color: var(--_panel-text-color);
  }

  .option:hover {
    background: color-mix(in srgb, var(--_panel-text-color) 8%, transparent);
  }

  .option.active {
    background: color-mix(in srgb, var(--_focus-color) 12%, transparent);
  }

  @media (prefers-reduced-motion: reduce) {
    .outline,
    .label {
      transition-duration: 1ms;
    }
  }

  @media (forced-colors: active) {
    .outline {
      border-color: CanvasText;
    }
    .field:focus-within .outline {
      border-color: Highlight;
    }
    .panel {
      outline: 1px solid CanvasText;
    }
    .option.active {
      background: Highlight;
      color: HighlightText;
    }
  }
`;
