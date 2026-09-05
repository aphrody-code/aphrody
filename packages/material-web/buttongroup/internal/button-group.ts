/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property, queryAssignedElements } from "lit/decorators.js";

/**
 * The `detail` payload of the `button-group:change` event.
 *
 * `value` is the currently selected value (single-select) or the array of
 * selected values (multiselect).
 */
export interface ButtonGroupChangeDetail {
  value: string | string[];
}

/**
 * A connected button group orchestrates a row of related buttons, applying the
 * Material 3 "connected" shape treatment (the leading child is rounded on the
 * leading edge, the trailing child on the trailing edge, inner children get a
 * small 4px radius) and a single/multi selection model.
 *
 * This is the M3 successor to the segmented button: the children are ordinary
 * buttons (or any focusable element) slotted as the default content. Selection
 * is tracked by each child's `value` attribute (falling back to its index).
 *
 * @fires button-group:change {CustomEvent<ButtonGroupChangeDetail>} Fired when
 *     the selection changes through user interaction.
 */
export class ButtonGroup extends LitElement {
  /** When true, multiple children can be selected independently (toggle). */
  @property({ type: Boolean, reflect: true }) multiselect = false;

  /**
   * The selected value. A single string in single-select mode, or a
   * comma-separated string of values reflected to the attribute in multiselect
   * mode. Read {@link selectedValues} for the parsed array.
   */
  @property({ type: String }) value = "";

  @queryAssignedElements({ flatten: true })
  private readonly items!: HTMLElement[];

  /** The parsed list of currently selected values. */
  selectedValues(): string[] {
    if (!this.value) {
      return [];
    }
    return this.multiselect ? this.value.split(",").filter(Boolean) : [this.value];
  }

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.setAttribute("role", "group");
    }
  }

  protected override render() {
    return html`
      <div
        class="container"
        @click=${this.handleClick}
        @keydown=${this.handleKeydown}
        @slotchange=${this.handleSlotChange}
      >
        <slot></slot>
      </div>
    `;
  }

  protected override updated() {
    this.syncChildren();
  }

  private itemValueOf(item: HTMLElement, index: number): string {
    return item.getAttribute("value") ?? String(index);
  }

  private handleSlotChange() {
    this.syncChildren();
  }

  /**
   * Reflects the current selection and roving-tabindex onto the slotted
   * children, and applies the connected-shape data attributes the styles key
   * off of.
   */
  private syncChildren() {
    const items = this.items;
    if (items.length === 0) {
      return;
    }
    const selected = this.selectedValues();
    let focusableAssigned = false;
    items.forEach((item, index) => {
      const itemValue = this.itemValueOf(item, index);
      const isSelected = selected.includes(itemValue);
      item.toggleAttribute("data-selected", isSelected);
      item.setAttribute("aria-pressed", String(isSelected));

      // Connected-shape position hint consumed by ::slotted() styles.
      const position =
        items.length === 1
          ? "only"
          : index === 0
            ? "first"
            : index === items.length - 1
              ? "last"
              : "middle";
      item.setAttribute("data-position", position);

      // Roving tabindex: the first selected (or the first item) is tabbable.
      const shouldFocus =
        !focusableAssigned && (isSelected || (selected.length === 0 && index === 0));
      if (shouldFocus) {
        item.tabIndex = 0;
        focusableAssigned = true;
      } else {
        item.tabIndex = -1;
      }
    });
    if (!focusableAssigned) {
      items[0].tabIndex = 0;
    }
  }

  private handleClick(event: Event) {
    const target = this.itemFromEvent(event);
    if (!target) {
      return;
    }
    const index = this.items.indexOf(target);
    this.select(this.itemValueOf(target, index));
  }

  private select(itemValue: string) {
    if (this.multiselect) {
      const set = new Set(this.selectedValues());
      if (set.has(itemValue)) {
        set.delete(itemValue);
      } else {
        set.add(itemValue);
      }
      this.value = [...set].join(",");
    } else {
      this.value = this.value === itemValue ? "" : itemValue;
    }
    this.syncChildren();
    this.dispatchEvent(
      new CustomEvent<ButtonGroupChangeDetail>("button-group:change", {
        detail: { value: this.multiselect ? this.selectedValues() : this.value },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleKeydown(event: KeyboardEvent) {
    const items = this.items;
    if (items.length === 0) {
      return;
    }
    const current = this.itemFromEvent(event);
    const currentIndex = current ? items.indexOf(current) : -1;

    if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      event.preventDefault();
      const dir = event.key === "ArrowRight" ? 1 : -1;
      const next = (currentIndex + dir + items.length) % items.length;
      items.forEach((item, i) => {
        item.tabIndex = i === next ? 0 : -1;
      });
      items[next].focus();
      return;
    }

    if ((event.key === "Enter" || event.key === " ") && current) {
      event.preventDefault();
      this.select(this.itemValueOf(current, currentIndex));
    }
  }

  private itemFromEvent(event: Event): HTMLElement | null {
    const path = event.composedPath();
    for (const item of this.items) {
      if (path.includes(item)) {
        return item;
      }
    }
    return null;
  }
}
