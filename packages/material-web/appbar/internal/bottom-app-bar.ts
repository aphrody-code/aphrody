/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement } from "lit";

/**
 * A bottom app bar displays navigation and key actions at the bottom of a
 * compact-window layout (80dp tall, surface-container). Slot up to four action
 * affordances into the default slot and an optional FAB into `slot="fab"`.
 */
export class BottomAppBar extends LitElement {
  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "toolbar");
  }

  protected override render() {
    return html`
      <div class="bar" part="bar">
        <div class="actions"><slot></slot></div>
        <div class="fab"><slot name="fab"></slot></div>
      </div>
    `;
  }
}
