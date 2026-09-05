/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { CarouselItem } from "./internal/carousel-item.js";
import { styles } from "./internal/carousel-item-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-carousel-item": MdCarouselItem;
  }
}

/**
 * @summary A single item within an `md-carousel`.
 *
 * @description
 * A rounded, clipped container for an image or arbitrary content. Set `size`
 * (`large` | `medium` | `small`) to control its prominence in the
 * `multi-browse` layout.
 *
 * ```html
 * <md-carousel-item size="large">
 *   <img src="photo.jpg" alt="">
 * </md-carousel-item>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-carousel-item")
export class MdCarouselItem extends CarouselItem {
  static override styles: CSSResultOrNative[] = [styles];
}
