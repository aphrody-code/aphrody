/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * The sizing behaviour of a pane within a multi-pane layout, mirroring
 * `PaneKind` in `crates/m3-tokens/src/adaptive.rs`.
 *
 * - `fixed` — fixed-width pane (e.g. a list). Uses {@link Pane.width}.
 * - `flexible` — absorbs the remaining width (e.g. a detail view).
 */
export type PaneVariant = "fixed" | "flexible";

/**
 * A single Material 3 layout pane: a self-contained content surface with
 * padding and independent scrolling. Panes are the only place content lives in
 * the M3 layout model; they are composed by `md-list-detail`,
 * `md-supporting-pane`, or laid out directly in a `md-scaffold` body.
 *
 * A `fixed` pane keeps a constant width ({@link width}); a `flexible` pane
 * grows to fill the remaining space. The host exposes `flex` accordingly so it
 * drops into a flex row without extra wrappers.
 *
 * @slot - The pane content.
 */
export class Pane extends LitElement {
  /**
   * Whether the pane is fixed-width or flexible. Reflected so containers and
   * CSS can target `[variant='fixed']` / `[variant='flexible']`.
   */
  @property({ reflect: true }) variant: PaneVariant = "flexible";

  /**
   * Width in CSS pixels applied when {@link variant} is `fixed`. Ignored for
   * flexible panes. Defaults to the M3 list-pane width of 360px.
   */
  @property({ type: Number }) width = 360;

  /** A human-readable name for the pane, exposed as the region's a11y label. */
  @property({ attribute: "pane-name" }) paneName = "";

  /**
   * When true, applies an M3 large corner radius to the pane surface (used for
   * explicit containment / emphasis). Reflected for CSS targeting.
   */
  @property({ type: Boolean, reflect: true }) rounded = false;

  protected override updated() {
    // Drive layout from the resolved variant so the host participates in a flex
    // row directly (fixed = constant basis, flexible = grow).
    if (this.variant === "fixed") {
      this.style.flex = `0 0 ${this.width}px`;
      this.style.inlineSize = `${this.width}px`;
    } else {
      this.style.flex = "1 1 0%";
      this.style.inlineSize = "";
    }
  }

  protected override render() {
    return html`
      <section class="pane" part="pane" role="region" aria-label=${this.paneName || "pane"}>
        <slot></slot>
      </section>
    `;
  }
}
