// SPDX-License-Identifier: Apache-2.0
import { Injectable, signal } from "@angular/core";

const STORAGE_KEY = "aphrody.theme";
type ThemeMode = "dark" | "light";

/**
 * Light/dark theme toggle. Flips the `aphrody-light` class on the document
 * element; `styles.scss` re-runs `mat.theme()` + the Gemini palette under that
 * selector, so every Material component switches palette live. The choice is
 * persisted across launches.
 */
@Injectable({ providedIn: "root" })
export class ThemeService {
  /** Reactive current mode for templates. */
  readonly mode = signal<ThemeMode>("dark");

  constructor() {
    const stored = this.readStored();
    this.apply(stored);
  }

  toggle(): void {
    this.apply(this.mode() === "dark" ? "light" : "dark");
  }

  private apply(mode: ThemeMode): void {
    this.mode.set(mode);
    const root = document.documentElement;
    root.classList.toggle("aphrody-light", mode === "light");
    try {
      localStorage.setItem(STORAGE_KEY, mode);
    } catch {
      // storage may be unavailable (private mode) — non-fatal
    }
  }

  private readStored(): ThemeMode {
    try {
      const v = localStorage.getItem(STORAGE_KEY);
      if (v === "light" || v === "dark") {
        return v;
      }
    } catch {
      // ignore
    }
    return "dark";
  }
}
