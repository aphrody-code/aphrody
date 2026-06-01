/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Scheduler } from "./internal/scheduler.js";
import { styles } from "./internal/scheduler-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-scheduler": MdScheduler;
  }
}

/**
 * @summary A scheduler / calendar that renders day, week, and month views with
 * navigation and time-positioned events.
 *
 * @description
 * Set `view` to `day`, `week`, or `month`, drive the visible range with the
 * `date` attribute (local `YYYY-MM-DD`), and pass `events`
 * (`{id, start, end, title, color?}[]`, with `start`/`end` as `Date` or ISO
 * strings) in JS. Prev/next/today navigation and view switching are built in.
 *
 * Selecting an empty slot fires `scheduler:select`, clicking an event fires
 * `scheduler:event-click`, and navigating fires `scheduler:navigate` — all
 * `{bubbles, composed}`.
 *
 * This is a Community baseline. Drag-and-drop, event resize, recurrence, and
 * resource/timeline rows (MUI X Pro/Premium features) are out of scope.
 *
 * ```html
 * <md-scheduler view="week"></md-scheduler>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-scheduler")
export class MdScheduler extends Scheduler {
  static override styles: CSSResultOrNative[] = [styles];
}
