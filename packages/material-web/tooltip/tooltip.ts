/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Tooltip } from "./internal/tooltip.js";
import { styles } from "./internal/tooltip-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-tooltip": MdTooltip;
  }
}

/**
 * @summary Tooltips display informative text when users hover over, focus on,
 * or touch an anchor element.
 *
 * @description
 * Set `variant="plain"` (default) for a single line of supporting text, or
 * `variant="rich"` for a container with a `headline` slot, default supporting
 * text and an `action` slot. Bind the tooltip to its trigger with the `for`
 * attribute (an element id) or place the trigger in `slot="trigger"`.
 *
 * ```html
 * <button id="save">Save</button>
 * <md-tooltip for="save" text="Save your changes"></md-tooltip>
 *
 * <md-tooltip variant="rich">
 *   <button slot="trigger">Help</button>
 *   <span slot="headline">Need a hand?</span>
 *   Detailed supporting text goes here.
 *   <md-text-button slot="action">Learn more</md-text-button>
 * </md-tooltip>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-tooltip")
export class MdTooltip extends Tooltip {
  static override styles: CSSResultOrNative[] = [styles];
}
