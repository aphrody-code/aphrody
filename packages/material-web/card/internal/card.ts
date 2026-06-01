/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */
import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * A Card component implementing Material Design 3 guidelines.
 */
export class Card extends LitElement {
  /**
   * The variant of the card.
   * Supported: 'elevated', 'filled', 'outlined'.
   */
  @property({ type: String, reflect: true }) variant: "elevated" | "filled" | "outlined" =
    "elevated";

  /**
   * If true, makes the card interactive and adds hover/active state visual effects.
   */
  @property({ type: Boolean, reflect: true }) clickable = false;

  protected override render() {
    return html`
      <div class="card-container">
        <slot name="media"></slot>
        <div class="card-content-area">
          <slot></slot>
        </div>
        <slot name="actions"></slot>
      </div>
    `;
  }
}
