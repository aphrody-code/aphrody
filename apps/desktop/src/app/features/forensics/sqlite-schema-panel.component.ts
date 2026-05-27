// SPDX-License-Identifier: Apache-2.0
import { Component, computed, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatButtonModule } from "@angular/material/button";
import { MatCardModule } from "@angular/material/card";
import { MatIconModule } from "@angular/material/icon";
import { MatProgressSpinnerModule } from "@angular/material/progress-spinner";
import { MatTooltipModule } from "@angular/material/tooltip";

import { AphrodyService } from "../../core/aphrody.service";

/**
 * One schema object from `sqlite_master`. Matches EXACTLY the Rust
 * `SqliteObject` DTO in `crates/cli/src/forensics_cmd.rs`:
 * `{ #[serde(rename = "type")] kind: String, name: String, sql: Option<String> }`.
 * The wire field is `type` (serde rename), so we mirror that name here.
 * `sql` (the CREATE statement) is `Option` -> nullable (null for internal
 * objects such as auto-indexes).
 */
export interface SqliteObject {
    type: string;
    name: string;
    sql: string | null;
}

/**
 * Top-level SQLite schema dump. Matches EXACTLY the Rust `SqliteReport`:
 * `{ db: String, object_count: usize, tables: Vec<SqliteObject> }`.
 * `tables` is always an array (never null) per the Rust DTO — it carries
 * EVERY object type (table / index / view / trigger), not only tables.
 */
export interface SqliteReport {
    db: string;
    object_count: number;
    tables: SqliteObject[];
}

/** A group of schema objects sharing the same `type`, for sectioned rendering. */
interface ObjectGroup {
    /** Raw `type` value as emitted by SQLite (table / index / view / trigger / other). */
    kind: string;
    /** French label for the group header. */
    label: string;
    /** Material symbol for the group header. */
    icon: string;
    /** Objects of this kind. */
    objects: SqliteObject[];
}

/** Display order for the known SQLite object kinds. */
const KIND_ORDER = ["table", "view", "index", "trigger"] as const;

/**
 * Typed M3 panel for `aphrody forensics sqlite --db <path>`.
 *
 * The CLI opens the database READ-ONLY and dumps `sqlite_master` only —
 * object type / name / CREATE statement, NEVER any row value (see the security
 * contract at the top of `forensics_cmd.rs`). There is no `--json` flag: the
 * command ALWAYS prints the {@link SqliteReport} JSON to stdout, so we parse
 * `res.stdout` directly.
 *
 * Renders a header card (db path + object count), then one section per object
 * kind (tables / views / indexes / triggers) — each object shown as a row with
 * a type chip and a collapsible CREATE statement. Falls back to a readable
 * error pane on non-zero exit or JSON parse failure (e.g. file not found, or a
 * path that is not a SQLite database).
 */
