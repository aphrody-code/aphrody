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
    /** Originating MCP server (added when flattening across servers). */
    server: string;
}

/**
 * One server line of `aphrody mcp list`. The command emits **NDJSON** — one such
 * object per line, one per configured server (aphrody, winclean-core, …) — so we
 * parse line-by-line rather than as a single object.
 */
interface McpServerJson {
    server?: string;
    server_info?: { name?: string; version?: string; protocol_version?: string };
    tools?: Omit<McpTool, "server">[];
    /** Present when a server failed to probe (rendered as a degraded row). */
    error?: string;
}

/** A parsed server with its (tagged) tools, for the summary bar. */
interface McpServer {
    name: string;
    version: string;
    protocol: string;
    toolCount: number;
    error?: string;
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
                <div class="head-icon">
                    <mat-icon class="material-symbols-outlined">hub</mat-icon>
                </div>
                <div>
                    <h1>MCP</h1>
                    <p>
                        Outils exposés par le serveur Model Context Protocol d'aphrody (<code
                            >aphrody mcp list</code
                        >).
                    </p>
                </div>
                <button
                    class="refresh"
                    (click)="load()"
                    [disabled]="loading()"
                    aria-label="Actualiser"
                >
                    <mat-icon class="material-symbols-outlined" [class.spin]="loading()"
                        >refresh</mat-icon
                    >
                </button>
            </header>

            @if (servers().length > 0) {
                <div class="server-bar">
                    @for (s of servers(); track s.name) {
                        <span class="badge" [class.badge-err]="s.error">
                            <mat-icon class="material-symbols-outlined">{{
                                s.error ? "error" : "dns"
                            }}</mat-icon>
                            {{ s.name }}
                            <span class="badge-count">{{ s.toolCount }}</span>
                        </span>
                    }
                    <span class="meta"
                        >{{ servers().length }} serveur(s) · {{ tools().length }} outil(s)</span
                    >
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
                <div class="state">
                    <mat-icon class="material-symbols-outlined spin">progress_activity</mat-icon>
                    Chargement des outils MCP…
                </div>
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
                                <mat-icon class="material-symbols-outlined tool-glyph"
                                    >build</mat-icon
                                >
                                <div class="tool-text">
                                    <span class="tool-name-row">
                                        <code class="tool-name">{{ t.name }}</code>
                                        <span class="tool-server">{{ t.server }}</span>
                                    </span>
                                    <span class="tool-desc">{{
                                        t.description || "(sans description)"
                                    }}</span>
                                </div>
                                @if (t.input_schema) {
                                    <button class="schema-toggle" aria-label="Afficher le schéma">
                                        <mat-icon class="material-symbols-outlined">{{
                                            expanded() === t.name ? "expand_less" : "data_object"
                                        }}</mat-icon>
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
            .badge-err {
                background: var(--mat-sys-error-container);
                color: var(--mat-sys-on-error-container);
            }
            .badge-count {
                font-size: 11px;
                padding: 1px 7px;
                border-radius: 999px;
                background: color-mix(
                    in srgb,
                    var(--mat-sys-on-primary-container) 16%,
                    transparent
                );
            }
            .tool-name-row {
                display: flex;
                align-items: center;
                gap: 8px;
                min-width: 0;
            }
            .tool-server {
                flex: 0 0 auto;
                font-size: 10px;
                text-transform: uppercase;
                letter-spacing: 0.04em;
                padding: 1px 7px;
                border-radius: 6px;
                background: var(--mat-sys-surface-container-highest);
                color: var(--mat-sys-on-surface-variant);
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
    /** All configured MCP servers (for the summary bar). */
    readonly servers = signal<McpServer[]>([]);
    readonly expanded = signal<string | null>(null);

    readonly filtered = computed(() => {
        const q = this.query().trim().toLowerCase();
        const list = this.tools();
        if (!q) return list;
        return list.filter(
            (t) =>
                t.name.toLowerCase().includes(q) ||
                (t.description ?? "").toLowerCase().includes(q) ||
                t.server.toLowerCase().includes(q),
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
            if (res.code !== 0 && !out) {
                this.error.set(
                    (res.stderr || "sortie inattendue").slice(0, 400) ||
                        "La commande mcp list n'a renvoyé aucune donnée.",
                );
                this.tools.set([]);
                this.servers.set([]);
                return;
            }

            // `aphrody mcp list` is NDJSON: one server object per line. Parse each
            // line independently so one malformed/oversized line does not blank the
            // whole view, and aggregate every server's tools into one tagged list.
            const servers: McpServer[] = [];
            const allTools: McpTool[] = [];
            for (const line of out.split("\n")) {
                const trimmed = line.trim();
                if (!trimmed.startsWith("{")) continue;
                let obj: McpServerJson;
                try {
                    obj = JSON.parse(trimmed) as McpServerJson;
                } catch {
                    continue; // skip an unparseable line rather than failing everything
                }
                const name = obj.server ?? obj.server_info?.name ?? "(serveur)";
                const tools = obj.tools ?? [];
                servers.push({
                    name,
                    version: obj.server_info?.version ?? "",
                    protocol: obj.server_info?.protocol_version ?? "",
                    toolCount: tools.length,
                    error: obj.error,
                });
                for (const t of tools) {
                    allTools.push({ ...t, server: name });
                }
            }

            if (servers.length === 0) {
                this.error.set((res.stderr || out || "aucun serveur MCP configuré").slice(0, 400));
                this.tools.set([]);
                this.servers.set([]);
                return;
            }

            this.servers.set(servers);
            this.tools.set(
                allTools
                    .slice()
                    .sort(
                        (a, b) => a.server.localeCompare(b.server) || a.name.localeCompare(b.name),
                    ),
            );
        } catch (err) {
            this.error.set(`Lecture impossible : ${String(err)}`);
            this.tools.set([]);
            this.servers.set([]);
        } finally {
            this.loading.set(false);
        }
    }
}
