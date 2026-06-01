/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement, nothing, TemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";

import { type DateDisabledPredicate, fromISO, toISO } from "./date-picker.js";
import {
  formatDisplayLocale,
  getDateLocaleData,
  monthName,
  parseDisplayLocale,
} from "./date-i18n.js";

/** The `detail` payload of the `date-range-picker:change` event. */
export interface DateRangeChangeDetail {
  /** Range start as an ISO `YYYY-MM-DD` string (or `''` if unset). */
  start: string;
  /** Range end as an ISO `YYYY-MM-DD` string (or `''` if unset). */
  end: string;
}

/**
 * A docked Material 3 date *range* picker: a month grid where the user picks a
 * start day and then an end day, with the days in between highlighted. Clicking
 * a day when a complete range already exists (or before the current start)
 * begins a new range. Supports `min`/`max`, a `disabledDate` predicate,
 * month/year navigation, and an optional pair of coupled editable fields.
 *
 * In addition to the high-level `date-range-picker:change` event, the element
 * fires native `input` and `change` events for form-control style consumption;
 * `event.target.value` is the `start/end` pair joined by `/` (e.g.
 * `2026-05-01/2026-05-08`).
 *
 * @fires date-range-picker:change {CustomEvent<DateRangeChangeDetail>} Fired
 *     when the start and/or end of the range changes.
 * @fires input Native event fired as the range changes.
 * @fires change Native event fired when a range edge is committed.
 */
/** A single rendered range-calendar cell descriptor (memoised). */
interface RangeDayCell {
  day: number;
  isStart: boolean;
  isEnd: boolean;
  inRange: boolean;
  isToday: boolean;
  disabled: boolean;
  selected: boolean;
  label: string;
}

export class DateRangePicker extends LitElement {
  /** Associates the element with a containing `<form>`. */
  static readonly formAssociated = true;

  private readonly internals = this.attachInternals();

  /** Range start as `YYYY-MM-DD`. */
  @property({ type: String }) start = "";

  /** Range end as `YYYY-MM-DD`. */
  @property({ type: String }) end = "";

  /** Earliest selectable date as `YYYY-MM-DD`. */
  @property({ type: String }) min = "";

  /** Latest selectable date as `YYYY-MM-DD`. */
  @property({ type: String }) max = "";

  /** Whether to show the coupled editable start/end fields above the grid. */
  @property({ type: Boolean }) editable = false;

  /** Optional predicate to disable individual dates. JS-only property. */
  @property({ attribute: false }) disabledDate?: DateDisabledPredicate;

  /** BCP-47 locale tag for month/weekday labels and field parsing. */
  @property({ type: String }) locale = "";

  /** Explicit display mask (`YYYY`/`MM`/`DD`) overriding the locale format. */
  @property({ type: String }) format = "";

  /** The first day of the currently displayed month. */
  @state() private viewDate: Date = new Date();

  /** Current text in the start/end editable fields. */
  @state() private startText = "";
  @state() private endText = "";

  /** Leading blank cells before day 1 of the displayed month. */
  private leadingBlanks = 0;

