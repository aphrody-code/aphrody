/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 step styles. The step host only renders its content panel; the
 * indicator and connector are drawn by the parent `<md-stepper>`. The content
 * is hidden entirely when the step is not active.
 */
export const styles = css`
  :host {
    display: block;
  }

  :host(:not([active])) {
    display: none;
  }

  .step-content {
    color: var(--md-sys-color-on-surface, #1d1b20);
    font-family: var(
      --md-sys-typescale-body-medium-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-medium-size, 14px);
    line-height: var(--md-sys-typescale-body-medium-line-height, 20px);
  }

  .step-content[hidden] {
    display: none;
  }
`;
