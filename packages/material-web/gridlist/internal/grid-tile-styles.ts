/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 grid-tile styles. The tile clips its content to a medium corner
 * radius and overlays a scrim-backed footer caption pinned to the bottom edge.
 * Inherits an aspect ratio from the parent grid-list (`--md-grid-tile-aspect-
 * ratio`) when the list uses ratio-based rows. Per-instance overrides via
 * `--md-grid-tile-*`.
 */
export const styles = css`
  :host {
    display: block;
    height: 100%;
    --_container-shape: var(
      --md-grid-tile-container-shape,
      var(--md-sys-shape-corner-medium, 12px)
    );
    --_footer-color: var(--md-grid-tile-footer-color, var(--md-sys-color-scrim, #000000));
    --_footer-text-color: var(
      --md-grid-tile-footer-text-color,
      var(--md-sys-color-on-surface, #ffffff)
    );
  }

  .tile {
    position: relative;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    border-radius: var(--_container-shape);
    overflow: hidden;
    aspect-ratio: var(--md-grid-tile-aspect-ratio, auto);
  }

  .body {
    width: 100%;
    height: 100%;
  }

  .body ::slotted(img),
  .body ::slotted(picture),
  .body ::slotted(video) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .footer {
    position: absolute;
    inset-inline: 0;
    inset-block-end: 0;
    box-sizing: border-box;
    padding: 8px 16px;
    background: color-mix(in srgb, var(--_footer-color) 60%, transparent);
    color: var(--_footer-text-color);
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

  .footer:not(:has(::slotted(*))) {
    display: none;
  }

  @media (forced-colors: active) {
    .tile {
      outline: 1px solid CanvasText;
    }
    .footer {
      background: Canvas;
      color: CanvasText;
      border-top: 1px solid CanvasText;
    }
  }
`;
