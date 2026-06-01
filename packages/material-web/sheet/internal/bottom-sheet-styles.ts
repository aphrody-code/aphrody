/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 bottom-sheet styles. Consumes the `--md-sys-*` design tokens
 * directly and paints elevation itself. Component-level
 * `--md-bottom-sheet-*` custom properties allow per-instance overrides.
 */
export const styles = css`
  :host {
    --_container-color: var(
      --md-bottom-sheet-container-color,
      var(--md-sys-color-surface-container-low, #f7f2fa)
    );
    --_container-shape: var(
      --md-bottom-sheet-container-shape,
      var(--md-sys-shape-corner-extra-large, 28px)
    );
    --_handle-color: var(--md-bottom-sheet-handle-color, var(--md-sys-color-outline, #79747e));

    display: contents;
    color: var(--md-sys-color-on-surface, #1d1b20);
    transition: display 300ms allow-discrete;
  }

  :host(:not([open])) {
    display: none;
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 9;
    background: rgba(0, 0, 0, 0.32);
    opacity: 0;
    transition:
      opacity 150ms linear,
      display 150ms allow-discrete;
  }

  :host([open]) .scrim {
    opacity: 1;
    transition:
      opacity 300ms linear,
      display 300ms allow-discrete;

    @starting-style {
      opacity: 0;
    }
  }

  .sheet {
    position: fixed;
    inset-inline: 0;
    inset-block-end: 0;
    z-index: 10;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 72px);
    margin-inline: auto;
    max-width: 640px;
    background: var(--_container-color);
    border-start-start-radius: var(--_container-shape);
    border-start-end-radius: var(--_container-shape);
    /* M3 elevation level 1 */
    box-shadow: var(
      --md-sys-elevation-level1,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15)
    );
    touch-action: none;

    /* Transition styles for closing (exit) */
    opacity: 0;
    transform: translateY(100%);
    transition:
      opacity 150ms cubic-bezier(0.3, 0, 0.8, 0.15),
      transform 150ms cubic-bezier(0.3, 0, 0.8, 0.15),
      display 150ms allow-discrete;
  }

  :host([open]) .sheet {
    opacity: 1;
    transform: translateY(0);
    transition:
      opacity 300ms cubic-bezier(0.2, 0, 0, 1),
      transform 300ms cubic-bezier(0.2, 0, 0, 1),
      display 300ms allow-discrete;

    @starting-style {
      opacity: 0;
      transform: translateY(100%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sheet {
      transform: none !important;
      transition-duration: 150ms !important;
    }
    :host([open]) .sheet {
      transform: none !important;
      transition-duration: 150ms !important;
    }
    :host([open]) .sheet {
      @starting-style {
        transform: none !important;
      }
    }
  }

  .handle {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 22px;
    padding-block: 9px;
    cursor: grab;
  }

  .handle:active {
    cursor: grabbing;
  }

  .handle:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #6750a4);
    outline-offset: -2px;
  }

  .grip {
    width: 32px;
    height: 4px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    background: var(--_handle-color);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding-inline: 16px;
    padding-block-end: 24px;
  }

  @media (forced-colors: active) {
    .sheet {
      outline: 1px solid CanvasText;
    }
  }
`;
