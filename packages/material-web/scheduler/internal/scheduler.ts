/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing, TemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";
import { styleMap } from "lit/directives/style-map.js";

/** Supported scheduler views. */
export type SchedulerView = "day" | "week" | "month";

/** Direction passed to navigation handlers / surfaced on `scheduler:navigate`. */
export type SchedulerNavigateDirection = "prev" | "next" | "today";

/**
 * A calendar event as accepted on the `events` property. `start`/`end` may be
 * either a `Date` or an ISO 8601 string (parsed with the native `Date`
 * constructor). `color` is any CSS color applied to the event chip.
 */
export interface SchedulerEvent {
  id: string;
  start: Date | string;
  end: Date | string;
  title: string;
  color?: string;
}

/** A `SchedulerEvent` with its endpoints normalised to `Date`. */
export interface NormalizedSchedulerEvent {
  id: string;
  start: Date;
  end: Date;
  title: string;
  color?: string;
}

/** `detail` payload of `scheduler:select`. */
export interface SchedulerSelectDetail {
  /** The slot/day that was activated (local `Date`). */
  date: Date;
}

/** `detail` payload of `scheduler:event-click`. */
export interface SchedulerEventClickDetail {
  /** The normalised event that was activated. */
  event: NormalizedSchedulerEvent;
}

/** `detail` payload of `scheduler:navigate`. */
export interface SchedulerNavigateDetail {
  /** The navigation direction. */
  direction: SchedulerNavigateDirection;
  /** The new current date after navigation (local `Date`). */
  date: Date;
}

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] as const;
const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
] as const;

/** Minutes in a day — used for vertical positioning in day/week views. */
const MINUTES_PER_DAY = 24 * 60;

/** Pads a number to a two-digit string. */
function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

/** Coerces a `Date | string` into a `Date` (invalid inputs become `Invalid Date`). */
function toDate(value: Date | string): Date {
  return value instanceof Date ? value : new Date(value);
}

/** Midnight (local) of the given date. */
function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

/** Whether two dates fall on the same local calendar day. */
function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/** Adds `days` to a date, returning a new local `Date`. */
function addDays(date: Date, days: number): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
}

/** The Sunday that starts the week containing `date`. */
function startOfWeek(date: Date): Date {
  return addDays(startOfDay(date), -date.getDay());
}

/** A single month-view cell descriptor (memoised in `willUpdate`). */
interface MonthCell {
  date: Date;
  day: number;
  inMonth: boolean;
  isToday: boolean;
  label: string;
  events: NormalizedSchedulerEvent[];
}

/** A positioned event chip for the day/week time grid (memoised). */
interface PositionedEvent {
  event: NormalizedSchedulerEvent;
  /** Top offset as a percentage of the day height (0–100). */
  top: number;
  /** Height as a percentage of the day height (0–100). */
  height: number;
}

/** A single day column for the day/week time grid (memoised). */
interface DayColumn {
  date: Date;
  isToday: boolean;
  label: string;
  events: PositionedEvent[];
}

/**
 * A Material 3 scheduler / calendar. Renders a `day`, `week`, or `month` view
 * computed from real `Date` arithmetic, with prev/next/today navigation and a
 * settable current `date`. Events supplied on the `events` property are laid
 * out into month cells or positioned vertically by time in the day/week time
 * grid.
 *
 * This is a Community baseline: it does not implement drag-and-drop, event
 * resize, recurrence, or resource/timeline rows (those are MUI X Pro/Premium
 * concerns and are intentionally out of scope).
 *
 * Grid math is memoised in `willUpdate` so `render()` stays pure. The element
 * is self-contained on the `--md-sys-*` tokens.
 *
 * @fires scheduler:select {CustomEvent<SchedulerSelectDetail>} Fired when an
 *     empty slot or day cell is activated.
 * @fires scheduler:event-click {CustomEvent<SchedulerEventClickDetail>} Fired
 *     when an event chip is activated.
 * @fires scheduler:navigate {CustomEvent<SchedulerNavigateDetail>} Fired when
 *     the view is navigated (prev/next/today).
 */
export class Scheduler extends LitElement {
  /** The active view. */
  @property({ type: String, reflect: true }) view: SchedulerView = "week";

  /** The current date as a local `YYYY-MM-DD` ISO string. */
  @property({ type: String }) date = "";

  /**
   * The events to render. Each event's `start`/`end` may be a `Date` or an ISO
   * string. This is a property, not an attribute, and must be set in JS.
   */
  @property({ attribute: false }) events: SchedulerEvent[] = [];

