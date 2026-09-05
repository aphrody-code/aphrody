/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/** Material 3 navigation-rail container styles (collapsed + expanded). */
export const styles = css`
  :host {
    --_container-color: var(
      --md-navigation-rail-container-color,
      var(--md-sys-color-surface, #fef7ff)
    );
    /* M3 Expressive: collapsed rail container width is 96dp (was 80dp). */
    --_width: var(--md-navigation-rail-container-width, 96px);
    --_width-expanded: var(--md-navigation-rail-container-width-expanded, 220px);

    display: block;
    height: 100%;
  }

  .rail {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    height: 100%;
    width: var(--_width);
    padding-block: 44px 56px;
    background: var(--_container-color);
    transition: width 300ms cubic-bezier(0.2, 0, 0, 1);
    overflow: hidden auto;
  }

  :host([expanded]) .rail {
    width: var(--_width-expanded);
    align-items: stretch;
    padding-inline: 20px;
  }

  .leading {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding-inline: 16px;
    margin-block-end: 4px;
  }

  :host([expanded]) .leading {
    align-items: flex-start;
    padding-inline: 0;
  }

  .items {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex: 1;
  }

  :host([alignment="center"]) .items {
    justify-content: center;
  }

  :host([alignment="bottom"]) .items {
    justify-content: flex-end;
  }
`;

/** Material 3 navigation-rail-item styles (active indicator pill + label). */
export const itemStyles = css`
  :host {
    --_active-indicator-color: var(
      --md-navigation-rail-item-active-indicator-color,
      var(--md-sys-color-secondary-container, #e8def8)
    );
    --_active-icon-color: var(
      --md-navigation-rail-item-active-icon-color,
      var(--md-sys-color-on-secondary-container, #1d192b)
    );
    --_icon-color: var(
      --md-navigation-rail-item-icon-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_active-label-color: var(
      --md-navigation-rail-item-active-label-color,
      var(--md-sys-color-on-secondary-container, #1d192b)
    );
    --_label-color: var(
      --md-navigation-rail-item-label-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );

    display: block;
  }

  .target {
    appearance: none;
    border: none;
    background: none;
    cursor: pointer;
    text-decoration: none;
    color: inherit;
    font: inherit;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    width: 100%;
    padding: 0;
    /* M3 Expressive: collapsed touch target is at least 64dp tall (the
       active indicator pill stays 56x32). */
    min-height: 64px;
  }

  :host([expanded]) .target {
    flex-direction: row;
    justify-content: flex-start;
    gap: 12px;
    padding-inline: 16px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    height: 56px;
    /* Expanded rows keep the 56dp height; reset the collapsed 64dp floor. */
    min-height: 0;
  }

  .indicator {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 56px;
    height: 32px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    transition: background-color 150ms cubic-bezier(0.2, 0, 0, 1);
  }

  :host([expanded]) .indicator {
    width: auto;
    height: auto;
    background: none !important;
  }

  :host([selected]) .indicator {
    background: var(--_active-indicator-color);
  }

  :host([expanded][selected]) .target {
    background: var(--_active-indicator-color);
  }

  /* State layer */
  .indicator::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--_active-icon-color);
    opacity: 0;
    transition: opacity 150ms cubic-bezier(0.2, 0, 0, 1);
  }

  .target:hover .indicator::before {
    opacity: 0.08;
  }

  .target:focus-visible .indicator::before {
    opacity: 0.12;
  }

  .target:active .indicator::before {
    opacity: 0.12;
  }

  .target:focus-visible {
    outline: none;
  }

  .target:focus-visible .indicator {
    outline: 3px solid var(--md-sys-color-secondary, #625b71);
    outline-offset: 2px;
  }

  .icon {
    display: inline-flex;
    width: 24px;
    height: 24px;
    color: var(--_icon-color);
    font-size: 24px;
    line-height: 24px;
  }

  :host([selected]) .icon {
    color: var(--_active-icon-color);
  }

  .icon ::slotted(*) {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  .label {
    color: var(--_label-color);
    font-family: var(
      --md-sys-typescale-label-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-label-medium-size, 12px);
    font-weight: 500;
    line-height: 16px;
    letter-spacing: 0.5px;
    text-align: center;
  }

  :host([expanded]) .label {
    flex: 1;
    text-align: start;
    font-size: var(--md-sys-typescale-label-large-size, 14px);
  }

  :host([selected]) .label {
    color: var(--_active-label-color);
  }

  .badge {
    position: absolute;
    background: var(--md-sys-color-error, #b3261e);
    color: var(--md-sys-color-on-error, #fff);
  }

  .badge.small {
    inset-block-start: 2px;
    inset-inline-end: 14px;
    width: 6px;
    height: 6px;
    border-radius: 3px;
  }

  .badge.large {
    inset-block-start: -4px;
    inset-inline-start: 50%;
    min-width: 16px;
    height: 16px;
    padding-inline: 4px;
    border-radius: 8px;
    font-size: 11px;
    line-height: 16px;
    text-align: center;
  }

  [hidden] {
    display: none;
  }
`;
