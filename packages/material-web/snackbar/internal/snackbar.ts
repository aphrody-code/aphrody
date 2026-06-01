/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement, nothing } from "lit";
import { property, query, queryAssignedElements } from "lit/decorators.js";
import { ClassInfo, classMap } from "lit/directives/class-map.js";

/**
 * The reason a snackbar closed, surfaced on the `closed`/`closing` events'
 * `detail.reason`.
 *
 * - `timeout` — the auto-dismiss timer elapsed.
 * - `action` — the user activated the action button.
 * - `dismiss` — the user activated the close affordance (or pressed Escape).
 * - `programmatic` — `close()` was called without a reason.
 */
export type SnackbarCloseReason = "timeout" | "action" | "dismiss" | "programmatic";

/**
 * A snackbar shows short, low-priority feedback about a process the app has
 * performed or will perform. It appears at the bottom of the screen, never
 * interrupts the user, and disappears on its own.
 *
 * Implements the Material 3 snackbar spec on top of the `--md-sys-*` tokens.
 * The surface uses the Popover API (`popover="manual"`) so it lives in the
 * browser top layer — it can never be occluded by dialogs or other overlays,
 * and multiple toasts coexist (the M3-recommended toast pattern). Entry/exit is
 * animated declaratively via `@starting-style` + `transition-behavior:
 * allow-discrete`, with a graceful fallback when the Popover API is absent.
 *
 * @fires opening {Event} Fired when the snackbar begins to open.
 * @fires opened {Event} Fired once the open animation finishes.
 * @fires closing {CustomEvent<{reason: SnackbarCloseReason}>} Fired when the
 *     snackbar begins to close. Cancelable: prevent default to keep it open.
 * @fires closed {CustomEvent<{reason: SnackbarCloseReason}>} Fired once the
 *     close animation finishes.
 */
export class Snackbar extends LitElement {
  /** Whether the snackbar is open. Reflected so CSS can target `[open]`. */
  @property({ type: Boolean, reflect: true }) open = false;

  /**
   * Auto-dismiss timeout in milliseconds. The M3 guidance is 4–10 s; the
   * default is 5 s. Set to a value `<= 0` to disable auto-dismiss (the
   * snackbar then stays until `close()` or the close affordance is used).
   */
  @property({ type: Number, attribute: "timeout-ms" }) timeoutMs = 5000;

  /** Supporting text shown in the snackbar (alternative to the default slot). */
  @property({ attribute: "label-text" }) labelText = "";

  /**
   * When true, renders a trailing close (✕) affordance and lets Escape dismiss
   * the snackbar.
   */
  @property({ type: Boolean }) closeable = false;

  /**
   * Stacks the action beneath the text. M3 uses this two-line layout when the
   * text plus action would not fit on a single line.
   */
  @property({ type: Boolean, reflect: true }) stacked = false;

  @query(".snackbar") private readonly surface!: HTMLElement | null;

  @queryAssignedElements({ slot: "action", flatten: true })
  private readonly actionElements!: HTMLElement[];

  private timer = -1;

  /**
   * Opens the snackbar and starts the auto-dismiss timer. Resolves after the
   * surface has been promoted to the top layer. Re-showing an already-open
   * snackbar restarts the timer.
   */
  async show() {
    if (this.open) {
      this.startTimer();
      return;
    }
    this.open = true;
    this.dispatchEvent(new Event("opening"));
    await this.updateComplete;
    this.showSurface();
    this.dispatchEvent(new Event("opened"));
    this.startTimer();
  }

  /**
   * Closes the snackbar. Dispatches a cancelable `closing` event; if it is
   * prevented the snackbar stays open.
   */
  async close(reason: SnackbarCloseReason = "programmatic") {
    if (!this.open) {
      return;
    }
    this.clearTimer();
    const closing = new CustomEvent("closing", {
      detail: { reason },
      cancelable: true,
    });
    this.dispatchEvent(closing);
    if (closing.defaultPrevented) {
      this.startTimer();
      return;
    }
    this.hideSurface();
    this.open = false;
    this.dispatchEvent(new CustomEvent("closed", { detail: { reason } }));
  }

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.addEventListener("mouseenter", this.pause);
      this.addEventListener("mouseleave", this.resume);
      this.addEventListener("focusin", this.pause);
      this.addEventListener("focusout", this.resume);
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    this.clearTimer();
  }

  protected override updated(changed: Map<string, unknown>) {
    // Keep the popover state in sync when `open` is toggled via the attribute.
    if (changed.has("open")) {
      if (this.open) {
        this.showSurface();
      } else {
        this.hideSurface();
      }
    }
  }

  /** Promote the surface to the top layer (no-op without Popover API). */
  private showSurface() {
    const s = this.surface;
    if (s && typeof s.showPopover === "function" && !s.matches(":popover-open")) {
      s.showPopover();
    }
  }

  /** Remove the surface from the top layer (no-op without Popover API). */
  private hideSurface() {
    const s = this.surface;
    if (s && typeof s.hidePopover === "function" && s.matches(":popover-open")) {
      s.hidePopover();
    }
  }

  protected override render() {
    const classes: ClassInfo = { stacked: this.stacked };
    return html`
      <div
        class="snackbar ${classMap(classes)}"
        popover="manual"
        role="status"
        aria-live="polite"
        @keydown=${this.handleKeydown}
      >
        <div class="text"><slot>${this.labelText}</slot></div>
        <div class="actions" @click=${this.handleActionClick}>
          <slot name="action"></slot>
          ${this.renderCloseButton()}
        </div>
      </div>
    `;
  }

  private renderCloseButton() {
    if (!this.closeable) {
      return nothing;
    }
    return html`
      <button class="close" aria-label="Dismiss" @click=${this.handleCloseClick}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M6.4 19 5 17.6l5.6-5.6L5 6.4 6.4 5l5.6 5.6L17.6 5 19 6.4 13.4 12l5.6 5.6-1.4 1.4-5.6-5.6Z"
          ></path>
        </svg>
      </button>
    `;
  }

  private handleCloseClick() {
    void this.close("dismiss");
  }

  private handleActionClick(event: Event) {
    const target = event.target as HTMLElement;
    if (this.actionElements.some((el) => el === target || el.contains(target))) {
      void this.close("action");
    }
  }

  private handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && this.closeable) {
      event.stopPropagation();
      void this.close("dismiss");
    }
  }

  private readonly pause = () => {
    this.clearTimer();
  };

  private readonly resume = () => {
    if (this.open) {
      this.startTimer();
    }
  };

  private startTimer() {
    this.clearTimer();
    if (this.timeoutMs > 0 && !isServer) {
      this.timer = window.setTimeout(() => {
        void this.close("timeout");
      }, this.timeoutMs);
    }
  }

  private clearTimer() {
    if (this.timer !== -1) {
      clearTimeout(this.timer);
      this.timer = -1;
    }
  }
}
