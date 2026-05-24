// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, inject, signal } from "@angular/core";
import { RouterLink } from "@angular/router";
import { MatIconModule } from "@angular/material/icon";

import { AccountService } from "../../core/account.service";
import { AgentService } from "../../core/agent.service";
import { AphrodyService } from "../../core/aphrody.service";
import { SettingsService } from "../../core/settings.service";
import { parseSkillsTable } from "../skills/skills.component";

/** A control-center status card's resolved state. */
interface CardState {
  /** "ok" | "warn" | "off" | "loading" — drives the badge colour. */
  status: "ok" | "warn" | "off" | "loading";
  /** Short status text (e.g. "v2.0.2", "boucle inactive"). */
  detail: string;
}

const BACKEND_LABEL: Record<string, string> = {
  agy: "Antigravity (token agy)",
  web: "Gemini Web (cookie)",
  stub: "Hors-ligne (local)",
};

/**
 * Accueil — the control center. Every card reflects REAL state, resolved on
 * load: Antigravity IDE (`ide info`), the agy autonomous loop (`agy-loop
 * status`), Claude Code (presence of its settings), the chat backend + signed-in
 * account, and the plugin surface (MCP tool count via `mcp list`, skills count
 * via the sibling). Quick actions deep-link into the relevant Settings tab.
 */
