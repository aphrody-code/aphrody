// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, inject, input, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatIconModule } from "@angular/material/icon";

import {
  AgentService,
  HostUnavailableError,
  PersonaDoc,
  PersonaWhich,
} from "../../../core/agent.service";

/** Seed templates offered when a persona file does not yet exist on disk. */
const SOUL_SEED = `---
tone: précis et chaleureux
brevity: balanced
humor: dry
bluntness: direct
opinions:
  - Citer les numéros de ligne et vérifier avant d'affirmer le succès.
  - Préférer des changements petits et testés.
boundaries:
  - Ne jamais exécuter de commande destructrice sans confirmation.
---
Je suis un agent d'ingénierie autonome. Je vais droit au but, je vérifie mon
travail, et je signale honnêtement ce qui n'est pas câblé.
`;

const IDENTITY_SEED = `---
name: aphrody
vibe: assistant IA autonome, précis et direct
---
Je suis aphrody, un agent autonome propulsé par Gemini.
`;

/**
 * Reusable editor for an agent-home persona file (SOUL.md / IDENTITY.md). Reads
 * the markdown via the scoped `read_agent_home` command, edits it in a textarea,
 * and saves via `write_agent_home`. When the file does not exist yet it offers a
 * sensible seed (never silently fabricates state). The on-disk path + workspace
 * are shown so the user knows exactly what is being edited.
 */
@Component({
  selector: "app-persona-tab",
  imports: [FormsModule, MatIconModule],
  template: `
    <section class="card">
      <h2>{{ heading() }}</h2>
      <p class="hint">{{ blurb() }}</p>

      @if (hostUnavailable()) {
        <p class="warn">Cette édition nécessite l'application de bureau (fichiers locaux).</p>
      } @else {
        @if (doc(); as d) {
          <div class="path-row">
            <mat-icon class="material-symbols-outlined">description</mat-icon>
            <span class="mono" [title]="d.path">{{ d.path }}</span>
            @if (d.exists) {
              <span class="tag ok">présent</span>
            } @else {
              <span class="tag muted">à créer</span>
            }
          </div>

          @if (!d.exists && !content()) {
            <div class="seed">
              <p>Ce fichier n'existe pas encore. Vous pouvez partir d'un modèle :</p>
              <button class="btn ghost" (click)="useSeed()">
                <mat-icon class="material-symbols-outlined">auto_fix_high</mat-icon>
                Insérer un modèle
              </button>
            </div>
          }

          <textarea
            class="editor"
            [value]="content()"
            (input)="content.set($any($event.target).value)"
            [placeholder]="placeholder()"
            spellcheck="false"
            aria-label="Contenu du fichier"
          ></textarea>

          <div class="actions">
            <button class="btn" (click)="save()" [disabled]="saving() || content() === original()">
              <mat-icon class="material-symbols-outlined">{{ saving() ? "progress_activity" : "save" }}</mat-icon>
              {{ saving() ? "Enregistrement…" : "Enregistrer" }}
            </button>
            @if (content() !== original()) {
              <button class="btn ghost" (click)="revert()" [disabled]="saving()">
                <mat-icon class="material-symbols-outlined">undo</mat-icon>
                Annuler
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
        margin: 0 0 12px;
      }
      .hint {
        margin: 0 0 16px;
        font-size: 13px;
        line-height: 1.5;
        color: var(--mat-sys-on-surface-variant);
      }
      .warn {
        margin: 12px 0 0;
        font-size: 13px;
        color: var(--mat-sys-error);
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
      .seed {
        margin-bottom: 12px;
        padding: 12px 14px;
        border-radius: 12px;
        background: var(--mat-sys-surface-container-low);
      }
      .seed p {
        margin: 0 0 10px;
        font-size: 13px;
        color: var(--mat-sys-on-surface-variant);
      }
      .editor {
        width: 100%;
        box-sizing: border-box;
        min-height: 280px;
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
      }
      .editor:focus {
        border-color: var(--mat-sys-primary);
      }
      .actions {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-top: 14px;
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
export class PersonaTabComponent implements OnInit {
  private readonly agent = inject(AgentService);

  /** "soul" | "identity" — which persona file this editor manages. */
  readonly which = input.required<PersonaWhich>();

  readonly doc = signal<PersonaDoc | null>(null);
  readonly content = signal("");
  readonly original = signal("");
  readonly saving = signal(false);
  readonly savedAt = signal("");
  readonly loadError = signal("");
  readonly saveError = signal("");
  readonly hostUnavailable = signal(false);

  heading(): string {
    return this.which() === "soul" ? "Âme de l'agent (SOUL.md)" : "Identité de l'agent (IDENTITY.md)";
  }

  blurb(): string {
    return this.which() === "soul"
      ? "L'âme définit la personnalité, le ton, les opinions et les limites de l'agent — elle est intégrée au prompt système. Markdown avec un en-tête YAML facultatif (tone, brevity, humor, bluntness, opinions, boundaries)."
      : "L'identité indique qui est l'agent : son nom et sa vibe. Elle ouvre le prompt système (« Tu es … »). Markdown avec un en-tête YAML facultatif (name, vibe).";
  }

  placeholder(): string {
    return this.which() === "soul"
      ? "---\ntone: …\n---\nLa personnalité de l'agent…"
      : "---\nname: …\n---\nQui est l'agent…";
  }

  async ngOnInit(): Promise<void> {
    try {
      const d = await this.agent.readAgentHome(this.which());
      this.doc.set(d);
      this.content.set(d.content);
      this.original.set(d.content);
    } catch (err) {
      if (err instanceof HostUnavailableError) {
        this.hostUnavailable.set(true);
      } else {
        this.loadError.set(`Lecture impossible : ${String(err)}`);
      }
    }
  }

  useSeed(): void {
    const seed = this.which() === "soul" ? SOUL_SEED : IDENTITY_SEED;
    this.content.set(seed);
  }

  revert(): void {
    this.content.set(this.original());
    this.saveError.set("");
  }

  async save(): Promise<void> {
    this.saving.set(true);
    this.saveError.set("");
    this.savedAt.set("");
    try {
      const d = await this.agent.writeAgentHome(this.which(), this.content());
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
