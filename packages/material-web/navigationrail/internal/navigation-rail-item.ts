/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing } from "lit";
import { property } from "lit/decorators.js";

/**
 * A single destination inside an `<md-navigation-rail>`. Renders the M3 active
 * indicator pill (56×32dp collapsed) around the icon, the label beneath (or
 * inline when the rail is expanded), and an optional badge.
 *
 * Slot an icon into the default slot (e.g. `<md-icon>` or an SVG); optionally
 * slot a distinct `active-icon` shown when selected.
 *
 * @fires navigation-rail-item:activate {Event} Fired when the item is
 *     activated; the parent rail listens for this to update its `value`.
 */
export class NavigationRailItem extends LitElement {
  /** Identifies this destination within the rail's `value`. */
  @property() value = "";

  /** The text label shown beneath (collapsed) or beside (expanded) the icon. */
  @property() label = "";

  /** Whether this destination is the active one. Managed by the rail. */
  @property({ type: Boolean, reflect: true }) selected = false;

  /** Whether the parent rail is expanded. Managed by the rail. */
  @property({ type: Boolean, reflect: true }) expanded = false;

  /** Optional badge text (large badge). A bare presence renders a small dot. */
  @property({ attribute: "badge-value" }) badgeValue = "";

  /** Renders a small dot badge with no text. */
  @property({ type: Boolean, attribute: "show-badge" }) showBadge = false;

  /** When set, the item renders as a link to this destination. */
  @property() href = "";

  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "listitem");
  }

  protected override render() {
    if (this.href !== "") {
      return html`<a
        class="target"
        href=${this.href}
        aria-current=${this.selected ? "page" : nothing}
        @click=${this.activate}
      >
        ${this.renderContent()}
      </a>`;
    }
    return html`<button
      class="target"
      type="button"
      aria-pressed=${this.selected ? "true" : "false"}
      @click=${this.activate}
    >
      ${this.renderContent()}
    </button>`;
  }

  private renderContent() {
    return html`
      <span class="indicator">
        <span class="icon">
          <slot name="active-icon" ?hidden=${!this.selected}></slot>
          <slot ?hidden=${this.selected && this.hasActiveIcon()}></slot>
        </span>
        ${this.renderBadge()}
      </span>
      ${this.label ? html`<span class="label">${this.label}</span>` : nothing}
    `;
  }

  private renderBadge() {
    if (this.badgeValue) {
      return html`<span class="badge large" aria-hidden="true">${this.badgeValue}</span>`;
    }
    if (this.showBadge) {
      return html`<span class="badge small" aria-hidden="true"></span>`;
    }
    return nothing;
  }

  private hasActiveIcon(): boolean {
    return this.querySelector('[slot="active-icon"]') !== null;
  }

  private activate() {
    this.dispatchEvent(
      new Event("navigation-rail-item:activate", {
        bubbles: true,
        composed: true,
      }),
    );
  }
}
