/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 connected button-group styles. Applies the connected-shape
 * treatment to slotted children via the `data-position` hint set by the host,
 * and the selected container/on-container colors via `data-selected`.
 */
export const styles = css`
  :host {
    --_selected-container-color: var(
      --md-button-group-selected-container-color,
      var(--md-sys-color-secondary-container, #e8def8)
    );
    --_selected-label-color: var(
      --md-button-group-selected-label-color,
      var(--md-sys-color-on-secondary-container, #1d192b)
    );
    --_shape-full: var(--md-sys-shape-corner-full, 9999px);
    --_shape-inner: var(--md-button-group-inner-shape, 4px);

    display: inline-block;
  }

  .container {
    display: inline-flex;
    flex-direction: row;
    align-items: stretch;
    gap: 2px;
  }

  /* Connected-shape treatment driven by the data-position hint. */
  ::slotted([data-position="first"]) {
    border-start-start-radius: var(--_shape-full);
    border-end-start-radius: var(--_shape-full);
    border-start-end-radius: var(--_shape-inner);
    border-end-end-radius: var(--_shape-inner);
  }

  ::slotted([data-position="last"]) {
    border-start-end-radius: var(--_shape-full);
    border-end-end-radius: var(--_shape-full);
    border-start-start-radius: var(--_shape-inner);
    border-end-start-radius: var(--_shape-inner);
  }

  ::slotted([data-position="middle"]) {
    border-radius: var(--_shape-inner);
  }

  ::slotted([data-position="only"]) {
    border-radius: var(--_shape-full);
  }

  /* Selected child: secondary-container / on-secondary-container. The active
     child morphs toward a fuller shape per the M3 selection cue. */
  ::slotted([data-selected]) {
    background: var(--_selected-container-color);
    color: var(--_selected-label-color);
    border-radius: var(--_shape-full);
  }

  @media (forced-colors: active) {
    ::slotted([data-selected]) {
      outline: 2px solid CanvasText;
    }
  }
`;
