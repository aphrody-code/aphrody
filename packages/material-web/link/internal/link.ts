/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing, PropertyValues } from "lit";
import { property, state } from "lit/decorators.js";

/**
 * How the link's underline behaves.
 *
 * - `none` — never underlined.
 * - `hover` — underlined on hover and keyboard focus only.
 * - `always` — always underlined (the default, per the M3/WCAG guidance that
 *   inline text links should be distinguishable without relying on colour).
 */
export type LinkUnderline = "none" | "hover" | "always";

/**
 * The Material 3 color role used to tint the link. Maps to the matching
 * `--md-sys-color-*` token.
 */
export type LinkColor =
  | "primary"
  | "secondary"
  | "tertiary"
  | "error"
  | "on-surface"
  | "on-surface-variant"
  | "inverse-primary";

/**
 * A Material 3 styled hyperlink. Renders a real `<a>` so it behaves like a
 * native link for assistive technology, middle-click, "open in new tab", and
 * the browser's status bar.
 *
 * Built entirely on the `--md-sys-*` tokens. Hover/focus use M3 state-layer
 * opacities, and keyboard focus draws a tokenised outline (no ripple).
 *
 * ```html
 * <md-link href="/pricing" underline="hover" color="primary">Pricing</md-link>
 * ```
 *
 * When `href` is omitted the element still renders an `<a>` but without an
 * `href` attribute, so it is presentational only (not focusable).
 */
export class Link extends LitElement {
  /**
   * Delegate focus to the inner `<a>` so the host is reachable by keyboard and
   * `:focus-within`/`delegatesFocus` semantics line up with the real anchor.
   */
  static override shadowRootOptions: ShadowRootInit = {
    ...LitElement.shadowRootOptions,
    delegatesFocus: true,
  };

  /** The URL the link points to. */
  @property() href = "";

  /** The anchor `target` (e.g. `_blank`). */
  @property() target = "";

  /**
   * The anchor `rel`. When `target="_blank"` and no `rel` is provided,
   * `noopener noreferrer` is applied automatically to avoid reverse-tabnabbing.
   */
  @property() rel = "";

  /** Controls underline behaviour. Defaults to `always`. */
  @property() underline: LinkUnderline = "always";

  /** The M3 color role used to tint the link. Defaults to `primary`. */
  @property({ reflect: true }) color: LinkColor = "primary";

  /**
   * Resolved `rel`, derived from `rel`/`target` before render. Defaulting to
   * `noopener noreferrer` for `target="_blank"` guards against reverse
   * tabnabbing.
   */
  @state() private resolvedRel = "";

  protected override willUpdate(changed: PropertyValues<this>) {
    if (changed.has("rel") || changed.has("target")) {
      this.resolvedRel = this.rel || (this.target === "_blank" ? "noopener noreferrer" : "");
    }
  }

  protected override render() {
    return html`
      <a
        class="link underline-${this.underline}"
        href=${this.href || nothing}
        target=${this.target || nothing}
        rel=${this.resolvedRel || nothing}
        part="link"
      >
        <slot></slot>
      </a>
    `;
  }
}
