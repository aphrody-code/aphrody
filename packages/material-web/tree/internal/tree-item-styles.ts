/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 tree-item styles. Each row is a full-radius (`corner-full`)
 * interactive surface with a hover/focus state layer (`color-mix`) and a
 * `secondary-container` background when selected. The chevron rotates 90° when
 * expanded. Consumes the `--md-sys-*` tokens; per-instance overrides via
 * `--md-tree-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_label-color: var(--md-tree-label-color, var(--md-sys-color-on-surface, #1d1b20));
    --_icon-color: var(--md-tree-icon-color, var(--md-sys-color-on-surface-variant, #49454f));
    --_selected-container-color: var(
      --md-tree-selected-container-color,
      var(--md-sys-color-secondary-container, #e8def8)
    );
    --_selected-label-color: var(
      --md-tree-selected-label-color,
      var(--md-sys-color-on-secondary-container, #1d192b)
    );
    font-family: var(
      --md-sys-typescale-body-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 40px;
    padding-block: 4px;
    padding-inline-end: 12px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    color: var(--_label-color);
    cursor: pointer;
    position: relative;
    user-select: none;
  }

  .row:hover {
    background: color-mix(in srgb, var(--_label-color) 8%, transparent);
  }

  :host([selected]) > .row {
    background: var(--_selected-container-color);
    color: var(--_selected-label-color);
  }

  :host([selected]) > .row:hover {
    background: color-mix(
      in srgb,
      var(--_selected-label-color) 8%,
      var(--_selected-container-color)
    );
  }

  /* Roving-focus target lives on the host; show a ring when focused. */
  :host(:focus-visible) {
    outline: none;
  }

  :host(:focus-visible) > .row {
    outline: 2px solid var(--md-sys-color-primary, #6750a4);
    outline-offset: -2px;
  }

  :host([disabled]) > .row {
    cursor: default;
    opacity: 0.38;
  }

  :host([disabled]) > .row:hover {
    background: none;
  }

  .checkbox {
    flex: 0 0 auto;
    margin-inline-end: 4px;
  }

  /* The label glyph rendered for data-driven icon strings (a ligature font). */
  .icon ::slotted(.data-icon),
  .icon .data-icon {
    font-family: "Material Symbols Outlined", "Material Icons", sans-serif;
    font-size: 20px;
    line-height: 1;
  }

  .chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    flex: 0 0 auto;
    color: var(--_icon-color);
    transition: transform 150ms cubic-bezier(0.2, 0, 0, 1);
  }

  :host([selected]) > .row .chevron {
    color: var(--_selected-label-color);
  }

  :host([expanded]) > .row .chevron.has-children {
    transform: rotate(90deg);
  }

  .chevron-icon {
    width: 20px;
    height: 20px;
    fill: currentColor;
  }

  .icon {
    display: inline-flex;
    align-items: center;
    color: var(--_icon-color);
  }

  :host([selected]) > .row .icon {
    color: var(--_selected-label-color);
  }

  .label {
    flex: 1 1 auto;
    font-size: var(--md-sys-typescale-body-large-size, 16px);
    line-height: var(--md-sys-typescale-body-large-line-height, 24px);
    letter-spacing: var(--md-sys-typescale-body-large-tracking, 0.5px);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .label-editor {
    flex: 1 1 auto;
    box-sizing: border-box;
    min-width: 0;
    margin: 0;
    padding: 2px 8px;
    border: 1px solid var(--md-sys-color-primary, #6750a4);
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    background: var(--md-sys-color-surface, #fef7ff);
    color: var(--_label-color);
    font-family: inherit;
    font-size: var(--md-sys-typescale-body-large-size, 16px);
    line-height: var(--md-sys-typescale-body-large-line-height, 24px);
    letter-spacing: var(--md-sys-typescale-body-large-tracking, 0.5px);
  }

  .label-editor:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #6750a4);
    outline-offset: 1px;
  }

  .children[hidden] {
    display: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .chevron {
      transition-duration: 1ms;
    }
  }

  @media (forced-colors: active) {
    :host([selected]) > .row {
      background: Highlight;
      color: HighlightText;
    }
    :host(:focus-visible) > .row {
      outline-color: Highlight;
    }
    .label-editor {
      border-color: CanvasText;
    }
    .label-editor:focus-visible {
      outline-color: Highlight;
    }
  }
`;
