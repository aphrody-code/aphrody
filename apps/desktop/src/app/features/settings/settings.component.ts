// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, inject, signal } from "@angular/core";
import { ActivatedRoute, Router } from "@angular/router";
import { MatIconModule } from "@angular/material/icon";
import { MatTabsModule } from "@angular/material/tabs";

import { AccountService } from "../../core/account.service";
import { AphrodyService, Meta } from "../../core/aphrody.service";
import { ChatBackend, SettingsService } from "../../core/settings.service";
import { ThemeService } from "../../core/theme.service";
import { ActionsTabComponent } from "./tabs/actions-tab.component";
import { AgentsTabComponent } from "./tabs/agents-tab.component";
import { ChannelsTabComponent } from "./tabs/channels-tab.component";
import { MemoryTabComponent } from "./tabs/memory-tab.component";
import { PersonaTabComponent } from "./tabs/persona-tab.component";

interface BackendOption {
  id: ChatBackend;
  label: string;
  desc: string;
  icon: string;
}

/** Ordered tab ids, used to map the `?tab=` query param to a tab index. */
const TAB_IDS = [
  "compte",
  "apparence",
  "conversation",
  "memoire",
  "soul",
  "identity",
  "channels",
  "actions",
  "agents",
  "apropos",
] as const;
type TabId = (typeof TAB_IDS)[number];

/**
 * Settings page, organised into Material tabs. Keeps the original Compte /
 * Apparence / Conversation / À propos content and adds the control-center tabs:
 * Mémoire, Soul, Identity, Channels, Actions, Agents — each wired to a real
 * aphrody command or a scoped, path-allowlisted Tauri command. The active tab is
 * driven by the `?tab=` query param so the Dashboard can deep-link into it.
 */
