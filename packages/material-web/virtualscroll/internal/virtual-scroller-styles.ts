/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/** Styles for `<md-virtual-scroller>`. */
export const styles = css`
  :host {
    display: block;
    block-size: 100%;
    color: var(--md-sys-color-on-surface, #1d1b20);
  }

  .viewport {
    block-size: 100%;
    inline-size: 100%;
    overflow: auto;
    contain: strict;
    -webkit-overflow-scrolling: touch;
  }

  .spacer {
    position: relative;
    inline-size: 100%;
  }

  .content {
    position: absolute;
    inset-block-start: 0;
    inset-inline: 0;
    will-change: transform;
  }

  .row {
    box-sizing: border-box;
    inline-size: 100%;
    overflow: hidden;
  }
`;