@Component({
  selector: "app-dashboard",
  imports: [RouterLink, MatIconModule],
  template: `
    <div class="page">
      <header class="head">
        <div>
          <h1>Accueil</h1>
          <p>Centre de contrôle — état réel des agents, du compte et des extensions.</p>
        </div>
        <button class="refresh" (click)="loadAll()" [disabled]="anyLoading()" aria-label="Tout actualiser">
          <mat-icon class="material-symbols-outlined" [class.spin]="anyLoading()">refresh</mat-icon>
        </button>
      </header>

      <!-- Account banner -->
      <section class="banner">
        @if (account.account(); as a) {
          @if (a.connected) {
            <div class="avatar">{{ a.initials }}</div>
            <div class="banner-text">
              <b>{{ a.name }}</b>
              <span>{{ a.email }}</span>
            </div>
            <span class="badge ok"><mat-icon class="material-symbols-outlined">check_circle</mat-icon>Connecté</span>
          } @else {
            <div class="avatar off"><mat-icon class="material-symbols-outlined">person_off</mat-icon></div>
            <div class="banner-text">
              <b>Non connecté</b>
              <span>{{ account.error() || "Aucune session Google active." }}</span>
            </div>
          }
        }
        <a class="banner-action" routerLink="/settings" aria-label="Paramètres du compte">
          <mat-icon class="material-symbols-outlined">settings</mat-icon>
        </a>
      </section>

      <h2 class="section-title">Agents</h2>
      <div class="cards">
        <div class="card">
          <div class="card-head">
            <mat-icon class="material-symbols-outlined">rocket</mat-icon>
            <span>Antigravity IDE</span>
            <span class="dot" [attr.data-s]="antigravity().status"></span>
          </div>
          <p class="card-detail">{{ antigravity().detail }}</p>
          @if (antigravityPath()) {
            <p class="card-path" [title]="antigravityPath()">{{ antigravityPath() }}</p>
          }
          <a class="quick" routerLink="/settings" [queryParams]="{ tab: 'agents' }">Configurer</a>
        </div>

        <div class="card">
          <div class="card-head">
            <mat-icon class="material-symbols-outlined">all_inclusive</mat-icon>
            <span>Boucle autonome (agy)</span>
            <span class="dot" [attr.data-s]="agyLoop().status"></span>
          </div>
          <p class="card-detail">{{ agyLoop().detail }}</p>
          <a class="quick" routerLink="/settings" [queryParams]="{ tab: 'actions' }">Piloter</a>
        </div>

        <div class="card">
          <div class="card-head">
            <mat-icon class="material-symbols-outlined">smart_toy</mat-icon>
            <span>Claude Code</span>
            <span class="dot" [attr.data-s]="claude().status"></span>
          </div>
          <p class="card-detail">{{ claude().detail }}</p>
          <a class="quick" routerLink="/settings" [queryParams]="{ tab: 'agents' }">Configurer</a>
        </div>

        <div class="card">
          <div class="card-head">
            <mat-icon class="material-symbols-outlined">forum</mat-icon>
            <span>Backend de conversation</span>
            <span class="dot" data-s="ok"></span>
          </div>
          <p class="card-detail">{{ backendLabel() }}</p>
          <a class="quick" routerLink="/settings" [queryParams]="{ tab: 'conversation' }">Changer</a>
        </div>
      </div>

      <h2 class="section-title">Extensions et plugins</h2>
      <div class="cards">
        <div class="card">
          <div class="card-head">
            <mat-icon class="material-symbols-outlined">hub</mat-icon>
            <span>Serveur MCP</span>
            <span class="dot" [attr.data-s]="mcp().status"></span>
          </div>
          <p class="card-detail">{{ mcp().detail }}</p>
          <a class="quick" routerLink="/mcp">Voir les outils</a>
        </div>

        <div class="card">
          <div class="card-head">
            <mat-icon class="material-symbols-outlined">extension</mat-icon>
            <span>Skills</span>
            <span class="dot" [attr.data-s]="skills().status"></span>
          </div>
          <p class="card-detail">{{ skills().detail }}</p>
          <a class="quick" routerLink="/skills">Parcourir</a>
        </div>
      </div>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
        height: 100%;
        overflow-y: auto;
      }
      .page {
        max-width: 960px;
        margin: 0 auto;
        padding: 28px 28px 64px;
      }
      .head {
        display: flex;
        align-items: center;
        gap: 16px;
        margin-bottom: 20px;
      }
      .head h1 {
        margin: 0;
        font-size: 26px;
        font-weight: 400;
      }
      .head p {
        margin: 2px 0 0;
        font-size: 13px;
        color: var(--mat-sys-on-surface-variant);
      }
      .refresh {
        margin-left: auto;
        width: 40px;
        height: 40px;
        border: none;
        border-radius: 50%;
        background: var(--mat-sys-surface-container-high);
        color: var(--mat-sys-on-surface-variant);
        cursor: pointer;
        display: grid;
        place-items: center;
      }
      .refresh:disabled {
        opacity: 0.6;
      }
      .spin {
        animation: db-spin 1s linear infinite;
      }
      @keyframes db-spin {
        to {
          transform: rotate(360deg);
        }
      }
      .banner {
        display: flex;
        align-items: center;
        gap: 14px;
        padding: 16px 18px;
        border-radius: 18px;
        background: var(--mat-sys-surface-container);
        margin-bottom: 24px;
      }
      .avatar {
        width: 46px;
        height: 46px;
        border-radius: 50%;
        display: grid;
        place-items: center;
        font-weight: 600;
        font-size: 17px;
        color: var(--mat-sys-on-primary-container);
        background: var(--mat-sys-primary-container);
        flex: 0 0 auto;
      }
      .avatar.off {
        color: var(--mat-sys-on-surface-variant);
        background: var(--mat-sys-surface-container-highest);
      }
      .banner-text {
        display: flex;
        flex-direction: column;
        flex: 1 1 auto;
        min-width: 0;
      }
      .banner-text b {
        font-size: 15px;
      }
      .banner-text span {
        font-size: 13px;
        color: var(--mat-sys-on-surface-variant);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .badge {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        font-size: 12px;
        padding: 4px 10px;
        border-radius: 999px;
      }
      .badge.ok {
        color: var(--mat-sys-tertiary);
        background: color-mix(in srgb, var(--mat-sys-tertiary) 16%, transparent);
      }
      .badge mat-icon {
        font-size: 16px;
        width: 16px;
        height: 16px;
      }
      .banner-action {
        flex: 0 0 auto;
        width: 38px;
        height: 38px;
        border-radius: 50%;
        display: grid;
        place-items: center;
        color: var(--mat-sys-on-surface-variant);
        text-decoration: none;
      }
      .banner-action:hover {
        background: color-mix(in srgb, var(--mat-sys-on-surface) 8%, transparent);
      }
      .section-title {
        font-size: 14px;
        font-weight: 500;
        color: var(--mat-sys-on-surface-variant);
        margin: 0 0 12px;
      }
      .cards {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
        gap: 12px;
        margin-bottom: 28px;
      }
      .card {
        display: flex;
        flex-direction: column;
        padding: 16px;
        border-radius: 16px;
        background: var(--mat-sys-surface-container-low);
      }
      .card-head {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 10px;
      }
      .card-head > mat-icon {
        color: var(--mat-sys-primary);
        font-size: 22px;
        width: 22px;
        height: 22px;
      }
      .card-head > span:not(.dot) {
        font-size: 14px;
        font-weight: 500;
        flex: 1 1 auto;
      }
      .dot {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        flex: 0 0 auto;
        background: var(--mat-sys-outline);
      }
      .dot[data-s="ok"] {
        background: #34a853;
      }
      .dot[data-s="warn"] {
        background: #fbbc04;
      }
      .dot[data-s="off"] {
        background: var(--mat-sys-outline);
      }
      .dot[data-s="loading"] {
        background: var(--mat-sys-primary);
        animation: db-spin 1s linear infinite;
      }
      .card-detail {
        margin: 0;
        font-size: 13px;
        color: var(--mat-sys-on-surface);
        flex: 1 1 auto;
      }
      .card-path {
        margin: 6px 0 0;
        font-size: 11px;
        font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
        color: var(--mat-sys-on-surface-variant);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .quick {
        margin-top: 12px;
        align-self: flex-start;
        font-size: 13px;
        color: var(--mat-sys-primary);
        text-decoration: none;
        padding: 6px 12px;
        border-radius: 999px;
        background: color-mix(in srgb, var(--mat-sys-primary) 10%, transparent);
      }
      .quick:hover {
        background: color-mix(in srgb, var(--mat-sys-primary) 18%, transparent);
      }
    `,
  ],
})
export class DashboardComponent implements OnInit {
  readonly account = inject(AccountService);
  private readonly aphrody = inject(AphrodyService);
  private readonly agent = inject(AgentService);
  private readonly settings = inject(SettingsService);

