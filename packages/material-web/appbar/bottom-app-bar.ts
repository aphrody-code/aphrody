/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { BottomAppBar } from "./internal/bottom-app-bar.js";
import { styles } from "./internal/bottom-app-bar-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-bottom-app-bar": MdBottomAppBar;
  }
}

/**
 * @summary A bottom app bar for navigation and key actions on compact windows.
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-bottom-app-bar")
export class MdBottomAppBar extends BottomAppBar {
  static override styles: CSSResultOrNative[] = [styles];
}
