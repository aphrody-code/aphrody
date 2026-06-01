/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * Locale-aware AM/PM (day-period) labels for the time picker, resolved through
 * `Intl` with a hard EN fallback. The picker's canonical value stays a 24-hour
 * `HH:MM` string regardless of locale.
 */

/** Resolved meridiem labels for a locale. */
export interface MeridiemLabels {
  am: string;
  pm: string;
}

const EN: MeridiemLabels = { am: "AM", pm: "PM" };

const cache = new Map<string, MeridiemLabels>();

/** Resolves (and memoises) AM/PM labels for a locale, EN fallback. */
export function getMeridiemLabels(locale: string): MeridiemLabels {
  const key = locale || "en";
  const cached = cache.get(key);
  if (cached) {
    return cached;
  }
  let labels = EN;
  try {
    const fmt = new Intl.DateTimeFormat(locale || undefined, {
      hour: "numeric",
      hour12: true,
    });
    const am = dayPeriod(fmt, new Date(2021, 0, 1, 9, 0));
    const pm = dayPeriod(fmt, new Date(2021, 0, 1, 21, 0));
    if (am && pm) {
      labels = { am, pm };
    }
  } catch {
    // Intl unavailable or invalid locale — keep EN fallback.
  }
  cache.set(key, labels);
  return labels;
}

/** Extracts the day-period (`AM`/`PM`) part from a formatted time. */
function dayPeriod(fmt: Intl.DateTimeFormat, date: Date): string {
  for (const part of fmt.formatToParts(date)) {
    if (part.type === "dayPeriod") {
      return part.value;
    }
  }
  return "";
}
