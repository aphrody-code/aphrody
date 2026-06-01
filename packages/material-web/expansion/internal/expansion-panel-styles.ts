/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 expansion-panel styles. The surface uses the `surface` token with
 * a medium corner radius and elevation level 1. The body is animated open/closed
 * with the modern `grid-template-rows: 0fr → 1fr` height technique, and the
 * header chevron rotates 180° on expand with an emphasized easing curve.
 * Per-instance overrides via `--md-expansion-panel-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_container-color: var(
      --md-expansion-panel-container-color,
      var(--md-sys-color-surface, #fef7ff)
    );
    --_container-shape: var(
      --md-expansion-panel-container-shape,
      var(--md-sys-shape-corner-medium, 12px)
    );
    --_text-color: var(--md-expansion-panel-text-color, var(--md-sys-color-on-surface, #1d1b20));
    --_description-color: var(
      --md-expansion-panel-description-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_icon-color: var(
      --md-expansion-panel-icon-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
  }

  .panel {
    box-sizing: border-box;
    border-radius: var(--_container-shape);
    background: var(--_container-color);
    color: var(--_text-color);
    overflow: hidden;
    /* M3 elevation level 1 */
    box-shadow: var(
      --md-sys-elevation-level1,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15)
    );
  }

  .header {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-height: 56px;
    padding: 16px 24px;
    cursor: pointer;
    user-select: none;
    position: relative;
    outline: none;
  }

  .header::before {
    content: "";
    position: absolute;
    inset: 0;
    background: transparent;
    pointer-events: none;
    transition: background 150ms linear;
  }

  .header:hover::before {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  .header:focus-visible::before {
    background: color-mix(in srgb, currentColor 12%, transparent);
  }

  .header:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #6750a4);
    outline-offset: -2px;
  }

  :host([disabled]) .header {
    cursor: default;
    opacity: 0.38;
  }

  :host([disabled]) .header::before {
    background: transparent;
  }

  .header-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-family: var(
      --md-sys-typescale-title-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-title-medium-size, 16px);
    line-height: var(--md-sys-typescale-title-medium-line-height, 24px);
    font-weight: var(--md-sys-typescale-title-medium-weight, 500);
    letter-spacing: var(--md-sys-typescale-title-medium-tracking, 0.15px);
  }

  .description {
    color: var(--_description-color);
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
  }

  .description:empty {
    display: none;
  }

  .chevron {
    flex: none;
    width: 24px;
    height: 24px;
    fill: var(--_icon-color);
    /* M3 emphasized easing, 300ms. */
    transition: transform 300ms cubic-bezier(0.2, 0, 0, 1);
  }

  :host([expanded]) .chevron {
    transform: rotate(180deg);
  }

  /* Modern height animation: 0fr → 1fr collapses/expands the row track. */
  .content-wrapper {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows 300ms cubic-bezier(0.2, 0, 0, 1);
  }

  :host([expanded]) .content-wrapper {
    grid-template-rows: 1fr;
  }

  .content {
    overflow: hidden;
    min-height: 0;
  }

  .content > slot {
    display: block;
    padding: 0 24px 16px;
  }

  @media (prefers-reduced-motion: reduce) {
    .chevron,
    .content-wrapper {
      transition-duration: 0ms;
    }
  }

  @media (forced-colors: active) {
    .panel {
      outline: 1px solid CanvasText;
    }
  }
`;