@Component({
    selector: "app-sqlite-schema-panel",
    imports: [
        FormsModule,
        MatButtonModule,
        MatCardModule,
        MatIconModule,
        MatProgressSpinnerModule,
        MatTooltipModule,
    ],
    template: `
        <mat-card class="run-card" appearance="outlined">
            <div class="run-row">
                <mat-icon class="material-symbols-outlined run-glyph">database</mat-icon>
                <div class="run-text">
                    <span class="run-title">Schéma SQLite (lecture seule)</span>
                    <span class="run-hint"
                        >Lit <code>sqlite_master</code> uniquement (noms + instructions CREATE),
                        jamais les valeurs (aphrody forensics sqlite).</span
                    >
                </div>
            </div>
            <div class="run-input">
                <input
                    class="path-input"
                    placeholder="Chemin de la base .db / .sqlite / .vscdb"
                    [value]="db()"
                    (input)="db.set($any($event.target).value)"
                    (keydown.enter)="analyze()"
                    [disabled]="running()"
                    aria-label="Chemin de la base SQLite à inspecter"
                />
                <button
                    mat-raised-button
                    color="primary"
                    (click)="analyze()"
                    [disabled]="running() || !db().trim()"
                >
                    <span class="btn-inner">
                        @if (running()) {
                            <mat-spinner [diameter]="18" />
                            <span>Analyse…</span>
                        } @else {
                            <mat-icon class="material-symbols-outlined">play_arrow</mat-icon>
                            <span>Analyser</span>
                        }
                    </span>
                </button>
            </div>
        </mat-card>

        @if (error(); as e) {
            <mat-card class="err-card" appearance="outlined">
                <div class="err-head">
                    <mat-icon class="material-symbols-outlined">error</mat-icon>
                    <b>Échec de l'inspection du schéma</b>
                </div>
                <pre class="err-body">{{ e }}</pre>
            </mat-card>
        }

        @if (report(); as r) {
            <!-- Header card -->
            <mat-card class="hdr-card" appearance="outlined">
                <div class="hdr-top">
                    <span class="fmt-chip">
                        <mat-icon class="material-symbols-outlined">database</mat-icon>
                        SQLite
                    </span>
                    <span class="kv"
                        ><span class="k">Objets de schéma</span
                        ><span class="v">{{ r.object_count }}</span></span
                    >
                </div>
                <div class="path-row">
                    <span class="k">Base</span>
                    <code class="db-path mono" [matTooltip]="r.db" matTooltipClass="path-tip">{{
                        r.db
                    }}</code>
                </div>
            </mat-card>

            @if (groups().length === 0) {
                <mat-card class="empty-card" appearance="outlined">
                    <mat-icon class="material-symbols-outlined">inbox</mat-icon>
                    <span>Aucun objet de schéma (base vide ou catalogue interne uniquement).</span>
                </mat-card>
            }

            @for (g of groups(); track g.kind) {
                <mat-card class="grp-card" appearance="outlined">
                    <div class="block-head">
                        <mat-icon class="material-symbols-outlined">{{ g.icon }}</mat-icon>
                        <span>{{ g.label }}</span>
                        <span class="count">{{ g.objects.length }}</span>
                    </div>
                    <ul class="obj-list">
                        @for (o of g.objects; track o.name; let i = $index) {
                            <li class="obj-item">
                                <button
                                    class="obj-row"
                                    type="button"
                                    (click)="toggle(g.kind, i)"
                                    [attr.aria-expanded]="isOpen(g.kind, i)"
                                    [disabled]="!o.sql"
                                >
                                    <span class="kind-chip" [attr.data-kind]="g.kind">{{
                                        o.type
                                    }}</span>
                                    <code class="obj-name mono">{{ o.name }}</code>
                                    @if (o.sql) {
                                        <mat-icon class="material-symbols-outlined chevron">{{
                                            isOpen(g.kind, i) ? "expand_less" : "expand_more"
                                        }}</mat-icon>
                                    } @else {
                                        <span class="no-sql">interne</span>
                                    }
                                </button>
                                @if (o.sql && isOpen(g.kind, i)) {
                                    <pre class="sql mono">{{ o.sql }}</pre>
                                }
                            </li>
                        }
                    </ul>
                </mat-card>
            }
        }
    `,
    styles: [
        `
            :host {
                display: flex;
                flex-direction: column;
                gap: 14px;
            }
            .mono {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
            }
            mat-card {
                border-radius: 16px;
                padding: 16px;
            }
            /* Run card */
            .run-row {
                display: flex;
                align-items: flex-start;
                gap: 12px;
                margin-bottom: 14px;
            }
            .run-glyph {
                color: var(--mat-sys-primary);
                flex: 0 0 auto;
            }
            .run-text {
                display: flex;
                flex-direction: column;
                min-width: 0;
            }
            .run-title {
                font-size: 15px;
                font-weight: 500;
            }
            .run-hint {
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
            }
            .run-hint code {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
            }
            .run-input {
                display: flex;
                gap: 10px;
                align-items: center;
                flex-wrap: wrap;
            }
            .path-input {
                flex: 1 1 320px;
                min-width: 0;
                padding: 11px 14px;
                border-radius: 12px;
                border: 1px solid var(--mat-sys-outline-variant);
                background: var(--mat-sys-surface-container);
                color: var(--mat-sys-on-surface);
                font: inherit;
                font-size: 14px;
                outline: none;
            }
            .path-input:focus {
                border-color: var(--mat-sys-primary);
            }
            .btn-inner {
                display: inline-flex;
                align-items: center;
                gap: 8px;
            }
            .btn-inner mat-spinner {
                display: inline-block;
            }
            /* Error card */
            .err-card {
                border-color: var(--mat-sys-error);
            }
            .err-head {
                display: flex;
                align-items: center;
                gap: 8px;
                color: var(--mat-sys-error);
                margin-bottom: 10px;
                font-size: 14px;
            }
            .err-body {
                margin: 0;
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 12px;
                line-height: 1.5;
                white-space: pre-wrap;
                word-break: break-word;
                color: var(--mat-sys-on-surface-variant);
                max-height: 280px;
                overflow: auto;
            }
            /* Header card */
            .hdr-top {
                display: flex;
                flex-wrap: wrap;
                align-items: center;
                gap: 12px 22px;
                margin-bottom: 14px;
            }
            .fmt-chip {
                display: inline-flex;
                align-items: center;
                gap: 6px;
                font-size: 13px;
                font-weight: 600;
                padding: 6px 14px;
                border-radius: 999px;
                background: color-mix(in srgb, #4285f4 22%, transparent);
                color: var(--mat-sys-on-surface);
            }
            .fmt-chip mat-icon {
                font-size: 18px;
                width: 18px;
                height: 18px;
            }
            .kv {
                display: flex;
                flex-direction: column;
                gap: 2px;
            }
            .kv .k,
            .path-row .k {
                font-size: 11px;
                text-transform: uppercase;
                letter-spacing: 0.04em;
                color: var(--mat-sys-on-surface-variant);
            }
            .kv .v {
                font-size: 14px;
                font-weight: 500;
            }
            .path-row {
                display: flex;
                align-items: center;
                gap: 12px;
                padding-top: 12px;
                border-top: 1px solid var(--mat-sys-outline-variant);
                min-width: 0;
            }
            .db-path {
                font-size: 13px;
                color: var(--mat-sys-on-surface);
                cursor: help;
                word-break: break-all;
            }
            /* Empty */
            .empty-card {
                display: flex;
                align-items: center;
                gap: 10px;
                color: var(--mat-sys-on-surface-variant);
                font-size: 13px;
            }
            .empty-card mat-icon {
                color: var(--mat-sys-on-surface-variant);
            }
            /* Block heads (shared) */
            .block-head {
                display: flex;
                align-items: center;
                gap: 8px;
                font-size: 14px;
                font-weight: 500;
                margin-bottom: 12px;
            }
            .block-head > mat-icon:first-child {
                color: var(--mat-sys-primary);
                font-size: 20px;
                width: 20px;
                height: 20px;
            }
            .block-head .count {
                font-size: 12px;
                font-weight: 600;
                padding: 2px 9px;
                border-radius: 999px;
                background: var(--mat-sys-secondary-container);
                color: var(--mat-sys-on-secondary-container);
            }
            /* Object list */
            .obj-list {
                list-style: none;
                margin: 0;
                padding: 0;
                display: flex;
                flex-direction: column;
                gap: 2px;
            }
            .obj-item {
                border-radius: 10px;
                overflow: hidden;
                background: var(--mat-sys-surface-container);
            }
            .obj-row {
                display: flex;
                align-items: center;
                gap: 10px;
                width: 100%;
                border: none;
                background: transparent;
                color: inherit;
                font: inherit;
                text-align: left;
                padding: 9px 12px;
                cursor: pointer;
            }
            .obj-row:disabled {
                cursor: default;
            }
            .obj-row:not(:disabled):hover {
                background: var(--mat-sys-surface-container-high);
            }
            .kind-chip {
                flex: 0 0 auto;
                font-size: 10px;
                font-weight: 600;
                letter-spacing: 0.03em;
                text-transform: uppercase;
                padding: 2px 8px;
                border-radius: 999px;
                background: var(--mat-sys-surface-container-highest);
                color: var(--mat-sys-on-surface-variant);
            }
            .kind-chip[data-kind="table"] {
                background: color-mix(in srgb, #4285f4 22%, transparent);
                color: var(--mat-sys-on-surface);
            }
            .kind-chip[data-kind="view"] {
                background: color-mix(in srgb, #34a853 22%, transparent);
                color: var(--mat-sys-on-surface);
            }
            .kind-chip[data-kind="index"] {
                background: color-mix(in srgb, #fbbc04 28%, transparent);
                color: var(--mat-sys-on-surface);
            }
            .kind-chip[data-kind="trigger"] {
                background: color-mix(in srgb, #ea4335 22%, transparent);
                color: var(--mat-sys-on-surface);
            }
            .obj-name {
                flex: 1 1 auto;
                font-size: 13px;
                min-width: 0;
                word-break: break-all;
            }
            .chevron {
                flex: 0 0 auto;
                color: var(--mat-sys-on-surface-variant);
            }
            .no-sql {
                flex: 0 0 auto;
                font-size: 11px;
                color: var(--mat-sys-on-surface-variant);
                font-style: italic;
            }
            .sql {
                margin: 0;
                padding: 12px 14px;
                font-size: 12px;
                line-height: 1.55;
                white-space: pre-wrap;
                word-break: break-word;
                color: var(--mat-sys-on-surface);
                background: var(--mat-sys-surface-container-low);
                border-top: 1px solid var(--mat-sys-outline-variant);
                max-height: 320px;
                overflow: auto;
            }
        `,
    ],
})
export class SqliteSchemaPanelComponent {
    private readonly aphrody = inject(AphrodyService);

