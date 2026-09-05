/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { AvatarGroup } from "./internal/avatar-group.js";
import { styles } from "./internal/avatar-group-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-avatar-group": MdAvatarGroup;
  }
}

/**
 * @summary Avatar groups stack a set of avatars with overlap and a "+N" surplus
 * pill.
 *
 * @description
 * Slot `<md-avatar>` children. Set `max` to cap how many are shown (the rest
 * collapse into a trailing surplus pill) and `overlap` to control how far each
 * avatar slides over its neighbour.
 *
 * ```html
 * <md-avatar-group max="3" overlap="8">
 *   <md-avatar src="a.jpg"></md-avatar>
 *   <md-avatar src="b.jpg"></md-avatar>
 *   <md-avatar src="c.jpg"></md-avatar>
 *   <md-avatar src="d.jpg"></md-avatar>
 * </md-avatar-group>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-avatar-group")
export class MdAvatarGroup extends AvatarGroup {
  static override styles: CSSResultOrNative[] = [styles];
}
