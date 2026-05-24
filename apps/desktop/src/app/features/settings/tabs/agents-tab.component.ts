// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatIconModule } from "@angular/material/icon";

import {
  AgentService,
  ConfigDoc,
  ConfigWhich,
  HostUnavailableError,
} from "../../../core/agent.service";

/** A selectable agent whose config the editor manages. */
interface AgentChoice {
  which: ConfigWhich;
  label: string;
  icon: string;
  blurb: string;
}

const CHOICES: AgentChoice[] = [
  {
    which: "agy",
    label: "agy CLI / Antigravity 2.0",
    icon: "rocket",
    blurb: "Configuration partagée du CLI Antigravity (agy) — ~/.gemini/config/config.json.",
  },
  {
    which: "claude",
    label: "Claude Code",
    icon: "smart_toy",
    blurb: "Réglages de Claude Code — ~/.claude/settings.json.",
  },
  {
    which: "aphrody-mcp",
    label: "Serveur MCP (aphrody)",
    icon: "hub",
    blurb: "Serveurs MCP déclarés — ~/.config/aphrody/mcp.json.",
  },
];

/**
 * Agents tab: the "bouton pour éditer la config agy cli / antigravity / claude
 * code". A 3-way selector loads the chosen agent's JSON config via
 * `read_agent_config`, edits it in a code textarea, validates JSON client-side
 * (mirroring the Rust-side validation that refuses a malformed write), and saves
 * via `write_agent_config`. The on-disk path is shown; non-existent files are
 * reported honestly and can be created by saving valid JSON.
 */
@Component({
  selector: "app-agents-tab",
  imports: [FormsModule, MatIconModule],
  template: `
    <section class="card">
      <h2>Configuration des agents</h2>
      <p class="hint">
        Éditez la configuration JSON de l'agent sélectionné. La validation JSON s'effectue avant
        l'enregistrement — un JSON invalide est refusé et le fichier existant reste intact.
      </p>

      <div class="selector">
        @for (c of choices; track c.which) {
          <button class="choice" [class.sel]="which() === c.which" (click)="select(c.which)">
            <mat-icon class="material-symbols-outlined">{{ c.icon }}</mat-icon>
            <span>{{ c.label }}</span>
          </button>
        }
      </div>
      <p class="blurb">{{ currentBlurb() }}</p>
    </section>

    <section class="card">
      @if (hostUnavailable()) {
        <p class="warn">L'édition de configuration nécessite l'application de bureau.</p>
      } @else if (doc(); as d) {
        <div class="path-row">
          <mat-icon class="material-symbols-outlined">description</mat-icon>
          <span class="mono" [title]="d.path">{{ d.path }}</span>
          @if (d.exists) {
            <span class="tag ok">présent</span>
          } @else {
            <span class="tag muted">à créer</span>
          }
        </div>

        <textarea
          class="editor"
          [value]="content()"
          (input)="onEdit($any($event.target).value)"
          [class.invalid]="jsonError() !== ''"
          spellcheck="false"
          placeholder="{}"
          aria-label="Configuration JSON"
        ></textarea>

        <div class="status-row">
          @if (jsonError()) {
            <span class="warn"><mat-icon class="material-symbols-outlined">error</mat-icon>{{ jsonError() }}</span>
          } @else {
            <span class="ok-text"><mat-icon class="material-symbols-outlined">check_circle</mat-icon>JSON valide</span>
          }
        </div>

        <div class="actions">
          <button
            class="btn"
            (click)="save()"
            [disabled]="saving() || jsonError() !== '' || content() === original()"
          >
            <mat-icon class="material-symbols-outlined">{{ saving() ? "progress_activity" : "save" }}</mat-icon>
            {{ saving() ? "Enregistrement…" : "Enregistrer" }}
          </button>
          <button class="btn ghost" (click)="format()" [disabled]="jsonError() !== ''">
            <mat-icon class="material-symbols-outlined">format_align_left</mat-icon> Formater
          </button>
          @if (content() !== original()) {
            <button class="btn ghost" (click)="revert()" [disabled]="saving()">
              <mat-icon class="material-symbols-outlined">undo</mat-icon> Annuler
            </button>
          }
          @if (savedAt()) {
            <span class="saved"><mat-icon class="material-symbols-outlined">check_circle</mat-icon>{{ savedAt() }}</span>
          }
        </div>
        @if (saveError()) {
          <p class="warn">{{ saveError() }}</p>
        }
      } @else if (loadError()) {
        <p class="warn">{{ loadError() }}</p>
      } @else {
        <p class="hint">Chargement…</p>
      }
    </section>
  `,
  styles: [
    `
      .card {
        background: var(--mat-sys-surface-container);
        border-radius: 20px;
        padding: 20px 22px;
        margin-bottom: 16px;
      }
      .card h2 {
        font-size: 16px;
        font-weight: 500;
        margin: 0 0 10px;
      }
      .hint {
        margin: 0 0 16px;
        font-size: 13px;
        line-height: 1.5;
        color: var(--mat-sys-on-surface-variant);
      }
      .selector {
        display: flex;
        gap: 8px;
        flex-wrap: wrap;
      }
      .choice {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        border: 1px solid var(--mat-sys-outline-variant);
        background: transparent;
        border-radius: 12px;
        padding: 10px 14px;
        font: inherit;
        font-size: 13px;
        color: var(--mat-sys-on-surface);
        cursor: pointer;
        transition: border-color 0.15s, background 0.15s;
      }
      .choice:hover {
        background: color-mix(in srgb, var(--mat-sys-on-surface) 5%, transparent);
      }
      .choice.sel {
        border-color: var(--mat-sys-primary);
        background: color-mix(in srgb, var(--mat-sys-primary) 10%, transparent);
        color: var(--mat-sys-primary);
      }
      .choice mat-icon {
        font-size: 18px;
        width: 18px;
        height: 18px;
      }
      .blurb {
        margin: 14px 0 0;
        font-size: 12px;
        color: var(--mat-sys-on-surface-variant);
      }
      .warn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        margin: 0;
        font-size: 13px;
        color: var(--mat-sys-error);
      }
      .warn mat-icon {
        font-size: 16px;
        width: 16px;
        height: 16px;
      }
      .path-row {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 12px;
        font-size: 12px;
        color: var(--mat-sys-on-surface-variant);
      }
      .path-row mat-icon {
        font-size: 18px;
        width: 18px;
        height: 18px;
      }
      .mono {
        font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        flex: 1 1 auto;
      }
      .tag {
        flex: 0 0 auto;
        font-size: 10px;
        padding: 2px 8px;
        border-radius: 999px;
      }
      .tag.ok {
        background: color-mix(in srgb, var(--mat-sys-tertiary) 16%, transparent);
        color: var(--mat-sys-tertiary);
      }
      .tag.muted {
        background: var(--mat-sys-surface-container-highest);
        color: var(--mat-sys-on-surface-variant);
      }
      .editor {
        width: 100%;
        box-sizing: border-box;
        min-height: 300px;
        resize: vertical;
        border: 1px solid var(--mat-sys-outline-variant);
        border-radius: 12px;
        background: var(--mat-sys-surface-container-low);
        color: var(--mat-sys-on-surface);
        font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
        font-size: 13px;
        line-height: 1.55;
        padding: 14px 16px;
        outline: none;
        tab-size: 2;
      }
      .editor:focus {
        border-color: var(--mat-sys-primary);
      }
      .editor.invalid {
        border-color: var(--mat-sys-error);
      }
      .status-row {
        margin: 8px 0 0;
        min-height: 20px;
      }
      .ok-text {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        font-size: 12px;
        color: var(--mat-sys-tertiary);
      }
      .ok-text mat-icon {
        font-size: 16px;
        width: 16px;
        height: 16px;
      }
      .actions {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-top: 12px;
        flex-wrap: wrap;
      }
      .btn {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        border: none;
        border-radius: 999px;
        padding: 9px 18px;
        font: inherit;
        font-size: 14px;
        cursor: pointer;
        color: var(--mat-sys-on-primary);
        background: var(--mat-sys-primary);
      }
      .btn.ghost {
        color: var(--mat-sys-on-surface-variant);
        background: var(--mat-sys-surface-container-high);
      }
      .btn:disabled {
        opacity: 0.55;
        cursor: default;
      }
      .btn mat-icon {
        font-size: 18px;
        width: 18px;
        height: 18px;
      }
      .saved {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        font-size: 12px;
        color: var(--mat-sys-tertiary);
      }
      .saved mat-icon {
        font-size: 16px;
        width: 16px;
        height: 16px;
      }
    `,
  ],
})
export class AgentsTabComponent implements OnInit {
  private readonly agent = inject(AgentService);

