/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Accordion } from "./internal/accordion.js";
import { styles } from "./internal/accordion-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-accordion": MdAccordion;
  }
}

/**
 * @summary An accordion groups expansion panels, optionally allowing only one
 * to be open at a time.
 *
 * @description
 * Place `md-expansion-panel` children in the default slot. By default the
 * accordion is single-expand: opening one panel collapses the others. Set
 * `multi` to allow multiple open panels.
 *
 * ```html
 * <md-accordion>
 *   <md-expansion-panel>
 *     <span slot="header">First</span>
 *     <p>One</p>
 *   </md-expansion-panel>
 *   <md-expansion-panel>
 *     <span slot="header">Second</span>
 *     <p>Two</p>
 *   </md-expansion-panel>
 * </md-accordion>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-accordion")
export class MdAccordion extends Accordion {
  static override styles: CSSResultOrNative[] = [styles];
}
