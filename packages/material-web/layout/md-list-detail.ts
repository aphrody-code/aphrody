/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { ListDetail } from "./internal/list-detail.js";
import { styles } from "./internal/list-detail-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-list-detail": MdListDetail;
  }
}

/**
 * @summary The Material 3 list-detail canonical layout — two panes side by side
 * when there's room, one at a time when there isn't.
 *
 * @description
 * From the Expanded breakpoint (>= 840dp) the list (fixed ~360px, leading) and
 * detail (flexible, trailing) show together, separated by the 24dp pane spacer.
 * Below that, only one pane shows; call `showDetail()` / `showList()` (or set
 * the `showing` attribute) to navigate. RTL-aware: the list always leads.
 *
 * ```html
 * <md-list-detail showing="list">
 *   <nav slot="list">…</nav>
 *   <article slot="detail">…</article>
 * </md-list-detail>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-list-detail")
export class MdListDetail extends ListDetail {
  static override styles: CSSResultOrNative[] = [styles];
}
