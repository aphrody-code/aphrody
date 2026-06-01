/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */
import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * A Badge component implementing Material Design 3 guidelines.
 */
export class Badge extends LitElement {
  /**
   * The text or counter value to display inside the badge.
   * If empty or undefined, the badge renders as a small status dot.
   */
  @property({ type: String }) value = "";

  /**
   * If true, forces the badge to render as a small status dot even if value is present.
   * Automatically derives from value if not set.
   */
  @property({ type: Boolean, reflect: true }) dot = false;

  /**
   * Helper property to absolutely position the badge at the top-right corner of its parent.
   */
  @property({ type: Boolean, reflect: true }) positioned = false;

  override willUpdate(changedProperties: Map<PropertyKey, unknown>) {
    if (changedProperties.has("value")) {
      // Auto dot state when value is falsy
      if (!this.dot && !this.value) {
        this.dot = true;
      } else if (this.value) {
        this.dot = false;
      }
    }
  }

  protected override render() {
    return html`${this.dot ? "" : this.value}`;
  }
}