  readonly antigravity = signal<CardState>({ status: "loading", detail: "Détection…" });
  readonly antigravityPath = signal("");
  readonly agyLoop = signal<CardState>({ status: "loading", detail: "Détection…" });
  readonly claude = signal<CardState>({ status: "loading", detail: "Détection…" });
  readonly mcp = signal<CardState>({ status: "loading", detail: "Détection…" });
  readonly skills = signal<CardState>({ status: "loading", detail: "Détection…" });

  backendLabel(): string {
    return BACKEND_LABEL[this.settings.backend()] ?? this.settings.backend();
  }

  anyLoading(): boolean {
    return [this.antigravity(), this.agyLoop(), this.claude(), this.mcp(), this.skills()].some(
      (c) => c.status === "loading",
    );
  }

  ngOnInit(): void {
    void this.account.refresh();
    void this.loadAll();
  }

  loadAll(): void {
    this.antigravity.set({ status: "loading", detail: "Détection…" });
    this.agyLoop.set({ status: "loading", detail: "Détection…" });
    this.claude.set({ status: "loading", detail: "Détection…" });
    this.mcp.set({ status: "loading", detail: "Détection…" });
    this.skills.set({ status: "loading", detail: "Détection…" });
    void this.loadAntigravity();
    void this.loadAgyLoop();
    void this.loadClaude();
    void this.loadMcp();
    void this.loadSkills();
  }

  private async loadAntigravity(): Promise<void> {
    try {
      const res = await this.aphrody.exec(["ide", "info"]);
      const out = res.stdout.trim();
      if (res.code === 0 && out.startsWith("{")) {
        const j = JSON.parse(out) as { ideVersion?: string; nameLong?: string; paths?: { root?: string } };
        this.antigravity.set({ status: "ok", detail: `Installé — v${j.ideVersion ?? "?"}` });
        this.antigravityPath.set(j.paths?.root ?? "");
      } else {
        this.antigravity.set({ status: "off", detail: "Non détecté sur ce poste." });
      }
    } catch {
      this.antigravity.set({ status: "off", detail: "Détection impossible." });
    }
  }

  private async loadAgyLoop(): Promise<void> {
    try {
      const res = await this.aphrody.exec(["agy-loop", "status"]);
      const out = (res.stdout || res.stderr).trim();
      // status emits text like "(boucle inactive …)" or a JSON/armed line.
      const inactive = /inactive/i.test(out);
      this.agyLoop.set({
        status: inactive ? "off" : "ok",
        detail: inactive ? "Boucle inactive" : out.slice(0, 80) || "Boucle armée",
      });
    } catch {
      this.agyLoop.set({ status: "off", detail: "État indisponible." });
    }
  }

  private async loadClaude(): Promise<void> {
    if (!this.agent.isTauri) {
      this.claude.set({ status: "warn", detail: "Détection requiert l'app de bureau." });
      return;
    }
    try {
      const doc = await this.agent.readAgentConfig("claude");
      this.claude.set(
        doc.exists
          ? { status: "ok", detail: "Configuration détectée." }
          : { status: "off", detail: "Aucune configuration trouvée." },
      );
    } catch {
      this.claude.set({ status: "off", detail: "Détection impossible." });
    }
  }

  private async loadMcp(): Promise<void> {
    try {
      const res = await this.aphrody.exec(["mcp", "list"]);
      const out = res.stdout.trim();
      if (res.code === 0 && out.startsWith("{")) {
        const j = JSON.parse(out) as { tools?: unknown[] };
        const n = j.tools?.length ?? 0;
        this.mcp.set({ status: "ok", detail: `${n} outil(s) exposé(s).` });
      } else {
        this.mcp.set({ status: "warn", detail: "Serveur MCP injoignable." });
      }
    } catch {
      this.mcp.set({ status: "warn", detail: "Serveur MCP injoignable." });
    }
  }

  private async loadSkills(): Promise<void> {
    if (!this.agent.isTauri) {
      this.skills.set({ status: "warn", detail: "Catalogue requiert l'app de bureau." });
      return;
    }
    try {
      const res = await this.agent.siblingExec("aphrody-skills", ["list"]);
      const list = parseSkillsTable(res.stdout);
      this.skills.set(
        list.length > 0
          ? { status: "ok", detail: `${list.length} skill(s) découvert(s).` }
          : { status: "warn", detail: "Catalogue vide." },
      );
    } catch {
      this.skills.set({ status: "warn", detail: "Catalogue indisponible." });
    }
  }
}
