/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Surface } from "./internal/surface.js";
import { styles } from "./internal/surface-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-surface": MdSurface;
  }
}

/**
 * @summary A surface container — the Material 3 equivalent of MUI's `Paper`.
 *
 * @description
 * Wraps content in a tonal M3 surface with an optional shadow, outline, and
 * rounded corners. `variant` chooses the treatment (`elevation`, `outlined`,
 * `filled`), `level` (0–5) drives the tonal container color and the
 * `<md-elevation>` shadow, and `shape` selects a corner token.
 *
 * ```html
 * <md-surface variant="elevation" level="2" shape="large">
 *   Card content
 * </md-surface>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-surface")
export class MdSurface extends Surface {
  static override styles: CSSResultOrNative[] = [styles];
}
