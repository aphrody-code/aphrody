/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Scaffold } from "./internal/scaffold.js";
import { styles } from "./internal/scaffold-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-scaffold": MdScaffold;
  }
}

/**
 * @summary An adaptive page scaffold that switches navigation and margins
 * across the Material 3 window size classes.
 *
 * @description
 * `md-scaffold` measures its own width with a `ResizeObserver`, classifies it
 * into an M3 breakpoint (`compact` < 600, `medium` 600–839, `expanded`
 * 840–1199, `large` 1200–1599, `extra-large` >= 1600), and reflects the result
 * on the `size-class` attribute. Slot a top app bar, a navigation surface, the
 * body, an optional bottom bar, and a FAB.
 *
 * ```html
 * <md-scaffold>
 *   <md-top-app-bar slot="top-bar">Inbox</md-top-app-bar>
 *   <md-navigation-bar slot="navigation">…</md-navigation-bar>
 *   <md-list-detail>…</md-list-detail>
 *   <md-fab slot="fab" aria-label="Compose"></md-fab>
 * </md-scaffold>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-scaffold")
export class MdScaffold extends Scaffold {
  static override styles: CSSResultOrNative[] = [styles];
}