  /** The resolved current date (defaults to today when `date` is empty/invalid). */
  @state() private current: Date = startOfDay(new Date());

  /** Memoised month grid (month view). */
  private monthCells: MonthCell[] = [];

  /** Memoised day columns (day/week views). */
  private dayColumns: DayColumn[] = [];

  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "grid");
    this.syncCurrentToDate();
  }

  override willUpdate(changed: Map<string, unknown>) {
    if (changed.has("date")) {
      this.syncCurrentToDate();
    }
    if (
      changed.has("date") ||
      changed.has("current") ||
      changed.has("view") ||
      changed.has("events")
    ) {
      this.recompute();
    }
  }

  private syncCurrentToDate() {
    const parsed = this.parseDate(this.date);
    this.current = parsed ?? startOfDay(new Date());
  }

  /** Parses a local `YYYY-MM-DD` string into a midnight `Date`, or `null`. */
  private parseDate(value: string): Date | null {
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

  /** Serialises a `Date` to a local `YYYY-MM-DD` string. */
  private toISODate(date: Date): string {
    return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`;
  }

  /** Normalises and validity-filters the `events` input. */
  private normalizeEvents(): NormalizedSchedulerEvent[] {
    const result: NormalizedSchedulerEvent[] = [];
    for (const event of this.events) {
      const start = toDate(event.start);
      const end = toDate(event.end);
      if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
        continue;
      }
      result.push({
        id: event.id,
        start,
        end,
        title: event.title,
        color: event.color,
      });
    }
    return result;
  }

  /** Recomputes the memoised grid for the active view. */
  private recompute(): void {
    const events = this.normalizeEvents();
    if (this.view === "month") {
      this.computeMonth(events);
      this.dayColumns = [];
    } else {
      this.computeColumns(events);
      this.monthCells = [];
    }
  }

  /** Builds the 6×7 month grid (leading/trailing days from adjacent months). */
  private computeMonth(events: NormalizedSchedulerEvent[]): void {
    const year = this.current.getFullYear();
    const month = this.current.getMonth();
    const firstOfMonth = new Date(year, month, 1);
    const gridStart = addDays(firstOfMonth, -firstOfMonth.getDay());
    const today = startOfDay(new Date());

    const cells: MonthCell[] = [];
    for (let i = 0; i < 42; i++) {
      const date = addDays(gridStart, i);
      cells.push({
        date,
        day: date.getDate(),
        inMonth: date.getMonth() === month,
        isToday: isSameDay(date, today),
        label: `${MONTHS[date.getMonth()]} ${date.getDate()}, ${date.getFullYear()}`,
        events: events
          .filter((event) => this.eventTouchesDay(event, date))
          .sort((a, b) => a.start.getTime() - b.start.getTime()),
      });
    }
    this.monthCells = cells;
  }

  /** Builds the day columns (1 for `day`, 7 for `week`) with positioned events. */
  private computeColumns(events: NormalizedSchedulerEvent[]): void {
    const today = startOfDay(new Date());
    const days = this.view === "day" ? [startOfDay(this.current)] : this.weekDays();

    this.dayColumns = days.map((date) => ({
      date,
      isToday: isSameDay(date, today),
      label: `${WEEKDAYS[date.getDay()]} ${MONTHS[date.getMonth()]} ${date.getDate()}`,
      events: events
        .filter((event) => this.eventTouchesDay(event, date))
        .sort((a, b) => a.start.getTime() - b.start.getTime())
        .map((event) => this.positionEvent(event, date)),
    }));
  }

  /** The seven days (Sun–Sat) of the week containing `current`. */
  private weekDays(): Date[] {
    const start = startOfWeek(this.current);
    const days: Date[] = [];
    for (let i = 0; i < 7; i++) {
      days.push(addDays(start, i));
    }
    return days;
  }

  /** Whether an event overlaps the given local day. */
  private eventTouchesDay(event: NormalizedSchedulerEvent, day: Date): boolean {
    const dayStart = startOfDay(day);
    const dayEnd = addDays(dayStart, 1);
    return event.start < dayEnd && event.end > dayStart;
  }

  /** Clamps an event to the given day and converts to top/height percentages. */
  private positionEvent(event: NormalizedSchedulerEvent, day: Date): PositionedEvent {
    const dayStart = startOfDay(day);
    const startMin = Math.max(0, (event.start.getTime() - dayStart.getTime()) / 60000);
    const endMin = Math.min(MINUTES_PER_DAY, (event.end.getTime() - dayStart.getTime()) / 60000);
    const top = (startMin / MINUTES_PER_DAY) * 100;
    const height = Math.max(1, ((endMin - startMin) / MINUTES_PER_DAY) * 100);
    return { event, top, height };
  }

  protected override render(): TemplateResult {
    return html`
      <div class="scheduler">
        ${this.renderToolbar()}
        ${this.view === "month" ? this.renderMonth() : this.renderTimeGrid()}
      </div>
    `;
  }

  private renderToolbar(): TemplateResult {
    return html`
      <div class="toolbar">
        <div class="nav-group">
          <button class="nav" aria-label="Previous" @click=${this.handlePrev}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M15.4 7.4 14 6l-6 6 6 6 1.4-1.4-4.6-4.6z"></path>
            </svg>
          </button>
          <button class="today" @click=${this.handleToday}>Today</button>
          <button class="nav" aria-label="Next" @click=${this.handleNext}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M8.6 16.6 13.2 12 8.6 7.4 10 6l6 6-6 6z"></path>
            </svg>
          </button>
        </div>
        <span class="title">${this.titleText()}</span>
        <div class="views" role="tablist" aria-label="Calendar view">
          ${(["day", "week", "month"] as const).map((view) => this.renderViewTab(view))}
        </div>
      </div>
    `;
  }

  private renderViewTab(view: SchedulerView): TemplateResult {
    const selected = this.view === view;
    return html`
      <button
        class=${classMap({ "view-tab": true, selected })}
        role="tab"
        aria-selected=${selected ? "true" : "false"}
        data-view=${view}
        @click=${this.handleViewClick}
      >
        ${view.charAt(0).toUpperCase() + view.slice(1)}
      </button>
    `;
  }

  /** Human-readable label for the current view/date. */
  private titleText(): string {
    const year = this.current.getFullYear();
    if (this.view === "month") {
      return `${MONTHS[this.current.getMonth()]} ${year}`;
    }
    if (this.view === "day") {
      return `${WEEKDAYS[this.current.getDay()]}, ${MONTHS[this.current.getMonth()]} ${this.current.getDate()}, ${year}`;
    }
    const start = startOfWeek(this.current);
    const end = addDays(start, 6);
    if (start.getMonth() === end.getMonth()) {
      return `${MONTHS[start.getMonth()]} ${start.getDate()} – ${end.getDate()}, ${year}`;
    }
    return `${MONTHS[start.getMonth()]} ${start.getDate()} – ${MONTHS[end.getMonth()]} ${end.getDate()}, ${year}`;
  }

  private renderMonth(): TemplateResult {
    return html`
      <div class="month">
        <div class="weekdays" role="row">
          ${WEEKDAYS.map((d) => html`<span class="weekday" role="columnheader">${d}</span>`)}
        </div>
        <div class="month-grid">${this.monthCells.map((c) => this.renderMonthCell(c))}</div>
      </div>
    `;
  }

  private renderMonthCell(cell: MonthCell): TemplateResult {
    const classes = classMap({
      "month-cell": true,
      "out-of-month": !cell.inMonth,
      today: cell.isToday,
    });
    return html`
      <div
        class=${classes}
        role="gridcell"
        aria-label=${cell.label}
        tabindex="-1"
        data-date=${this.toISODate(cell.date)}
        @click=${this.handleSlotClick}
        @keydown=${this.handleSlotKeydown}
      >
        <span class="month-day-number">${cell.day}</span>
        <div class="month-events">
          ${cell.events.map((event) => this.renderEventChip(event, true))}
        </div>
      </div>
    `;
  }

  private renderTimeGrid(): TemplateResult {
    return html`
      <div class=${classMap({ "time-grid": true, single: this.view === "day" })}>
        <div class="time-axis" aria-hidden="true">${this.renderHourLabels()}</div>
        <div class="columns">
          ${this.dayColumns.map((column) => this.renderColumnHeader(column))}
          ${this.dayColumns.map((column) => this.renderColumnBody(column))}
        </div>
      </div>
    `;
  }

  private renderHourLabels(): TemplateResult[] {
    const labels: TemplateResult[] = [];
    for (let hour = 0; hour < 24; hour++) {
      const label =
        hour === 0 ? "12 AM" : hour < 12 ? `${hour} AM` : hour === 12 ? "12 PM" : `${hour - 12} PM`;
      labels.push(html`<span class="hour-label">${label}</span>`);
    }
    return labels;
  }

  private renderColumnHeader(column: DayColumn): TemplateResult {
    return html`
      <div class=${classMap({ "column-header": true, today: column.isToday })} role="columnheader">
        ${column.label}
      </div>
    `;
  }

  private renderColumnBody(column: DayColumn): TemplateResult {
    const hours: TemplateResult[] = [];
    for (let hour = 0; hour < 24; hour++) {
      const slotDate = new Date(
        column.date.getFullYear(),
        column.date.getMonth(),
        column.date.getDate(),
        hour,
      );
      hours.push(html`
        <div
          class="hour-slot"
          role="gridcell"
          aria-label=${`${column.label}, ${this.hourLabel(hour)}`}
          tabindex="-1"
          data-date=${slotDate.toISOString()}
          @click=${this.handleSlotClick}
          @keydown=${this.handleSlotKeydown}
        ></div>
      `);
    }
    return html`
      <div class="column-body">
        ${hours}
        <div class="column-events">
          ${column.events.map((positioned) => this.renderPositionedEvent(positioned))}
        </div>
      </div>
    `;
  }

  private hourLabel(hour: number): string {
    if (hour === 0) {
      return "12 AM";
    }
    if (hour < 12) {
      return `${hour} AM`;
    }
    if (hour === 12) {
      return "12 PM";
    }
    return `${hour - 12} PM`;
  }

  private renderPositionedEvent(positioned: PositionedEvent): TemplateResult {
    const style = styleMap({
      top: `${positioned.top}%`,
      height: `${positioned.height}%`,
      ...(positioned.event.color ? { "background-color": positioned.event.color } : {}),
    });
    return html`
      <button
        class="event positioned"
        style=${style}
        data-event-id=${positioned.event.id}
        @click=${this.handleEventClick}
      >
        <span class="event-title">${positioned.event.title}</span>
      </button>
    `;
  }

  private renderEventChip(event: NormalizedSchedulerEvent, compact: boolean): TemplateResult {
    const style = event.color ? styleMap({ "background-color": event.color }) : nothing;
    return html`
      <button
        class=${classMap({ event: true, chip: compact })}
        style=${style}
        data-event-id=${event.id}
        @click=${this.handleEventClick}
      >
        <span class="event-title">${event.title}</span>
      </button>
    `;
  }

  private handleViewClick(event: Event) {
    const button = event.currentTarget as HTMLButtonElement;
    const view = button.dataset["view"] as SchedulerView | undefined;
    if (view && view !== this.view) {
      this.view = view;
    }
  }

  private handlePrev() {
    this.navigate("prev");
  }

  private handleNext() {
    this.navigate("next");
  }

  private handleToday() {
    this.navigate("today");
  }

  /** Moves the current date per the view granularity and fires `scheduler:navigate`. */
  private navigate(direction: SchedulerNavigateDirection) {
    let next: Date;
    if (direction === "today") {
      next = startOfDay(new Date());
    } else {
      const sign = direction === "next" ? 1 : -1;
      if (this.view === "month") {
        next = new Date(
          this.current.getFullYear(),
          this.current.getMonth() + sign,
          this.current.getDate(),
        );
      } else if (this.view === "week") {
        next = addDays(this.current, 7 * sign);
      } else {
        next = addDays(this.current, sign);
      }
    }
    this.date = this.toISODate(next);
    this.current = next;
    this.dispatchEvent(
      new CustomEvent<SchedulerNavigateDetail>("scheduler:navigate", {
        detail: { direction, date: next },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleSlotClick(event: Event) {
    const target = event.currentTarget as HTMLElement;
    this.selectSlot(target.dataset["date"]);
  }

  private handleSlotKeydown(event: KeyboardEvent) {
    if (event.key !== "Enter" && event.key !== " ") {
      return;
    }
    event.preventDefault();
    const target = event.currentTarget as HTMLElement;
    this.selectSlot(target.dataset["date"]);
  }

  private selectSlot(raw: string | undefined) {
    if (!raw) {
      return;
    }
    const date = new Date(raw);
    if (Number.isNaN(date.getTime())) {
      return;
    }
    this.dispatchEvent(
      new CustomEvent<SchedulerSelectDetail>("scheduler:select", {
        detail: { date },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleEventClick(domEvent: Event) {
    domEvent.stopPropagation();
    const button = domEvent.currentTarget as HTMLButtonElement;
    const id = button.dataset["eventId"];
    if (!id) {
      return;
    }
    const found = this.normalizeEvents().find((event) => event.id === id);
    if (!found) {
      return;
    }
    this.dispatchEvent(
      new CustomEvent<SchedulerEventClickDetail>("scheduler:event-click", {
        detail: { event: found },
        bubbles: true,
        composed: true,
      }),
    );
  }
}
