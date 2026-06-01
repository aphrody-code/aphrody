/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 side-sheet styles. Consumes the `--md-sys-*` design tokens
 * directly and paints elevation itself. Component-level `--md-side-sheet-*`
 * custom properties allow per-instance overrides.
 */
export const styles = css`
  :host {
    --_container-color: var(--md-side-sheet-container-color, var(--md-sys-color-surface, #fef7ff));
    --_container-shape: var(
      --md-side-sheet-container-shape,
      var(--md-sys-shape-corner-large, 16px)
    );

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
    inset-block: 0;
    z-index: 10;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    width: clamp(256px, 25vw, 400px);
    background: var(--_container-color);
    /* M3 elevation level 1 */
    box-shadow: var(
      --md-sys-elevation-level1,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15)
    );

    /* Transition styles for closing (exit) */
    opacity: 0;
    transition:
      opacity 150ms cubic-bezier(0.3, 0, 0.8, 0.15),
      transform 150ms cubic-bezier(0.3, 0, 0.8, 0.15),
      display 150ms allow-discrete;
  }

  /* Trailing edge (default): rounded on the inner (leading) corners. */
  :host([position="end"]) .sheet {
    inset-inline-end: 0;
    border-start-start-radius: var(--_container-shape);
    border-end-start-radius: var(--_container-shape);
    transform: translateX(100%);
  }

  /* Leading edge: rounded on the inner (trailing) corners. */
  :host([position="start"]) .sheet {
    inset-inline-start: 0;
    border-start-end-radius: var(--_container-shape);
    border-end-end-radius: var(--_container-shape);
    transform: translateX(-100%);
  }

  :host([open]) .sheet {
    opacity: 1;
    transform: translateX(0);
    transition:
      opacity 300ms cubic-bezier(0.2, 0, 0, 1),
      transform 300ms cubic-bezier(0.2, 0, 0, 1),
      display 300ms allow-discrete;
  }

  :host([open][position="end"]) .sheet {
    @starting-style {
      opacity: 0;
      transform: translateX(100%);
    }
  }

  :host([open][position="start"]) .sheet {
    @starting-style {
      opacity: 0;
      transform: translateX(-100%);
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
    :host([open][position="end"]) .sheet {
      @starting-style {
        transform: none !important;
      }
    }
    :host([open][position="start"]) .sheet {
      @starting-style {
        transform: none !important;
      }
    }
  }

  .headline {
    padding-block: 16px 8px;
    padding-inline: 24px;
    font-family: var(
      --md-sys-typescale-title-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-title-large-size, 22px);
    line-height: var(--md-sys-typescale-title-large-line-height, 28px);
    font-weight: var(--md-sys-typescale-title-large-weight, 400);
  }

  .headline:empty {
    display: none;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding-inline: 24px;
    padding-block: 8px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-inline: 24px;
    padding-block: 16px;
  }

  .actions:empty {
    display: none;
  }

  @media (forced-colors: active) {
    .sheet {
      outline: 1px solid CanvasText;
    }
  }
`;
