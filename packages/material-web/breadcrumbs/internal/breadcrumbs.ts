/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing, TemplateResult } from "lit";
import { property, queryAssignedElements, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";

/**
 * A Material 3 breadcrumb trail. Shows the path of pages leading to the current
 * one as a list of links separated by a customizable separator.
 *
 * Slot the trail items (typically `<md-link>`, `<a>`, or other inline
 * elements) into the default slot, in order from the root to the current page.
 * The separator defaults to `/` and can be overridden with the `separator`
 * property or, for rich content, the `separator` slot.
 *
 * When the number of items exceeds `max-items`, the middle is collapsed behind
 * an ellipsis (`…`) affordance; activating it expands the full trail.
 *
 * Renders a `<nav aria-label>` wrapping an ordered list (`<ol>`) for correct
 * assistive-technology semantics.
 *
 * ```html
 * <md-breadcrumbs aria-label="Breadcrumb" max-items="4">
 *   <md-link href="/">Home</md-link>
 *   <md-link href="/library">Library</md-link>
 *   <md-link href="/library/data">Data</md-link>
 *   <span>Current</span>
 * </md-breadcrumbs>
 * ```
 */
export class Breadcrumbs extends LitElement {
  /**
   * The separator rendered between items. Defaults to `/`. For rich separators
   * use the `separator` slot instead, which takes precedence.
   */
  @property() separator = "/";

  /**
   * The accessible label for the wrapping `<nav>`. Defaults to `Breadcrumb`.
   */
  @property({ attribute: "label" }) label = "Breadcrumb";

  /**
   * Collapse the trail when it contains more than this many items. The first
   * `items-before-collapse` and last `items-after-collapse` items stay visible
   * with an ellipsis between them. Set to `0` (or a value large enough) to
   * never collapse.
   */
  @property({ type: Number, attribute: "max-items" }) maxItems = 0;

  /** Number of items to show before the collapse ellipsis. */
  @property({ type: Number, attribute: "items-before-collapse" })
  itemsBeforeCollapse = 1;

  /** Number of items to show after the collapse ellipsis. */
  @property({ type: Number, attribute: "items-after-collapse" })
  itemsAfterCollapse = 1;

  /** The accessible label for the expand (ellipsis) button. */
  @property({ attribute: "expand-text" }) expandText = "Show path";

  @queryAssignedElements({ flatten: true })
  private readonly items!: HTMLElement[];

  @state() private expanded = false;

  @state() private itemCount = 0;

  /** Whether the trail is currently collapsed behind an ellipsis. */
  @state() private collapsing = false;

  /** Source indices of the items to render, in order. Derived before render. */
  @state() private visibleIndices: number[] = [];

  protected override willUpdate(changed: Map<string, unknown>) {
    // Recompute the collapse layout whenever any input that affects it changes,
    // so render() stays a pure function of derived state.
    if (
      changed.has("expanded") ||
      changed.has("itemCount") ||
      changed.has("maxItems") ||
      changed.has("itemsBeforeCollapse") ||
      changed.has("itemsAfterCollapse")
    ) {
      const total = this.itemCount;
      this.collapsing =
        !this.expanded &&
        this.maxItems > 0 &&
        total > this.maxItems &&
        total > this.itemsBeforeCollapse + this.itemsAfterCollapse;

      if (this.collapsing) {
        const before = Array.from(
          { length: Math.max(0, this.itemsBeforeCollapse) },
          (_unused, i) => i,
        );
        const after = Array.from(
          { length: Math.max(0, this.itemsAfterCollapse) },
          (_unused, i) => total - this.itemsAfterCollapse + i,
        );
        this.visibleIndices = [...before, ...after];
      } else {
        this.visibleIndices = Array.from({ length: total }, (_unused, i) => i);
      }
    }
  }

  protected override render() {
    return html`
      <nav aria-label=${this.label} part="nav">
        <ol class="list" part="list">
          ${this.renderItems(this.visibleIndices, this.collapsing, this.itemCount)}
        </ol>
      </nav>
      <!-- Hidden host slot; items are projected per-index by renderItems. -->
      <slot class="source" @slotchange=${this.handleSlotChange} hidden aria-hidden="true"></slot>
    `;
  }

  private renderItems(visible: number[], collapsing: boolean, total: number): TemplateResult[] {
    const lis: TemplateResult[] = [];
    visible.forEach((index, position) => {
      const atCollapsePoint = collapsing && position === this.itemsBeforeCollapse;
      if (atCollapsePoint) {
        // Insert the ellipsis (with surrounding separators) between the
        // "before" block and the "after" block.
        if (position > 0) {
          lis.push(this.renderSeparator());
        }
        lis.push(this.renderEllipsis());
        lis.push(this.renderSeparator());
      } else if (position > 0) {
        lis.push(this.renderSeparator());
      }
      const isLast = index === total - 1;
      lis.push(html`
        <li class=${classMap({ item: true, current: isLast })} part="item">
          <slot name="item-${index}" aria-current=${isLast ? "page" : nothing}></slot>
        </li>
      `);
    });
    return lis;
  }

  private renderSeparator() {
    return html`
      <li class="separator" aria-hidden="true" part="separator">
        <slot name="separator">${this.separator}</slot>
      </li>
    `;
  }

  private renderEllipsis() {
    return html`
      <li class="ellipsis-item" part="ellipsis">
        <button
          class="ellipsis"
          type="button"
          aria-label=${this.expandText}
          aria-expanded="false"
          @click=${this.handleExpand}
        >
          …
        </button>
      </li>
    `;
  }

  private handleExpand() {
    this.expanded = true;
  }

  private handleSlotChange() {
    const els = this.items;
    this.itemCount = els.length;
    // Assign a stable, index-based slot name to each child so the rendered list
    // can place items individually around separators.
    els.forEach((el, i) => {
      el.setAttribute("slot", `item-${i}`);
    });
  }
}
