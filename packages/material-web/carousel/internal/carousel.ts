/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property, query, state } from "lit/decorators.js";

import { prefersReducedMotion } from "../../internal/motion/easing-and-duration.js";

/**
 * The carousel layout strategy.
 *
 * - `hero` — a single large item dominates, with a peek of the next.
 * - `multi-browse` — multiple items of decreasing size are visible at once.
 */
export type CarouselLayout = "hero" | "multi-browse";

/**
 * A carousel lays out a scrollable, snapping row of items. It implements the
 * Material 3 carousel spec using CSS scroll-snap and exposes `next()`/`prev()`
 * navigation plus optional arrow controls that hide when there is no overflow.
 *
 * Slot `md-carousel-item`s into the default slot.
 *
 * ```html
 * <md-carousel layout="multi-browse">
 *   <md-carousel-item size="large"><img src="a.jpg" alt=""></md-carousel-item>
 *   <md-carousel-item size="medium"><img src="b.jpg" alt=""></md-carousel-item>
 *   <md-carousel-item size="small"><img src="c.jpg" alt=""></md-carousel-item>
 * </md-carousel>
 * ```
 */
export class Carousel extends LitElement {
  /** The layout strategy. Reflected so CSS can target `[layout]`. */
  @property({ reflect: true }) layout: CarouselLayout = "multi-browse";

  /** When true, render previous/next arrow controls. */
  @property({ type: Boolean, attribute: "show-arrows" }) showArrows = false;

  @state() private hasOverflow = false;

  @query(".scroller") private readonly scroller!: HTMLElement | null;

  private resizeObserver?: ResizeObserver;
  private activeUpdatePending = false;
  private readonly supportsScrollSnap = !isServer && "onscrollsnapchange" in HTMLElement.prototype;

  /** Scrolls forward by roughly one item width. */
  next() {
    this.scrollByItem(1);
  }

