/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * Vertical alignment of the navigation items group within the rail.
 *
 * - `top` — items hug the top, beneath the optional menu/FAB.
 * - `center` — items are vertically centered (the M3 default for a rail with
 *   no FAB).
 * - `bottom` — items hug the bottom.
 */
export type NavigationRailAlignment = "top" | "center" | "bottom";

/**
 * A navigation rail lets people switch between UI views on mid-sized devices.
 * It implements the Material 3 collapsed (80dp) and expanded (≥220dp) variants
 * with optional leading menu button and FAB slots.
 *
 * Slot `menu` for a leading menu/hamburger affordance, `fab` for a FAB or
 * extended FAB, and the default slot for `<md-navigation-rail-item>`s.
 *
 * @fires navigation-rail:change {CustomEvent<{value: string}>} Fired when the
 *     active item changes, with the newly-selected item's `value`.
 */
export class NavigationRail extends LitElement {
  /** Expands the rail to its wide layout with inline labels. */
  @property({ type: Boolean, reflect: true }) expanded = false;

  /** Vertical placement of the items group. */
  @property({ reflect: true }) alignment: NavigationRailAlignment = "top";

  /**
   * The `value` of the currently-selected item. Two-way: set it to select an
   * item, read it after a `navigation-rail:change`.
   */
  @property() value = "";

  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "navigation");
    this.addEventListener("navigation-rail-item:activate", this.handleActivate);
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    this.removeEventListener("navigation-rail-item:activate", this.handleActivate);
  }

  protected override updated(changed: Map<string, unknown>) {
    if (changed.has("value") || changed.has("expanded")) {
      this.syncItems();
    }
  }

  private get items(): NavigationRailItemLike[] {
    return Array.from(
      this.querySelectorAll("md-navigation-rail-item"),
    ) as unknown as NavigationRailItemLike[];
  }

  private syncItems() {
    for (const item of this.items) {
      item.selected = item.value !== "" && item.value === this.value;
      item.expanded = this.expanded;
    }
  }

  private readonly handleActivate = (event: Event) => {
    const item = event.target as NavigationRailItemLike;
    if (!item || item.value === undefined) {
      return;
    }
    this.value = item.value;
    this.dispatchEvent(
      new CustomEvent("navigation-rail:change", {
        detail: { value: this.value },
        bubbles: true,
        composed: true,
      }),
    );
  };

  protected override render() {
    return html`
      <div class="rail" part="rail">
        <div class="leading">
          <slot name="menu"></slot>
          <slot name="fab"></slot>
        </div>
        <div class="items" role="list">
          <slot @slotchange=${this.syncItems}></slot>
        </div>
      </div>
    `;
  }
}

/** Structural view of `md-navigation-rail-item` used by the rail container. */
interface NavigationRailItemLike extends HTMLElement {
  value: string;
  selected: boolean;
  expanded: boolean;
}
