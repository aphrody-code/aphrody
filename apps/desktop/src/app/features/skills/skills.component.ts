// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, computed, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatIconModule } from "@angular/material/icon";

import { AgentService, HostUnavailableError } from "../../core/agent.service";

/** One discovered skill parsed from `aphrody-skills list`. */
export interface Skill {
  name: string;
  source: string;
  mode: string;
  loc: number;
  description: string;
}

/** Source -> Material Symbol + tonal accent class. */
function sourceIcon(source: string): string {
  switch (source) {
    case "claude-code":
      return "smart_toy";
    case "antigravity":
      return "rocket_launch";
    case "gemini":
      return "auto_awesome";
    default:
      return "extension";
  }
}

/**
 * Skills view: lists every SKILL.md discovered by the sibling `aphrody-skills`
 * binary (`list`), parsed from its fixed-width table. Filterable cards with the
 * skill name + source + description; expanding a card runs `info <name>` to show
 * the full SKILL.md. Honest empty state when the sibling is unreachable (e.g.
 * running in a plain browser, or the binary is missing next to aphrody).
 */
@Component({
  selector: "app-skills",
  imports: [FormsModule, MatIconModule],
  template: `
    <div class="page">
      <header class="head">
        <div class="head-icon"><mat-icon class="material-symbols-outlined">extension</mat-icon></div>
        <div>
          <h1>Skills</h1>
          <p>Catalogue des compétences SKILL.md (<code>aphrody-skills list</code>).</p>
        </div>
        <button class="refresh" (click)="load()" [disabled]="loading()" aria-label="Actualiser">
          <mat-icon class="material-symbols-outlined" [class.spin]="loading()">refresh</mat-icon>
        </button>
      </header>

      <div class="search-bar">
        <mat-icon class="material-symbols-outlined">search</mat-icon>
        <input
          placeholder="Filtrer les skills…"
          [value]="query()"
          (input)="query.set($any($event.target).value)"
          aria-label="Filtrer les skills"
        />
        @if (skills().length > 0) {
          <span class="count">{{ filtered().length }} / {{ skills().length }}</span>
        }
      </div>

      @if (loading()) {
        <div class="state"><mat-icon class="material-symbols-outlined spin">progress_activity</mat-icon> Chargement du catalogue…</div>
      } @else if (error()) {
        <div class="state err">
          <mat-icon class="material-symbols-outlined">error</mat-icon>
          <div>
            <b>Catalogue de skills indisponible.</b>
            <p>{{ error() }}</p>
          </div>
        </div>
      } @else if (filtered().length === 0) {
        <div class="state">
          <mat-icon class="material-symbols-outlined">search_off</mat-icon>
          Aucun skill ne correspond à « {{ query() }} ».
        </div>
      } @else {
        <div class="grid">
          @for (s of filtered(); track s.name) {
            <div class="skill-card">
              <div class="skill-row" (click)="open(s)">
                <mat-icon class="material-symbols-outlined skill-glyph">{{ icon(s.source) }}</mat-icon>
                <div class="skill-text">
                  <div class="skill-name-row">
                    <span class="skill-name">{{ s.name }}</span>
                    <span class="src-tag">{{ s.source }}</span>
                    @if (s.mode) {
                      <span class="mode-tag">{{ s.mode }}</span>
                    }
                  </div>
                  <span class="skill-desc">{{ s.description || "(sans description)" }}</span>
                </div>
                <button class="open-btn" aria-label="Ouvrir le SKILL.md">
                  <mat-icon class="material-symbols-outlined">{{ expanded() === s.name ? "expand_less" : "unfold_more" }}</mat-icon>
                </button>
              </div>
              @if (expanded() === s.name) {
                <div class="skill-body">
                  @if (bodyLoading()) {
                    <span class="body-note"><mat-icon class="material-symbols-outlined spin">progress_activity</mat-icon> Lecture du SKILL.md…</span>
                  } @else {
                    <pre>{{ body() }}</pre>
                  }
                </div>
              }
            </div>
          }
        </div>
      }
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
        max-width: 920px;
        margin: 0 auto;
        padding: 28px 28px 64px;
      }
      .head {
        display: flex;
        align-items: center;
        gap: 16px;
        margin-bottom: 18px;
      }
      .head-icon {
        width: 48px;
        height: 48px;
        border-radius: 14px;
        display: grid;
        place-items: center;
        background: var(--mat-sys-secondary-container);
        color: var(--mat-sys-on-secondary-container);
      }
      .head h1 {
        margin: 0;
        font-size: 24px;
        font-weight: 400;
      }
      .head p {
        margin: 2px 0 0;
        font-size: 13px;
        color: var(--mat-sys-on-surface-variant);
      }
      .head p code {
        font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
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
        animation: sk-spin 1s linear infinite;
      }
      @keyframes sk-spin {
        to {
          transform: rotate(360deg);
        }
      }
      .search-bar {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 10px 16px;
        border-radius: 14px;
        background: var(--mat-sys-surface-container);
        margin-bottom: 16px;
      }
      .search-bar mat-icon {
        color: var(--mat-sys-on-surface-variant);
      }
      .search-bar input {
        flex: 1 1 auto;
        border: none;
        outline: none;
        background: transparent;
        color: var(--mat-sys-on-surface);
        font: inherit;
        font-size: 14px;
      }
      .count {
        font-size: 12px;
        color: var(--mat-sys-on-surface-variant);
      }
      .grid {
        display: flex;
        flex-direction: column;
        gap: 8px;
      }
      .skill-card {
        border-radius: 14px;
        background: var(--mat-sys-surface-container-low);
        overflow: hidden;
      }
      .skill-row {
        display: flex;
        align-items: center;
        gap: 14px;
        padding: 14px 16px;
        cursor: pointer;
      }
      .skill-row:hover {
        background: color-mix(in srgb, var(--mat-sys-on-surface) 4%, transparent);
      }
      .skill-glyph {
        color: var(--mat-sys-primary);
        flex: 0 0 auto;
      }
      .skill-text {
        display: flex;
        flex-direction: column;
        flex: 1 1 auto;
        min-width: 0;
      }
      .skill-name-row {
        display: flex;
        align-items: center;
        gap: 8px;
        flex-wrap: wrap;
      }
      .skill-name {
        font-size: 14px;
        font-weight: 500;
        color: var(--mat-sys-on-surface);
      }
      .src-tag,
      .mode-tag {
        font-size: 10px;
        padding: 2px 8px;
        border-radius: 999px;
        background: var(--mat-sys-surface-container-highest);
        color: var(--mat-sys-on-surface-variant);
      }
      .mode-tag {
        background: color-mix(in srgb, var(--mat-sys-tertiary) 18%, transparent);
        color: var(--mat-sys-tertiary);
      }
      .skill-desc {
        font-size: 12px;
        color: var(--mat-sys-on-surface-variant);
        line-height: 1.4;
        overflow: hidden;
        text-overflow: ellipsis;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
      }
      .open-btn {
        flex: 0 0 auto;
        border: none;
        background: transparent;
        color: var(--mat-sys-on-surface-variant);
        cursor: pointer;
      }
      .skill-body {
        padding: 4px 16px 16px;
      }
      .skill-body pre {
        margin: 0;
        padding: 14px 16px;
        font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
        font-size: 12px;
        line-height: 1.55;
        color: var(--mat-sys-on-surface-variant);
        background: var(--mat-sys-surface-container);
        border-radius: 10px;
        white-space: pre-wrap;
        word-break: break-word;
        max-height: 360px;
        overflow: auto;
      }
      .body-note {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        font-size: 13px;
        color: var(--mat-sys-on-surface-variant);
      }
      .body-note mat-icon {
        font-size: 18px;
        width: 18px;
        height: 18px;
      }
      .state {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 28px 20px;
        border-radius: 14px;
        background: var(--mat-sys-surface-container-low);
        color: var(--mat-sys-on-surface-variant);
        font-size: 14px;
      }
      .state.err {
        align-items: flex-start;
        color: var(--mat-sys-on-surface);
      }
      .state.err mat-icon {
        color: var(--mat-sys-error);
      }
      .state.err p {
        margin: 4px 0 0;
        font-size: 13px;
        color: var(--mat-sys-on-surface-variant);
      }
    `,
  ],
})
export class SkillsComponent implements OnInit {
  private readonly agent = inject(AgentService);

