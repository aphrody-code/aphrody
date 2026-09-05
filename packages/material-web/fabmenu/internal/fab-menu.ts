/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property, query, queryAssignedElements } from "lit/decorators.js";

import {
  animationOptions,
  prefersReducedMotion,
} from "../../internal/motion/easing-and-duration.js";

/**
 * A FAB menu: a primary floating action button that, when opened, reveals a
 * stack of `md-fab-menu-item` actions above it with a staggered entrance. The
 * trigger icon morphs from a plus to a close (✕) by rotating 45°.
 *
 * @fires fab-menu:open {Event} Fired after the menu opens.
 * @fires fab-menu:close {Event} Fired after the menu closes.
 */
export class FabMenu extends LitElement {
  /** Whether the menu is open. Reflected so CSS can target `[open]`. */
  @property({ type: Boolean, reflect: true }) open = false;

  /** Accessible label for the trigger FAB. */
  @property({ type: String }) label = "Menu";

  @query(".items") private readonly itemsContainer!: HTMLElement | null;

  @queryAssignedElements({ flatten: true })
  private readonly items!: HTMLElement[];

  /** Opens the menu and animates the items in. */
  show() {
    if (this.open) {
      return;
    }
    this.open = true;
    this.animateItems(true);
    this.dispatchEvent(new Event("fab-menu:open"));
    const first = this.items[0];
    if (first && !isServer) {
      first.tabIndex = 0;
      first.focus();
    }
  }

  /** Closes the menu and animates the items out. */
  close() {
    if (!this.open) {
      return;
    }
    this.animateItems(false);
    this.open = false;
    for (const item of this.items) {
      item.tabIndex = -1;
    }
    this.dispatchEvent(new Event("fab-menu:close"));
  }

  /** Toggles the open state. */
  toggle() {
    if (this.open) {
      this.close();
    } else {
      this.show();
    }
  }

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.addEventListener("keydown", this.handleKeydown);
      this.addEventListener("fab-menu-item:click", this.handleItemClick);
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    if (!isServer) {
      this.removeEventListener("keydown", this.handleKeydown);
      this.removeEventListener("fab-menu-item:click", this.handleItemClick);
    }
  }

  protected override render() {
    return html`
      <div class="scrim" @click=${this.handleScrimClick}></div>
      <div class="items" role="menu" aria-label=${this.label}>
        <slot></slot>
      </div>
      <button
        class="trigger"
        aria-label=${this.label}
        aria-haspopup="menu"
        aria-expanded=${this.open ? "true" : "false"}
        @click=${this.handleTriggerClick}
      >
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M11 13H5v-2h6V5h2v6h6v2h-6v6h-2z"></path>
        </svg>
      </button>
    `;
  }

  private handleTriggerClick() {
    this.toggle();
  }

  private handleScrimClick() {
    this.close();
  }

  private readonly handleItemClick = () => {
    this.close();
  };

  private readonly handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape" && this.open) {
      event.stopPropagation();
      this.close();
      return;
    }
    if (!this.open || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) {
      return;
    }
    const items = this.items;
    if (items.length === 0) {
      return;
    }
    event.preventDefault();
    const active = items.findIndex(
      (item) => item.matches(":focus-within") || item === document.activeElement,
    );
    const dir = event.key === "ArrowDown" ? 1 : -1;
    const next = active === -1 ? 0 : (active + dir + items.length) % items.length;
    items.forEach((item, i) => {
      item.tabIndex = i === next ? 0 : -1;
    });
    items[next].focus();
  };

  /**
   * Staggers the items in or out. Each item is offset by an incremental delay
   * so they cascade per the M3 FAB-menu motion.
   */
  private animateItems(opening: boolean) {
    const container = this.itemsContainer;
    if (!container || isServer || prefersReducedMotion() || !container.animate) {
      return;
    }
    const items = this.items;
    const stagger = 40;
    items.forEach((item, index) => {
      if (!item.animate) {
        return;
      }
      const order = opening ? items.length - 1 - index : index;
      const from: Keyframe = { opacity: "0", transform: "translateY(8px) scale(0.8)" };
      const to: Keyframe = { opacity: "1", transform: "translateY(0) scale(1)" };
      const frames = opening ? [from, to] : [to, from];
      const anim = item.animate(
        frames,
        animationOptions(
          opening ? "SHORT4" : "SHORT3",
          opening ? "EMPHASIZED_DECELERATE" : "EMPHASIZED_ACCELERATE",
          { delay: order * stagger },
        ),
      );
      // Swallow cancellation when toggled rapidly.
      anim.finished.catch(() => {});
    });
  }
}
