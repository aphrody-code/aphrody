/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement, nothing, TemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";

import { getMeridiemLabels } from "./time-i18n.js";

/** The `detail` payload of the `time-picker:change` event. */
export interface TimePickerChangeDetail {
  /** Normalized 24-hour `HH:MM` string. */
  value: string;
}

/** The hour display format. */
export type TimeFormat = "12h" | "24h";

/** Pads a number to a two-digit string. */
function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

/** Clamps `n` into the inclusive `[lo, hi]` range. */
function clamp(n: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, n));
}

/** Parses a 24-hour `HH:MM` string into `[hours, minutes]`, or `null`. */
export function parseTime(value: string): [number, number] | null {
  const match = /^(\d{1,2}):(\d{1,2})$/.exec(value.trim());
  if (!match) {
    return null;
  }
  const h = Number(match[1]);
  const m = Number(match[2]);
  if (h > 23 || m > 59) {
    return null;
  }
  return [h, m];
}

/**
 * Parses a user-typed time string robustly. Accepts `HH:MM`, `H:MM`, `HHMM`,
 * and an optional AM/PM (or locale day-period) suffix, returning a 24-hour
 * `[hours, minutes]` pair or `null`.
 */
export function parseTimeInput(
  value: string,
  meridiem: { am: string; pm: string },
): [number, number] | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  const lower = trimmed.toLowerCase();
  let pm: boolean | null = null;
  if (lower.includes("pm") || lower.includes(meridiem.pm.toLowerCase())) {
    pm = true;
  } else if (lower.includes("am") || lower.includes(meridiem.am.toLowerCase())) {
    pm = false;
  }
  const nums = trimmed.match(/\d+/g);
  if (!nums) {
    return null;
  }
  let h: number;
  let m: number;
  if (nums.length >= 2) {
    h = Number(nums[0]);
    m = Number(nums[1]);
  } else {
    const digits = nums[0];
    if (digits.length <= 2) {
      h = Number(digits);
      m = 0;
    } else {
      h = Number(digits.slice(0, digits.length - 2));
      m = Number(digits.slice(-2));
    }
  }
  if (pm !== null) {
    h = h % 12;
    if (pm) {
      h += 12;
    }
  }
  if (h > 23 || m > 59) {
    return null;
  }
  return [h, m];
}

/**
 * A Material 3 time picker (input variant): two numeric fields for hours and
 * minutes separated by a colon, plus an AM/PM toggle in 12-hour mode. The value
 * is always emitted as a normalized 24-hour `HH:MM` string.
 *
 * The element is form-associated (via `ElementInternals`) and, when `editable`
 * is set, exposes a coupled single text field whose value is parsed leniently.
 * In addition to the high-level `time-picker:change` event it fires native
 * `input`/`change` events so it can be consumed like a form control.
 *
 * @fires time-picker:change {CustomEvent<TimePickerChangeDetail>} Fired when the
 *     time changes through user interaction.
 * @fires input Native event fired as the value changes.
 * @fires change Native event fired when a value is committed.
 */
export class TimePicker extends LitElement {
  /** Associates the element with a containing `<form>`. */
  static readonly formAssociated = true;

  private readonly internals = this.attachInternals();

  /** The time as a 24-hour `HH:MM` string. */
  @property({ type: String }) value = "";

  /** `'12h'` shows an AM/PM toggle; `'24h'` shows 0–23 hours. */
  @property({ type: String }) format: TimeFormat = "24h";

  /** Whether to show a single coupled editable text field instead of dials. */
  @property({ type: Boolean }) editable = false;

  /** Accessible label / placeholder for the editable field. */
  @property({ type: String, attribute: "field-label" }) fieldLabel = "Time";

  /** BCP-47 locale tag driving AM/PM (day-period) labels via `Intl`. */
  @property({ type: String }) locale = "";

  /** Parsed hours in 24h space (0–23). `-1` when unset/invalid. */
  @state() private hours24 = -1;

  /** Parsed minutes (0–59). `-1` when unset/invalid. */
  @state() private minutes = -1;

  /** Current (possibly invalid) text in the editable field. */
  @state() private fieldText = "";

  /** Whether the field currently holds an unparseable value. */
  @state() private fieldInvalid = false;

