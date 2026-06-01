/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing, TemplateResult } from "lit";
import { property } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";
import { styleMap } from "lit/directives/style-map.js";

/**
 * The visual treatment of the stepper's progress indicator.
 *
 * - `dots` — one dot per step; the active step's dot uses the primary color.
 * - `text` — a compact `"N / M"` label.
 * - `progress` — a linear progress bar filled to the current step.
 */
export type MobileStepperVariant = "dots" | "text" | "progress";

/**
 * Where the stepper anchors itself. `static` flows inline with the document;
 * `bottom`/`top` fix it to the corresponding edge of the viewport.
 */
export type MobileStepperPosition = "bottom" | "top" | "static";

/**
 * A mobile stepper displays progress through a sequence of steps and is
 * typically used at the bottom of mobile screens for paginated content such as
 * onboarding carousels.
 *
 * Implements the Material 3 design language on top of the `--md-sys-*` tokens.
 * The app supplies its own back/next controls through the `back` and `next`
 * slots so it owns navigation behavior; this element renders only the progress
 * affordance (`dots`, `text`, or `progress`) between them.
 *
 * ```html
 * <md-mobile-stepper steps="6" active-step="2" variant="dots">
 *   <md-text-button slot="back">Back</md-text-button>
 *   <md-text-button slot="next">Next</md-text-button>
 * </md-mobile-stepper>
 * ```
 */
export class MobileStepper extends LitElement {
  /** Total number of steps. */
  @property({ type: Number }) steps = 0;

  /** The zero-based index of the active step. */
  @property({ type: Number, attribute: "active-step" }) activeStep = 0;

  /** The progress indicator treatment. Reflected so CSS can target it. */
  @property({ reflect: true }) variant: MobileStepperVariant = "dots";

  /** Where the stepper anchors itself. Reflected so CSS can target it. */
  @property({ reflect: true }) position: MobileStepperPosition = "bottom";

  protected override render(): TemplateResult {
    return html`
      <div class="stepper" role="group" aria-label="Progress" aria-roledescription="stepper">
        <div class="control back"><slot name="back"></slot></div>
        <div class="progress">${this.renderProgress()}</div>
        <div class="control next"><slot name="next"></slot></div>
      </div>
    `;
  }

  private renderProgress(): TemplateResult {
    switch (this.variant) {
      case "text":
        return this.renderText();
      case "progress":
        return this.renderBar();
      default:
        return this.renderDots();
    }
  }

  private renderDots(): TemplateResult {
    const dots: TemplateResult[] = [];
    const count = Math.max(0, this.steps);
    for (let i = 0; i < count; i++) {
      const classes = { dot: true, active: i === this.activeStep };
      dots.push(html`<span class=${classMap(classes)}></span>`);
    }
    return html`
      <div class="dots" role="tablist" aria-label=${this.stepCountLabel()}>${dots}</div>
    `;
  }

  private renderText(): TemplateResult {
    return html`
      <div class="text" role="status" aria-label=${this.stepCountLabel()}>
        ${this.activeStep + 1} / ${this.steps}
      </div>
    `;
  }

  private renderBar(): TemplateResult {
    const total = Math.max(1, this.steps - 1);
    const ratio = total > 0 ? this.activeStep / total : 0;
    const fraction = Math.min(1, Math.max(0, ratio));
    const styles = { "inline-size": `${fraction * 100}%` };
    return html`
      <div
        class="bar"
        role="progressbar"
        aria-label=${this.stepCountLabel()}
        aria-valuemin=${0}
        aria-valuemax=${Math.max(0, this.steps - 1)}
        aria-valuenow=${this.activeStep}
      >
        <div class="track"></div>
        <div class="indicator" style=${styleMap(styles)}></div>
      </div>
      ${nothing}
    `;
  }

  private stepCountLabel(): string {
    return `Step ${this.activeStep + 1} of ${this.steps}`;
  }
}
