// SPDX-License-Identifier: Apache-2.0
import { Injectable, signal } from "@angular/core";

/**
 * Which transport `aphrody chat` uses:
 * - `agy`  : default — the Antigravity OAuth token (Google AI Ultra tier).
 * - `web`  : keyless Gemini web app (signed-in Google cookie jar).
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
   * The extra `aphrody chat` flags implied by the current backend. `agy` is the
   * CLI default and needs no flag; `web`/`stub` map to their opt-in flags.
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
    return "agy";
  }
}