@Component({
  selector: "app-settings",
  imports: [
    MatIconModule,
    MatTabsModule,
    MemoryTabComponent,
    PersonaTabComponent,
    ChannelsTabComponent,
    ActionsTabComponent,
    AgentsTabComponent,
  ],
  template: `
    <div class="settings">
      <h1 class="page-title">Paramètres</h1>

      <mat-tab-group
        [selectedIndex]="selectedIndex()"
        (selectedIndexChange)="onTabChange($event)"
        animationDuration="180ms"
        mat-stretch-tabs="false"
      >
        <!-- ── Compte ──────────────────────────────────────────────────── -->
        <mat-tab label="Compte">
          <section class="card">
            <h2>Compte</h2>
            @if (account.account(); as a) {
              @if (a.connected) {
                <div class="account-row">
                  <div class="avatar">{{ a.initials }}</div>
                  <div class="who">
                    <b>{{ a.name }}</b>
                    <span>{{ a.email }}</span>
                  </div>
                  <span class="badge ok">
                    <mat-icon class="material-symbols-outlined">check_circle</mat-icon>
                    Connecté
                  </span>
                </div>
                <p class="hint">
                  Compte Google lié à aphrody (token agy / cookie Google). Les conversations
                  utilisent ce compte via Gemini.
                </p>
              } @else {
                <div class="account-row">
                  <div class="avatar disconnected">
                    <mat-icon class="material-symbols-outlined">person_off</mat-icon>
                  </div>
                  <div class="who">
                    <b>Non connecté</b>
                    <span>{{ account.error() || "Aucune session Google active." }}</span>
                  </div>
                </div>
                <p class="hint">
                  Lancez l'authentification Antigravity (agy) en arrière-plan pour régénérer la
                  session, puis actualisez.
                </p>
              }
            }
            <button class="btn" (click)="refreshAccount()" [disabled]="account.loading()">
              <mat-icon class="material-symbols-outlined">{{ account.loading() ? "progress_activity" : "refresh" }}</mat-icon>
              {{ account.loading() ? "Vérification…" : "Actualiser le compte" }}
            </button>
          </section>
        </mat-tab>

        <!-- ── Apparence ───────────────────────────────────────────────── -->
        <mat-tab label="Apparence">
          <section class="card">
            <h2>Apparence</h2>
            <p class="row-label">Thème</p>
            <div class="seg">
              <button class="seg-btn" [class.sel]="theme.mode() === 'dark'" (click)="setTheme('dark')">
                <mat-icon class="material-symbols-outlined">dark_mode</mat-icon> Sombre
              </button>
              <button class="seg-btn" [class.sel]="theme.mode() === 'light'" (click)="setTheme('light')">
                <mat-icon class="material-symbols-outlined">light_mode</mat-icon> Clair
              </button>
            </div>
          </section>
        </mat-tab>

        <!-- ── Conversation ────────────────────────────────────────────── -->
        <mat-tab label="Conversation">
          <section class="card">
            <h2>Conversation</h2>
            <p class="row-label">Backend Gemini</p>
            <div class="options">
              @for (o of backends; track o.id) {
                <button class="option" [class.sel]="settings.backend() === o.id" (click)="settings.setBackend(o.id)">
                  <mat-icon class="material-symbols-outlined">{{ o.icon }}</mat-icon>
                  <span class="opt-text"><b>{{ o.label }}</b><span>{{ o.desc }}</span></span>
                  @if (settings.backend() === o.id) {
                    <mat-icon class="material-symbols-outlined check">check</mat-icon>
                  }
                </button>
              }
            </div>
          </section>
        </mat-tab>

        <!-- ── Mémoire ─────────────────────────────────────────────────── -->
        <mat-tab label="Mémoire">
          <app-memory-tab />
        </mat-tab>

        <!-- ── Soul ────────────────────────────────────────────────────── -->
        <mat-tab label="Âme">
          <app-persona-tab which="soul" />
        </mat-tab>

        <!-- ── Identity ────────────────────────────────────────────────── -->
        <mat-tab label="Identité">
          <app-persona-tab which="identity" />
        </mat-tab>

        <!-- ── Channels ────────────────────────────────────────────────── -->
        <mat-tab label="Canaux">
          <app-channels-tab />
        </mat-tab>

        <!-- ── Actions ─────────────────────────────────────────────────── -->
        <mat-tab label="Actions">
          <app-actions-tab />
        </mat-tab>

        <!-- ── Agents ──────────────────────────────────────────────────── -->
        <mat-tab label="Agents">
          <app-agents-tab />
        </mat-tab>

        <!-- ── À propos ────────────────────────────────────────────────── -->
        <mat-tab label="À propos">
          <section class="card">
            <h2>À propos</h2>
            <div class="about-head">
              <img class="logo" src="assets/aphrody.webp" alt="aphrody" />
              <div>
                <b>aphrody</b>
                <p class="tagline">Agent autonome — assistant IA propulsé par Gemini.</p>
              </div>
            </div>
            @if (meta(); as m) {
              <div class="grid">
                <div class="kv"><span>Version</span><b>{{ m.app_version }}</b></div>
                <div class="kv"><span>Système</span><b>{{ m.target_os }}</b></div>
                <div class="kv"><span>Architecture</span><b>{{ m.target_arch }}</b></div>
                <div class="kv"><span>Famille</span><b>{{ m.family }}</b></div>
              </div>
            }
            <p class="note">Angular 21 + Angular Material 21 dans une coque Tauri, propulsé en local par le binaire aphrody.</p>
          </section>
        </mat-tab>
      </mat-tab-group>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
        height: 100%;
        overflow-y: auto;
      }
      .settings {
        max-width: 720px;
        margin: 0 auto;
        padding: 36px 28px 64px;
      }
      .page-title {
        font-size: 28px;
        font-weight: 400;
        margin: 0 0 18px;
        letter-spacing: -0.3px;
      }
      ::ng-deep .mat-mdc-tab-body-wrapper {
        padding-top: 18px;
      }
      .card {
        background: var(--mat-sys-surface-container);
        border-radius: 20px;
        padding: 20px 22px;
        margin-bottom: 16px;
      }
      .card h2 {
        font-size: 16px;
        font-weight: 500;
        margin: 0 0 14px;
        color: var(--mat-sys-on-surface);
      }
      /* account */
      .account-row {
        display: flex;
        align-items: center;
        gap: 14px;
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
      .avatar.disconnected {
        color: var(--mat-sys-on-surface-variant);
        background: var(--mat-sys-surface-container-highest);
      }
      .who {
        display: flex;
        flex-direction: column;
        min-width: 0;
        flex: 1 1 auto;
      }
      .who b {
        font-size: 15px;
      }
      .who span {
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
        flex: 0 0 auto;
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
      .hint {
        margin: 12px 0 0;
        font-size: 13px;
        line-height: 1.5;
        color: var(--mat-sys-on-surface-variant);
      }
      .btn {
        margin-top: 16px;
        display: inline-flex;
        align-items: center;
        gap: 8px;
        border: none;
        border-radius: 999px;
        padding: 9px 18px;
        font: inherit;
        font-size: 14px;
        cursor: pointer;
        color: var(--mat-sys-on-secondary-container);
        background: var(--mat-sys-secondary-container);
      }
      .btn:hover {
        filter: brightness(1.08);
      }
      .btn:disabled {
        opacity: 0.6;
        cursor: default;
      }
      .btn mat-icon {
        font-size: 18px;
        width: 18px;
        height: 18px;
      }
      /* segmented (theme) */
      .row-label {
        margin: 0 0 10px;
        font-size: 13px;
        color: var(--mat-sys-on-surface-variant);
      }
      .seg {
        display: inline-flex;
        gap: 6px;
        background: var(--mat-sys-surface-container-high);
        padding: 4px;
        border-radius: 999px;
      }
      .seg-btn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        border: none;
        background: transparent;
        border-radius: 999px;
        padding: 8px 16px;
        font: inherit;
        font-size: 14px;
        color: var(--mat-sys-on-surface-variant);
        cursor: pointer;
      }
      .seg-btn.sel {
        background: var(--mat-sys-secondary-container);
        color: var(--mat-sys-on-secondary-container);
      }
      .seg-btn mat-icon {
        font-size: 18px;
        width: 18px;
        height: 18px;
      }
      /* backend options */
      .options {
        display: flex;
        flex-direction: column;
        gap: 8px;
      }
      .option {
        display: flex;
        align-items: center;
        gap: 14px;
        text-align: left;
        border: 1px solid var(--mat-sys-outline-variant);
        background: transparent;
        border-radius: 14px;
        padding: 14px 16px;
        font: inherit;
        cursor: pointer;
        color: var(--mat-sys-on-surface);
        transition: border-color 0.15s, background 0.15s;
      }
      .option:hover {
        background: color-mix(in srgb, var(--mat-sys-on-surface) 5%, transparent);
      }
      .option.sel {
        border-color: var(--mat-sys-primary);
        background: color-mix(in srgb, var(--mat-sys-primary) 10%, transparent);
      }
      .opt-text {
        display: flex;
        flex-direction: column;
        flex: 1 1 auto;
      }
      .opt-text b {
        font-size: 14px;
      }
      .opt-text span {
        font-size: 12px;
        color: var(--mat-sys-on-surface-variant);
      }
      .option > mat-icon {
        color: var(--mat-sys-on-surface-variant);
      }
      .option.sel > mat-icon:first-child {
        color: var(--mat-sys-primary);
      }
      .option .check {
        color: var(--mat-sys-primary);
      }
      /* about */
      .about-head {
        display: flex;
        align-items: center;
        gap: 16px;
        margin-bottom: 16px;
      }
      .about-head .logo {
        width: 54px;
        height: 54px;
        border-radius: 12px;
        object-fit: cover;
      }
      .about-head b {
        font-size: 18px;
      }
      .tagline {
        margin: 2px 0 0;
        font-size: 13px;
        color: var(--mat-sys-on-surface-variant);
      }
      .grid {
        display: flex;
        flex-direction: column;
        gap: 1px;
        border-radius: 12px;
        overflow: hidden;
        background: var(--mat-sys-outline-variant);
      }
      .kv {
        display: flex;
        justify-content: space-between;
        padding: 12px 16px;
        background: var(--mat-sys-surface-container-low);
        font-size: 13px;
      }
      .kv span {
        color: var(--mat-sys-on-surface-variant);
      }
      .note {
        margin: 16px 0 0;
        font-size: 12px;
        line-height: 1.6;
        color: var(--mat-sys-on-surface-variant);
      }
    `,
  ],
})
export class SettingsComponent implements OnInit {
  readonly account = inject(AccountService);
  readonly theme = inject(ThemeService);
  readonly settings = inject(SettingsService);
  private readonly aphrody = inject(AphrodyService);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);

  readonly meta = signal<Meta | null>(null);
  readonly selectedIndex = signal(0);

  readonly backends: BackendOption[] = [
    { id: "agy", label: "Antigravity (recommandé)", desc: "Compte Google AI — Gemini 3.5 Flash", icon: "bolt" },
    { id: "web", label: "Gemini Web", desc: "Cookie Google signé, sans clé", icon: "language" },
    { id: "stub", label: "Hors-ligne", desc: "Réponse locale, aucun réseau", icon: "cloud_off" },
  ];

  async ngOnInit(): Promise<void> {
    void this.account.refresh();
    try {
      this.meta.set(await this.aphrody.meta());
    } catch {
      // non-fatal
    }
    // Honour a ?tab= deep-link (e.g. from the Dashboard quick actions).
    const tab = this.route.snapshot.queryParamMap.get("tab") as TabId | null;
    if (tab) {
      const idx = TAB_IDS.indexOf(tab);
      if (idx >= 0) this.selectedIndex.set(idx);
    }
  }

  onTabChange(index: number): void {
    this.selectedIndex.set(index);
    // Reflect the tab in the URL so it is shareable / restorable.
    void this.router.navigate([], {
      relativeTo: this.route,
      queryParams: { tab: TAB_IDS[index] },
      replaceUrl: true,
    });
  }

  setTheme(mode: "dark" | "light"): void {
    if (this.theme.mode() !== mode) {
      this.theme.toggle();
    }
  }

  refreshAccount(): void {
    void this.account.refresh();
  }
}
