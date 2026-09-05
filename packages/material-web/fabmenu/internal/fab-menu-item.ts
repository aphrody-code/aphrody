/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * A single action row inside an `md-fab-menu`. Renders a label chip
 * (`surface-container-high`) trailed by a small (40px) FAB carrying the icon.
 *
 * Slot an icon as the default content; provide the visible text with the
 * `label` property (also used as the accessible name).
 *
 * @fires fab-menu-item:click {Event} Fired when the item is activated. The host
 *     menu closes itself in response.
 */
export class FabMenuItem extends LitElement {
  /** The visible (and accessible) label for this action. */
  @property({ type: String }) label = "";

  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "menuitem");
    if (!this.hasAttribute("tabindex")) {
      this.tabIndex = -1;
    }
  }

  protected override render() {
    return html`
      <button
        class="item"
        aria-label=${this.label}
        @click=${this.handleClick}
        @keydown=${this.handleKeydown}
      >
        <span class="chip">${this.label}</span>
        <span class="mini-fab" aria-hidden="true">
          <slot></slot>
        </span>
      </button>
    `;
  }

  private handleClick() {
    this.dispatchEvent(new Event("fab-menu-item:click", { bubbles: true, composed: true }));
  }

  private handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      this.handleClick();
    }
  }
}
