/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement, nothing } from "lit";
import { property, query, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";

/** A single autocomplete option. */
export interface AutocompleteOption {
  /** The stable value emitted on selection. */
  value: string;
  /** The human-readable label shown in the list. */
  label: string;
}

/** Filtering strategy for matching options against the typed text. */
export type AutocompleteFilter = "contains" | "startsWith";

let autocompleteIdCounter = 0;

/**
 * An autocomplete combines a Material 3 outlined text field with a filtered,
 * floating list of suggestions. As the user types, the option list is filtered
 * in real time (case-insensitive) using the configured `filter` strategy.
 *
 * Keyboard support: ArrowDown / ArrowUp move the highlight through visible
 * options, Enter selects the highlighted option, and Escape closes the panel.
 * Clicking an option selects it. Implements the WAI-ARIA combobox pattern
 * (`role="combobox"` input owning a `role="listbox"` popup with
 * `aria-activedescendant`).
 *
 * @fires input {Event} Re-dispatched whenever the typed text changes.
 * @fires autocomplete:select {CustomEvent<{value: string, label: string}>}
 *     Fired when an option is chosen (click or Enter).
 */
export class Autocomplete extends LitElement {
  /** The list of selectable options. Set via the `.options` property. */
  @property({ attribute: false }) options: AutocompleteOption[] = [];

  /** The current text value of the input. */
  @property() value = "";

  /** Floating label shown above the field. */
  @property() label = "";

  /** Filtering strategy. Defaults to substring (`contains`) matching. */
  @property() filter: AutocompleteFilter = "contains";

  /** Whether the suggestions panel is open. Reflected for CSS targeting. */
  @property({ type: Boolean, reflect: true }) open = false;

  /** Whether the field is disabled. */
  @property({ type: Boolean, reflect: true }) disabled = false;

  /** Index of the highlighted option within the *filtered* list, or -1. */
  @state() private activeIndex = -1;

  @query("input") private readonly input!: HTMLInputElement | null;

  private readonly listboxId = `md-autocomplete-list-${autocompleteIdCounter++}`;

  /** Options matching the current `value` under the active filter strategy. */
  get filteredOptions(): AutocompleteOption[] {
    const query = this.value.trim().toLowerCase();
    if (query === "") {
      return this.options;
    }
    return this.options.filter((option) => {
      const label = option.label.toLowerCase();
      return this.filter === "startsWith" ? label.startsWith(query) : label.includes(query);
    });
  }

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.addEventListener("focusout", this.handleFocusOut);
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    if (!isServer) {
      this.removeEventListener("focusout", this.handleFocusOut);
    }
  }

  /** Opens the suggestions panel. */
  show() {
    if (this.disabled) {
      return;
    }
    this.open = true;
  }

  /** Closes the suggestions panel and clears the highlight. */
  close() {
    this.open = false;
    this.activeIndex = -1;
  }

  protected override render() {
    const filtered = this.filteredOptions;
    const hasValue = this.value.length > 0;
    const activeId =
      this.open && this.activeIndex >= 0 && this.activeIndex < filtered.length
        ? `${this.listboxId}-opt-${this.activeIndex}`
        : nothing;
    return html`
      <div class="field ${classMap({ populated: hasValue, disabled: this.disabled })}">
        <input
          type="text"
          part="input"
          .value=${this.value}
          ?disabled=${this.disabled}
          role="combobox"
          aria-autocomplete="list"
          aria-expanded=${this.open ? "true" : "false"}
          aria-controls=${this.listboxId}
          aria-activedescendant=${activeId}
          @input=${this.handleInput}
          @keydown=${this.handleKeydown}
          @focus=${this.handleFocus}
          @click=${this.handleFocus}
        />
        ${this.label ? html`<label class="label">${this.label}</label>` : nothing}
        <div class="outline" aria-hidden="true"></div>
      </div>
      <ul
        id=${this.listboxId}
        class="panel"
        role="listbox"
        ?hidden=${!this.open || filtered.length === 0}
      >
        ${filtered.map((option, index) => this.renderOption(option, index))}
      </ul>
    `;
  }

  private renderOption(option: AutocompleteOption, index: number) {
    const selected = index === this.activeIndex;
    return html`
      <li
        id=${`${this.listboxId}-opt-${index}`}
        class="option ${classMap({ active: selected })}"
        role="option"
        aria-selected=${selected ? "true" : "false"}
        @click=${() => {
          this.selectOption(option);
        }}
        @pointerenter=${() => {
          this.activeIndex = index;
        }}
      >
        ${option.label}
      </li>
    `;
  }

  private handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    this.value = target.value;
    this.activeIndex = -1;
    this.open = true;
    // Re-dispatch a composed `input` event so consumers see the text change.
    this.dispatchEvent(new Event("input", { bubbles: true, composed: true }));
  }

  private readonly handleFocus = () => {
    if (!this.disabled && this.options.length > 0) {
      this.open = true;
    }
  };

  private readonly handleFocusOut = (event: FocusEvent) => {
    // Close only when focus leaves the component entirely.
    const next = event.relatedTarget as Node | null;
    if (next && this.contains(next)) {
      return;
    }
    this.close();
  };

  private handleKeydown(event: KeyboardEvent) {
    const filtered = this.filteredOptions;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        if (!this.open) {
          this.open = true;
        }
        this.moveActive(1, filtered.length);
        break;
      case "ArrowUp":
        event.preventDefault();
        if (!this.open) {
          this.open = true;
        }
        this.moveActive(-1, filtered.length);
        break;
      case "Enter":
        if (this.open && this.activeIndex >= 0 && this.activeIndex < filtered.length) {
          event.preventDefault();
          this.selectOption(filtered[this.activeIndex]);
        }
        break;
      case "Escape":
        if (this.open) {
          event.preventDefault();
          event.stopPropagation();
          this.close();
        }
        break;
      default:
        break;
    }
  }

  private moveActive(delta: number, count: number) {
    if (count === 0) {
      this.activeIndex = -1;
      return;
    }
    let next = this.activeIndex + delta;
    if (next < 0) {
      next = count - 1;
    } else if (next >= count) {
      next = 0;
    }
    this.activeIndex = next;
  }

  private selectOption(option: AutocompleteOption) {
    this.value = option.label;
    this.close();
    if (this.input) {
      this.input.value = option.label;
    }
    this.dispatchEvent(
      new CustomEvent("autocomplete:select", {
        detail: { value: option.value, label: option.label },
        bubbles: true,
        composed: true,
      }),
    );
  }
}
