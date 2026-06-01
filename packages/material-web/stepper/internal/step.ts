/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement } from "lit";
import { property } from "lit/decorators.js";

/**
 * A single step within an `<md-stepper>`. Holds the step's metadata (label,
 * completion/optional/editable flags) and its content in the default slot. The
 * content is only rendered when the step is the active one — the parent
 * `<md-stepper>` drives the `active` flag.
 *
 * `<md-step>` is intentionally a thin container: the numbered indicator, label
 * and connector are drawn by the parent stepper's header so the layout can flow
 * horizontally or vertically without each step measuring itself.
 */
export class Step extends LitElement {
  /** The label shown beneath/next to the step indicator. */
  @property() label = "";

  /** Whether the step has been completed (renders a check mark). */
  @property({ type: Boolean, reflect: true }) completed = false;

  /** Whether the step is optional (renders an "Optional" caption). */
  @property({ type: Boolean, reflect: true }) optional = false;

  /**
   * Whether a completed step may be re-edited by clicking its indicator. When
   * false a completed step's indicator still shows a check but the parent will
   * not re-select it in linear mode.
   */
  @property({ type: Boolean }) editable = true;

  /**
   * Whether this step is the active one. Set by the parent `<md-stepper>`;
   * reflected so the host can be hidden via CSS when inactive.
   */
  @property({ type: Boolean, reflect: true }) active = false;

  protected override render() {
    return html`
      <div class="step-content" role="tabpanel" ?hidden=${!this.active}>
        <slot></slot>
      </div>
    `;
  }
}
