/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Popover } from "./internal/popover.js";
import { styles } from "./internal/popover-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-popover": MdPopover;
  }
}

/**
 * @summary A floating, anchored surface — the M3 equivalent of MUI's
 * `Popover`/`Popper`.
 *
 * @description
 * Uses the native Popover API for top-layer rendering and light dismiss, and
 * CSS Anchor Positioning when available (with a `getBoundingClientRect` JS
 * fallback otherwise). Set `anchor` to an element or its `id`, choose a
 * `placement`, and toggle `open` (or call `show()`/`close()`/`toggle()`).
 *
 * ```html
 * <button id="trigger">Open</button>
 * <md-popover anchor="trigger" placement="bottom-start">
 *   <p>Floating content</p>
 * </md-popover>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-popover")
export class MdPopover extends Popover {
  static override styles: CSSResultOrNative[] = [styles];
}
