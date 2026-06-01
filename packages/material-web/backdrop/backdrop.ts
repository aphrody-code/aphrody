/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Backdrop } from "./internal/backdrop.js";
import { styles } from "./internal/backdrop-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-backdrop": MdBackdrop;
  }
}

/**
 * @summary A full-screen scrim — the Material 3 equivalent of MUI's `Backdrop`.
 *
 * @description
 * A fixed, full-viewport overlay tinted with `--md-sys-color-scrim`. Toggle
 * `open` to fade it in/out, listen for `click` to dismiss the surface it backs,
 * use `invisible` for a transparent click-catcher, and `blur` to frost the
 * content behind it. The scrim is `aria-hidden`.
 *
 * ```html
 * <md-backdrop open @click=${onDismiss}></md-backdrop>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-backdrop")
export class MdBackdrop extends Backdrop {
  static override styles: CSSResultOrNative[] = [styles];
}
