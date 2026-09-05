/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 surface styles. The host is the surface itself; the tonal
 * container color steps with the reflected `level` attribute across the
 * `--md-sys-color-surface-container*` ramp. The `elevation` variant lays an
 * `<md-elevation>` behind the content for the shadow; `outlined` draws an
 * outline-variant border and `filled` uses the highest container tone with no
 * shadow. Per-instance overrides via `--md-surface-*`.
 */
export const styles = css`
  :host {
    display: block;
    position: relative;
    box-sizing: border-box;
    border-radius: var(--md-surface-shape, var(--md-sys-shape-corner-medium, 12px));
    color: var(--md-surface-text-color, var(--md-sys-color-on-surface, #1d1b20));
    /* Level 0 default. */
    background: var(--md-surface-container-color, var(--md-sys-color-surface, #fef7ff));
  }

  /* Tonal container ramp — higher level, higher container tone. */
  :host([level="1"]) {
    background: var(
      --md-surface-container-color,
      var(--md-sys-color-surface-container-low, #f7f2fa)
    );
  }

  :host([level="2"]) {
    background: var(--md-surface-container-color, var(--md-sys-color-surface-container, #f3edf7));
  }

  :host([level="3"]) {
    background: var(
      --md-surface-container-color,
      var(--md-sys-color-surface-container-high, #ece6f0)
    );
  }

  :host([level="4"]),
  :host([level="5"]) {
    background: var(
      --md-surface-container-color,
      var(--md-sys-color-surface-container-highest, #e6e0e9)
    );
  }

  /* The filled variant is a flat, higher-emphasis tonal surface. */
  :host([variant="filled"]) {
    background: var(
      --md-surface-container-color,
      var(--md-sys-color-surface-container-highest, #e6e0e9)
    );
  }

  /* The outlined variant is flat with an outline-variant border. */
  :host([variant="outlined"]) {
    background: var(--md-surface-container-color, var(--md-sys-color-surface, #fef7ff));
    border: 1px solid var(--md-surface-outline-color, var(--md-sys-color-outline-variant, #cac4d0));
  }

  /* The shared elevation element renders the shadow behind the content. */
  md-elevation {
    z-index: -1;
  }

  /* Shape token aliases applied via the reflected shape attribute. */
  :host([shape="none"]) {
    border-radius: var(--md-sys-shape-corner-none, 0px);
  }
  :host([shape="extra-small"]) {
    border-radius: var(--md-sys-shape-corner-extra-small, 4px);
  }
  :host([shape="small"]) {
    border-radius: var(--md-sys-shape-corner-small, 8px);
  }
  :host([shape="medium"]) {
    border-radius: var(--md-sys-shape-corner-medium, 12px);
  }
  :host([shape="large"]) {
    border-radius: var(--md-sys-shape-corner-large, 16px);
  }
  :host([shape="extra-large"]) {
    border-radius: var(--md-sys-shape-corner-extra-large, 28px);
  }
  :host([shape="full"]) {
    border-radius: var(--md-sys-shape-corner-full, 9999px);
  }

  @media (forced-colors: active) {
    :host {
      outline: 1px solid CanvasText;
    }
  }
`;
