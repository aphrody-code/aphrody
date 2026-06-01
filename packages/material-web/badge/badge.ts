/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */
import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Badge } from "./internal/badge.js";
import { styles } from "./internal/badge-styles.cssresult.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-badge": MdBadge;
  }
}

/**
 * @summary Badges draw attention to dynamic information, such as counts or status.
 *
 * @description Badges are decorative indicators that can show values or status indicators
 * on top of other components like icon buttons, list items, or navigation elements.
 *
 * @final
 */
@customElement("md-badge")
export class MdBadge extends Badge {
  static override styles: CSSResultOrNative[] = [styles];
}
export default MdBadge;
