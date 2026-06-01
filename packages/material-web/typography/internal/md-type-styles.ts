/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/** Styles for `<md-type>`. */
export const styles = css`
  :host {
    display: contents;
  }

  :host([block]) {
    display: block;
  }

  .text {
    /* Inherit color from context; type carries no color of its own. */
    color: inherit;
    font-optical-sizing: auto;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
  }

  :host([animated]) .text {
    transition:
      font-variation-settings 300ms cubic-bezier(0.2, 0, 0, 1),
      font-weight 300ms cubic-bezier(0.2, 0, 0, 1),
      letter-spacing 300ms cubic-bezier(0.2, 0, 0, 1);
  }

  @media (prefers-reduced-motion: reduce) {
    :host([animated]) .text {
      transition: none;
    }
  }
`;
