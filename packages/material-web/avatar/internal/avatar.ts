/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing, PropertyValues, TemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { ClassInfo, classMap } from "lit/directives/class-map.js";
import { styleMap } from "lit/directives/style-map.js";

/**
 * The shape of an avatar.
 *
 * - `circular` — a fully rounded circle (the M3 default).
 * - `rounded` — a square with rounded corners.
 * - `square` — a square with no corner rounding.
 */
export type AvatarVariant = "circular" | "rounded" | "square";

/**
 * An avatar represents a user or entity with an image, initials, or icon.
 *
 * It renders, in priority order: an image (`src`), then the default slot /
 * initials text, then a fallback. If the image fails to load it transparently
 * falls back to the slotted content (initials or an icon). The fallback surface
 * uses the `--md-sys-color-primary-container` / `on-primary-container` tokens.
 *
 * ```html
 * <md-avatar src="user.jpg" alt="Ada Lovelace"></md-avatar>
 * <md-avatar>AL</md-avatar>
 * <md-avatar><svg slot="icon" ...></svg></md-avatar>
 * ```
 *
 * @fires error {Event} Re-dispatched when the underlying image fails to load.
 */
export class Avatar extends LitElement {
  /** Image source URL. When set and loadable, the image is shown. */
  @property() src = "";

  /**
   * Alternative text for the image, also used as the accessible label for the
   * avatar as a whole.
   */
  @property() alt = "";

  /** The avatar shape. */
  @property({ reflect: true }) variant: AvatarVariant = "circular";

  /**
   * The avatar size. Accepts any CSS length (e.g. `40px`, `3rem`) or a number,
   * interpreted as pixels. Drives both width and height.
   */
  @property() size: string | number = "40px";

  /** Tracks whether the current `src` failed to load. */
  @state() private hasError = false;

  /** The size coerced to a CSS length, derived in `willUpdate`. */
  @state() private cssSize = "40px";

  override willUpdate(changed: PropertyValues<this>): void {
    // Reset the error state whenever a new source is provided so a previously
    // failed avatar can recover.
    if (changed.has("src")) {
      this.hasError = false;
    }
    // Coerce the size to a CSS length once per change, keeping render() pure.
    if (changed.has("size")) {
      this.cssSize =
        typeof this.size === "number" || /^\d+(\.\d+)?$/.test(String(this.size))
          ? `${this.size}px`
          : String(this.size);
    }
  }

  protected override render(): TemplateResult {
    const showImage = !!this.src && !this.hasError;
    const classes: ClassInfo = {
      avatar: true,
      "has-image": showImage,
    };
    return html`
      <div
        class=${classMap(classes)}
        style=${styleMap({ "--_size": this.cssSize })}
        role="img"
        aria-label=${this.alt || nothing}
      >
        ${showImage ? this.renderImage() : this.renderFallback()}
      </div>
    `;
  }

  private renderImage(): TemplateResult {
    return html`
      <img class="image" src=${this.src} alt=${this.alt || nothing} @error=${this.handleError} />
    `;
  }

  private renderFallback(): TemplateResult {
    return html`
      <span class="fallback" aria-hidden=${this.alt ? "true" : nothing}>
        <slot name="icon"></slot>
        <slot></slot>
      </span>
    `;
  }

  private handleError(): void {
    this.hasError = true;
    this.dispatchEvent(new Event("error"));
  }
}
