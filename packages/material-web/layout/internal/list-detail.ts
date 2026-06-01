/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property, state } from "lit/decorators.js";

import { classifyWidth, type WindowSizeClass } from "./scaffold.js";

/** Which pane is shown when the layout is single-pane (compact / medium). */
export type ListDetailPane = "list" | "detail";

/** The detail surfaced on the `list-detail:showing-change` event. */
export interface ListDetailShowingChangeDetail {
  showing: ListDetailPane;
  sizeClass: WindowSizeClass;
}

/**
 * Material 3 list-detail canonical layout. From the Expanded breakpoint up
 * (>= 840dp), the list and detail panes appear side by side — a fixed ~360px
 * list leading, a flexible detail trailing — separated by the 24dp pane spacer
 * (`PANE_SPACER_DP` in `adaptive.rs`). In Compact / Medium (< 840dp) only one
 * pane shows at a time; {@link showDetail} / {@link showList} navigate between
 * them.
 *
 * The leading edge is logical (`list` leads), so the layout mirrors correctly
 * for right-to-left languages without extra work.
 *
 * @slot list - The list / index pane (leading).
 * @slot detail - The detail pane (trailing, flexible).
 *
 * @fires list-detail:showing-change {CustomEvent<ListDetailShowingChangeDetail>}
 *     Fired when the visible pane changes in single-pane mode.
 */
export class ListDetail extends LitElement {
  /**
   * Which pane is visible in single-pane mode. Reflected so CSS can target
   * `[showing='list']` / `[showing='detail']`. Ignored when both panes are
   * shown (expanded and up).
   */
  @property({ reflect: true }) showing: ListDetailPane = "list";

  /** Width in pixels of the fixed list pane in dual-pane mode (M3 ~360px). */
  @property({ type: Number, attribute: "list-width" }) listWidth = 360;

  /**
   * The resolved window size class. Reflected to `size-class`. `true` dual-pane
   * threshold is Expanded (>= 840dp) per the M3 list-detail guidance.
   */
  @property({ reflect: true, attribute: "size-class" })
  sizeClass: WindowSizeClass = "compact";

  @state() private dual = false;

  private resizeObserver: ResizeObserver | null = null;

  /** Whether both panes are visible side by side (Expanded breakpoint and up). */
  get isDualPane(): boolean {
    return this.dual;
  }

  /** Shows the detail pane (single-pane mode); no-op in dual-pane mode. */
  showDetail() {
    this.setShowing("detail");
  }

  /** Shows the list pane (single-pane mode); no-op in dual-pane mode. */
  showList() {
    this.setShowing("list");
  }

  private setShowing(pane: ListDetailPane) {
    if (this.dual || this.showing === pane) {
      return;
    }
    const update = () => {
      this.showing = pane;
      this.dispatchEvent(
        new CustomEvent<ListDetailShowingChangeDetail>("list-detail:showing-change", {
          detail: { showing: pane, sizeClass: this.sizeClass },
          bubbles: true,
          composed: true,
        }),
      );
      return this.updateComplete;
    };

    if (!isServer && typeof document !== "undefined" && "startViewTransition" in document) {
      (document as any).startViewTransition(update);
    } else {
      update();
    }
  }

  override connectedCallback() {
    super.connectedCallback();
    if (isServer || typeof ResizeObserver === "undefined") {
      return;
    }
    this.resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        this.applyWidth(entry.contentRect.width);
      }
    });
    this.resizeObserver.observe(this);
    this.applyWidth(this.getBoundingClientRect().width);
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
  }

  private applyWidth(width: number) {
    if (width <= 0) {
      return;
    }
    const nextSizeClass = classifyWidth(width);
    const nextDual = width >= 840;
    if (nextSizeClass === this.sizeClass && nextDual === this.dual) {
      return;
    }
    const update = () => {
      this.sizeClass = nextSizeClass;
      this.dual = nextDual;
      return this.updateComplete;
    };

    if (!isServer && typeof document !== "undefined" && "startViewTransition" in document) {
      (document as any).startViewTransition(update);
    } else {
      update();
    }
  }

  protected override render() {
    const listStyle = this.dual
      ? `flex: 0 0 ${this.listWidth}px; inline-size: ${this.listWidth}px;`
      : "";
    return html`
      <div class="container" part="container">
        <div class="list" part="list" style=${listStyle}>
          <slot name="list"></slot>
        </div>
        <div class="detail" part="detail">
          <slot name="detail"></slot>
        </div>
      </div>
    `;
  }
}
