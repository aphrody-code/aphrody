/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 accordion styles. Lays out its slotted expansion panels in a
 * vertical stack with a consistent gap. Per-instance overrides via
 * `--md-accordion-*`.
 */
export const styles = css`
  :host {
    display: flex;
    flex-direction: column;
    gap: var(--md-accordion-gap, 8px);
  }

  ::slotted(md-expansion-panel) {
    display: block;
  }
`;
