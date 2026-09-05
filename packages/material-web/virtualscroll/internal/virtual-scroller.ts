/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property, query, state } from "lit/decorators.js";
import { styleMap } from "lit/directives/style-map.js";

/**
 * A fixed-size virtual scroller — the web-component equivalent of Angular CDK's
 * `cdk-virtual-scroll-viewport` (fixed item size). It renders only the rows
 * intersecting the viewport (plus a small overscan buffer), so a list of tens
 * of thousands of items scrolls smoothly with a constant DOM size.
 *
 * Data-driven: set `.items` (the full array) and `.renderItem` (a function
 * returning a Lit template or string for one item). All items must share the
 * same `itemHeight`.
 *
 * @fires virtual-scroll:range {CustomEvent<{start: number; end: number}>} Fired
 *     when the rendered window changes.
 */
export class VirtualScroller extends LitElement {
  /** The full backing data set. */
  @property({ attribute: false }) items: readonly unknown[] = [];

  /** Fixed row height in px. */
  @property({ type: Number, attribute: "item-height" }) itemHeight = 48;

  /** Extra rows rendered above/below the viewport to mask fast scrolling. */
  @property({ type: Number }) buffer = 4;

  /** Renders one item; defaults to its string coercion. */
  @property({ attribute: false })
  renderItem: (item: unknown, index: number) => unknown = (item) => String(item);

  @state() private scrollPos = 0;
  @state() private viewportHeight = 0;

  @query(".viewport") private readonly viewport!: HTMLElement | null;

  private resizeObserver: ResizeObserver | null = null;

  override connectedCallback() {
    super.connectedCallback();
    if (isServer || typeof ResizeObserver === "undefined") {
      return;
    }
    this.resizeObserver = new ResizeObserver((entries) => {
      const h = entries[0]?.contentRect.height ?? 0;
      if (h !== this.viewportHeight) {
        this.viewportHeight = h;
      }
    });
  }

  protected override firstUpdated() {
    if (this.viewport) {
      this.resizeObserver?.observe(this.viewport);
      this.viewportHeight = this.viewport.clientHeight;
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
  }

  /** Scroll so that the item at `index` is at the top of the viewport. */
  scrollToIndex(index: number) {
    this.viewport?.scrollTo({ top: index * this.itemHeight });
  }

  private get range(): { start: number; end: number } {
    const count = this.items.length;
    if (count === 0 || this.itemHeight <= 0) {
      return { start: 0, end: 0 };
    }
    const visible = Math.ceil((this.viewportHeight || 0) / this.itemHeight);
    const start = Math.max(0, Math.floor(this.scrollPos / this.itemHeight) - this.buffer);
    const end = Math.min(count, start + visible + this.buffer * 2);
    return { start, end };
  }

  private readonly handleScroll = (event: Event) => {
    this.scrollPos = (event.target as HTMLElement).scrollTop;
  };

  protected override updated(changed: Map<string, unknown>) {
    if (changed.has("scrollPos") || changed.has("items") || changed.has("viewportHeight")) {
      const { start, end } = this.range;
      this.dispatchEvent(new CustomEvent("virtual-scroll:range", { detail: { start, end } }));
    }
  }

  protected override render() {
    const { start, end } = this.range;
    const total = this.items.length * this.itemHeight;
    const offset = start * this.itemHeight;
    const rows = [];
    for (let i = start; i < end; i++) {
      rows.push(
        html`<div class="row" role="listitem" style=${styleMap({ height: `${this.itemHeight}px` })}>
          ${this.renderItem(this.items[i], i)}
        </div>`,
      );
    }
    return html`
      <div class="viewport" role="list" @scroll=${this.handleScroll}>
        <div class="spacer" style=${styleMap({ height: `${total}px` })}>
          <div class="content" style=${styleMap({ transform: `translateY(${offset}px)` })}>
            ${rows}
          </div>
        </div>
      </div>
    `;
  }
}
