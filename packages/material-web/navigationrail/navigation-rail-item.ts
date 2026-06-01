/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { NavigationRailItem } from "./internal/navigation-rail-item.js";
import { itemStyles } from "./internal/navigation-rail-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-navigation-rail-item": MdNavigationRailItem;
  }
}

/**
 * @summary A single destination inside an `<md-navigation-rail>`.
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-navigation-rail-item")
export class MdNavigationRailItem extends NavigationRailItem {
  static override styles: CSSResultOrNative[] = [itemStyles];
}
