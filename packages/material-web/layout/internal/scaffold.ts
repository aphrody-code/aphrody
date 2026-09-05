/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * Material Design 3 window size class (breakpoint), classifying a container
 * **width** in density-independent pixels (dp).
 *
 * Mirrors the canonical Rust source of truth
 * (`crates/m3-tokens/src/adaptive.rs`, `Breakpoint`):
 *
 * | Class         | Width (dp)  | Panes | Navigation         |
 * |---------------|-------------|-------|--------------------|
 * | `compact`     | `< 600`     | 1     | Navigation bar     |
 * | `medium`      | `600–839`   | 1     | Collapsed nav rail |
 * | `expanded`    | `840–1199`  | 2     | Collapsed nav rail |
 * | `large`       | `1200–1599` | 2     | Expanded nav rail  |
 * | `extra-large` | `>= 1600`   | 3     | Expanded nav rail  |
 */
export type WindowSizeClass = "compact" | "medium" | "expanded" | "large" | "extra-large";

/**
 * Classify a container width (dp/px) into its M3 window size class. The
 * boundaries match `Breakpoint::from_width_dp` in `adaptive.rs` byte-for-byte:
 * `600`, `840`, `1200`, `1600`.
 */
export function classifyWidth(width: number): WindowSizeClass {
  if (width < 600) {
    return "compact";
  }
  if (width < 840) {
    return "medium";
  }
  if (width < 1200) {
    return "expanded";
  }
  if (width < 1600) {
    return "large";
  }
  return "extra-large";
}

/** Window margin (dp) for a size class — `16` for compact, `24` otherwise. */
export function marginFor(sizeClass: WindowSizeClass): number {
  return sizeClass === "compact" ? 16 : 24;
}

/**
 * The detail surfaced on the `scaffold:size-class-change` event.
 */
export interface ScaffoldSizeClassChangeDetail {
  sizeClass: WindowSizeClass;
  width: number;
}

/**
 * Adaptive page scaffold implementing the M3 "parts of layout" model: bars
 * frame the content, the navigation rail/bar fills the perimeter, and the body
 * holds the panes.
 *
 * The component observes its own width with a {@link ResizeObserver} and
 * reflects the current window size class onto the `size-class` attribute, so
 * authors can style the slots responsively. In `compact`, navigation docks at
 * the bottom (navigation bar); in `medium` and up it docks at the leading edge
 * as a rail.
 *
 * @slot top-bar - Top app bar, pinned to the top across the full width.
 * @slot navigation - Navigation surface. Rendered as a bottom bar in compact,
 *     a leading rail otherwise.
 * @slot - (default) The body / pane region. Scrolls independently.
 * @slot bottom-bar - Optional bottom app bar (compact-oriented), below the body.
 * @slot fab - Floating action button, anchored to the bottom-trailing corner.
 *
 * @fires scaffold:size-class-change {CustomEvent<ScaffoldSizeClassChangeDetail>}
 *     Fired whenever the resolved window size class changes.
 */
export class Scaffold extends LitElement {
  /**
   * The current window size class. Reflected to the `size-class` attribute so
   * CSS (and descendant components) can adapt. Computed from the host width;
   * setting it manually is overridden on the next resize.
   */
  @property({ reflect: true, attribute: "size-class" })
  sizeClass: WindowSizeClass = "compact";

  private resizeObserver: ResizeObserver | null = null;

  override connectedCallback() {
    super.connectedCallback();
    if (isServer || typeof ResizeObserver === "undefined") {
      return;
    }
    this.resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const width = entry.contentRect.width;
        this.updateSizeClass(width);
      }
    });
    this.resizeObserver.observe(this);
    // Seed from the current layout box if already laid out.
    this.updateSizeClass(this.getBoundingClientRect().width);
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
  }

  private updateSizeClass(width: number) {
    if (width <= 0) {
      return;
    }
    const next = classifyWidth(width);
    if (next === this.sizeClass) {
      return;
    }
    const update = () => {
      this.sizeClass = next;
      this.dispatchEvent(
        new CustomEvent<ScaffoldSizeClassChangeDetail>("scaffold:size-class-change", {
          detail: { sizeClass: next, width },
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

  protected override render() {
    return html`
      <div class="scaffold" part="scaffold">
        <header class="top-bar" part="top-bar">
          <slot name="top-bar"></slot>
        </header>
        <nav class="navigation" part="navigation">
          <slot name="navigation"></slot>
        </nav>
        <main class="body" part="body">
          <slot></slot>
        </main>
        <footer class="bottom-bar" part="bottom-bar">
          <slot name="bottom-bar"></slot>
        </footer>
        <div class="fab" part="fab">
          <slot name="fab"></slot>
        </div>
      </div>
    `;
  }
}
