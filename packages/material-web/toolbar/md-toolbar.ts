/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Toolbar } from "./internal/toolbar.js";
import { styles } from "./internal/toolbar-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-toolbar": MdToolbar;
  }
}

/**
 * @summary Toolbars group a set of actions into a docked or floating surface.
 *
 * @description
 * Set `variant="docked"` (default) for a full-width 64px bar, or
 * `variant="floating"` for a rounded, elevated pill. Slot icon buttons or other
 * action affordances into the default slot.
 *
 * ```html
 * <md-toolbar variant="floating">
 *   <md-icon-button><md-icon>undo</md-icon></md-icon-button>
 *   <md-icon-button><md-icon>redo</md-icon></md-icon-button>
 * </md-toolbar>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-toolbar")
export class MdToolbar extends Toolbar {
  static override styles: CSSResultOrNative[] = [styles];
}
