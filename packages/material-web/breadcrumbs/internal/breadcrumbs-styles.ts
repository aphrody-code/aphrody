/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 breadcrumbs styles. The trail is laid out as a wrapping flex `<ol>`
 * tinted with `--md-sys-color-on-surface-variant`; the current (last) item uses
 * `--md-sys-color-on-surface`. Typography follows the body typescale. Consumes
 * the `--md-sys-*` tokens; per-instance overrides via `--md-breadcrumbs-*`.
 */
export const styles = css`
  :host {
    display: block;
    --_text-color: var(
      --md-breadcrumbs-text-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_current-text-color: var(
      --md-breadcrumbs-current-text-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );
    --_separator-color: var(--md-breadcrumbs-separator-color, var(--md-sys-color-outline, #79747e));
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    color: var(--_text-color);
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

  .item {
    display: inline-flex;
    align-items: center;
    color: inherit;
    min-width: 0;
  }

  .item.current {
    color: var(--_current-text-color);
  }

  /* Tint slotted links/anchors to match the trail colour by default. */
  .item ::slotted(*) {
    color: inherit;
    --md-link-color: currentColor;
  }

  .separator {
    display: inline-flex;
    align-items: center;
    user-select: none;
    color: var(--_separator-color);
  }

  .ellipsis-item {
    display: inline-flex;
    align-items: center;
  }

  .ellipsis {
    appearance: none;
    border: none;
    background: none;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 4px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    color: var(--_text-color);
    font: inherit;
    line-height: 1;
  }

  .ellipsis:hover {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  .ellipsis:focus-visible {
    outline: 3px solid var(--md-sys-color-secondary, #625b71);
    outline-offset: 2px;
  }

  @media (forced-colors: active) {
    .ellipsis:focus-visible {
      outline-color: CanvasText;
    }
  }
`;
