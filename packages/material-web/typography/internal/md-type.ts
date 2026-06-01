/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";
import { styleMap } from "lit/directives/style-map.js";

import { AxisSettings, fontVariationSettings } from "./google-sans-flex-axes.js";
import { TYPE_SCALE, TypeScaleRole } from "./type-scale.js";

/**
 * `<md-type>` renders slotted text in a Material 3 type-scale role using Google
 * Sans Flex, and exposes every variable axis as an attribute for maximum
 * flexibility. Set `scale` to pick a role; override any single axis
 * (`wght`/`opsz`/`wdth`/`grad`/`slnt`/`rond`) without disturbing the others.
 * With `animated`, axis changes tween smoothly (font-variation-settings is an
 * animatable CSS property), enabling expressive typographic motion.
 *
 * The host is `display: contents` by default so it does not disturb layout;
 * set `block` to render as a block-level element.
 */
export class MdType extends LitElement {
  /** The M3 type-scale role to apply. */
  @property() scale: TypeScaleRole = "body-large";

  /** Weight axis override (`wght`, 100–900). */
  @property({ type: Number }) wght?: number;
  /** Optical-size axis override (`opsz`, 9–144). */
  @property({ type: Number }) opsz?: number;
  /** Width axis override (`wdth`, 75–125). */
  @property({ type: Number }) wdth?: number;
  /** Grade axis override (`GRAD`, -200–150). */
  @property({ type: Number }) grad?: number;
  /** Slant axis override (`slnt`, -10–0). */
  @property({ type: Number }) slnt?: number;
  /** Roundness axis override (`ROND`, 0–100). */
  @property({ type: Number }) rond?: number;

  /**
   * Render in Google Sans Code (monospace face) instead of Google Sans Flex —
   * for code, terminals, and tabular data. Uses the `MONO`/`wght` axes.
   */
  @property({ type: Boolean, reflect: true }) code = false;

  /** Google Sans Code `MONO` axis (0 = proportional, 1 = monospace). */
  @property({ type: Number }) mono?: number;

  /** Tween axis changes (font-variation-settings transition). */
  @property({ type: Boolean, reflect: true }) animated = false;

  /** Render as a block element instead of `display: contents`. */
  @property({ type: Boolean, reflect: true }) block = false;

  /** Resolve the effective axis settings: scale defaults + attribute overrides. */
  private resolveAxes(): AxisSettings {
    const base = TYPE_SCALE[this.scale].axes;
    return {
      wght: this.wght ?? base.wght,
      opsz: this.opsz ?? base.opsz,
      wdth: this.wdth ?? base.wdth,
      grad: this.grad ?? base.grad,
      slnt: this.slnt ?? base.slnt,
      rond: this.rond ?? base.rond,
    };
  }

  protected override render() {
    const s = TYPE_SCALE[this.scale];
    const axes = this.resolveAxes();
    const weight = axes.wght ?? 400;
    const styles: Record<string, string> = this.code
      ? {
          "font-family":
            "var(--md-sys-typescale-code-font, 'Google Sans Code', ui-monospace, 'Cascadia Code', monospace)",
          "font-size": `${s.sizePx}px`,
          "line-height": `${s.lineHeightPx}px`,
          "letter-spacing": `${s.trackingPx}px`,
          "font-weight": `${weight}`,
          // Google Sans Code exposes MONO (0..1) + wght only.
          "font-variation-settings": `"MONO" ${this.mono ?? 1}, "wght" ${weight}`,
        }
      : {
          "font-family":
            "var(--md-sys-typescale-font, 'Google Sans Flex', Roboto, system-ui, sans-serif)",
          "font-size": `${s.sizePx}px`,
          "line-height": `${s.lineHeightPx}px`,
          "letter-spacing": `${s.trackingPx}px`,
          "font-weight": `${weight}`,
          "font-variation-settings": fontVariationSettings(axes),
        };
    return html`<span class="text" style=${styleMap(styles)}><slot></slot></span>`;
  }
}
