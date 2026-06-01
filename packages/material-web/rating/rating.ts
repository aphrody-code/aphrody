/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Rating } from "./internal/rating.js";
import { styles } from "./internal/rating-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-rating": MdRating;
  }
}

/**
 * @summary Ratings let users view and set a value on an ordinal star scale.
 *
 * @description
 * The Material 3 equivalent of MUI `Rating`. Renders a row of stars; click,
 * hover, or use the arrow keys to set the value. Supports half-star precision,
 * a `readonly` display mode, `disabled`, and form participation via `name`.
 * Slot a custom icon (e.g. an `<svg>`) into the default slot to replace the
 * star. The active portion uses `--md-sys-color-primary`, the inactive portion
 * `--md-sys-color-outline`.
 *
 * ```html
 * <md-rating name="score" value="3" max="5" precision="0.5"></md-rating>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-rating")
export class MdRating extends Rating {
  static override styles: CSSResultOrNative[] = [styles];
}
