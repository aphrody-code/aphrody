/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * A grid list lays out a collection of `md-grid-tile` children in a uniform
 * CSS grid. The number of columns, the inter-tile gap, and the row height are
 * all configurable.
 *
 * The `row-height` attribute accepts three forms:
 * - a pixel number (e.g. `120`) — every row is that tall;
 * - an aspect ratio (e.g. `16:9`) — each row's height tracks the column width;
 * - `fit` — rows share the host's height equally.
 *
 * Slot:
 * - default — the `md-grid-tile` children.
 */
export class GridList extends LitElement {
  /** Number of equal-width columns. */
  @property({ type: Number }) cols = 4;

  /** Gap between tiles, in pixels. */
  @property({ type: Number }) gap = 8;

  /**
   * Row sizing. A bare number is treated as pixels; `'fit'` distributes the
   * host height across rows; a `'W:H'` string sets the row aspect ratio.
   */
  @property({ attribute: "row-height" }) rowHeight: string = "1:1";

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.setAttribute("role", "list");
    }
  }

  protected override updated() {
    this.applyLayout();
  }

  protected override firstUpdated() {
    this.applyLayout();
  }

  private applyLayout() {
    const cols = this.cols > 0 ? this.cols : 1;
    this.style.setProperty("--_cols", String(cols));
    this.style.setProperty("--_gap", `${this.gap}px`);

    const value = (this.rowHeight ?? "").toString().trim();
    const ratioMatch = /^(\d+(?:\.\d+)?)\s*:\s*(\d+(?:\.\d+)?)$/.exec(value);
    if (value === "fit") {
      // Equal share of the host height across the implied number of rows.
      this.style.setProperty("--_row-height", "auto");
      this.style.setProperty("--_grid-auto-rows", "1fr");
      this.style.setProperty("--_aspect-ratio", "auto");
    } else if (ratioMatch) {
      const w = Number(ratioMatch[1]);
      const h = Number(ratioMatch[2]);
      this.style.setProperty("--_aspect-ratio", w > 0 && h > 0 ? `${w} / ${h}` : "1 / 1");
      this.style.setProperty("--_grid-auto-rows", "auto");
      this.style.setProperty("--_row-height", "auto");
    } else {
      const px = Number(value);
      const height = Number.isFinite(px) && px > 0 ? px : 120;
      this.style.setProperty("--_row-height", `${height}px`);
      this.style.setProperty("--_grid-auto-rows", `${height}px`);
      this.style.setProperty("--_aspect-ratio", "auto");
    }
  }

  protected override render() {
    return html`<div class="grid"><slot></slot></div>`;
  }
}
