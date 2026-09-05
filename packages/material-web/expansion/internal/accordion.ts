/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement } from "lit";
import { property, queryAssignedElements } from "lit/decorators.js";

import { ExpansionPanel, ExpansionToggleDetail } from "./expansion-panel.js";

/**
 * A Material 3 accordion: a container of `md-expansion-panel` children. In the
 * default single-expand mode the accordion listens for each panel's
 * `expansion:toggle` event and collapses the other panels when one opens. Set
 * `multi` to let several panels stay open simultaneously.
 *
 * Slot:
 * - default — the `md-expansion-panel` children.
 */
export class Accordion extends LitElement {
  /**
   * When true, multiple panels may be expanded at once. When false (default),
   * opening one panel collapses any other open panel.
   */
  @property({ type: Boolean, reflect: true }) multi = false;

  @queryAssignedElements({ flatten: true })
  private readonly items!: HTMLElement[];

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.addEventListener("expansion:toggle", this.handleToggle);
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    if (!isServer) {
      this.removeEventListener("expansion:toggle", this.handleToggle);
    }
  }

  private readonly handleToggle = (event: Event) => {
    if (this.multi) {
      return;
    }
    const detail = (event as CustomEvent<ExpansionToggleDetail>).detail;
    if (!detail || !detail.expanded) {
      return;
    }
    const opened = event.target as HTMLElement | null;
    for (const panel of this.expansionPanels()) {
      if (panel !== opened && panel.expanded) {
        panel.collapse();
      }
    }
  };

  /** Returns the slotted `md-expansion-panel` children. */
  private expansionPanels(): ExpansionPanel[] {
    return this.items.filter((el): el is ExpansionPanel => el instanceof ExpansionPanel);
  }

  protected override render() {
    return html`<slot role="presentation"></slot>`;
  }
}