  readonly query = signal("");
  readonly loading = signal(false);
  readonly error = signal("");
  readonly skills = signal<Skill[]>([]);
  readonly expanded = signal<string | null>(null);
  readonly body = signal("");
  readonly bodyLoading = signal(false);

  readonly icon = sourceIcon;

  readonly filtered = computed(() => {
    const q = this.query().trim().toLowerCase();
    const list = this.skills();
    if (!q) return list;
    return list.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.source.toLowerCase().includes(q),
    );
  });

  ngOnInit(): void {
    void this.load();
  }

  async load(): Promise<void> {
    this.loading.set(true);
    this.error.set("");
    this.expanded.set(null);
    try {
      const res = await this.agent.siblingExec("aphrody-skills", ["list"]);
      if (res.code !== 0 && !res.stdout.trim()) {
        this.error.set(
          (res.stderr || "aphrody-skills a échoué").slice(0, 400) +
            "\nVérifiez que aphrody-skills est installé à côté d'aphrody.",
        );
        this.skills.set([]);
        return;
      }
      this.skills.set(parseSkillsTable(res.stdout));
      if (this.skills().length === 0) {
        this.error.set("Aucun skill découvert (catalogue vide).");
      }
    } catch (err) {
      if (err instanceof HostUnavailableError) {
        this.error.set(
          "Le catalogue de skills nécessite l'application de bureau (le binaire aphrody-skills s'exécute en local).",
        );
      } else {
        this.error.set(`Lecture impossible : ${String(err)}`);
      }
      this.skills.set([]);
    } finally {
      this.loading.set(false);
    }
  }

  async open(s: Skill): Promise<void> {
    if (this.expanded() === s.name) {
      this.expanded.set(null);
      return;
    }
    this.expanded.set(s.name);
    this.body.set("");
    this.bodyLoading.set(true);
    try {
      const res = await this.agent.siblingExec("aphrody-skills", ["info", s.name]);
      this.body.set(res.stdout.trim() || res.stderr.trim() || "(SKILL.md vide)");
    } catch (err) {
      this.body.set(`Lecture du SKILL.md impossible : ${String(err)}`);
    } finally {
      this.bodyLoading.set(false);
    }
  }
}

