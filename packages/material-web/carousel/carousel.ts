/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Carousel } from "./internal/carousel.js";
import { styles } from "./internal/carousel-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-carousel": MdCarousel;
  }
}

/**
 * @summary Carousels lay out a scrollable, snapping row of items.
 *
 * @description
 * Set `layout` to `hero` or `multi-browse` (default), and `show-arrows` to
 * render navigation controls when the content overflows. Use `next()`/`prev()`
 * to scroll programmatically. Slot `md-carousel-item`s into the default slot.
 *
 * ```html
 * <md-carousel layout="multi-browse" show-arrows>
 *   <md-carousel-item size="large"><img src="a.jpg" alt=""></md-carousel-item>
 *   <md-carousel-item size="medium"><img src="b.jpg" alt=""></md-carousel-item>
 * </md-carousel>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-carousel")
export class MdCarousel extends Carousel {
  static override styles: CSSResultOrNative[] = [styles];
}
