/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 avatar-group styles. Self-contained: consumes the `--md-sys-*`
 * tokens directly with baseline fallbacks. The `--_overlap` custom property is
 * set inline by the element and pulls each avatar (and the surplus pill) over
 * its predecessor. The surplus pill uses the surface-variant tokens.
 */
export const styles = css`
  :host {
    display: inline-flex;
    vertical-align: middle;
    --_surplus-color: var(
      --md-avatar-group-surplus-color,
      var(--md-sys-color-surface-variant, #e7e0ec)
    );
    --_surplus-label-color: var(
      --md-avatar-group-surplus-label-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_surplus-size: var(--md-avatar-group-surplus-size, 40px);
  }

  .group {
    display: inline-flex;
    align-items: center;
    flex-direction: row;
  }

  /* Pull every item except the first over its predecessor. The ring matches
     the page background so overlapping avatars read as separated. */
  ::slotted(*:not(:first-child)) {
    margin-inline-start: calc(var(--_overlap, 8px) * -1);
  }

  ::slotted(*) {
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    outline: 2px solid var(--md-avatar-group-ring-color, var(--md-sys-color-surface, #fef7ff));
  }

  .surplus {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: var(--_surplus-size, 40px);
    block-size: var(--_surplus-size, 40px);
    margin-inline-start: calc(var(--_overlap, 8px) * -1);
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    background: var(--_surplus-color);
    color: var(--_surplus-label-color);
    outline: 2px solid var(--md-avatar-group-ring-color, var(--md-sys-color-surface, #fef7ff));
    user-select: none;
    font-family: var(
      --md-sys-typescale-label-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-label-large-size, 14px);
    font-weight: var(--md-sys-typescale-label-large-weight, 500);
    line-height: 1;
    letter-spacing: 0.1px;
  }

  @media (forced-colors: active) {
    .surplus {
      outline: 1px solid CanvasText;
    }
  }
`;
