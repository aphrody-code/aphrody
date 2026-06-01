/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement, nothing } from "lit";
import { property, query, state } from "lit/decorators.js";

import {
  animationOptions,
  prefersReducedMotion,
} from "../../internal/motion/easing-and-duration.js";

const supportsCSSAnimation =
  !isServer && window.CSS && CSS.supports("transition-behavior", "allow-discrete");

async function waitForTransitionEnd(element: HTMLElement) {
  return new Promise<void>((resolve) => {
    let done = false;
    const handler = (event: TransitionEvent) => {
      if (event.target === element) {
        element.removeEventListener("transitionend", handler);
        done = true;
        resolve();
      }
    };
    element.addEventListener("transitionend", handler);
    setTimeout(() => {
      if (!done) {
        element.removeEventListener("transitionend", handler);
        resolve();
      }
    }, 1000);
  });
}

/**
 * The edge a side sheet docks to. RTL-aware via logical `inset-inline`.
 *
 * - `end` — the trailing edge (right in LTR).
 * - `start` — the leading edge (left in LTR).
 */
export type SideSheetPosition = "end" | "start";

/**
 * A side sheet anchors supplementary content to the leading or trailing edge of
 * the screen. It slides in horizontally and, when `modal`, dims the rest of the
 * UI with a scrim that closes the sheet when clicked.
 *
 * Implements the Material 3 side-sheet spec on top of the `--md-sys-*` design
 * tokens, painting its own elevation so it carries no dependency on
 * `md-elevation`.
 *
 * @fires side-sheet:opening {Event} Fired when the sheet begins to open.
 * @fires side-sheet:opened {Event} Fired once the open animation finishes.
 * @fires side-sheet:closing {Event} Fired when the sheet begins to close.
 * @fires side-sheet:closed {Event} Fired once the close animation finishes.
 */
export class SideSheet extends LitElement {
  /** Whether the sheet is open. Reflected so CSS can target `[open]`. */
  @property({ type: Boolean, reflect: true }) open = false;

  /**
   * When true, renders a scrim that dims the rest of the UI and closes the
   * sheet when clicked, and lets Escape dismiss the sheet.
   */
  @property({ type: Boolean }) modal = false;

  /**
   * The edge the sheet docks to. Reflected so CSS can target `[position]`.
   */
  @property({ reflect: true }) position: SideSheetPosition = "end";

  @state() private animating = false;

  @query(".sheet") private readonly surface!: HTMLElement | null;

  /**
   * Opens the sheet. Resolves when the open animation has finished.
   */
  async show() {
    if (this.open) {
      return;
    }
    this.open = true;
    this.dispatchEvent(new Event("side-sheet:opening"));
    await this.animateOpen(true);
    this.dispatchEvent(new Event("side-sheet:opened"));
  }

  /**
   * Closes the sheet. Resolves once the close animation has finished.
   */
  async close() {
    if (!this.open) {
      return;
    }
    this.dispatchEvent(new Event("side-sheet:closing"));
    if (supportsCSSAnimation) {
      this.open = false;
      await this.animateOpen(false);
    } else {
      await this.animateOpen(false);
      this.open = false;
    }
    this.dispatchEvent(new Event("side-sheet:closed"));
  }

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.addEventListener("keydown", this.handleKeydown);
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    if (!isServer) {
      this.removeEventListener("keydown", this.handleKeydown);
    }
  }

  protected override render() {
    return html`
      ${this.modal ? this.renderScrim() : nothing}
      <div
        class="sheet ${this.animating ? "animating" : ""}"
        role="dialog"
        aria-modal=${this.modal ? "true" : "false"}
      >
        <div class="headline"><slot name="headline"></slot></div>
        <div class="content"><slot></slot></div>
        <div class="actions"><slot name="actions"></slot></div>
      </div>
    `;
  }

  private renderScrim() {
    return html`<div class="scrim" @click=${this.handleScrimClick}></div>`;
  }

  private handleScrimClick() {
    void this.close();
  }

  private readonly handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape" && this.modal && this.open) {
      event.stopPropagation();
      void this.close();
    }
  };

  private async animateOpen(opening: boolean) {
    const surface = this.surface;
    if (!surface || isServer || prefersReducedMotion()) {
      return;
    }
    this.animating = true;
    if (supportsCSSAnimation) {
      await waitForTransitionEnd(surface);
    } else if (surface.animate) {
      // Slide from the docked edge: trailing edge slides from the right (+100%),
      // leading edge from the left (-100%). Logical direction handled via the RTL
      // mirroring of `position`.
      const offscreen = this.position === "start" ? "-100%" : "100%";
      const from: Keyframe = { transform: `translateX(${offscreen})`, opacity: "0" };
      const to: Keyframe = { transform: "translateX(0)", opacity: "1" };
      const frames = opening ? [from, to] : [to, from];
      const anim = surface.animate(
        frames,
        animationOptions(
          opening ? "MEDIUM2" : "SHORT4",
          opening ? "EMPHASIZED_DECELERATE" : "EMPHASIZED_ACCELERATE",
        ),
      );
      try {
        await anim.finished;
      } catch {
        // Animation cancelled; ignore.
      }
    }
    this.animating = false;
  }
}
