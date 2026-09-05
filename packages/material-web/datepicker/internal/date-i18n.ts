/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * Locale-aware date formatting/parsing helpers shared by the date, date-range
 * and date-time pickers. Built on the platform `Intl` APIs so month names,
 * weekday headers and the display format follow the active `locale`, with a
 * hard EN fallback baked in (used on platforms without full ICU data and as the
 * canonical parse target).
 */

import { fromISO, pad2 } from "./date-picker.js";

/** English month names — canonical fallback. */
const EN_MONTHS = [
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

/** English single-letter weekday headers (Sunday-first) — canonical fallback. */
const EN_WEEKDAYS = ["S", "M", "T", "W", "T", "F", "S"] as const;

/** A resolved bundle of localized display strings for one locale. */
export interface DateLocaleData {
  /** Full month names, January-first (length 12). */
  months: readonly string[];
  /** Narrow weekday headers, Sunday-first (length 7). */
  weekdays: readonly string[];
}

const cache = new Map<string, DateLocaleData>();

/** Resolves (and memoises) month/weekday labels for a locale, EN fallback. */
export function getDateLocaleData(locale: string): DateLocaleData {
  const key = locale || "en";
  const cached = cache.get(key);
  if (cached) {
    return cached;
  }
  let data: DateLocaleData = { months: EN_MONTHS, weekdays: EN_WEEKDAYS };
  try {
    const monthFmt = new Intl.DateTimeFormat(locale || undefined, {
      month: "long",
    });
    const weekdayFmt = new Intl.DateTimeFormat(locale || undefined, {
      weekday: "narrow",
    });
    const months: string[] = [];
    for (let m = 0; m < 12; m++) {
      // Use a fixed mid-month day in a non-DST-sensitive year.
      months.push(monthFmt.format(new Date(2021, m, 15)));
    }
    const weekdays: string[] = [];
    // 2021-08-01 is a Sunday — walk a full week from there.
    for (let d = 1; d <= 7; d++) {
      weekdays.push(weekdayFmt.format(new Date(2021, 7, d)));
    }
    data = { months, weekdays };
  } catch {
    // Intl unavailable or invalid locale — keep EN fallback.
  }
  cache.set(key, data);
  return data;
}

/** A long month name for the given 0-based month index in `locale`. */
export function monthName(locale: string, month: number): string {
  return getDateLocaleData(locale).months[month] ?? EN_MONTHS[month];
}

/**
 * Formats a `Date` for the editable field. When an explicit `format` mask is
 * given it is honoured verbatim; otherwise the locale's short numeric date
 * format (via `Intl`) is used, falling back to `MM/DD/YYYY`.
 */
export function formatDisplayLocale(date: Date, locale: string, format: string): string {
  if (format) {
    return applyMask(date, format);
  }
  try {
    return new Intl.DateTimeFormat(locale || undefined, {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    }).format(date);
  } catch {
    return applyMask(date, "MM/DD/YYYY");
  }
}

/** A locale-appropriate placeholder mask, e.g. `MM/DD/YYYY`. */
export function placeholderMask(locale: string, format: string): string {
  if (format) {
    return format;
  }
  try {
    const parts = new Intl.DateTimeFormat(locale || undefined, {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    }).formatToParts(new Date(2021, 11, 31));
    let mask = "";
    for (const part of parts) {
      if (part.type === "year") {
        mask += "YYYY";
      } else if (part.type === "month") {
        mask += "MM";
      } else if (part.type === "day") {
        mask += "DD";
      } else if (part.type === "literal") {
        mask += part.value;
      }
    }
    return mask || "MM/DD/YYYY";
  } catch {
    return "MM/DD/YYYY";
  }
}

/** Applies a `YYYY`/`MM`/`DD` mask to a `Date`. */
function applyMask(date: Date, format: string): string {
  return format
    .replace(/YYYY/g, String(date.getFullYear()))
    .replace(/MM/g, pad2(date.getMonth() + 1))
    .replace(/DD/g, pad2(date.getDate()));
}

/**
 * Parses a user-typed value robustly. Accepts ISO `YYYY-MM-DD`, the explicit
 * `format` mask (when supplied), and the locale's numeric order inferred from
 * `Intl`. Falls back to the common `M/D/Y` and `D/M/Y` numeric forms so input
 * never silently fails across locales.
 */
export function parseDisplayLocale(value: string, locale: string, format: string): Date | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  const iso = fromISO(trimmed);
  if (iso) {
    return iso;
  }
  const order = format ? maskFieldOrder(format) : localeFieldOrder(locale);
  const nums = trimmed.match(/\d+/g);
  if (!nums || nums.length < 3) {
    return null;
  }
  const parsed = buildFromOrder(nums, order);
  if (parsed) {
    return parsed;
  }
  // Last-resort: assume month-first then day-first.
  return buildFromOrder(nums, ["M", "D", "Y"]) ?? buildFromOrder(nums, ["D", "M", "Y"]);
}

/** The numeric field order (e.g. `['M','D','Y']`) implied by a mask. */
function maskFieldOrder(format: string): Array<"Y" | "M" | "D"> {
  const order: Array<"Y" | "M" | "D"> = [];
  const re = /YYYY|MM|DD/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(format)) !== null) {
    order.push(match[0][0] as "Y" | "M" | "D");
  }
  return order.length === 3 ? order : ["M", "D", "Y"];
}

/** The numeric field order inferred from a locale's short date format. */
function localeFieldOrder(locale: string): Array<"Y" | "M" | "D"> {
  try {
    const parts = new Intl.DateTimeFormat(locale || undefined, {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    }).formatToParts(new Date(2021, 11, 31));
    const order: Array<"Y" | "M" | "D"> = [];
    for (const part of parts) {
      if (part.type === "year") {
        order.push("Y");
      } else if (part.type === "month") {
        order.push("M");
      } else if (part.type === "day") {
        order.push("D");
      }
    }
    return order.length === 3 ? order : ["M", "D", "Y"];
  } catch {
    return ["M", "D", "Y"];
  }
}

/** Builds a valid `Date` from numeric tokens in the given field order. */
function buildFromOrder(nums: string[], order: Array<"Y" | "M" | "D">): Date | null {
  let year = -1;
  let month = -1;
  let day = -1;
  for (let i = 0; i < 3; i++) {
    const n = Number(nums[i]);
    if (order[i] === "Y") {
      year = n;
    } else if (order[i] === "M") {
      month = n - 1;
    } else {
      day = n;
    }
  }
  if (year < 0 || month < 0 || day < 0 || year < 100) {
    return null;
  }
  const date = new Date(year, month, day);
  if (date.getFullYear() !== year || date.getMonth() !== month || date.getDate() !== day) {
    return null;
  }
  return date;
}