/**
 * Parse the fixed-width table emitted by `aphrody-skills list`:
 *
 * ```
 * NAME      SOURCE   MODE   LOC  DESCRIPTION
 * --------- -------- ------ ---- -----------
 * material  antigrav        97   Google's Material…
 * ```
 *
 * The header row defines the column start offsets; each data row is sliced on
 * those offsets so multi-word descriptions and empty MODE cells parse cleanly.
 */
export function parseSkillsTable(raw: string): Skill[] {
  const lines = raw.split(/\r?\n/);
  const headerIdx = lines.findIndex((l) => /^\s*NAME\s+SOURCE/.test(l));
  if (headerIdx < 0) return [];
  const header = lines[headerIdx];
  const cols = ["NAME", "SOURCE", "MODE", "LOC", "DESCRIPTION"];
  const offsets = cols.map((c) => header.indexOf(c)).filter((i) => i >= 0);
  if (offsets.length < 5) return [];
  const [nameAt, sourceAt, modeAt, locAt, descAt] = offsets;

  const skills: Skill[] = [];
  for (let i = headerIdx + 1; i < lines.length; i++) {
    const line = lines[i];
    if (!line.trim() || /^[-\s]+$/.test(line)) continue; // skip the rule + blanks
    const name = line.slice(nameAt, sourceAt).trim();
    if (!name) continue;
    const source = line.slice(sourceAt, modeAt).trim();
    const mode = line.slice(modeAt, locAt).trim();
    const locStr = line.slice(locAt, descAt).trim();
    const description = line.slice(descAt).replace(/…$/, "").trim();
    skills.push({
      name,
      source,
      mode,
      loc: Number.parseInt(locStr, 10) || 0,
      description,
    });
  }
  return skills.sort((a, b) => a.name.localeCompare(b.name));
}
