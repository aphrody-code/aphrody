/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Snackbar } from "./internal/snackbar.js";
import { styles } from "./internal/snackbar-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-snackbar": MdSnackbar;
  }
}

/**
 * @summary Snackbars show short, low-priority feedback at the bottom of the
 * screen and dismiss themselves.
 *
 * @description
 * Use `show()`/`close()` to control the snackbar imperatively, or toggle the
 * `open` attribute. Slot supporting text as the default content and an action
 * (`<md-text-button>`, `<button>`, or `<a>`) into `slot="action"`; set
 * `closeable` for a trailing dismiss affordance.
 *
 * ```html
 * <md-snackbar timeout-ms="6000" closeable>
 *   Message archived
 *   <md-text-button slot="action">Undo</md-text-button>
 * </md-snackbar>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-snackbar")
export class MdSnackbar extends Snackbar {
  static override styles: CSSResultOrNative[] = [styles];
}
