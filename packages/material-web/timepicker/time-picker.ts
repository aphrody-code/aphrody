/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { TimePicker } from "./internal/time-picker.js";
import { styles } from "./internal/time-picker-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-time-picker": MdTimePicker;
  }
}

/**
 * @summary A Material 3 time picker (input variant) — numeric hour/minute
 * fields with an optional AM/PM toggle.
 *
 * @description
 * Bind `value` (a 24-hour `HH:MM` string) and set `format` to `'12h'` or
 * `'24h'`. Listen for `time-picker:change`; the emitted value is always
 * normalized to 24-hour `HH:MM`.
 *
 * ```html
 * <md-time-picker value="09:30" format="12h"></md-time-picker>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-time-picker")
export class MdTimePicker extends TimePicker {
  static override styles: CSSResultOrNative[] = [styles];
}