  /** Memoised day cells for the displayed month (computed in `willUpdate`). */
  private dayCells: RangeDayCell[] = [];

  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "group");
    this.syncViewToValue();
    this.syncFieldsToValue();
    this.internals.setFormValue(this.value);
  }

  override willUpdate(changed: Map<string, unknown>) {
    if (changed.has("start") || changed.has("end")) {
      this.syncViewToValue();
      this.syncFieldsToValue();
      this.internals.setFormValue(this.value);
    }
    if (changed.has("locale") || changed.has("format")) {
      this.syncFieldsToValue();
    }
    if (
      changed.has("start") ||
      changed.has("end") ||
      changed.has("viewDate") ||
      changed.has("min") ||
      changed.has("max") ||
      changed.has("disabledDate") ||
      changed.has("locale")
    ) {
      this.computeDayCells();
    }
  }

  private syncViewToValue() {
    const base = fromISO(this.start) ?? fromISO(this.end) ?? new Date();
    this.viewDate = new Date(base.getFullYear(), base.getMonth(), 1);
  }

  private syncFieldsToValue() {
    const s = fromISO(this.start);
    const e = fromISO(this.end);
    this.startText = s ? formatDisplayLocale(s, this.locale, this.format) : "";
    this.endText = e ? formatDisplayLocale(e, this.locale, this.format) : "";
  }

  private isDisabled(date: Date): boolean {
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

  protected override render() {
    const year = this.viewDate.getFullYear();
    const month = this.viewDate.getMonth();
    const weekdays = getDateLocaleData(this.locale).weekdays;
    return html`
      <div class="picker">
        ${this.editable ? this.renderFields() : nothing}
        <div class="header">
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

  private renderFields(): TemplateResult {
    return html`
      <div class="fields">
        <div class="field">
          <input
            class="field-input"
            type="text"
            inputmode="numeric"
            placeholder="Start"
            aria-label="Range start"
            .value=${this.startText}
            @input=${this.handleStartInput}
            @change=${this.handleFieldCommit}
          />
        </div>
        <span class="field-sep" aria-hidden="true">–</span>
        <div class="field">
          <input
            class="field-input"
            type="text"
            inputmode="numeric"
            placeholder="End"
            aria-label="Range end"
            .value=${this.endText}
            @input=${this.handleEndInput}
            @change=${this.handleFieldCommit}
          />
        </div>
      </div>
    `;
  }

  /**
   * Computes the displayed month's range grid once per relevant update (in
   * `willUpdate`) instead of inside `render()`, keeping render pure.
   */
  private computeDayCells(): void {
    const year = this.viewDate.getFullYear();
    const month = this.viewDate.getMonth();
    this.leadingBlanks = new Date(year, month, 1).getDay();
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const start = fromISO(this.start);
    const end = fromISO(this.end);
    const startISO = start ? toISO(start) : "";
    const endISO = end ? toISO(end) : "";
    const todayISO = toISO(new Date());

    const cells: RangeDayCell[] = [];
    for (let day = 1; day <= daysInMonth; day++) {
      const date = new Date(year, month, day);
      const iso = toISO(date);
      const isStart = startISO !== "" && startISO === iso;
      const isEnd = endISO !== "" && endISO === iso;
      const inRange = start != null && end != null && date > start && date < end;
      cells.push({
        day,
        isStart,
        isEnd,
        inRange,
        isToday: iso === todayISO,
        disabled: this.isDisabled(date),
        selected: isStart || isEnd,
        label: `${monthName(this.locale, month)} ${day}, ${year}`,
      });
    }
    this.dayCells = cells;
  }

  private renderDays(_year?: number, _month?: number) {
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
        "range-start": c.isStart,
        "range-end": c.isEnd,
        "in-range": c.inRange,
        today: c.isToday && !c.isStart && !c.isEnd,
        disabled: c.disabled,
      });
      return html`
        <button
          class=${classes}
          role="gridcell"
          aria-selected=${c.selected ? "true" : "false"}
          aria-label=${c.label}
          tabindex=${c.selected ? "0" : "-1"}
          data-day=${c.day}
          ?disabled=${c.disabled}
          @click=${this.handleDayClick}
        >
          <span class="day-bg"></span>
          <span class="day-num">${c.day}</span>
        </button>
      `;
    });
    return [...blanks, ...days];
  }

  private handleStartInput(event: Event) {
    const input = event.target as HTMLInputElement;
    this.startText = input.value;
    const parsed = parseDisplayLocale(input.value, this.locale, this.format);
    if (parsed && !this.isDisabled(parsed)) {
      this.start = toISO(parsed);
      this.normalizeOrder();
      this.emit("input");
    }
  }

  private handleEndInput(event: Event) {
    const input = event.target as HTMLInputElement;
    this.endText = input.value;
    const parsed = parseDisplayLocale(input.value, this.locale, this.format);
    if (parsed && !this.isDisabled(parsed)) {
      this.end = toISO(parsed);
      this.normalizeOrder();
      this.emit("input");
    }
  }

  private handleFieldCommit() {
    this.syncFieldsToValue();
    this.emit("change");
  }

  private normalizeOrder() {
    const s = fromISO(this.start);
    const e = fromISO(this.end);
    if (s && e && s > e) {
      const tmp = this.start;
      this.start = this.end;
      this.end = tmp;
    }
  }

  private handlePrev() {
    this.viewDate = new Date(this.viewDate.getFullYear(), this.viewDate.getMonth() - 1, 1);
  }

  private handleNext() {
    this.viewDate = new Date(this.viewDate.getFullYear(), this.viewDate.getMonth() + 1, 1);
  }

  private handleDayClick(event: Event) {
    const button = event.currentTarget as HTMLButtonElement;
    const day = Number(button.dataset["day"]);
    if (!day) {
      return;
    }
    const date = new Date(this.viewDate.getFullYear(), this.viewDate.getMonth(), day);
    if (this.isDisabled(date)) {
      return;
    }
    this.pick(date);
  }

  /**
   * Range-selection state machine: with no start (or a complete range), the
   * click sets a new start and clears the end; with a start but no end, the
   * click sets the end (swapping if earlier than the start).
   */
  private pick(date: Date) {
    const iso = toISO(date);
    const hasStart = fromISO(this.start) != null;
    const hasEnd = fromISO(this.end) != null;
    if (!hasStart || hasEnd) {
      this.start = iso;
      this.end = "";
    } else {
      const startDate = fromISO(this.start)!;
      if (date < startDate) {
        this.end = this.start;
        this.start = iso;
      } else {
        this.end = iso;
      }
    }
    this.syncFieldsToValue();
    this.emit("input");
    this.emit("change");
  }

  private emit(native?: "input" | "change") {
    if (native) {
      this.dispatchEvent(new Event(native, { bubbles: true, composed: true }));
    }
    this.dispatchEvent(
      new CustomEvent<DateRangeChangeDetail>("date-range-picker:change", {
        detail: { start: this.start, end: this.end },
        bubbles: true,
        composed: true,
      }),
    );
  }

  /** The combined value, `start/end`, for native form-control consumption. */
  get value(): string {
    return `${this.start}/${this.end}`;
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
    // Navigate from the most recently set edge (end, else start, else today).
    const anchor = fromISO(this.end) ?? fromISO(this.start) ?? this.viewDate;
    const next = new Date(anchor.getFullYear(), anchor.getMonth(), anchor.getDate() + delta);
    if (this.isDisabled(next)) {
      return;
    }
    this.pick(next);
    this.viewDate = new Date(next.getFullYear(), next.getMonth(), 1);
    if (!isServer) {
      this.focusEdgeCell();
    }
  }

  private focusEdgeCell() {
    this.updateComplete.then(() => {
      const root = this.renderRoot as ParentNode;
      const cell = root.querySelector<HTMLElement>(".cell.range-end, .cell.range-start");
      if (cell) {
        cell.focus();
      }
    });
  }
}
