// SPDX-License-Identifier: Apache-2.0
import { Injectable, signal } from "@angular/core";

/**
 * Which transport `aphrody chat` uses:
 * - `web`  : DEFAULT — keyless Gemini web app (signed-in Google cookie jar).
 *   This is the only transport that serves real Gemini 3.5 Flash. When the
 *   cookie jar is absent the CLI falls back to the agy token for the turn.
 * - `agy`  : the Antigravity OAuth token (Google AI Ultra tier). Its Cloud Code
 *   backend does NOT serve gemini-3.5-flash (404) -- it serves gemini-2.5-flash.
 * - `stub` : offline deterministic reply (no network, no auth).
 */
export type ChatBackend = "agy" | "web" | "stub";

const STORAGE_KEY = "aphrody.backend";

/**
 * User-facing settings that change real CLI behaviour. The chat backend choice
 * is persisted and consumed by the Assistant when it builds the
 * `aphrody chat …` argv (see {@link extraChatArgs}), so the setting is genuinely
 * functional rather than cosmetic.
 */
@Injectable({ providedIn: "root" })
export class SettingsService {
  /** Selected conversation backend (reactive, persisted). */
  readonly backend = signal<ChatBackend>(this.readStored());

  /** Persist + apply a backend choice. */
  setBackend(b: ChatBackend): void {
    this.backend.set(b);
    try {
      localStorage.setItem(STORAGE_KEY, b);
    } catch {
      // storage unavailable (private mode) — non-fatal, in-memory only
    }
  }

  /**
   * The extra `aphrody chat` flags implied by the current backend. `web` (the
   * default, real Gemini 3.5 Flash) and `stub` map to their flags; `agy` is the
   * CLI's own default backend and needs no flag.
   */
  extraChatArgs(): string[] {
    switch (this.backend()) {
      case "web":
        return ["--web"];
      case "stub":
        return ["--stub"];
      default:
        return [];
    }
  }

  private readStored(): ChatBackend {
    try {
      const v = localStorage.getItem(STORAGE_KEY);
      if (v === "agy" || v === "web" || v === "stub") {
        return v;
      }
    } catch {
      // ignore
    }
    // Default to the web transport: it is the one that serves real Gemini 3.5
    // Flash. The CLI gracefully falls back to the agy token when the Google
    // cookie jar is absent, so this default never leaves the chat broken.
    return "web";
  }
}
