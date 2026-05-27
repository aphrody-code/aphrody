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
 * Stdout summary of `aphrody forensics map --target <dir> --out <dir>`.
 *
 * IMPORTANT: the rich per-file map (the `MapReport` with the full `files`
 * array) is written to `<out>/map.json` ON DISK — it is NOT printed. stdout
 * carries ONLY this machine-readable summary object, emitted by the `map`
 * function in `crates/cli/src/forensics_cmd.rs`:
 *   `{ "wrote": String, "file_count": usize, "hashed_count": usize,
 *      "secret_meta_only_count": usize }`.
 * (serde_json::json! field names are verbatim.) The desktop shell exposes no
 * arbitrary-path file read, so we surface this typed summary; the operator can
 * open the written `map.json` for the full per-file detail.
 */
export interface MapSummary {
    wrote: string;
    file_count: number;
    hashed_count: number;
    secret_meta_only_count: number;
}

/** One stat tile rendered from the summary. */
interface StatTile {
    label: string;
    value: number;
    icon: string;
    /** Visual tone — "secret" tints the secret-skipped count amber. */
    tone: "default" | "ok" | "secret";
    hint: string;
}

/**
 * Typed M3 panel for `aphrody forensics map`.
 *
 * Walks a target directory in parallel and writes a classified map to
 * `<out>/map.json` — extension + size + mtime per file, SHA-256 for non-secret
 * files at or below 1 MiB, and metadata ONLY (never opened) for anything on the
 * secret denylist (cookies / tokens / leveldb / Login Data / …). File CONTENTS
 * of secret-bearing paths are never read, so no secret bytes can leak (see the
 * security contract at the top of `forensics_cmd.rs`).
 *
 * stdout carries only the {@link MapSummary}; this panel renders it as stat
 * tiles plus the path of the written `map.json`. There is no `--json` flag —
 * the summary is always JSON. Falls back to a readable error pane on non-zero
 * exit (e.g. target is not a directory) or JSON parse failure.
 */
