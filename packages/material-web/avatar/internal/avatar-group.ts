/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing, TemplateResult } from "lit";
import { property, queryAssignedElements, state } from "lit/decorators.js";
import { styleMap } from "lit/directives/style-map.js";

/**
 * An avatar group lays out a set of `<md-avatar>` children with a configurable
 * overlap and optionally collapses any surplus past `max` into a trailing
 * "+N" surplus pill (using the `--md-sys-color-surface-variant` /
 * `on-surface-variant` tokens).
 *
 * ```html
 * <md-avatar-group max="3" overlap="8">
 *   <md-avatar src="a.jpg"></md-avatar>
 *   <md-avatar src="b.jpg"></md-avatar>
 *   <md-avatar src="c.jpg"></md-avatar>
 *   <md-avatar src="d.jpg"></md-avatar>
 * </md-avatar-group>
 * ```
 */
export class AvatarGroup extends LitElement {
  /**
   * Maximum number of avatars to display. Any beyond this count are hidden and
   * summarised by the surplus pill. A value `<= 0` shows all avatars.
   */
  @property({ type: Number }) max = 0;

  /** Overlap between adjacent avatars, in pixels. */
  @property({ type: Number }) overlap = 8;

  /** Total number of slotted avatars. */
  @state() private total = 0;

  @queryAssignedElements({ flatten: true })
  private readonly items!: HTMLElement[];

  protected override render(): TemplateResult {
    const overflow = this.max > 0 ? Math.max(0, this.total - this.max) : 0;
    return html`
      <div class="group" role="group" style=${styleMap({ "--_overlap": `${this.overlap}px` })}>
        <slot @slotchange=${this.handleSlotChange}></slot>
        ${overflow > 0 ? this.renderSurplus(overflow) : nothing}
      </div>
    `;
  }

  private renderSurplus(overflow: number): TemplateResult {
    return html` <span class="surplus" aria-label=${`${overflow} more`}> +${overflow} </span> `;
  }

  private handleSlotChange(): void {
    const items = this.items;
    this.total = items.length;
    // Hide avatars past `max` so the layout matches the surplus count.
    items.forEach((el, i) => {
      const hidden = this.max > 0 && i >= this.max;
      el.toggleAttribute("hidden", hidden);
    });
  }
}
