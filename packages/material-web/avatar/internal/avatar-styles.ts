/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 avatar styles. Self-contained: consumes the `--md-sys-*` tokens
 * directly with baseline fallbacks (color / shape / typescale). Per-instance
 * overrides via `--md-avatar-*`. The `--_size` custom property is set inline by
 * the element and drives both dimensions.
 */
export const styles = css`
  :host {
    display: inline-flex;
    vertical-align: middle;
    --_container-color: var(
      --md-avatar-container-color,
      var(--md-sys-color-primary-container, #eaddff)
    );
    --_label-text-color: var(
      --md-avatar-label-text-color,
      var(--md-sys-color-on-primary-container, #21005d)
    );
  }

  .avatar {
    box-sizing: border-box;
    inline-size: var(--_size, 40px);
    block-size: var(--_size, 40px);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    user-select: none;
    background: var(--_container-color);
    color: var(--_label-text-color);
    border-radius: var(--md-avatar-container-shape, var(--md-sys-shape-corner-full, 9999px));
  }

  :host([variant="rounded"]) .avatar {
    border-radius: var(--md-avatar-container-shape, var(--md-sys-shape-corner-medium, 12px));
  }

  :host([variant="square"]) .avatar {
    border-radius: var(--md-avatar-container-shape, var(--md-sys-shape-corner-none, 0px));
  }

  .avatar.has-image {
    background: transparent;
  }

  .image {
    inline-size: 100%;
    block-size: 100%;
    object-fit: cover;
    display: block;
  }

  .fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 100%;
    block-size: 100%;
    /* Scale initials/icon relative to the avatar size. */
    font-size: calc(var(--_size, 40px) * 0.4);
    line-height: 1;
    font-family: var(
      --md-sys-typescale-label-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-weight: var(--md-sys-typescale-label-large-weight, 500);
    letter-spacing: 0.1px;
    text-transform: uppercase;
  }

  .fallback ::slotted(svg),
  .fallback ::slotted([slot="icon"]) {
    inline-size: calc(var(--_size, 40px) * 0.6);
    block-size: calc(var(--_size, 40px) * 0.6);
    fill: currentColor;
  }

  @media (forced-colors: active) {
    .avatar {
      outline: 1px solid CanvasText;
    }
  }
`;
