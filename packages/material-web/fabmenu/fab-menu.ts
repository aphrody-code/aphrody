/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { FabMenu } from "./internal/fab-menu.js";
import { styles } from "./internal/fab-menu-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-fab-menu": MdFabMenu;
  }
}

/**
 * @summary A FAB menu — a primary FAB that reveals a stack of action items.
 *
 * @description
 * Slot `md-fab-menu-item` children. Toggle the `open` attribute or call
 * `show()`/`close()`/`toggle()`.
 *
 * ```html
 * <md-fab-menu label="Create">
 *   <md-fab-menu-item label="New doc"><svg ...></svg></md-fab-menu-item>
 *   <md-fab-menu-item label="New folder"><svg ...></svg></md-fab-menu-item>
 * </md-fab-menu>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-fab-menu")
export class MdFabMenu extends FabMenu {
  static override styles: CSSResultOrNative[] = [styles];
}
