/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

// Registers the `<md-time-picker>` element used inside the time row.
import "../timepicker/time-picker.js";

import { DateTimePicker } from "./internal/date-time-picker.js";
import { styles } from "./internal/date-time-picker-styles.js";

export { type DateTimePickerChangeDetail } from "./internal/date-time-picker.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-date-time-picker": MdDateTimePicker;
  }
}

/**
 * @summary A docked Material 3 date-time picker — a month calendar grid coupled
 * to an editable time field.
 *
 * @description
 * Bind `value` (an ISO `YYYY-MM-DDTHH:MM` string) and optionally `min`/`max`
 * (date-only ISO), `locale`/`format` for the calendar, and `timeFormat`
 * (`'12h'` or `'24h'`) for the time row. Listen for `date-time-picker:change`
 * (or the native `change`) to react to selection.
 *
 * ```html
 * <md-date-time-picker
 *   value="2026-05-22T09:30"
 *   time-format="12h"
 * ></md-date-time-picker>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-date-time-picker")
export class MdDateTimePicker extends DateTimePicker {
  static override styles: CSSResultOrNative[] = styles;
}