  override willUpdate(changed: Map<string, unknown>) {
    if (changed.has("value")) {
      this.parseValue();
      this.syncFieldToValue();
      this.internals.setFormValue(this.value);
    }
    if ((changed.has("locale") || changed.has("format")) && !changed.has("value")) {
      this.syncFieldToValue();
    }
  }

  override connectedCallback() {
    super.connectedCallback();
    this.setAttribute("role", "group");
    if (!this.hasAttribute("aria-label")) {
      this.setAttribute("aria-label", "Time");
    }
    this.parseValue();
    this.syncFieldToValue();
    this.internals.setFormValue(this.value);
  }

  private parseValue() {
    const parsed = parseTime(this.value);
    if (!parsed) {
      this.hours24 = -1;
      this.minutes = -1;
      return;
    }
    [this.hours24, this.minutes] = parsed;
  }

  private syncFieldToValue() {
    this.fieldText = this.hours24 < 0 ? "" : this.formatFieldValue();
    this.fieldInvalid = false;
  }

  /** Renders the current time for the editable field, respecting `format`. */
  private formatFieldValue(): string {
    if (this.format === "24h") {
      return `${pad2(this.hours24)}:${this.displayMinutes}`;
    }
    const labels = getMeridiemLabels(this.locale);
    const suffix = this.isPm ? labels.pm : labels.am;
    return `${this.displayHours}:${this.displayMinutes} ${suffix}`;
  }

  /** Whether the current parsed time is in the PM half (12h mode). */
  private get isPm(): boolean {
    return this.hours24 >= 12;
  }

  /** Hours shown to the user, respecting the active format. */
  private get displayHours(): string {
    if (this.hours24 < 0) {
      return "";
    }
    if (this.format === "24h") {
      return pad2(this.hours24);
    }
    const h12 = this.hours24 % 12;
    return pad2(h12 === 0 ? 12 : h12);
  }

  private get displayMinutes(): string {
    return this.minutes < 0 ? "" : pad2(this.minutes);
  }

  protected override render() {
    if (this.editable) {
      return this.renderField();
    }
    const is12h = this.format === "12h";
    return html`
      <div class="picker">
        <div class="fields">
          <input
            class="field hours"
            type="text"
            inputmode="numeric"
            maxlength="2"
            aria-label="Hours"
            .value=${this.displayHours}
            @input=${this.handleHoursInput}
            @blur=${this.handleHoursBlur}
            @keydown=${this.handleHoursKeydown}
          />
          <span class="separator" aria-hidden="true">:</span>
          <input
            class="field minutes"
            type="text"
            inputmode="numeric"
            maxlength="2"
            aria-label="Minutes"
            .value=${this.displayMinutes}
            @input=${this.handleMinutesInput}
            @blur=${this.handleMinutesBlur}
            @keydown=${this.handleMinutesKeydown}
          />
        </div>
        ${is12h ? this.renderAmPm() : nothing}
      </div>
    `;
  }

  private renderField(): TemplateResult {
    const placeholder = this.format === "24h" ? "HH:MM" : "HH:MM AM";
    return html`
      <div class=${classMap({ "text-field": true, invalid: this.fieldInvalid })}>
        <input
          class="text-field-input"
          type="text"
          inputmode="numeric"
          placeholder=${placeholder}
          aria-label=${this.fieldLabel}
          aria-invalid=${this.fieldInvalid ? "true" : "false"}
          .value=${this.fieldText}
          @input=${this.handleFieldInput}
          @change=${this.handleFieldChange}
        />
      </div>
    `;
  }

  private renderAmPm() {
    const labels = getMeridiemLabels(this.locale);
    const pm = this.hours24 >= 0 && this.isPm;
    const am = this.hours24 >= 0 && !this.isPm;
    return html`
      <div class="ampm" role="group" aria-label="AM or PM">
        <button
          class=${classMap({ "ampm-button": true, top: true, on: am })}
          aria-pressed=${am ? "true" : "false"}
          @click=${this.handleAmClick}
        >
          ${labels.am}
        </button>
        <button
          class=${classMap({ "ampm-button": true, bottom: true, on: pm })}
          aria-pressed=${pm ? "true" : "false"}
          @click=${this.handlePmClick}
        >
          ${labels.pm}
        </button>
      </div>
    `;
  }

