/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Autocomplete } from "./internal/autocomplete.js";
import { styles } from "./internal/autocomplete-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-autocomplete": MdAutocomplete;
  }
}

/**
 * @summary An outlined text field that suggests options from a filtered,
 * floating list as the user types.
 *
 * @description
 * Provide the candidate options via the `.options` property (an array of
 * `{value, label}`). The list filters in real time (case-insensitive) using the
 * `filter` strategy (`contains` or `startsWith`). Navigate with the arrow keys,
 * select with Enter or a click, and dismiss with Escape. Selection fires
 * `autocomplete:select`; typing re-dispatches `input`.
 *
 * ```html
 * <md-autocomplete
 *   label="Fruit"
 *   .options=${[{value: 'a', label: 'Apple'}, {value: 'b', label: 'Banana'}]}
 * ></md-autocomplete>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-autocomplete")
export class MdAutocomplete extends Autocomplete {
  static override styles: CSSResultOrNative[] = [styles];
}
