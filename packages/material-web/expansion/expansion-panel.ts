/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { ExpansionPanel } from "./internal/expansion-panel.js";
import { styles } from "./internal/expansion-panel-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-expansion-panel": MdExpansionPanel;
  }
}

/**
 * @summary Expansion panels show a header that toggles a region of content open
 * and closed.
 *
 * @description
 * Place the panel title in `slot="header"`, optional secondary text in
 * `slot="description"`, and the collapsible body in the default slot. Toggle the
 * `expanded` attribute or call `expand()`/`collapse()`/`toggle()`.
 *
 * ```html
 * <md-expansion-panel expanded>
 *   <span slot="header">Personal data</span>
 *   <span slot="description">Type your name and address</span>
 *   <p>Body content here.</p>
 * </md-expansion-panel>
 * ```
 *
 * @fires expansion:toggle {CustomEvent<{expanded: boolean}>} Fired after the
 *     panel expands or collapses.
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-expansion-panel")
export class MdExpansionPanel extends ExpansionPanel {
  static override styles: CSSResultOrNative[] = [styles];
}
