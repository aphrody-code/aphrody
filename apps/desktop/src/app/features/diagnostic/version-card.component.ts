// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, computed, inject, signal } from "@angular/core";
import { MatIconModule } from "@angular/material/icon";

import { AphrodyService, Meta } from "../../core/aphrody.service";

/** Full envelope of `aphrody version --json`, matching `commands.rs:46`. */
export interface VersionInfo {
    version: string;
    commit: string;
    /** Build time as a Unix epoch in seconds (stringified by the CLI). */
    built: string;
    target: string;
    profile: string;
    repo: string;
    license: string;
    a2a: string;
}

/** One key/value row rendered inside the card. */
interface KvRow {
    label: string;
    value: string;
    /** When true, render the value monospaced (versions, targets, hashes). */
    mono?: boolean;
}

/**
 * Version + system card — parses `aphrody version --json` into a typed Material 3
 * card, crossed with host metadata from `meta()` (os / arch / family). The repo
 * URL opens via `@tauri-apps/plugin-opener` when running inside the Tauri shell,
 * and degrades to plain text in a browser. Defensive parse: non-JSON output is
 * reported with the raw stdout/stderr + exit code.
 */
@Component({
    selector: "app-version-card",
    imports: [MatIconModule],
    template: `
        <div class="card">
            <div class="card-head">
                <mat-icon class="material-symbols-outlined">info</mat-icon>
                <span>Version et système</span>
                @if (loading()) {
                    <mat-icon class="material-symbols-outlined spin">progress_activity</mat-icon>
                }
                <button
                    class="refresh"
                    (click)="load()"
                    [disabled]="loading()"
                    aria-label="Rafraîchir la version"
                >
                    <mat-icon class="material-symbols-outlined" [class.spin]="loading()"
                        >refresh</mat-icon
                    >
                </button>
            </div>

            @if (info(); as v) {
                <div class="banner">
                    <div class="banner-version">
                        <span class="ver-label">Version</span>
                        <b class="ver-num">{{ v.version }}</b>
                    </div>
                    <span class="chip">{{ v.license }}</span>
                    <span class="chip subtle">{{ v.profile }}</span>
                </div>

                <div class="kv">
                    @for (row of rows(); track row.label) {
                        <div class="kv-row">
                            <span class="kv-key">{{ row.label }}</span>
                            <span class="kv-val" [class.mono]="row.mono">{{ row.value }}</span>
                        </div>
                    }
                    <div class="kv-row">
                        <span class="kv-key">Dépôt</span>
                        <span class="kv-val">
                            @if (isTauri) {
                                <button class="link" (click)="openRepo(v.repo)">
                                    {{ v.repo }}
                                </button>
                            } @else {
                                <a
                                    class="link"
                                    [href]="v.repo"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    >{{ v.repo }}</a
                                >
                            }
                        </span>
                    </div>
                </div>
            } @else if (parseError()) {
                <div class="err-box">
                    <p><b>Version illisible.</b> {{ parseError() }}</p>
                    <p class="muted">Code de sortie : {{ rawCode() }}</p>
                    @if (rawStdout()) {
                        <pre class="raw">{{ rawStdout() }}</pre>
                    }
                    @if (rawStderr()) {
                        <pre class="raw err-raw">{{ rawStderr() }}</pre>
                    }
                </div>
            }
        </div>
    `,
    styles: [
        `
            :host {
                display: block;
            }
            .card {
                border-radius: 16px;
                background: var(--mat-sys-surface-container-low);
                padding: 16px 18px;
            }
            .card-head {
                display: flex;
                align-items: center;
                gap: 8px;
                margin-bottom: 14px;
            }
            .card-head > mat-icon:first-child {
                color: var(--mat-sys-primary);
                font-size: 20px;
                width: 20px;
                height: 20px;
            }
            .card-head > span {
                font-size: 15px;
                font-weight: 500;
                flex: 1 1 auto;
            }
            .refresh {
                width: 34px;
                height: 34px;
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
                animation: vc-spin 1s linear infinite;
            }
            @keyframes vc-spin {
                to {
                    transform: rotate(360deg);
                }
            }
            .banner {
                display: flex;
                align-items: center;
                gap: 12px;
                padding: 14px 16px;
                border-radius: 14px;
                background: var(--mat-sys-primary-container);
                color: var(--mat-sys-on-primary-container);
                margin-bottom: 14px;
            }
            .banner-version {
                display: flex;
                flex-direction: column;
                flex: 1 1 auto;
            }
            .ver-label {
                font-size: 12px;
                opacity: 0.8;
            }
            .ver-num {
                font-size: 22px;
                font-weight: 500;
                letter-spacing: -0.3px;
            }
            .chip {
                font-size: 12px;
                font-weight: 600;
                padding: 5px 12px;
                border-radius: 999px;
                flex: 0 0 auto;
                background: var(--mat-sys-on-primary-container);
                color: var(--mat-sys-primary-container);
            }
            .chip.subtle {
                background: color-mix(
                    in srgb,
                    var(--mat-sys-on-primary-container) 18%,
                    transparent
                );
                color: var(--mat-sys-on-primary-container);
                font-weight: 500;
            }
            .kv {
                display: flex;
                flex-direction: column;
                gap: 1px;
                border-radius: 12px;
                overflow: hidden;
                background: var(--mat-sys-outline-variant);
            }
            .kv-row {
                display: flex;
                gap: 16px;
                padding: 10px 14px;
                background: var(--mat-sys-surface-container);
                font-size: 13px;
            }
            .kv-key {
                flex: 0 0 38%;
                color: var(--mat-sys-on-surface-variant);
            }
            .kv-val {
                flex: 1 1 62%;
                color: var(--mat-sys-on-surface);
                word-break: break-word;
            }
            .kv-val.mono {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
            }
            .link {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 13px;
                color: var(--mat-sys-primary);
                background: none;
                border: none;
                padding: 0;
                cursor: pointer;
                text-decoration: none;
                word-break: break-all;
                text-align: left;
            }
            .link:hover {
                text-decoration: underline;
            }
            .err-box p {
                margin: 0 0 4px;
                font-size: 13px;
                color: var(--mat-sys-on-surface);
            }
            .err-box .muted {
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
            }
            .raw {
                margin: 8px 0 0;
                padding: 12px 14px;
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 12px;
                line-height: 1.5;
                color: var(--mat-sys-on-surface-variant);
                background: var(--mat-sys-surface-container);
                border-radius: 12px;
                white-space: pre-wrap;
                word-break: break-word;
                max-height: 220px;
                overflow: auto;
            }
            .raw.err-raw {
                color: var(--mat-sys-error);
            }
        `,
    ],
})
export class VersionCardComponent implements OnInit {
    private readonly aphrody = inject(AphrodyService);
    readonly isTauri = this.aphrody.isTauri;

