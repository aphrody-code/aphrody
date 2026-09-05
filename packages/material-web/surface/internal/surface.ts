/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing, PropertyValues } from "lit";
import { property, state } from "lit/decorators.js";
import { styleMap } from "lit/directives/style-map.js";

import "../../elevation/elevation.js";

/**
 * The visual treatment of a surface.
 *
 * - `elevation` — a tonal surface that casts a shadow (uses `<md-elevation>`).
 * - `outlined` — a flat surface with an outline-variant border, no shadow.
 * - `filled` — a flat, higher-emphasis tonal surface with no shadow.
 */
export type SurfaceVariant = "elevation" | "outlined" | "filled";

/**
 * A Material 3 surface container, the M3 equivalent of MUI's `Paper`.
 *
 * Wraps arbitrary content in a tonal surface with an optional shadow, outline,
 * and rounded corners. Built entirely on `--md-sys-*` tokens; the shadow is
 * delegated to the shared `<md-elevation>` element so it matches the M3
 * elevation curve exactly.
 *
 * The surface tonal color steps with `level` (0–5) across the
 * `--md-sys-color-surface-container*` ramp, mirroring M3's "higher elevation =
 * higher container tone" guidance. The `elevation` variant additionally casts
 * the matching shadow; `outlined` and `filled` stay flat.
 *
 * ```html
 * <md-surface variant="elevation" level="2">Card content</md-surface>
 * <md-surface variant="outlined" shape="medium">Bordered</md-surface>
 * ```
 */
export class Surface extends LitElement {
  /** The surface treatment. */
  @property({ reflect: true }) variant: SurfaceVariant = "elevation";

  /**
   * The elevation level, 0–5. Drives both the tonal container color and, for
   * the `elevation` variant, the `<md-elevation>` shadow.
   */
  @property({ type: Number, reflect: true }) level = 0;

  /**
   * The corner shape token to use, e.g. `none`, `extra-small`, `small`,
   * `medium`, `large`, `extra-large`, or `full`. Maps to
   * `--md-sys-shape-corner-<shape>`.
   */
  @property({ reflect: true }) shape = "medium";

  /**
   * `level` clamped to the valid 0–5 M3 range, derived before render so
   * `render()` stays pure and the elevation shadow never falls outside the
   * M3 curve.
   */
  @state() private clampedLevel = 0;

  protected override willUpdate(changed: PropertyValues<this>) {
    if (changed.has("level")) {
      this.clampedLevel = Math.max(0, Math.min(5, Math.trunc(this.level) || 0));
    }
  }

  protected override render() {
    const showShadow = this.variant === "elevation";
    return html`
      ${showShadow
        ? html`<md-elevation
            part="elevation"
            style=${styleMap({ "--md-elevation-level": `${this.clampedLevel}` })}
          ></md-elevation>`
        : nothing}
      <slot></slot>
    `;
  }
}