@Component({
    selector: "app-fs-map-panel",
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
                <mat-icon class="material-symbols-outlined run-glyph">account_tree</mat-icon>
                <div class="run-text">
                    <span class="run-title">Carte du système de fichiers</span>
                    <span class="run-hint"
                        >Parcours parallèle + classification (extension, taille, SHA-256). Les
                        fichiers sensibles (cookies, jetons, leveldb…) sont recensés sans jamais
                        être ouverts.</span
                    >
                </div>
            </div>
            <div class="run-fields">
                <label class="field">
                    <span class="field-label">Répertoire à cartographier</span>
                    <input
                        class="path-input"
                        placeholder="ex. C:\\Users\\moi\\.gemini"
                        [value]="target()"
                        (input)="target.set($any($event.target).value)"
                        (keydown.enter)="analyze()"
                        [disabled]="running()"
                        aria-label="Répertoire cible à cartographier"
                    />
                </label>
                <label class="field">
                    <span class="field-label">Répertoire de sortie (reçoit map.json)</span>
                    <input
                        class="path-input"
                        placeholder="ex. var/data/forensics"
                        [value]="out()"
                        (input)="out.set($any($event.target).value)"
                        (keydown.enter)="analyze()"
                        [disabled]="running()"
                        aria-label="Répertoire de sortie pour map.json"
                    />
                </label>
            </div>
            <div class="run-actions">
                <button
                    mat-raised-button
                    color="primary"
                    (click)="analyze()"
                    [disabled]="running() || !target().trim() || !out().trim()"
                >
                    <span class="btn-inner">
                        @if (running()) {
                            <mat-spinner [diameter]="18" />
                            <span>Analyse…</span>
                        } @else {
                            <mat-icon class="material-symbols-outlined">play_arrow</mat-icon>
                            <span>Cartographier</span>
                        }
                    </span>
                </button>
                <span class="safety">
                    <mat-icon class="material-symbols-outlined">shield</mat-icon>
                    Lecture seule — aucun contenu sensible n'est ouvert ni copié.
                </span>
            </div>
        </mat-card>

        @if (error(); as e) {
            <mat-card class="err-card" appearance="outlined">
                <div class="err-head">
                    <mat-icon class="material-symbols-outlined">error</mat-icon>
                    <b>Échec de la cartographie</b>
                </div>
                <pre class="err-body">{{ e }}</pre>
            </mat-card>
        }

        @if (report(); as r) {
            <div class="tiles">
                @for (t of tiles(); track t.label) {
                    <mat-card class="tile" appearance="outlined" [attr.data-tone]="t.tone">
                        <mat-icon class="material-symbols-outlined tile-icon">{{
                            t.icon
                        }}</mat-icon>
                        <span class="tile-value">{{ t.value.toLocaleString("fr-FR") }}</span>
                        <span class="tile-label">{{ t.label }}</span>
                        <span class="tile-hint">{{ t.hint }}</span>
                    </mat-card>
                }
            </div>

            <mat-card class="wrote-card" appearance="outlined">
                <div class="block-head">
                    <mat-icon class="material-symbols-outlined">description</mat-icon>
                    <span>Carte écrite</span>
                </div>
                <code class="wrote-path mono" [matTooltip]="r.wrote" matTooltipClass="path-tip">{{
                    r.wrote
                }}</code>
                <p class="wrote-note">
                    Le détail par fichier (chemin, taille, extension, SHA-256, horodatage) est dans
                    ce
                    <code>map.json</code>. Ouvrez-le pour la vue exhaustive.
                </p>
            </mat-card>
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
            .run-fields {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
                gap: 12px;
                margin-bottom: 14px;
            }
            .field {
                display: flex;
                flex-direction: column;
                gap: 5px;
                min-width: 0;
            }
            .field-label {
                font-size: 11px;
                text-transform: uppercase;
                letter-spacing: 0.04em;
                color: var(--mat-sys-on-surface-variant);
            }
            .path-input {
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
            .run-actions {
                display: flex;
                align-items: center;
                gap: 14px;
                flex-wrap: wrap;
            }
            .btn-inner {
                display: inline-flex;
                align-items: center;
                gap: 8px;
            }
            .btn-inner mat-spinner {
                display: inline-block;
            }
            .safety {
                display: inline-flex;
                align-items: center;
                gap: 6px;
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
            }
            .safety mat-icon {
                font-size: 16px;
                width: 16px;
                height: 16px;
                color: #34a853;
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
            /* Stat tiles */
            .tiles {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
                gap: 12px;
            }
            .tile {
                display: flex;
                flex-direction: column;
                gap: 2px;
                padding: 16px;
            }
            .tile-icon {
                color: var(--mat-sys-primary);
                font-size: 22px;
                width: 22px;
                height: 22px;
                margin-bottom: 4px;
            }
            .tile[data-tone="ok"] .tile-icon {
                color: #34a853;
            }
            .tile[data-tone="secret"] .tile-icon {
                color: #c9920a;
            }
            .tile-value {
                font-size: 26px;
                font-weight: 500;
                font-variant-numeric: tabular-nums;
                line-height: 1.1;
            }
            .tile-label {
                font-size: 13px;
                font-weight: 500;
                color: var(--mat-sys-on-surface);
            }
            .tile-hint {
                font-size: 11px;
                color: var(--mat-sys-on-surface-variant);
            }
            /* Wrote card */
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
            .wrote-path {
                display: block;
                font-size: 13px;
                color: var(--mat-sys-on-surface);
                word-break: break-all;
                cursor: help;
                padding: 10px 12px;
                border-radius: 10px;
                background: var(--mat-sys-surface-container);
            }
            .wrote-note {
                margin: 10px 0 0;
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
                line-height: 1.5;
            }
            .wrote-note code {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
            }
        `,
    ],
})
export class FsMapPanelComponent {
    private readonly aphrody = inject(AphrodyService);

    /** Target directory to walk. */
    readonly target = signal("");
    /** Output directory (receives map.json). */
    readonly out = signal("");
    /** True while a run is in flight. */
    readonly running = signal(false);
    /** Parsed summary, or null before the first / failed run. */
    readonly report = signal<MapSummary | null>(null);
    /** Human-readable error text on failure (null = no error). */
    readonly error = signal<string | null>(null);

    /** Summary turned into stat tiles for the grid. */
    readonly tiles = computed<StatTile[]>(() => {
        const r = this.report();
        if (!r) {
            return [];
        }
        return [
            {
                label: "Fichiers cartographiés",
                value: r.file_count,
                icon: "description",
                tone: "default",
                hint: "Total des fichiers réguliers parcourus.",
            },
            {
                label: "Fichiers hachés",
                value: r.hashed_count,
                icon: "fingerprint",
                tone: "ok",
                hint: "SHA-256 calculé (non sensibles, ≤ 1 Mio).",
            },
            {
                label: "Sensibles (métadonnées seules)",
                value: r.secret_meta_only_count,
                icon: "lock",
                tone: "secret",
                hint: "Chemins sur liste sensible — jamais ouverts.",
            },
        ];
    });

    /** Run `aphrody forensics map --target <dir> --out <dir>` and parse the summary. */
    async analyze(): Promise<void> {
        const t = this.target().trim();
        const o = this.out().trim();
        if (!t || !o || this.running()) {
            return;
        }
        this.running.set(true);
        this.error.set(null);
        this.report.set(null);
        try {
            const res = await this.aphrody.exec(["forensics", "map", "--target", t, "--out", o]);
            if (res.code !== 0) {
                this.error.set(
                    (res.stderr || res.stdout || `La commande a renvoyé le code ${res.code}.`)
                        .trim()
                        .slice(0, 2000),
                );
                return;
            }
            const out = res.stdout.trim();
            const start = out.indexOf("{");
            if (start < 0) {
                this.error.set(
                    (res.stderr || "La commande n'a renvoyé aucun objet JSON exploitable.")
                        .trim()
                        .slice(0, 2000),
                );
                return;
            }
            const parsed = JSON.parse(out.slice(start)) as MapSummary;
            this.report.set({
                wrote: parsed.wrote ?? "",
                file_count: typeof parsed.file_count === "number" ? parsed.file_count : 0,
                hashed_count: typeof parsed.hashed_count === "number" ? parsed.hashed_count : 0,
                secret_meta_only_count:
                    typeof parsed.secret_meta_only_count === "number"
                        ? parsed.secret_meta_only_count
                        : 0,
            });
        } catch (err) {
            this.error.set(`Sortie illisible (JSON invalide) : ${String(err)}`);
        } finally {
            this.running.set(false);
        }
    }
}