  readonly choices = CHOICES;
  readonly which = signal<ConfigWhich>("agy");
  readonly doc = signal<ConfigDoc | null>(null);
  readonly content = signal("");
  readonly original = signal("");
  readonly jsonError = signal("");
  readonly saving = signal(false);
  readonly savedAt = signal("");
  readonly loadError = signal("");
  readonly saveError = signal("");
  readonly hostUnavailable = signal(false);

  ngOnInit(): void {
    void this.load();
  }

  currentBlurb(): string {
    return CHOICES.find((c) => c.which === this.which())?.blurb ?? "";
  }

  select(which: ConfigWhich): void {
    if (this.which() === which) return;
    this.which.set(which);
    void this.load();
  }

  private async load(): Promise<void> {
    this.doc.set(null);
    this.loadError.set("");
    this.saveError.set("");
    this.savedAt.set("");
    this.jsonError.set("");
    try {
      const d = await this.agent.readAgentConfig(this.which());
      this.doc.set(d);
      const initial = d.content.trim() ? d.content : "{}";
      this.content.set(initial);
      this.original.set(initial);
      this.validate(initial);
    } catch (err) {
      if (err instanceof HostUnavailableError) {
        this.hostUnavailable.set(true);
      } else {
        this.loadError.set(`Lecture impossible : ${String(err)}`);
      }
    }
  }

  onEdit(value: string): void {
    this.content.set(value);
    this.validate(value);
  }

  private validate(value: string): void {
    try {
      JSON.parse(value);
      this.jsonError.set("");
    } catch (err) {
      this.jsonError.set(`JSON invalide : ${(err as Error).message}`);
    }
  }

  format(): void {
    try {
      const pretty = JSON.stringify(JSON.parse(this.content()), null, 2);
      this.content.set(pretty);
      this.jsonError.set("");
    } catch {
      // validate() already surfaced the error
    }
  }

  revert(): void {
    this.content.set(this.original());
    this.validate(this.original());
    this.saveError.set("");
  }

  async save(): Promise<void> {
    if (this.jsonError()) return;
    this.saving.set(true);
    this.saveError.set("");
    this.savedAt.set("");
    try {
      const d = await this.agent.writeAgentConfig(this.which(), this.content());
      this.doc.set(d);
      this.original.set(this.content());
      this.savedAt.set(`Enregistré à ${new Date().toLocaleTimeString("fr-FR")}`);
    } catch (err) {
      this.saveError.set(`Enregistrement impossible : ${String(err)}`);
    } finally {
      this.saving.set(false);
    }
  }
}
