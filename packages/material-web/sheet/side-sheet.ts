/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { SideSheet } from "./internal/side-sheet.js";
import { styles } from "./internal/side-sheet-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-side-sheet": MdSideSheet;
  }
}

/**
 * @summary Side sheets show supplementary content anchored to the leading or
 * trailing edge of the screen.
 *
 * @description
 * Use `show()`/`close()` or toggle the `open` attribute. Set `position` to
 * `end` (default) or `start`, and `modal` to dim the rest of the UI with a
 * scrim that dismisses the sheet on click. Slot `headline`, default content,
 * and `actions`.
 *
 * ```html
 * <md-side-sheet modal position="end">
 *   <span slot="headline">Details</span>
 *   <p>Sheet content</p>
 *   <md-text-button slot="actions">Close</md-text-button>
 * </md-side-sheet>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-side-sheet")
export class MdSideSheet extends SideSheet {
  static override styles: CSSResultOrNative[] = [styles];
}
