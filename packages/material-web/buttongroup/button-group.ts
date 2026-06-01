/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { ButtonGroup } from "./internal/button-group.js";
import { styles } from "./internal/button-group-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-button-group": MdButtonGroup;
  }
}

/**
 * @summary A connected button group — the Material 3 successor to the segmented
 * button — that orchestrates the shape and selection of its child buttons.
 *
 * @description
 * Slot ordinary buttons (or toggles) as the default content; give each a
 * `value` attribute. The group applies the connected-shape treatment and
 * tracks selection. Use `multiselect` to allow independent toggles.
 *
 * ```html
 * <md-button-group value="bold" multiselect>
 *   <button value="bold">Bold</button>
 *   <button value="italic">Italic</button>
 *   <button value="underline">Underline</button>
 * </md-button-group>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-button-group")
export class MdButtonGroup extends ButtonGroup {
  static override styles: CSSResultOrNative[] = [styles];
}
