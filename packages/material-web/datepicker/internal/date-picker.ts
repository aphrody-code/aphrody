/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement, nothing, TemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";

import {
  formatDisplayLocale,
  getDateLocaleData,
  monthName,
  parseDisplayLocale,
  placeholderMask,
} from "./date-i18n.js";

/** The `detail` payload of the `date-picker:change` event. */
export interface DatePickerChangeDetail {
  /** Selected date as an ISO `YYYY-MM-DD` string. */
  value: string;
}

/**
 * Predicate used to disable individual dates. Receives a local `Date` (midnight
 * of the day) and returns `true` if that day should be unselectable.
 */
export type DateDisabledPredicate = (date: Date) => boolean;

/** Pads a number to a two-digit string. */
export function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

/** Formats a `Date` as a local `YYYY-MM-DD` ISO date string. */
export function toISO(date: Date): string {
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`;
}

/** Parses a `YYYY-MM-DD` string into a local `Date`, or `null` if invalid. */
export function fromISO(value: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value.trim());
  if (!match) {
    return null;
  }
  const year = Number(match[1]);
  const month = Number(match[2]) - 1;
  const day = Number(match[3]);
  const date = new Date(year, month, day);
  if (date.getFullYear() !== year || date.getMonth() !== month || date.getDate() !== day) {
    return null;
  }
  return date;
}

/**
 * Formats a `Date` for display in the editable field (locale `MM/DD/YYYY`).
 * Mirrors MUI X's masked text field; parsing accepts both `MM/DD/YYYY` and the
 * ISO `YYYY-MM-DD` form.
 */
export function formatDisplay(date: Date): string {
  return `${pad2(date.getMonth() + 1)}/${pad2(date.getDate())}/${date.getFullYear()}`;
}

/** Parses a user-typed field value (`MM/DD/YYYY` or `YYYY-MM-DD`). */
export function parseDisplay(value: string): Date | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  const iso = fromISO(trimmed);
  if (iso) {
    return iso;
  }
  const match = /^(\d{1,2})\/(\d{1,2})\/(\d{4})$/.exec(trimmed);
  if (!match) {
    return null;
  }
  const month = Number(match[1]) - 1;
  const day = Number(match[2]);
  const year = Number(match[3]);
  const date = new Date(year, month, day);
  if (date.getFullYear() !== year || date.getMonth() !== month || date.getDate() !== day) {
    return null;
  }
  return date;
}

/**
 * A docked Material 3 date picker: an optional editable text field coupled to a
 * month grid with prev/next month and year navigation and a single selected
 * day. The grid is computed from real `Date` arithmetic (first weekday of the
 * month, day count), and keyboard arrows move the selection. Individual dates
 * can be disabled via `min`/`max` or the `disabledDate` predicate.
 *
 * In addition to the high-level `date-picker:change` event, the element fires
 * native `input` and `change` events (whose `target.value` is the current ISO
 * string) so it can be consumed like a form control.
 *
 * @fires date-picker:change {CustomEvent<DatePickerChangeDetail>} Fired when a
 *     day is selected.
 * @fires input Native event fired as the value changes (e.g. typing or grid
 *     navigation). `event.target.value` is the ISO string.
 * @fires change Native event fired when a selection is committed.
 */
/** A single rendered calendar cell descriptor (memoised in `willUpdate`). */
interface DayCell {
  day: number;
  iso: string;
  isSelected: boolean;
  isToday: boolean;
  disabled: boolean;
  label: string;
}

export class DatePicker extends LitElement {
  /** Associates the element with a containing `<form>`. */
  static readonly formAssociated = true;

  private readonly internals = this.attachInternals();

  /** Selected date as `YYYY-MM-DD`. */
  @property({ type: String }) value = "";

  /** Earliest selectable date as `YYYY-MM-DD`. */
  @property({ type: String }) min = "";

  /** Latest selectable date as `YYYY-MM-DD`. */
  @property({ type: String }) max = "";

  /** Whether to show the coupled editable text field above the calendar. */
  @property({ type: Boolean }) editable = false;

  /** Placeholder shown in the editable field when empty. */
  @property({ type: String, attribute: "field-label" }) fieldLabel = "Date";

  /**
   * BCP-47 locale tag driving month/weekday labels and the editable field's
   * numeric date order (via `Intl`). Empty uses the runtime default; EN is the
   * hard fallback.
   */
  @property({ type: String }) locale = "";

  /**
   * Explicit display mask (`YYYY`/`MM`/`DD`, e.g. `DD.MM.YYYY`). When set it
   * overrides the locale-derived format for both rendering and parsing.
   */
  @property({ type: String }) format = "";

  /**
   * Optional predicate to disable individual dates (in addition to `min`/`max`).
   * This is a property, not an attribute, and must be set in JS.
   */
  @property({ attribute: false }) disabledDate?: DateDisabledPredicate;

  /** The first day of the currently displayed month. */
  @state() protected viewDate: Date = new Date();

  /** Current (possibly invalid) text in the editable field. */
  @state() private fieldText = "";

  /** Whether the field currently holds an unparseable value. */
  @state() private fieldInvalid = false;

  /** Number of leading blank cells before day 1 of the displayed month. */
  private leadingBlanks = 0;

  /** Memoised day cells for the displayed month (computed in `willUpdate`). */
  private dayCells: DayCell[] = [];

  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "group");
    this.syncViewToValue();
    this.syncFieldToValue();
    this.internals.setFormValue(this.value);
  }

  override willUpdate(changed: Map<string, unknown>) {
    if (changed.has("value")) {
      this.syncViewToValue();
      this.syncFieldToValue();
      this.internals.setFormValue(this.value);
    }
    // Recompute the month grid once per update (not on every render) — it
    // depends on the displayed month plus the disabling inputs.
    if (changed.has("locale") || changed.has("format")) {
      this.syncFieldToValue();
    }
    if (
      changed.has("value") ||
      changed.has("viewDate") ||
      changed.has("min") ||
      changed.has("max") ||
      changed.has("disabledDate") ||
      changed.has("locale")
    ) {
      this.computeDayCells();
    }
  }

  /**
   * The currently selected `Date`, or `null`. Defined as a seam so subclasses
   * whose `value` carries extra data (e.g. a date-time picker) can parse just
   * the date portion without re-implementing the grid logic.
   */
  protected selectedDate(): Date | null {
    return fromISO(this.value);
  }

  protected syncViewToValue() {
    const selected = this.selectedDate();
    const base = selected ?? new Date();
    this.viewDate = new Date(base.getFullYear(), base.getMonth(), 1);
  }

  private syncFieldToValue() {
    const selected = this.selectedDate();
    this.fieldText = selected ? formatDisplayLocale(selected, this.locale, this.format) : "";
    this.fieldInvalid = false;
  }

  /** Whether a given date is outside `min`/`max` or matched by `disabledDate`. */
  protected isDisabled(date: Date): boolean {
    const minDate = fromISO(this.min);
    const maxDate = fromISO(this.max);
    if (minDate != null && date < minDate) {
      return true;
    }
    if (maxDate != null && date > maxDate) {
      return true;
    }
    return this.disabledDate ? this.disabledDate(date) : false;
  }

  protected override render(): TemplateResult {
    const year = this.viewDate.getFullYear();
    const month = this.viewDate.getMonth();
    const weekdays = getDateLocaleData(this.locale).weekdays;
    return html`
      <div class="picker">
        ${this.editable ? this.renderField() : nothing}
        <div class="header">
          <button class="nav" aria-label="Previous year" @click=${this.handlePrevYear}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M18 7.4 16.6 6l-6 6 6 6L18 16.6 13.4 12zM11 7.4 9.6 6l-6 6 6 6L11 16.6 6.4 12z"
              ></path>
            </svg>
          </button>
          <button class="nav" aria-label="Previous month" @click=${this.handlePrev}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M15.4 7.4 14 6l-6 6 6 6 1.4-1.4-4.6-4.6z"></path>
            </svg>
          </button>
          <span class="month-label">${monthName(this.locale, month)} ${year}</span>
          <button class="nav" aria-label="Next month" @click=${this.handleNext}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M8.6 16.6 13.2 12 8.6 7.4 10 6l6 6-6 6z"></path>
            </svg>
          </button>
          <button class="nav" aria-label="Next year" @click=${this.handleNextYear}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M6 7.4 7.4 6l6 6-6 6L6 16.6 10.6 12zM13 7.4 14.4 6l6 6-6 6L13 16.6 17.6 12z"
              ></path>
            </svg>
          </button>
        </div>
        <div class="weekdays" aria-hidden="true">
          ${weekdays.map((d) => html`<span class="weekday">${d}</span>`)}
        </div>
        <div class="grid" role="grid" @keydown=${this.handleKeydown}>
          ${this.renderDays(year, month)}
        </div>
      </div>
    `;
  }

  private renderField(): TemplateResult {
    return html`
      <div class=${classMap({ field: true, invalid: this.fieldInvalid })}>
        <input
          class="field-input"
          type="text"
          inputmode="numeric"
          placeholder=${placeholderMask(this.locale, this.format)}
          aria-label=${this.fieldLabel}
          aria-invalid=${this.fieldInvalid ? "true" : "false"}
          .value=${this.fieldText}
          @input=${this.handleFieldInput}
          @change=${this.handleFieldChange}
        />
      </div>
    `;
  }

  /**
   * Computes the displayed month's grid (leading blanks + day descriptors).
   * Runs once per relevant update in `willUpdate` so `render()` stays pure and
   * the (date-arithmetic-heavy) grid is not rebuilt on unrelated renders.
   */
  protected computeDayCells(): void {
    const year = this.viewDate.getFullYear();
    const month = this.viewDate.getMonth();
    this.leadingBlanks = new Date(year, month, 1).getDay();
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const selected = this.selectedDate();
    const selectedISO = selected ? toISO(selected) : "";
    const todayISO = toISO(new Date());

    const cells: DayCell[] = [];
    for (let day = 1; day <= daysInMonth; day++) {
      const date = new Date(year, month, day);
      const iso = toISO(date);
      cells.push({
        day,
        iso,
        isSelected: selectedISO !== "" && selectedISO === iso,
        isToday: iso === todayISO,
        disabled: this.isDisabled(date),
        label: `${monthName(this.locale, month)} ${day}, ${year}`,
      });
    }
    this.dayCells = cells;
  }

  protected renderDays(_year?: number, _month?: number) {
    if (this.dayCells.length === 0) {
      return nothing;
    }
    const blanks = [];
    for (let i = 0; i < this.leadingBlanks; i++) {
      blanks.push(html`<span class="cell empty" role="gridcell"></span>`);
    }
    const days = this.dayCells.map((c) => {
      const classes = classMap({
        cell: true,
        day: true,
        selected: c.isSelected,
        today: c.isToday && !c.isSelected,
        disabled: c.disabled,
      });
      return html`
        <button
          class=${classes}
          role="gridcell"
          aria-selected=${c.isSelected ? "true" : "false"}
          aria-label=${c.label}
          tabindex=${c.isSelected ? "0" : "-1"}
          data-day=${c.day}
          ?disabled=${c.disabled}
          @click=${this.handleDayClick}
        >
          ${c.day}
        </button>
      `;
    });
    return [...blanks, ...days];
  }

  private handleFieldInput(event: Event) {
    const input = event.target as HTMLInputElement;
    this.fieldText = input.value;
    const parsed = parseDisplayLocale(input.value, this.locale, this.format);
    if (parsed && !this.isDisabled(parsed)) {
      this.fieldInvalid = false;
      this.value = toISO(parsed);
      this.dispatchNative("input");
      this.dispatchChangeEvent();
    } else {
      this.fieldInvalid = input.value.trim().length > 0;
    }
  }

  private handleFieldChange() {
    if (this.fieldInvalid) {
      // Revert the field text to the last valid value on commit.
      this.syncFieldToValue();
      return;
    }
    this.dispatchNative("change");
  }

  private handlePrev() {
    this.viewDate = new Date(this.viewDate.getFullYear(), this.viewDate.getMonth() - 1, 1);
  }

  private handleNext() {
    this.viewDate = new Date(this.viewDate.getFullYear(), this.viewDate.getMonth() + 1, 1);
  }

  private handlePrevYear() {
    this.viewDate = new Date(this.viewDate.getFullYear() - 1, this.viewDate.getMonth(), 1);
  }

  private handleNextYear() {
    this.viewDate = new Date(this.viewDate.getFullYear() + 1, this.viewDate.getMonth(), 1);
  }

  private handleDayClick(event: Event) {
    const button = event.currentTarget as HTMLButtonElement;
    const day = Number(button.dataset["day"]);
    if (!day) {
      return;
    }
    this.selectDay(day);
  }

  protected selectDay(day: number) {
    const date = new Date(this.viewDate.getFullYear(), this.viewDate.getMonth(), day);
    if (this.isDisabled(date)) {
      return;
    }
    this.commitDate(date);
  }

  /**
   * Commits a newly selected `Date` to `value` and emits events. Defined as a
   * seam so subclasses (e.g. a date-time picker) can preserve extra value parts
   * such as a time component.
   */
  protected commitDate(date: Date) {
    this.value = toISO(date);
    this.syncFieldToValue();
    this.dispatchNative("input");
    this.dispatchChangeEvent();
    this.dispatchNative("change");
  }

  protected dispatchChangeEvent() {
    this.dispatchEvent(
      new CustomEvent<DatePickerChangeDetail>("date-picker:change", {
        detail: { value: this.value },
        bubbles: true,
        composed: true,
      }),
    );
  }

  /** Fires a native `input`/`change` event so `event.target.value` works. */
  protected dispatchNative(type: "input" | "change") {
    this.dispatchEvent(new Event(type, { bubbles: true, composed: true }));
  }

  private handleKeydown(event: KeyboardEvent) {
    const deltas: Record<string, number> = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -7,
      ArrowDown: 7,
    };
    const delta = deltas[event.key];
    if (delta === undefined) {
      return;
    }
    event.preventDefault();
    const current = this.selectedDate() ?? this.viewDate;
    const next = new Date(current.getFullYear(), current.getMonth(), current.getDate() + delta);
    if (this.isDisabled(next)) {
      return;
    }
    this.viewDate = new Date(next.getFullYear(), next.getMonth(), 1);
    this.commitDate(next);
    if (!isServer) {
      this.focusSelectedCell();
    }
  }

  private focusSelectedCell() {
    this.updateComplete.then(() => {
      const root = this.renderRoot as ParentNode;
      const cell = root.querySelector<HTMLElement>(".cell.selected");
      if (cell) {
        cell.focus();
      }
    });
  }
}
