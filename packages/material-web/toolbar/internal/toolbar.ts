/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * The visual variant of a toolbar.
 *
 * - `docked` — full-width bar pinned to an edge of the layout.
 * - `floating` — a rounded, elevated pill that floats above content.
 */
export type ToolbarVariant = "docked" | "floating";

/**
 * A toolbar groups a set of actions (typically icon buttons) into a single
 * horizontal surface. It implements the Material 3 docked and floating toolbar
 * specs on top of the `--md-sys-*` design tokens, painting its own elevation
 * so it carries no dependency on `md-elevation`.
 *
 * Slot the actions into the default slot:
 *
 * ```html
 * <md-toolbar variant="floating">
 *   <md-icon-button><md-icon>format_bold</md-icon></md-icon-button>
 *   <md-icon-button><md-icon>format_italic</md-icon></md-icon-button>
 * </md-toolbar>
 * ```
 */
export class Toolbar extends LitElement {
  /**
   * The toolbar variant. Reflected so CSS can target `[variant="floating"]`.
   */
  @property({ reflect: true }) variant: ToolbarVariant = "docked";

  protected override render() {
    return html`
      <div class="toolbar" role="toolbar">
        <slot></slot>
      </div>
    `;
  }
}