  private handleFieldInput(event: Event) {
    const input = event.target as HTMLInputElement;
    this.fieldText = input.value;
    const parsed = parseTimeInput(input.value, getMeridiemLabels(this.locale));
    if (parsed) {
      this.fieldInvalid = false;
      this.setTime(parsed[0], parsed[1], false);
      this.dispatchNative("input");
    } else {
      this.fieldInvalid = input.value.trim().length > 0;
    }
  }

  private handleFieldChange() {
    if (this.fieldInvalid) {
      this.syncFieldToValue();
      return;
    }
    this.dispatchNative("change");
  }

  private handleHoursInput(event: Event) {
    const raw = (event.target as HTMLInputElement).value.replace(/\D/g, "");
    if (raw === "") {
      return;
    }
    let h = Number(raw);
    if (this.format === "24h") {
      h = clamp(h, 0, 23);
      this.setTime(h, this.minutes < 0 ? 0 : this.minutes);
    } else {
      h = clamp(h, 1, 12);
      this.setTime(this.to24(h, this.isPm), this.minutes < 0 ? 0 : this.minutes);
    }
  }

  private handleHoursBlur(event: Event) {
    (event.target as HTMLInputElement).value = this.displayHours;
  }

  private handleMinutesInput(event: Event) {
    const raw = (event.target as HTMLInputElement).value.replace(/\D/g, "");
    if (raw === "") {
      return;
    }
    const m = clamp(Number(raw), 0, 59);
    this.setTime(this.hours24 < 0 ? 0 : this.hours24, m);
  }

  private handleMinutesBlur(event: Event) {
    (event.target as HTMLInputElement).value = this.displayMinutes;
  }

  private handleHoursKeydown(event: KeyboardEvent) {
    const delta = this.arrowDelta(event);
    if (delta === 0) {
      return;
    }
    event.preventDefault();
    const base = this.hours24 < 0 ? 0 : this.hours24;
    const h = (base + delta + 24) % 24;
    this.setTime(h, this.minutes < 0 ? 0 : this.minutes);
  }

  private handleMinutesKeydown(event: KeyboardEvent) {
    const delta = this.arrowDelta(event);
    if (delta === 0) {
      return;
    }
    event.preventDefault();
    const base = this.minutes < 0 ? 0 : this.minutes;
    const m = (base + delta + 60) % 60;
    this.setTime(this.hours24 < 0 ? 0 : this.hours24, m);
  }

  private arrowDelta(event: KeyboardEvent): number {
    if (event.key === "ArrowUp") {
      return 1;
    }
    if (event.key === "ArrowDown") {
      return -1;
    }
    return 0;
  }

  private handleAmClick() {
    if (this.isPm) {
      this.setTime(this.hours24 - 12, this.minutes < 0 ? 0 : this.minutes);
    } else if (this.hours24 < 0) {
      this.setTime(0, 0);
    }
  }

  private handlePmClick() {
    if (!this.isPm) {
      const base = this.hours24 < 0 ? 0 : this.hours24;
      this.setTime(base + 12, this.minutes < 0 ? 0 : this.minutes);
    }
  }

  /** Converts a 12-hour clock hour + meridiem flag into a 0–23 hour. */
  private to24(h12: number, pm: boolean): number {
    const base = h12 % 12;
    return pm ? base + 12 : base;
  }

  /**
   * Commits a new time, updates the form value, and dispatches events. When
   * `syncField` is `false` the editable field text is left as-typed (so the
   * user's in-progress input is not clobbered mid-edit).
   */
  private setTime(hours24: number, minutes: number, syncField = true) {
    this.hours24 = clamp(hours24, 0, 23);
    this.minutes = clamp(minutes, 0, 59);
    this.value = `${pad2(this.hours24)}:${pad2(this.minutes)}`;
    this.internals.setFormValue(this.value);
    if (syncField) {
      this.syncFieldToValue();
    }
    this.dispatchEvent(
      new CustomEvent<TimePickerChangeDetail>("time-picker:change", {
        detail: { value: this.value },
        bubbles: true,
        composed: true,
      }),
    );
    if (syncField && !isServer) {
      this.dispatchNative("input");
      this.dispatchNative("change");
    }
  }

  /** Fires a native `input`/`change` event so `event.target.value` works. */
  private dispatchNative(type: "input" | "change") {
    this.dispatchEvent(new Event(type, { bubbles: true, composed: true }));
  }
}
