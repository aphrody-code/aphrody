// SPDX-License-Identifier: Apache-2.0
import { Injectable, inject, signal } from "@angular/core";

import { AphrodyService } from "./aphrody.service";

/** The signed-in Google identity, as resolved from aphrody's auth at runtime. */
export interface Account {
  /** True once a Google profile was resolved from the live session. */
  connected: boolean;
  /** Google account e-mail (empty when disconnected). */
  email: string;
  /** Display name (falls back to the e-mail local part). */
  name: string;
  /** 1-2 letter monogram for the avatar (computed locally, never remote). */
  initials: string;
}

const DISCONNECTED: Account = { connected: false, email: "", name: "", initials: "" };

/**
 * The Google account linked to aphrody, resolved LIVE from `aphrody auth` — i.e.
 * `aphrody antigravity whoami` (Google OpenID userinfo, using the agy token /
 * Google cookie). Nothing personal is ever hard-coded or persisted: the identity
 * is fetched at runtime and held only in memory. The avatar is rendered as
 * locally-computed initials — the remote `picture` URL is deliberately NOT
 * loaded, so the app issues zero third-party request and stays fully offline.
 */
@Injectable({ providedIn: "root" })
export class AccountService {
  private readonly aphrody = inject(AphrodyService);

  /** Current account (reactive). Starts disconnected until {@link refresh}. */
  readonly account = signal<Account>(DISCONNECTED);
  /** True while a `whoami` round-trip is in flight. */
  readonly loading = signal(false);
  /** Last non-fatal error message (e.g. expired session), for the UI. */
  readonly error = signal<string>("");

  /** Re-query the live session. Safe to call repeatedly; never throws. */
  async refresh(): Promise<void> {
    this.loading.set(true);
    this.error.set("");
    try {
      const res = await this.aphrody.exec(["antigravity", "whoami"]);
      const out = res.stdout.trim();
      if (res.code === 0 && out.startsWith("{")) {
        const j = JSON.parse(out) as {
          email?: string;
          name?: string;
          given_name?: string;
          family_name?: string;
        };
        const name = (j.name ?? [j.given_name, j.family_name].filter(Boolean).join(" ")).trim();
        const email = (j.email ?? "").trim();
        this.account.set({
          connected: email.length > 0,
          email,
          name: name || email,
          initials: this.monogram(name, email),
        });
      } else {
        this.account.set(DISCONNECTED);
        this.error.set("Session Google indisponible. Reconnectez aphrody (agy).");
      }
    } catch {
      this.account.set(DISCONNECTED);
      this.error.set("Impossible de lire le compte (backend hors-ligne).");
    } finally {
      this.loading.set(false);
    }
  }

  /** First letters of the name (or e-mail), upper-cased. */
  private monogram(name: string, email: string): string {
    const src = name || email;
    if (!src) return "?";
    const parts = src.split(/[\s@._-]+/).filter(Boolean);
    const a = parts[0]?.[0] ?? "";
    const b = parts.length > 1 ? (parts[1]?.[0] ?? "") : "";
    return (a + b).toUpperCase() || "?";
  }
}
