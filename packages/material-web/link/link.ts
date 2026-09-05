/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Link } from "./internal/link.js";
import { styles } from "./internal/link-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-link": MdLink;
  }
}

/**
 * @summary Links navigate to another location and are styled with Material 3
 * color, typography, and state layers.
 *
 * @description
 * Renders a real `<a>` element. Set `href`, `target`, `rel`, `underline`
 * (`none` | `hover` | `always`) and `color` (an M3 color role). Slot the link
 * text as the default content.
 *
 * ```html
 * <md-link href="/pricing" underline="hover">Pricing</md-link>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-link")
export class MdLink extends Link {
  static override styles: CSSResultOrNative[] = [styles];
}