    /** DB path entered by the user. */
    readonly db = signal("");
    /** True while a run is in flight. */
    readonly running = signal(false);
    /** Parsed report, or null before the first / failed run. */
    readonly report = signal<SqliteReport | null>(null);
    /** Human-readable error text on failure (null = no error). */
    readonly error = signal<string | null>(null);
    /** Set of expanded object keys ("<kind>#<index>"). */
    private readonly openKeys = signal<ReadonlySet<string>>(new Set());

    /** Schema objects grouped by kind, in a stable display order. */
    readonly groups = computed<ObjectGroup[]>(() => {
        const objs = this.report()?.tables ?? [];
        const byKind = new Map<string, SqliteObject[]>();
        for (const o of objs) {
            const key = (o.type ?? "").toLowerCase();
            const bucket = byKind.get(key);
            if (bucket) {
                bucket.push(o);
            } else {
                byKind.set(key, [o]);
            }
        }
        const ordered: ObjectGroup[] = [];
        // Known kinds first, in canonical order…
        for (const k of KIND_ORDER) {
            const list = byKind.get(k);
            if (list && list.length > 0) {
                ordered.push(this.makeGroup(k, list));
                byKind.delete(k);
            }
        }
        // …then any unexpected kinds, alphabetically.
        for (const k of [...byKind.keys()].sort((a, b) => a.localeCompare(b))) {
            ordered.push(this.makeGroup(k, byKind.get(k) as SqliteObject[]));
        }
        return ordered;
    });

