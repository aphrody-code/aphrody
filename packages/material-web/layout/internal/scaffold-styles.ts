/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 adaptive scaffold styles. Consumes the `--md-sys-*` design tokens
 * directly (with hex fallbacks so the component is fully autonomous). The
 * grid template switches on the reflected `size-class` attribute: navigation
 * sits at the bottom in `compact` and at the leading edge from `medium` up.
 *
 * Window margins follow `adaptive.rs`: 16dp in compact, 24dp otherwise.
 */
export const styles = css`
  :host {
    --_surface: var(--md-sys-color-surface, #fef7ff);
    --_on-surface: var(--md-sys-color-on-surface, #1d1b20);
    --_margin: 16px;

    display: block;
    block-size: 100%;
    inline-size: 100%;
    box-sizing: border-box;
    background: var(--_surface);
    color: var(--_on-surface);
  }

  .scaffold {
    box-sizing: border-box;
    block-size: 100%;
    inline-size: 100%;
    display: grid;
  }

  /* Compact: bars + bottom navigation, body scrolls in the middle. */
  :host([size-class="compact"]) .scaffold {
    grid-template-columns: 1fr;
    grid-template-rows: auto 1fr auto auto;
    grid-template-areas:
      "top-bar"
      "body"
      "bottom-bar"
      "navigation";
  }

  /* Medium and up: navigation rail on the leading edge, body to the side. */
  :host([size-class="medium"]) .scaffold,
  :host([size-class="expanded"]) .scaffold,
  :host([size-class="large"]) .scaffold,
  :host([size-class="extra-large"]) .scaffold {
    grid-template-columns: auto 1fr;
    grid-template-rows: auto 1fr auto;
    grid-template-areas:
      "navigation top-bar"
      "navigation body"
      "navigation bottom-bar";
  }

  /* Window margin per breakpoint (compact 16dp, others 24dp). */
  :host([size-class="medium"]),
  :host([size-class="expanded"]),
  :host([size-class="large"]),
  :host([size-class="extra-large"]) {
    --_margin: 24px;
  }

  .top-bar {
    grid-area: top-bar;
    position: sticky;
    inset-block-start: 0;
    z-index: 4;
    display: block;
  }

  .navigation {
    grid-area: navigation;
    display: block;
  }

  /* In compact the navigation is a horizontal bottom bar. */
  :host([size-class="compact"]) .navigation {
    position: sticky;
    inset-block-end: 0;
    z-index: 4;
  }

  .body {
    grid-area: body;
    min-block-size: 0;
    min-inline-size: 0;
    overflow: auto;
    padding-inline: var(--_margin);
    padding-block: var(--_margin);
    box-sizing: border-box;
  }

  .bottom-bar {
    grid-area: bottom-bar;
    display: block;
  }

  /* The FAB floats over the body in the bottom-trailing corner. */
  .fab {
    grid-area: body;
    align-self: end;
    justify-self: end;
    z-index: 6;
    margin: var(--_margin);
    pointer-events: none;
    display: flex;
  }

  .fab ::slotted(*) {
    pointer-events: auto;
  }

  /* Hide empty bar/fab regions so they do not reserve space. */
  .top-bar:not(:has(*)),
  .bottom-bar:not(:has(*)),
  .fab:not(:has(*)) {
    display: none;
  }

  @media (forced-colors: active) {
    :host {
      background: Canvas;
      color: CanvasText;
    }
  }
`;
