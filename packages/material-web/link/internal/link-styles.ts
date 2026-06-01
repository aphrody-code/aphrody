/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 link styles. The anchor is tinted with an `--md-sys-color-*` role
 * (selected via the `color` attribute) and overlays an M3 state layer on
 * hover/press. Keyboard focus draws a tokenised outline rather than a ripple.
 * Per-instance overrides via `--md-link-*`.
 */
export const styles = css`
  :host {
    display: inline;
    /* Resolved colour role; overridable per-instance via --md-link-color. */
    --_color: var(--md-link-color, var(--md-sys-color-primary, #6750a4));
    --_focus-outline-color: var(
      --md-link-focus-outline-color,
      var(--md-sys-color-secondary, #625b71)
    );
    --_hover-state-layer-opacity: var(--md-link-hover-state-layer-opacity, 0.08);
    --_pressed-state-layer-opacity: var(--md-link-pressed-state-layer-opacity, 0.12);
  }

  :host([color="secondary"]) {
    --_color: var(--md-link-color, var(--md-sys-color-secondary, #625b71));
  }
  :host([color="tertiary"]) {
    --_color: var(--md-link-color, var(--md-sys-color-tertiary, #7d5260));
  }
  :host([color="error"]) {
    --_color: var(--md-link-color, var(--md-sys-color-error, #b3261e));
  }
  :host([color="on-surface"]) {
    --_color: var(--md-link-color, var(--md-sys-color-on-surface, #1d1b20));
  }
  :host([color="on-surface-variant"]) {
    --_color: var(--md-link-color, var(--md-sys-color-on-surface-variant, #49454f));
  }
  :host([color="inverse-primary"]) {
    --_color: var(--md-link-color, var(--md-sys-color-inverse-primary, #d0bcff));
  }

  .link {
    color: var(--_color);
    cursor: pointer;
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
    text-decoration-color: currentColor;
    text-underline-offset: 0.15em;
    /* M3 standard easing for the state transition. */
    transition: text-decoration-color 150ms cubic-bezier(0.2, 0, 0, 1);
    font-family: var(
      --md-sys-typescale-body-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
  }

  /* Render the state layer with a background tint that follows the text. */
  .link:hover {
    background: color-mix(
      in srgb,
      currentColor calc(var(--_hover-state-layer-opacity) * 100%),
      transparent
    );
  }

  .link:active {
    background: color-mix(
      in srgb,
      currentColor calc(var(--_pressed-state-layer-opacity) * 100%),
      transparent
    );
  }

  /* Underline behaviour. */
  .underline-none {
    text-decoration-line: none;
  }
  .underline-always {
    text-decoration-line: underline;
  }
  .underline-hover {
    text-decoration-line: none;
  }
  .underline-hover:hover,
  .underline-hover:focus-visible {
    text-decoration-line: underline;
  }

  /* Tokenised focus outline (no ripple / focus-ring element). */
  .link:focus-visible {
    outline: 3px solid var(--_focus-outline-color);
    outline-offset: 2px;
  }

  /* Anchors without an href are presentational only. */
  .link:not([href]) {
    cursor: default;
  }

  @media (prefers-reduced-motion: reduce) {
    .link {
      transition: none;
    }
  }

  @media (forced-colors: active) {
    .link {
      color: LinkText;
    }
    .link:focus-visible {
      outline-color: CanvasText;
    }
  }
`;
