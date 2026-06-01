/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { TreeItem } from "./internal/tree-item.js";
import { styles } from "./internal/tree-item-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-tree-item": MdTreeItem;
  }
}

/**
 * @summary A single node within an `<md-tree>`.
 *
 * @description
 * Renders an indented row with an expand/collapse chevron (when it has child
 * items), an optional leading icon (slot `icon`) and the `label`. Nested
 * `<md-tree-item>` children live in the default slot and appear only when the
 * node is `expanded`.
 *
 * ```html
 * <md-tree-item label="Documents" value="docs">
 *   <md-icon slot="icon">folder</md-icon>
 *   <md-tree-item label="Resume.pdf" value="resume"></md-tree-item>
 * </md-tree-item>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-tree-item")
export class MdTreeItem extends TreeItem {
  static override styles: CSSResultOrNative[] = [styles];
}
