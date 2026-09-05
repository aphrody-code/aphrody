/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * The relative size of a carousel item, used by the `multi-browse` layout to
 * lay out items of decreasing prominence.
 */
export type CarouselItemSize = "large" | "medium" | "small";

/**
 * A single item within an `md-carousel`. It is a rounded, clipped container for
 * an image or arbitrary content, sized by the `size` attribute so the parent
 * carousel can compose its layout.
 */
export class CarouselItem extends LitElement {
  /** The relative size. Reflected so CSS can target `[size]`. */
  @property({ reflect: true }) size: CarouselItemSize = "large";

  /** When true, this item is the currently snapped or active item in the view. */
  @property({ type: Boolean, reflect: true }) active = false;

  protected override render() {
    return html`
      <div class="item">
        <slot></slot>
      </div>
    `;
  }
}