  /** Scrolls backward by roughly one item width. */
  prev() {
    this.scrollByItem(-1);
  }

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer && typeof ResizeObserver === "function") {
      this.resizeObserver = new ResizeObserver(() => {
        this.updateOverflow();
        this.updateActiveItem();
      });
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    this.resizeObserver?.disconnect();
  }

  protected override firstUpdated() {
    const scroller = this.scroller;
    if (scroller && this.resizeObserver) {
      this.resizeObserver.observe(scroller);
    }
    this.updateOverflow();
    this.updateActiveItem();
  }

  protected override render() {
    return html`
      <div class="carousel">
        ${this.renderArrow("prev")}
        <div
          class="scroller"
          @scroll=${this.handleScroll}
          @scrollsnapchange=${this.handleScrollSnapChange}
          @scrollsnapchanging=${this.handleScrollSnapChanging}
        >
          <slot @slotchange=${this.handleSlotChange}></slot>
        </div>
        ${this.renderArrow("next")}
      </div>
    `;
  }

  private renderArrow(direction: "prev" | "next") {
    if (!this.showArrows || !this.hasOverflow) {
      return html``;
    }
    const isPrev = direction === "prev";
    return html`
      <button
        class="arrow ${direction}"
        aria-label=${isPrev ? "Previous" : "Next"}
        @click=${isPrev ? this.handlePrevClick : this.handleNextClick}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          ${isPrev
            ? html`<path d="M15.4 7.4 14 6l-6 6 6 6 1.4-1.4-4.6-4.6Z"></path>`
            : html`<path d="M8.6 16.6 10 18l6-6-6-6-1.4 1.4 4.6 4.6Z"></path>`}
        </svg>
      </button>
    `;
  }

  private handlePrevClick() {
    this.prev();
  }

  private handleNextClick() {
    this.next();
  }

  private handleScroll() {
    this.updateOverflow();
    if (this.supportsScrollSnap) {
      return;
    }
    if (!this.activeUpdatePending) {
      this.activeUpdatePending = true;
      requestAnimationFrame(() => {
        this.updateActiveItem();
        this.activeUpdatePending = false;
      });
    }
  }

  private handleScrollSnapChange(event: Event) {
    const snapTarget = (event as any).snapTargetInline as HTMLElement | null;
    if (snapTarget) {
      this.setActiveItem(snapTarget);
    }
  }

  private handleScrollSnapChanging(event: Event) {
    const snapTarget = (event as any).snapTargetInline as HTMLElement | null;
    if (snapTarget) {
      this.setActiveItem(snapTarget);
    }
  }

  private handleSlotChange() {
    this.updateOverflow();
    this.updateActiveItem();
  }

  private setActiveItem(closestItem: HTMLElement) {
    const slot = this.shadowRoot?.querySelector("slot");
    if (!slot) {
      return;
    }

    const items = slot.assignedElements({ flatten: true }) as HTMLElement[];
    if (items.length === 0) {
      return;
    }

    let changed = false;
    let activeIndex = -1;

    items.forEach((item, index) => {
      const active = item === closestItem;
      if ("active" in item) {
        const oldActive = (item as any).active;
        if (oldActive !== active) {
          (item as any).active = active;
          changed = true;
        }
        if (active) {
          activeIndex = index;
        }
      }
    });

    if (changed && activeIndex !== -1) {
      this.dispatchEvent(
        new CustomEvent("carousel-change", {
          detail: { index: activeIndex, item: closestItem },
          bubbles: true,
          composed: true,
        }),
      );
    }
  }

  private updateActiveItem() {
    const scroller = this.scroller;
    if (!scroller) {
      return;
    }

    const slot = this.shadowRoot?.querySelector("slot");
    if (!slot) {
      return;
    }

    const items = slot.assignedElements({ flatten: true }) as HTMLElement[];
    if (items.length === 0) {
      return;
    }

    const scrollerRect = scroller.getBoundingClientRect();

    // Fallback tracking logic aligned with CSS scroll-snap-align
    const firstItem = items[0];
    const itemComputedStyle = getComputedStyle(firstItem);
    const snapAlign = itemComputedStyle.scrollSnapAlign || "start";
    const align = snapAlign.split(" ")[0] || "start";

    let closestItem: HTMLElement | null = null;
    let minDistance = Infinity;

    for (const item of items) {
      const rect = item.getBoundingClientRect();
      let distance = 0;

      if (align === "start") {
        const paddingLeft =
          parseFloat(getComputedStyle(scroller).scrollPaddingLeft) ||
          parseFloat(getComputedStyle(scroller).scrollPaddingInlineStart) ||
          0;
        distance = Math.abs(rect.left - (scrollerRect.left + paddingLeft));
      } else if (align === "center") {
        const scrollerCenter = scrollerRect.left + scrollerRect.width / 2;
        const itemCenter = rect.left + rect.width / 2;
        distance = Math.abs(itemCenter - scrollerCenter);
      } else if (align === "end") {
        const paddingRight =
          parseFloat(getComputedStyle(scroller).scrollPaddingRight) ||
          parseFloat(getComputedStyle(scroller).scrollPaddingInlineEnd) ||
          0;
        distance = Math.abs(rect.right - (scrollerRect.right - paddingRight));
      } else {
        distance = Math.abs(rect.left - scrollerRect.left);
      }

      if (distance < minDistance) {
        minDistance = distance;
        closestItem = item;
      }
    }

    if (closestItem) {
      this.setActiveItem(closestItem);
    }
  }

  private scrollByItem(direction: number) {
    const scroller = this.scroller;
    if (!scroller) {
      return;
    }
    const slot = this.shadowRoot?.querySelector("slot");
    if (!slot) {
      return;
    }
    const items = slot.assignedElements({ flatten: true }) as HTMLElement[];
    if (items.length === 0) {
      return;
    }

    const activeItem = items.find((item) => (item as any).active) || items[0];
    const currentIndex = items.indexOf(activeItem);
    let targetIndex = currentIndex + direction;
    if (targetIndex < 0) {
      targetIndex = 0;
    }
    if (targetIndex >= items.length) {
      targetIndex = items.length - 1;
    }

    const targetItem = items[targetIndex];
    if (targetItem) {
      const scrollerRect = scroller.getBoundingClientRect();
      const itemRect = targetItem.getBoundingClientRect();
      const itemScrollLeft = itemRect.left - scrollerRect.left + scroller.scrollLeft;

      const computedStyle = getComputedStyle(scroller);
      const itemComputedStyle = getComputedStyle(targetItem);
      const snapAlign = itemComputedStyle.scrollSnapAlign || "start";
      const align = snapAlign.split(" ")[0] || "start";

      let targetScrollLeft = scroller.scrollLeft;
      const containerWidth = scroller.clientWidth;
      const itemWidth = targetItem.offsetWidth;

      if (align === "start") {
        const paddingLeft =
          parseFloat(computedStyle.scrollPaddingLeft) ||
          parseFloat(computedStyle.scrollPaddingInlineStart) ||
          0;
        targetScrollLeft = itemScrollLeft - paddingLeft;
      } else if (align === "center") {
        targetScrollLeft = itemScrollLeft - (containerWidth - itemWidth) / 2;
      } else if (align === "end") {
        const paddingRight =
          parseFloat(computedStyle.scrollPaddingRight) ||
          parseFloat(computedStyle.scrollPaddingInlineEnd) ||
          0;
        targetScrollLeft = itemScrollLeft - containerWidth + itemWidth + paddingRight;
      } else {
        targetScrollLeft = itemScrollLeft;
      }

      scroller.scrollTo({
        left: targetScrollLeft,
        behavior: prefersReducedMotion() ? "auto" : "smooth",
      });
    }
  }

  private updateOverflow() {
    const scroller = this.scroller;
    if (!scroller) {
      return;
    }
    this.hasOverflow = scroller.scrollWidth > scroller.clientWidth + 1;
  }
}
