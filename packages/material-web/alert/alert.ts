/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Alert } from "./internal/alert.js";
import { styles } from "./internal/alert-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-alert": MdAlert;
  }
}

/**
 * @summary Alerts display a short, important message that attracts the user's
 * attention without interrupting their task.
 *
 * @description
 * The default slot is the message. Slot a heading into `slot="title"`, an
 * action (e.g. `<md-text-button>`) into `slot="action"`, and override the
 * leading severity icon via `slot="icon"`. Set `severity`
 * (`success`|`info`|`warning`|`error`), `variant`
 * (`standard`|`filled`|`outlined`), and `closeable` for a trailing dismiss
 * affordance that fires a `close` event.
 *
 * ```html
 * <md-alert severity="success" variant="standard" closeable>
 *   <span slot="title">Saved</span>
 *   Your changes were saved.
 *   <md-text-button slot="action">Undo</md-text-button>
 * </md-alert>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-alert")
export class MdAlert extends Alert {
  static override styles: CSSResultOrNative[] = [styles];
}
