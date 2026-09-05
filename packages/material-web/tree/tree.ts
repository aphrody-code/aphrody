/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Tree } from "./internal/tree.js";
import { styles } from "./internal/tree-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-tree": MdTree;
  }
}

/**
 * @summary A hierarchical list of expandable/collapsible `<md-tree-item>` nodes.
 *
 * @description
 * Nest `<md-tree-item>` elements (children go in each item's default slot). The
 * tree manages roving focus and keyboard navigation: arrows move between and
 * expand/collapse nodes, Home/End jump to the ends, and Enter/Space select.
 * Selection fires `tree:select`.
 *
 * ```html
 * <md-tree>
 *   <md-tree-item label="Fruit" value="fruit">
 *     <md-tree-item label="Apple" value="apple"></md-tree-item>
 *     <md-tree-item label="Banana" value="banana"></md-tree-item>
 *   </md-tree-item>
 * </md-tree>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-tree")
export class MdTree extends Tree {
  static override styles: CSSResultOrNative[] = [styles];
}
