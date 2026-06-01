/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * Detail payload for the `expansion:toggle` event dispatched whenever a panel's
 * expanded state changes.
 */
export interface ExpansionToggleDetail {
  expanded: boolean;
}

let nextId = 0;

/**
 * A Material 3 expansion panel: a header that toggles a region of content open
 * and closed. The header carries a rotating chevron, the content animates its
 * height via the modern `grid-template-rows: 0fr → 1fr` technique.
 *
 * Slots:
 * - `header` — the panel title shown in the clickable header.
 * - `description` — optional secondary text in the header.
 * - default — the collapsible body content.
 *
 * @fires expansion:toggle {CustomEvent<ExpansionToggleDetail>} Fired after the
 *     panel expands or collapses.
 */
export class ExpansionPanel extends LitElement {
  /** Whether the panel is expanded. Reflected so CSS can target `[expanded]`. */
  @property({ type: Boolean, reflect: true }) expanded = false;

  /** When true the header cannot be toggled by the user. */
  @property({ type: Boolean, reflect: true }) disabled = false;

  private readonly regionId = `md-expansion-region-${nextId++}`;

  /** Expands the panel (no-op if already expanded or disabled). */
  expand() {
    if (this.disabled || this.expanded) {
      return;
    }
    this.expanded = true;
    this.notifyToggle();
  }

  /** Collapses the panel (no-op if already collapsed or disabled). */
  collapse() {
    if (this.disabled || !this.expanded) {
      return;
    }
    this.expanded = false;
    this.notifyToggle();
  }

  /** Toggles the expanded state. */
  toggle() {
    if (this.disabled) {
      return;
    }
    this.expanded = !this.expanded;
    this.notifyToggle();
  }

  private notifyToggle() {
    if (isServer) {
      return;
    }
    this.dispatchEvent(
      new CustomEvent<ExpansionToggleDetail>("expansion:toggle", {
        detail: { expanded: this.expanded },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleHeaderClick() {
    this.toggle();
  }

  private handleHeaderKeydown(event: KeyboardEvent) {
    if (this.disabled) {
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      this.toggle();
    }
  }

  protected override render() {
    return html`
      <div class="panel">
        <div
          class="header"
          role="button"
          tabindex=${this.disabled ? -1 : 0}
          aria-expanded=${this.expanded ? "true" : "false"}
          aria-controls=${this.regionId}
          aria-disabled=${this.disabled ? "true" : "false"}
          @click=${this.handleHeaderClick}
          @keydown=${this.handleHeaderKeydown}
        >
          <div class="header-text">
            <div class="title"><slot name="header"></slot></div>
            <div class="description"><slot name="description"></slot></div>
          </div>
          <svg class="chevron" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M7.4 8.6 12 13.2l4.6-4.6L18 10l-6 6-6-6 1.4-1.4Z"></path>
          </svg>
        </div>
        <div class="content-wrapper" role="region" id=${this.regionId}>
          <div class="content">
            <slot></slot>
          </div>
        </div>
      </div>
    `;
  }
}
