/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { DateRangePicker } from "./internal/date-range-picker.js";
import { styles } from "./internal/date-range-picker-styles.js";

export { type DateRangeChangeDetail } from "./internal/date-range-picker.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-date-range-picker": MdDateRangePicker;
  }
}

/**
 * @summary A docked Material 3 date *range* picker — a month calendar grid with
 * a highlighted start-to-end span.
 *
 * @description
 * Bind `start` and `end` (`YYYY-MM-DD` strings) and optionally `min`/`max` or a
 * `disabledDate` predicate. Set `editable` to show coupled start/end text
 * fields. Listen for `date-range-picker:change` (or the native `change`) to
 * react to selection.
 *
 * ```html
 * <md-date-range-picker
 *   start="2026-05-04"
 *   end="2026-05-12"
 *   editable
 * ></md-date-range-picker>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-date-range-picker")
export class MdDateRangePicker extends DateRangePicker {
  static override styles: CSSResultOrNative[] = [styles];
}
