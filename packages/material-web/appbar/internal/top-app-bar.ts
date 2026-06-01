/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property, state } from "lit/decorators.js";

/**
 * Top app bar layout variant.
 *
 * - `small` — single 64dp row, title leading next to the nav icon.
 * - `center` — single 64dp row, title centered.
 * - `medium` — 112dp, title on a second line (headline-small).
 * - `large` — 152dp, title on a second line (headline-medium).
 */
export type TopAppBarVariant = "small" | "center" | "medium" | "large";

/**
 * A top app bar displays navigation, actions, and the title of the current
 * screen at the top of a layout. Implements the four Material 3 variants and
 * the on-scroll container-color fill.
 *
 * Slot a leading nav affordance into `slot="leading"`, the title into the
 * default slot, and trailing actions into `slot="trailing"`.
 */
export class TopAppBar extends LitElement {
  /** The bar layout variant. */
  @property({ reflect: true }) variant: TopAppBarVariant = "small";

  /**
   * A scrollable element whose scroll position drives the on-scroll fill. When
   * unset, the bar listens to the document scroll. Provide the element (not a
   * selector).
   */
  @property({ attribute: false }) scrollTarget: HTMLElement | null = null;

  @state() private scrolled = false;

  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "banner");
    if (!isServer) {
      this.attachScroll();
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    this.detachScroll();
  }

  protected override updated(changed: Map<string, unknown>) {
    if (changed.has("scrollTarget")) {
      this.detachScroll();
      this.attachScroll();
    }
  }

  private get scroller(): HTMLElement | Window {
    return this.scrollTarget ?? window;
  }

  /**
   * Prefer the CSS scroll-driven animation (declarative, runs off the main
   * thread). Only fall back to a JS scroll listener when a custom scroll target
   * is set (which CSS `scroll()` can't reference) or the browser lacks
   * scroll-driven-animation support. The `js-scroll` attribute disables the CSS
   * path so the two never fight.
   */
  private attachScroll() {
    const cssDriven =
      !this.scrollTarget &&
      typeof CSS !== "undefined" &&
      CSS.supports("(animation-timeline: scroll()) and (animation-range: 0% 100%)");
    if (cssDriven) {
      this.removeAttribute("js-scroll");
      return;
    }
    this.setAttribute("js-scroll", "");
    this.scroller.addEventListener("scroll", this.handleScroll, {
      passive: true,
    });
    this.handleScroll();
  }

  private detachScroll() {
    this.scroller.removeEventListener("scroll", this.handleScroll);
  }

  private readonly handleScroll = () => {
    const top = this.scrollTarget ? this.scrollTarget.scrollTop : window.scrollY;
    const scrolled = top > 0;
    if (scrolled !== this.scrolled) {
      this.scrolled = scrolled;
    }
  };

  protected override render() {
    const twoLine = this.variant === "medium" || this.variant === "large";
    return html`
      <header class="bar ${this.scrolled ? "scrolled" : ""}" part="bar">
        <div class="row">
          <span class="leading"><slot name="leading"></slot></span>
          ${twoLine
            ? html`<span class="spacer"></span>`
            : html`<h1 class="headline inline"><slot></slot></h1>`}
          <span class="trailing"><slot name="trailing"></slot></span>
        </div>
        ${twoLine ? html`<h1 class="headline block"><slot></slot></h1>` : ""}
      </header>
    `;
  }
}
