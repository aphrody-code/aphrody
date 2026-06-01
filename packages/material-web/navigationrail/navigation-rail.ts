/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { NavigationRail } from "./internal/navigation-rail.js";
import { styles } from "./internal/navigation-rail-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-navigation-rail": MdNavigationRail;
  }
}

/**
 * @summary A navigation rail switches between top-level views on medium and
 * larger windows, in a collapsed (80dp) or expanded (≥220dp) layout.
 *
 * @description
 * ```html
 * <md-navigation-rail value="home" alignment="center">
 *   <md-icon-button slot="menu"><md-icon>menu</md-icon></md-icon-button>
 *   <md-fab slot="fab" aria-label="Compose"></md-fab>
 *   <md-navigation-rail-item value="home" label="Home">…</md-navigation-rail-item>
 *   <md-navigation-rail-item value="search" label="Search">…</md-navigation-rail-item>
 * </md-navigation-rail>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-navigation-rail")
export class MdNavigationRail extends NavigationRail {
  static override styles: CSSResultOrNative[] = [styles];
}
