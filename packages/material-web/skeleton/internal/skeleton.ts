/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, PropertyValues, TemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { StyleInfo, styleMap } from "lit/directives/style-map.js";

/**
 * The shape of a skeleton placeholder.
 *
 * - `text` — a single line of text, with rounded ends and a slight vertical
 *   inset so it sits on the text baseline rhythm.
 * - `circular` — a circle (use equal `width`/`height` for a true circle).
 * - `rectangular` — a sharp-cornered block.
 * - `rounded` — a block with the M3 medium corner radius.
 */
export type SkeletonVariant = "text" | "circular" | "rectangular" | "rounded";

/**
 * The loading animation. `false` renders a static placeholder.
 *
 * - `pulse` — opacity pulses (the MUI default).
 * - `wave` — a highlight sweeps across the surface.
 */
export type SkeletonAnimation = "pulse" | "wave" | false;

/**
 * A skeleton displays a placeholder preview of content before the data loads,
 * reducing perceived latency. Equivalent to MUI's `Skeleton`.
 *
 * Set `variant` for the shape, `width`/`height` for the dimensions (numbers are
 * treated as pixels, strings are used verbatim), and `animation` for the
 * loading effect. The element is `aria-hidden` so assistive tech ignores the
 * placeholder. Colors are sourced from the `--md-sys-color-*` surface roles
 * with baseline fallbacks, so it is self-contained.
 */
export class Skeleton extends LitElement {
  /** The placeholder shape. Reflected so CSS can target `[variant="…"]`. */
  @property({ reflect: true }) variant: SkeletonVariant = "text";

  /**
   * The width. A number is interpreted as pixels; a string (e.g. `60%`,
   * `12rem`) is used as-is. Defaults to filling the inline axis.
   */
  @property() width?: string | number;

  /**
   * The height. A number is interpreted as pixels; a string is used as-is.
   * For `text` it defaults to the current line height.
   */
  @property() height?: string | number;

  /**
   * The loading animation. Reflected so CSS can target `[animation="…"]`. Use
   * the empty string / `false` to disable.
   */
  @property({ reflect: true }) animation: SkeletonAnimation = "pulse";

  /** Derived inline dimensions, computed in `willUpdate` to keep render pure. */
  @state() private dims: StyleInfo = {};

  override willUpdate(changed: PropertyValues<this>): void {
    if (changed.has("width") || changed.has("height")) {
      const dims: StyleInfo = {};
      if (this.width != null && this.width !== "") {
        dims["width"] = toCssSize(this.width);
      }
      if (this.height != null && this.height !== "") {
        dims["height"] = toCssSize(this.height);
      }
      this.dims = dims;
    }
  }

  protected override render(): TemplateResult {
    return html` <div class="skeleton" aria-hidden="true" style=${styleMap(this.dims)}></div> `;
  }
}

/** Coerce a `width`/`height` value to a CSS size (bare numbers → pixels). */
function toCssSize(value: string | number): string {
  if (typeof value === "number") {
    return `${value}px`;
  }
  // A purely numeric string is also treated as pixels.
  return /^\d+(\.\d+)?$/.test(value.trim()) ? `${value}px` : value;
}
