/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 popover styles. The `.surface` is a top-layer popover with an M3
 * elevation-level-3 surface, large corner shape, and a fade/scale in-out
 * animation via `@starting-style` + `transition-behavior: allow-discrete`.
 *
 * Positioning has two paths:
 * - When CSS Anchor Positioning is supported, the `@supports` block binds the
 *   surface to the anchor's `anchor-name` via `position-area`, switching by the
 *   reflected `placement` attribute. The host exposes `--md-popover-anchor` for
 *   the consumer to set the `anchor-name` on the trigger.
 * - Otherwise the JS fallback in the element sets `position: fixed` + top/left.
 *
 * Per-instance overrides via `--md-popover-*`.
 */
export const styles = css`
  :host {
    display: contents;
    --_container-color: var(
      --md-popover-container-color,
      var(--md-sys-color-surface-container, #f3edf7)
    );
    --_container-shape: var(--md-popover-container-shape, var(--md-sys-shape-corner-large, 16px));
  }

  .surface {
    position: fixed;
    inset: auto;
    box-sizing: border-box;
    margin: 0;
    min-width: 112px;
    max-width: min(560px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow: auto;
    padding: 8px;
    border: none;
    border-radius: var(--_container-shape);
    background: var(--_container-color);
    color: var(--md-sys-color-on-surface, #1d1b20);
    /* M3 elevation level 2–3 via the shared elevation element. */
    --md-elevation-level: var(--md-popover-elevation-level, 3);
    /* Closed / exit state. */
    opacity: 0;
    transform: scale(0.9);
    transition:
      opacity 100ms cubic-bezier(0.3, 0, 0.8, 0.15),
      transform 100ms cubic-bezier(0.3, 0, 0.8, 0.15),
      display 100ms allow-discrete,
      overlay 100ms allow-discrete;
  }

  .surface:popover-open {
    opacity: 1;
    transform: scale(1);
    transition:
      opacity 150ms cubic-bezier(0.05, 0.7, 0.1, 1),
      transform 150ms cubic-bezier(0.05, 0.7, 0.1, 1),
      display 150ms allow-discrete,
      overlay 150ms allow-discrete;
  }

  @starting-style {
    .surface:popover-open {
      opacity: 0;
      transform: scale(0.9);
    }
  }

  /*
   * When the host has no Popover API support the surface is not a top-layer
   * popover, so :popover-open never matches. Mirror the open state off the
   * reflected host [open] attribute as well.
   */
  :host([open]) .surface {
    opacity: 1;
    transform: scale(1);
  }

  md-elevation {
    z-index: -1;
  }

  /* Declarative anchor positioning when the platform supports it. */
  @supports (anchor-name: --x) {
    .surface {
      position: fixed;
      inset: auto;
      position-anchor: var(--md-popover-anchor, --md-popover-anchor);
      margin: 8px;
    }

    :host([placement="bottom"]) .surface,
    :host([placement="bottom-start"]) .surface,
    :host([placement="bottom-end"]) .surface {
      position-area: block-end center;
    }
    :host([placement="top"]) .surface,
    :host([placement="top-start"]) .surface,
    :host([placement="top-end"]) .surface {
      position-area: block-start center;
    }
    :host([placement="left"]) .surface,
    :host([placement="left-start"]) .surface,
    :host([placement="left-end"]) .surface {
      position-area: inline-start center;
    }
    :host([placement="right"]) .surface,
    :host([placement="right-start"]) .surface,
    :host([placement="right-end"]) .surface {
      position-area: inline-end center;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .surface,
    .surface:popover-open {
      transform: none;
      transition-duration: 80ms;
    }
    @starting-style {
      .surface:popover-open {
        transform: none;
      }
    }
  }

  @media (forced-colors: active) {
    .surface {
      outline: 1px solid CanvasText;
    }
  }
`;