    readonly loading = signal(false);
    readonly info = signal<VersionInfo | null>(null);
    readonly meta = signal<Meta | null>(null);
    readonly parseError = signal("");
    readonly rawCode = signal(0);
    readonly rawStdout = signal("");
    readonly rawStderr = signal("");

    /** Combined version + host rows for the key/value list. */
    readonly rows = computed<KvRow[]>(() => {
        const v = this.info();
        if (!v) return [];
        const m = this.meta();
        const out: KvRow[] = [
            { label: "Commit", value: this.shortCommit(v.commit), mono: true },
            { label: "Compilé le", value: this.formatBuilt(v.built), mono: true },
            { label: "Cible", value: v.target, mono: true },
            { label: "A2A", value: v.a2a },
        ];
        if (m) {
            out.push(
                { label: "Système hôte", value: m.target_os },
                { label: "Architecture", value: m.target_arch },
                { label: "Famille", value: m.family },
            );
        }
        return out;
    });

    ngOnInit(): void {
        void this.load();
    }

    /** Open the repo URL via the Tauri opener plugin (guarded dynamic import). */
    async openRepo(url: string): Promise<void> {
        try {
            const mod = (await import("@tauri-apps/plugin-opener")) as {
                openUrl: (u: string) => Promise<void>;
            };
            await mod.openUrl(url);
        } catch {
            // Plugin unavailable (e.g. web): fall back to a new tab.
            globalThis.open?.(url, "_blank", "noopener");
        }
    }

    /** Truncate a git SHA to its short form for display. */
    private shortCommit(commit: string): string {
        const c = commit.trim();
        if (!c || c === "unknown") return "inconnu";
        return c.length > 12 ? c.slice(0, 12) : c;
    }

    /**
     * Format the build timestamp. The CLI emits `built` as a Unix epoch in
     * seconds (stringified); convert it to a readable local date-time. Falls back
     * to the raw value when it is not a positive integer.
     */
    private formatBuilt(built: string): string {
        const epoch = Number.parseInt(built, 10);
        if (!Number.isFinite(epoch) || epoch <= 0) {
            return built && built !== "0" ? built : "inconnu";
        }
        const d = new Date(epoch * 1000);
        if (Number.isNaN(d.getTime())) return built;
        return d.toLocaleString("fr-FR", {
            year: "numeric",
            month: "short",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
        });
    }

    async load(): Promise<void> {
        this.loading.set(true);
        this.parseError.set("");
        try {
            const [res, meta] = await Promise.all([
                this.aphrody.exec(["version", "--json"]),
                this.aphrody.meta().catch(() => null),
            ]);
            this.meta.set(meta);
            this.rawCode.set(res.code);
            this.rawStdout.set(res.stdout);
            this.rawStderr.set(res.stderr);

            const out = res.stdout.trim();
            const start = out.indexOf("{");
            if (start < 0) {
                this.info.set(null);
                this.parseError.set(
                    res.stderr.trim() ||
                        "La commande version n'a renvoyé aucun objet JSON exploitable.",
                );
                return;
            }
            const parsed = JSON.parse(out.slice(start)) as VersionInfo;
            if (!parsed.version) {
                this.info.set(null);
                this.parseError.set("Le JSON de version ne contient pas de champ « version ».");
                return;
            }
            this.info.set(parsed);
        } catch (err) {
            this.info.set(null);
            this.parseError.set(`Lecture impossible : ${String(err)}`);
        } finally {
            this.loading.set(false);
        }
    }
}
