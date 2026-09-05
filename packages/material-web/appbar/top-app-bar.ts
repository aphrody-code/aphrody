/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { TopAppBar } from "./internal/top-app-bar.js";
import { styles } from "./internal/top-app-bar-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-top-app-bar": MdTopAppBar;
  }
}

/**
 * @summary A top app bar with the four Material 3 variants and on-scroll fill.
 *
 * @description
 * ```html
 * <md-top-app-bar variant="large">
 *   <md-icon-button slot="leading"><md-icon>menu</md-icon></md-icon-button>
 *   Inbox
 *   <md-icon-button slot="trailing"><md-icon>search</md-icon></md-icon-button>
 * </md-top-app-bar>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-top-app-bar")
export class MdTopAppBar extends TopAppBar {
  static override styles: CSSResultOrNative[] = [styles];
}
