/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement, nothing } from "lit";
import { property, queryAssignedElements } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";

import { Step } from "./step.js";

/**
 * A stepper guides a user through a sequence of logical, numbered steps. It
 * renders a header (a row of numbered indicators connected by lines in
 * horizontal mode, or a vertical stack in vertical mode) plus the content of
 * the currently-selected `<md-step>`.
 *
 * Implements the Material 3 stepper pattern on top of the `--md-sys-*` tokens.
 * In `linear` mode the user may not jump ahead to an incomplete future step.
 *
 * @fires stepper:change {CustomEvent<{selectedIndex: number}>} Fired whenever
 *     the selected step changes (via `select`, `next`, `previous`, or a click
 *     on a step indicator).
 */
export class Stepper extends LitElement {
  /** Layout orientation. Reflected so CSS can target `[orientation]`. */
  @property({ reflect: true }) orientation: "horizontal" | "vertical" = "horizontal";

  /** Index of the currently selected step. */
  @property({ type: Number, attribute: "selected-index" }) selectedIndex = 0;

  /**
   * When true, the user cannot select a future step until all prior steps are
   * completed (the classic "wizard" behaviour).
   */
  @property({ type: Boolean, reflect: true }) linear = false;

  @queryAssignedElements({ flatten: true, selector: "md-step" })
  private readonly stepElements!: Step[];

  /** Returns the live list of `<md-step>` children. */
  get steps(): Step[] {
    return this.stepElements ?? [];
  }

  /** Selects the next step, if any (respecting `linear`). */
  next() {
    this.select(this.selectedIndex + 1);
  }

  /** Selects the previous step, if any. */
  previous() {
    this.select(this.selectedIndex - 1);
  }

  /**
   * Selects the step at `index`. Out-of-range indices are ignored. In linear
   * mode, selecting a future step is rejected unless every step before it is
   * completed.
   */
  select(index: number) {
    const steps = this.steps;
    if (index < 0 || index >= steps.length || index === this.selectedIndex) {
      return;
    }
    if (this.linear && index > this.selectedIndex && !this.canReach(index)) {
      return;
    }
    this.selectedIndex = index;
    this.syncActive();
    this.dispatchEvent(
      new CustomEvent("stepper:change", {
        detail: { selectedIndex: index },
        bubbles: true,
        composed: true,
      }),
    );
  }

  /**
   * In linear mode a step is reachable only if every preceding step is
   * completed. The target step's own completion is irrelevant.
   */
  private canReach(index: number): boolean {
    const steps = this.steps;
    for (let i = 0; i < index; i++) {
      if (!steps[i]?.completed) {
        return false;
      }
    }
    return true;
  }

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      // Defer until children are upgraded so `steps` is populated.
      void this.updateComplete.then(() => {
        this.syncActive();
      });
    }
  }

  private handleSlotChange() {
    this.syncActive();
    this.requestUpdate();
  }

  /** Toggles the `active` flag on each child step to match `selectedIndex`. */
  private syncActive() {
    const steps = this.steps;
    if (this.selectedIndex >= steps.length) {
      this.selectedIndex = Math.max(0, steps.length - 1);
    }
    for (let i = 0; i < steps.length; i++) {
      steps[i].active = i === this.selectedIndex;
    }
  }

  protected override render() {
    const steps = this.steps;
    return html`
      <div
        class="header ${classMap({ vertical: this.orientation === "vertical" })}"
        role="tablist"
        aria-orientation=${this.orientation}
      >
        ${steps.map((step, i) => this.renderStepHeader(step, i, steps.length))}
      </div>
      <div class="content">
        <slot @slotchange=${this.handleSlotChange}></slot>
      </div>
    `;
  }

  private renderStepHeader(step: Step, index: number, total: number) {
    const isActive = index === this.selectedIndex;
    const isCompleted = step.completed;
    const reachable = !this.linear || index <= this.selectedIndex || this.canReach(index);
    const state = isActive ? "active" : isCompleted ? "completed" : "inactive";
    return html`
      <div
        class="step-header state-${state}"
        role="tab"
        tabindex=${isActive ? "0" : "-1"}
        aria-selected=${isActive ? "true" : "false"}
        aria-disabled=${reachable ? "false" : "true"}
        @click=${() => {
          this.handleHeaderClick(index, step, isCompleted);
        }}
        @keydown=${(event: KeyboardEvent) => {
          this.handleHeaderKeydown(event, index);
        }}
      >
        <div class="indicator">
          ${isCompleted
            ? html`<svg viewBox="0 0 24 24" aria-hidden="true" class="check">
                <path d="M9.55 17.3 4.25 12l1.4-1.4 3.9 3.9 8.4-8.4 1.4 1.4Z"></path>
              </svg>`
            : html`<span class="number">${index + 1}</span>`}
        </div>
        <div class="labels">
          <span class="label">${step.label}</span>
          ${step.optional ? html`<span class="optional">Optional</span>` : nothing}
        </div>
      </div>
      ${index < total - 1 ? html`<div class="connector" aria-hidden="true"></div>` : nothing}
    `;
  }

  private handleHeaderClick(index: number, step: Step, isCompleted: boolean) {
    // A completed but non-editable step cannot be re-selected in linear mode.
    if (this.linear && isCompleted && !step.editable && index !== this.selectedIndex) {
      return;
    }
    this.select(index);
  }

  private handleHeaderKeydown(event: KeyboardEvent, index: number) {
    const steps = this.steps;
    const horizontal = this.orientation === "horizontal";
    let handled = true;
    switch (event.key) {
      case "Enter":
      case " ":
        this.select(index);
        break;
      case "ArrowRight":
        if (horizontal) {
          this.focusHeader(index + 1);
        } else {
          handled = false;
        }
        break;
      case "ArrowDown":
        if (!horizontal) {
          this.focusHeader(index + 1);
        } else {
          handled = false;
        }
        break;
      case "ArrowLeft":
        if (horizontal) {
          this.focusHeader(index - 1);
        } else {
          handled = false;
        }
        break;
      case "ArrowUp":
        if (!horizontal) {
          this.focusHeader(index - 1);
        } else {
          handled = false;
        }
        break;
      case "Home":
        this.focusHeader(0);
        break;
      case "End":
        this.focusHeader(steps.length - 1);
        break;
      default:
        handled = false;
        break;
    }
    if (handled) {
      event.preventDefault();
    }
  }

  private focusHeader(index: number) {
    const steps = this.steps;
    if (index < 0 || index >= steps.length) {
      return;
    }
    const headers = this.renderRoot.querySelectorAll<HTMLElement>(".step-header");
    headers[index]?.focus();
  }
}
