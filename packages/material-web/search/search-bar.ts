/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { SearchBar } from "./internal/search-bar.js";
import { styles } from "./internal/search-bar-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-search-bar": MdSearchBar;
  }
}

/**
 * @summary A search bar with an expanding (docked or fullscreen) results view.
 *
 * @description
 * ```html
 * <md-search-bar placeholder="Search mail" view="docked">
 *   <md-icon slot="leading">search</md-icon>
 *   <md-list><md-list-item>Recent result</md-list-item></md-list>
 * </md-search-bar>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-search-bar")
export class MdSearchBar extends SearchBar {
  static override styles: CSSResultOrNative[] = [styles];
}
