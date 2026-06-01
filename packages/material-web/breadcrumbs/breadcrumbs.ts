/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Breadcrumbs } from "./internal/breadcrumbs.js";
import { styles } from "./internal/breadcrumbs-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-breadcrumbs": MdBreadcrumbs;
  }
}

/**
 * @summary Breadcrumbs show the navigation path to the current page and let
 * users move back up the hierarchy.
 *
 * @description
 * Slot trail items (e.g. `<md-link>`, `<a>`) into the default slot, from root
 * to current page. Customize the divider with the `separator` property or the
 * `separator` slot. Set `max-items` to collapse long trails behind a clickable
 * ellipsis. Renders `nav[aria-label] > ol` for accessibility.
 *
 * ```html
 * <md-breadcrumbs label="Breadcrumb" max-items="4">
 *   <md-link href="/">Home</md-link>
 *   <md-link href="/library">Library</md-link>
 *   <span>Current</span>
 * </md-breadcrumbs>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-breadcrumbs")
export class MdBreadcrumbs extends Breadcrumbs {
  static override styles: CSSResultOrNative[] = [styles];
}
