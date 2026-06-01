/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing } from "lit";
import { property } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";
import { styleMap } from "lit/directives/style-map.js";

/**
 * The Material 3 Expressive loading indicator: an active shape that rotates and
 * morphs between rounded polygon forms. When `value` is provided it acts as a
 * determinate progress indicator; otherwise it is indeterminate.
 *
 * Built entirely from animated SVG paths and CSS `@keyframes`; it carries no
 * dependency on any compiled style asset. Respects reduced-motion by falling
 * back to a static shape (driven via the `@media (prefers-reduced-motion)`
 * rule in the stylesheet).
 */
export class LoadingIndicator extends LitElement {
  /**
   * Progress in the range 0..1. When set, the indicator is determinate and
   * exposes `aria-valuenow`. When unset (`undefined`), it is indeterminate.
   */
  @property({ type: Number }) value?: number;

  /** Accessible label for the indicator. */
  @property({ attribute: "aria-label" }) override ariaLabel = "Loading";

  protected override render() {
    const determinate = typeof this.value === "number";
    const clamped = determinate ? Math.min(1, Math.max(0, this.value ?? 0)) : 0;
    const classes = {
      indicator: true,
      determinate: determinate,
      indeterminate: !determinate,
    };
    // Scale the active shape with progress so the morphing form fills in.
    const innerStyle = determinate
      ? styleMap({ transform: `scale(${0.4 + 0.6 * clamped})` })
      : nothing;
    return html`
      <div
        class=${classMap(classes)}
        role="progressbar"
        aria-label=${this.ariaLabel}
        aria-valuemin=${determinate ? 0 : nothing}
        aria-valuemax=${determinate ? 1 : nothing}
        aria-valuenow=${determinate ? clamped : nothing}
      >
        <div class="shape" style=${innerStyle}></div>
      </div>
    `;
  }
}