    /** Build a display group (label + icon) for a SQLite object kind. */
    private makeGroup(kind: string, objects: SqliteObject[]): ObjectGroup {
        switch (kind) {
            case "table":
                return { kind, label: "Tables", icon: "table_chart", objects };
            case "view":
                return { kind, label: "Vues", icon: "view_agenda", objects };
            case "index":
                return { kind, label: "Index", icon: "format_list_numbered", objects };
            case "trigger":
                return { kind, label: "Déclencheurs", icon: "bolt", objects };
            default:
                return {
                    kind,
                    label: kind ? `Type « ${kind} »` : "Objets",
                    icon: "category",
                    objects,
                };
        }
    }

    /** Stable key for an object within its group. */
    private keyFor(kind: string, index: number): string {
        return `${kind}#${index}`;
    }

    /** True when the CREATE statement for this object is expanded. */
    isOpen(kind: string, index: number): boolean {
        return this.openKeys().has(this.keyFor(kind, index));
    }

    /** Toggle the CREATE statement disclosure for one object. */
    toggle(kind: string, index: number): void {
        const key = this.keyFor(kind, index);
        this.openKeys.update((prev) => {
            const next = new Set(prev);
            if (next.has(key)) {
                next.delete(key);
            } else {
                next.add(key);
            }
            return next;
        });
    }

    /** Run `aphrody forensics sqlite --db <path>` and parse the JSON report. */
    async analyze(): Promise<void> {
        const d = this.db().trim();
        if (!d || this.running()) {
            return;
        }
        this.running.set(true);
        this.error.set(null);
        this.report.set(null);
        this.openKeys.set(new Set());
        try {
            const res = await this.aphrody.exec(["forensics", "sqlite", "--db", d]);
            if (res.code !== 0) {
                this.error.set(
                    (res.stderr || res.stdout || `La commande a renvoyé le code ${res.code}.`)
                        .trim()
                        .slice(0, 2000),
                );
                return;
            }
            const out = res.stdout.trim();
            // Be tolerant of any leading log lines before the JSON object.
            const start = out.indexOf("{");
            if (start < 0) {
                this.error.set(
                    (res.stderr || "La commande n'a renvoyé aucun objet JSON exploitable.")
                        .trim()
                        .slice(0, 2000),
                );
                return;
            }
            const parsed = JSON.parse(out.slice(start)) as SqliteReport;
            // Defensive normalisation: the Rust DTO guarantees `tables` is an array,
            // but coerce so the template is safe against an unexpected payload.
            this.report.set({
                db: parsed.db ?? d,
                object_count: typeof parsed.object_count === "number" ? parsed.object_count : 0,
                tables: Array.isArray(parsed.tables)
                    ? parsed.tables.map((o) => ({
                          type: o.type ?? "",
                          name: o.name ?? "",
                          sql: o.sql ?? null,
                      }))
                    : [],
            });
        } catch (err) {
            this.error.set(`Sortie illisible (JSON invalide) : ${String(err)}`);
        } finally {
            this.running.set(false);
        }
    }
}
