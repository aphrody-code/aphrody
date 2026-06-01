/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * A single tile within an `md-grid-list`. It can span multiple columns or rows
 * via `colspan`/`rowspan`, clips its content to a medium corner radius, and
 * exposes a `footer` slot for an overlay strip at the bottom.
 *
 * Slots:
 * - default — the tile content (e.g. an image).
 * - `footer` — an overlay caption strip pinned to the bottom of the tile.
 */
export class GridTile extends LitElement {
  /** Number of columns this tile spans. */
  @property({ type: Number }) colspan = 1;

  /** Number of rows this tile spans. */
  @property({ type: Number }) rowspan = 1;

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.setAttribute("role", "listitem");
    }
  }

  protected override updated() {
    const colspan = this.colspan > 0 ? this.colspan : 1;
    const rowspan = this.rowspan > 0 ? this.rowspan : 1;
    this.style.gridColumn = `span ${colspan}`;
    this.style.gridRow = `span ${rowspan}`;
  }

  protected override render() {
    return html`
      <div class="tile">
        <div class="body"><slot></slot></div>
        <div class="footer"><slot name="footer"></slot></div>
      </div>
    `;
  }
}
