// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, computed, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatIconModule } from "@angular/material/icon";

import { AphrodyService } from "../../core/aphrody.service";

/** A single MCP tool as advertised by `aphrody mcp list`. */
interface McpTool {
  name: string;
  description: string;
  /** JSON schema of the tool's arguments (rendered on demand). */
  input_schema?: unknown;
}

/** The `aphrody mcp list` JSON envelope (only the fields we render). */
interface McpListJson {
  server?: string;
  server_info?: { name?: string; version?: string; protocol_version?: string };
  tools?: McpTool[];
}

/**
 * MCP view: parses `aphrody mcp list` (real JSON) into a filterable list of the
 * aphrody MCP server's tools. Each tool shows its name + description; the input
 * schema can be expanded inline. Honest empty/error state when the command
 * fails or returns nothing.
 */
@Component({
  selector: "app-mcp",
  imports: [FormsModule, MatIconModule],
  template: `
    <div class="page">
      <header class="head">
        <div class="head-icon"><mat-icon class="material-symbols-outlined">hub</mat-icon></div>
        <div>
          <h1>MCP</h1>
          <p>
            Outils exposés par le serveur Model Context Protocol d'aphrody
            (<code>aphrody mcp list</code>).
          </p>
        </div>
        <button class="refresh" (click)="load()" [disabled]="loading()" aria-label="Actualiser">
          <mat-icon class="material-symbols-outlined" [class.spin]="loading()">refresh</mat-icon>
        </button>
      </header>

      @if (server(); as s) {
        <div class="server-bar">
          <span class="badge"><mat-icon class="material-symbols-outlined">dns</mat-icon>{{ s }}</span>
          @if (serverVersion()) {
            <span class="meta">rmcp {{ serverVersion() }}</span>
          }
          @if (protocol()) {
            <span class="meta">protocole {{ protocol() }}</span>
          }
          <span class="meta">{{ tools().length }} outil(s)</span>
        </div>
      }

      <div class="search-bar">
        <mat-icon class="material-symbols-outlined">search</mat-icon>
        <input
          placeholder="Filtrer les outils…"
          [value]="query()"
          (input)="query.set($any($event.target).value)"
          aria-label="Filtrer les outils"
        />
      </div>

      @if (loading()) {
        <div class="state"><mat-icon class="material-symbols-outlined spin">progress_activity</mat-icon> Chargement des outils MCP…</div>
      } @else if (error()) {
        <div class="state err">
          <mat-icon class="material-symbols-outlined">error</mat-icon>
          <div>
            <b>Serveur MCP indisponible.</b>
            <p>{{ error() }}</p>
          </div>
        </div>
      } @else if (filtered().length === 0) {
        <div class="state">
          <mat-icon class="material-symbols-outlined">search_off</mat-icon>
          Aucun outil ne correspond à « {{ query() }} ».
        </div>
      } @else {
        <div class="grid">
          @for (t of filtered(); track t.name) {
            <div class="tool-card">
              <div class="tool-row" (click)="toggle(t.name)">
                <mat-icon class="material-symbols-outlined tool-glyph">build</mat-icon>
                <div class="tool-text">
                  <code class="tool-name">{{ t.name }}</code>
                  <span class="tool-desc">{{ t.description || "(sans description)" }}</span>
                </div>
                @if (t.input_schema) {
                  <button class="schema-toggle" aria-label="Afficher le schéma">
                    <mat-icon class="material-symbols-outlined">{{ expanded() === t.name ? "expand_less" : "data_object" }}</mat-icon>
                  </button>
                }
              </div>
              @if (expanded() === t.name && t.input_schema) {
                <pre class="schema">{{ pretty(t.input_schema) }}</pre>
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
      .head p code,
      .tool-name {
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
        animation: mcp-spin 1s linear infinite;
      }
      @keyframes mcp-spin {
        to {
          transform: rotate(360deg);
        }
      }
      .server-bar {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 10px;
        margin-bottom: 16px;
      }
      .badge {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        font-size: 13px;
        padding: 5px 12px;
        border-radius: 999px;
        background: var(--mat-sys-primary-container);
        color: var(--mat-sys-on-primary-container);
      }
      .badge mat-icon {
        font-size: 16px;
        width: 16px;
        height: 16px;
      }
      .meta {
        font-size: 12px;
        color: var(--mat-sys-on-surface-variant);
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
      .grid {
        display: flex;
        flex-direction: column;
        gap: 8px;
      }
      .tool-card {
        border-radius: 14px;
        background: var(--mat-sys-surface-container-low);
        overflow: hidden;
      }
      .tool-row {
        display: flex;
        align-items: center;
        gap: 14px;
        padding: 14px 16px;
        cursor: pointer;
      }
      .tool-row:hover {
        background: color-mix(in srgb, var(--mat-sys-on-surface) 4%, transparent);
      }
      .tool-glyph {
        color: var(--mat-sys-primary);
        flex: 0 0 auto;
      }
      .tool-text {
        display: flex;
        flex-direction: column;
        flex: 1 1 auto;
        min-width: 0;
      }
      .tool-name {
        font-size: 14px;
        color: var(--mat-sys-on-surface);
      }
      .tool-desc {
        font-size: 12px;
        color: var(--mat-sys-on-surface-variant);
        line-height: 1.4;
      }
      .schema-toggle {
        flex: 0 0 auto;
        border: none;
        background: transparent;
        color: var(--mat-sys-on-surface-variant);
        cursor: pointer;
      }
      .schema {
        margin: 0;
        padding: 14px 16px;
        font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
        font-size: 12px;
        line-height: 1.5;
        color: var(--mat-sys-on-surface-variant);
        background: var(--mat-sys-surface-container);
        white-space: pre-wrap;
        word-break: break-word;
        max-height: 320px;
        overflow: auto;
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
export class McpComponent implements OnInit {
  private readonly aphrody = inject(AphrodyService);

  readonly query = signal("");
  readonly loading = signal(false);
  readonly error = signal("");
  readonly tools = signal<McpTool[]>([]);
  readonly server = signal("");
  readonly serverVersion = signal("");
  readonly protocol = signal("");
  readonly expanded = signal<string | null>(null);

  readonly filtered = computed(() => {
    const q = this.query().trim().toLowerCase();
    const list = this.tools();
    if (!q) return list;
    return list.filter(
      (t) => t.name.toLowerCase().includes(q) || (t.description ?? "").toLowerCase().includes(q),
    );
  });

  ngOnInit(): void {
    void this.load();
  }

  toggle(name: string): void {
    this.expanded.set(this.expanded() === name ? null : name);
  }

  pretty(value: unknown): string {
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  }

  async load(): Promise<void> {
    this.loading.set(true);
    this.error.set("");
    try {
      const res = await this.aphrody.exec(["mcp", "list"]);
      const out = res.stdout.trim();
      if (res.code !== 0 || !out.startsWith("{")) {
        this.error.set(
          (res.stderr || out || "sortie inattendue").slice(0, 400) ||
            "La commande mcp list n'a renvoyé aucune donnée.",
        );
        this.tools.set([]);
        return;
      }
      const json = JSON.parse(out) as McpListJson;
      this.server.set(json.server ?? "aphrody");
      this.serverVersion.set(json.server_info?.version ?? "");
      this.protocol.set(json.server_info?.protocol_version ?? "");
      this.tools.set(
        (json.tools ?? []).slice().sort((a, b) => a.name.localeCompare(b.name)),
      );
    } catch (err) {
      this.error.set(`Lecture impossible : ${String(err)}`);
      this.tools.set([]);
    } finally {
      this.loading.set(false);
    }
  }
}
