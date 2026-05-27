// SPDX-License-Identifier: Apache-2.0
import { Component, computed, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatButtonModule } from "@angular/material/button";
import { MatCardModule } from "@angular/material/card";
import { MatIconModule } from "@angular/material/icon";
import { MatProgressSpinnerModule } from "@angular/material/progress-spinner";

import { AphrodyService } from "../../core/aphrody.service";

/**
 * Typed result of a DNS OSINT reconnaissance run.
 *
 * `aphrody dns <domaine>` emits PLAIN TEXT (no `--json` flag exists). The Rust
 * `DnsCommand` (crates/cli/src/commands.rs:421) calls
 * `backend::dns::DnsRecon::run_osint` (which returns `Vec<String>` of subdomain
 * hostnames) and prints:
 *
 * ```text
 * [====== WINCLEAN FORENSIC - MAXIMUM DNS RECON ======]
 * [~] Cible: <domaine>
 * [~] 1/3 - Lancement de l'OSINT Passif (Agrégation multi-sources)...
 * [+] Découverte OSINT terminée: <N> sous-domaines uniques trouvés !
 *   - sub1.example.com
 *   - sub2.example.com
 *   ... (jusqu'à 10 lignes)
 *   ... et <M> autres
 * ```
 *
 * On error it prints `[-] Erreur lors de la résolution OSINT: <e>`.
 *
 * IMPORTANT: the CLI TRUNCATES the printed list to the first 10 subdomains
 * (`results.iter().take(10)`), so `subdomains` here holds at most 10 entries
 * even when `total` is larger. `total` is read from the `[+]` summary line so
 * the header counter is accurate; `hiddenCount` reflects the un-printed
 * remainder (`... et M autres`).
 */
export interface DnsReport {
    /** Resolved target domain (echoed from the `[~] Cible:` line). */
    domain: string;
    /** Total unique subdomains found, parsed from the `[+]` summary line. */
    total: number;
    /** The subdomains actually printed by the CLI (capped at 10). */
    subdomains: string[];
    /** Count of subdomains found but not printed (`... et M autres`). */
    hiddenCount: number;
}

/**
 * Typed M3 panel for `aphrody dns`. Takes a domain, runs `dns <domaine>`
 * in-process via {@link AphrodyService}, parses the plain-text OSINT report into
 * a strongly-typed {@link DnsReport}, and renders a header card (target + unique
 * subdomain counter) plus a searchable/scrollable subdomain grid. Falls back to
 * a readable error pane on non-zero exit, an OSINT error line, or unparseable
 * output.
 */
@Component({
    selector: "app-dns-recon-panel",
    imports: [FormsModule, MatButtonModule, MatCardModule, MatIconModule, MatProgressSpinnerModule],
    template: `
        <mat-card class="run-card" appearance="outlined">
            <div class="run-row">
                <mat-icon class="material-symbols-outlined run-glyph">dns</mat-icon>
                <div class="run-text">
                    <span class="run-title">Reconnaissance DNS typée</span>
                    <span class="run-hint"
                        >Énumération OSINT passive des sous-domaines via crt.sh + HackerTarget
                        (aphrody dns).</span
                    >
                </div>
            </div>
            <div class="run-input">
                <input
                    class="text-input"
                    placeholder="Domaine (ex. example.com)"
                    [value]="domain()"
                    (input)="domain.set($any($event.target).value)"
                    (keydown.enter)="run()"
                    [disabled]="running()"
                    aria-label="Domaine à analyser"
                    autocapitalize="off"
                    autocorrect="off"
                    spellcheck="false"
                />
                <button
                    mat-raised-button
                    color="primary"
                    (click)="run()"
                    [disabled]="running() || !domain().trim()"
                >
                    <span class="btn-inner">
                        @if (running()) {
                            <mat-spinner [diameter]="18" />
                            <span>Reconnaissance…</span>
                        } @else {
                            <mat-icon class="material-symbols-outlined">travel_explore</mat-icon>
                            <span>Reconnaissance</span>
                        }
                    </span>
                </button>
            </div>
        </mat-card>

        @if (error(); as e) {
            <mat-card class="err-card" appearance="outlined">
                <div class="err-head">
                    <mat-icon class="material-symbols-outlined">error</mat-icon>
                    <b>Échec de la reconnaissance DNS</b>
                </div>
                <pre class="err-body">{{ e }}</pre>
            </mat-card>
        }

        @if (report(); as r) {
            <mat-card class="hdr-card" appearance="outlined">
                <div class="hdr-top">
                    <span class="target-chip">
                        <mat-icon class="material-symbols-outlined">public</mat-icon>
                        {{ r.domain }}
                    </span>
                    <span class="kv">
                        <span class="k">Sous-domaines uniques</span>
                        <span class="v">{{ r.total }}</span>
                    </span>
                    @if (r.hiddenCount > 0) {
                        <span class="kv">
                            <span class="k">Affichés par le CLI</span>
                            <span class="v"
                                >{{ r.subdomains.length }}
                                <span class="dim">/ {{ r.total }}</span></span
                            >
                        </span>
                    }
                </div>
                @if (r.hiddenCount > 0) {
                    <p class="note">
                        <mat-icon class="material-symbols-outlined">info</mat-icon>
                        Le CLI ne liste que les 10 premiers sous-domaines ;
                        {{ r.hiddenCount }} autres ne sont pas détaillés.
                    </p>
                }
            </mat-card>

            @if (r.subdomains.length > 0) {
                <mat-card class="list-card" appearance="outlined">
                    <div class="block-head">
                        <mat-icon class="material-symbols-outlined">lan</mat-icon>
                        <span>Sous-domaines</span>
                        <span class="count">{{ filtered().length }}</span>
                        @if (r.subdomains.length > 6) {
                            <input
                                class="filter-input"
                                placeholder="Filtrer…"
                                [value]="filter()"
                                (input)="filter.set($any($event.target).value)"
                                aria-label="Filtrer les sous-domaines"
                                autocapitalize="off"
                                autocorrect="off"
                                spellcheck="false"
                            />
                        }
                    </div>
                    @if (filtered().length === 0) {
                        <p class="empty">Aucun sous-domaine ne correspond au filtre.</p>
                    } @else {
                        <ul class="sub-grid">
                            @for (sub of filtered(); track sub) {
                                <li class="sub-item">
                                    <mat-icon class="material-symbols-outlined sub-glyph"
                                        >subdirectory_arrow_right</mat-icon
                                    >
                                    <code class="mono">{{ sub }}</code>
                                </li>
                            }
                        </ul>
                    }
                </mat-card>
            } @else {
                <mat-card class="list-card" appearance="outlined">
                    <p class="empty">
                        <mat-icon class="material-symbols-outlined">search_off</mat-icon>
                        Aucun sous-domaine détaillé n'a été retourné pour ce domaine.
                    </p>
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
            .dim {
                color: var(--mat-sys-on-surface-variant);
            }
            mat-card {
                border-radius: 16px;
                padding: 16px;
            }
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
            .run-input {
                display: flex;
                gap: 10px;
                align-items: center;
                flex-wrap: wrap;
            }
            .text-input {
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
            .text-input:focus {
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
            .hdr-top {
                display: flex;
                flex-wrap: wrap;
                align-items: center;
                gap: 12px 22px;
            }
            .target-chip {
                display: inline-flex;
                align-items: center;
                gap: 6px;
                font-size: 13px;
                font-weight: 600;
                padding: 6px 14px;
                border-radius: 999px;
                background: color-mix(in srgb, #4285f4 22%, transparent);
                color: var(--mat-sys-on-surface);
                word-break: break-all;
            }
            .target-chip mat-icon {
                font-size: 18px;
                width: 18px;
                height: 18px;
            }
            .kv {
                display: flex;
                flex-direction: column;
                gap: 2px;
            }
            .kv .k {
                font-size: 11px;
                text-transform: uppercase;
                letter-spacing: 0.04em;
                color: var(--mat-sys-on-surface-variant);
            }
            .kv .v {
                font-size: 18px;
                font-weight: 600;
            }
            .note {
                display: flex;
                align-items: center;
                gap: 8px;
                margin: 14px 0 0;
                padding-top: 12px;
                border-top: 1px solid var(--mat-sys-outline-variant);
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
            }
            .note mat-icon {
                font-size: 18px;
                width: 18px;
                height: 18px;
                flex: 0 0 auto;
            }
            .block-head {
                display: flex;
                align-items: center;
                gap: 8px;
                font-size: 14px;
                font-weight: 500;
                margin-bottom: 12px;
                flex-wrap: wrap;
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
            .filter-input {
                margin-left: auto;
                flex: 0 1 220px;
                min-width: 120px;
                padding: 7px 12px;
                border-radius: 10px;
                border: 1px solid var(--mat-sys-outline-variant);
                background: var(--mat-sys-surface-container);
                color: var(--mat-sys-on-surface);
                font: inherit;
                font-size: 13px;
                outline: none;
            }
            .filter-input:focus {
                border-color: var(--mat-sys-primary);
            }
            .sub-grid {
                list-style: none;
                margin: 0;
                padding: 0;
                display: grid;
                grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
                gap: 8px;
                max-height: 360px;
                overflow: auto;
            }
            .sub-item {
                display: flex;
                align-items: center;
                gap: 8px;
                padding: 9px 12px;
                border-radius: 12px;
                background: var(--mat-sys-surface-container);
                min-width: 0;
            }
            .sub-glyph {
                font-size: 18px;
                width: 18px;
                height: 18px;
                flex: 0 0 auto;
                color: var(--mat-sys-on-surface-variant);
            }
            .sub-item code {
                font-size: 13px;
                word-break: break-all;
                color: var(--mat-sys-on-surface);
            }
            .empty {
                display: flex;
                align-items: center;
                gap: 8px;
                margin: 0;
                font-size: 13px;
                color: var(--mat-sys-on-surface-variant);
            }
            .empty mat-icon {
                font-size: 20px;
                width: 20px;
                height: 20px;
            }
        `,
    ],
})
export class DnsReconPanelComponent {
    private readonly aphrody = inject(AphrodyService);

    /** Domain entered by the user. */
    readonly domain = signal("");
    /** True while a run is in flight. */
    readonly running = signal(false);
    /** Parsed report, or null before the first/failed run. */
    readonly report = signal<DnsReport | null>(null);
    /** Human-readable error text on failure (null = no error). */
    readonly error = signal<string | null>(null);
    /** Client-side subdomain filter. */
    readonly filter = signal("");

    /** Subdomains matching the current filter (case-insensitive substring). */
    readonly filtered = computed(() => {
        const subs = this.report()?.subdomains ?? [];
        const q = this.filter().trim().toLowerCase();
        if (!q) {
            return subs;
        }
        return subs.filter((s) => s.toLowerCase().includes(q));
    });

    /** Run `aphrody dns <domaine>` and parse the plain-text OSINT report. */
    async run(): Promise<void> {
        const d = this.domain().trim();
        if (!d || this.running()) {
            return;
        }
        this.running.set(true);
        this.error.set(null);
        this.report.set(null);
        this.filter.set("");
        try {
            const res = await this.aphrody.exec(["dns", d]);
            if (res.code !== 0) {
                this.error.set(
                    (res.stderr || res.stdout || `La commande a renvoyé le code ${res.code}.`)
                        .trim()
                        .slice(0, 2000),
                );
                return;
            }
            const parsed = this.parse(res.stdout, d);
            if (!parsed) {
                // No usable summary line: surface the raw output rather than guess.
                this.error.set((res.stdout || res.stderr || "Sortie vide.").trim().slice(0, 2000));
                return;
            }
            this.report.set(parsed);
        } catch (err) {
            this.error.set(`Reconnaissance impossible : ${String(err)}`);
        } finally {
            this.running.set(false);
        }
    }

    /**
     * Parse the plain-text `aphrody dns` output. Returns null when no usable
     * summary/subdomain data is present (caller surfaces the raw output instead).
     */
    private parse(stdout: string, domain: string): DnsReport | null {
        const lines = stdout.split(/\r?\n/);

        // OSINT error line short-circuits to the error pane via a null return so
        // the caller shows the raw stdout (which contains the error detail).
        if (lines.some((l) => l.includes("[-] Erreur lors de la résolution OSINT"))) {
            return null;
        }

        let total: number | null = null;
        let hiddenCount = 0;
        const subdomains: string[] = [];

        // Echoed target ("[~] Cible: <domaine>") — prefer it over the raw input.
        let resolvedDomain = domain;

        for (const raw of lines) {
            const line = raw.trim();
            const cible = /^\[~\]\s*Cible\s*:\s*(.+)$/.exec(line);
            if (cible) {
                resolvedDomain = cible[1].trim() || domain;
                continue;
            }
            // "[+] Découverte OSINT terminée: <N> sous-domaines uniques trouvés !"
            const summary = /(\d+)\s+sous-domaines?\s+uniques/.exec(line);
            if (summary) {
                total = Number.parseInt(summary[1], 10);
                continue;
            }
            // "  ... et <M> autres"
            const more = /^\.\.\.\s*et\s+(\d+)\s+autres?$/.exec(line);
            if (more) {
                hiddenCount = Number.parseInt(more[1], 10);
                continue;
            }
            // "  - <subdomain>"
            const sub = /^-\s+(\S.*)$/.exec(line);
            if (sub) {
                subdomains.push(sub[1].trim());
            }
        }

        // Require at least a summary count or a printed subdomain to consider the
        // run parseable; otherwise the output is unexpected.
        if (total === null && subdomains.length === 0) {
            return null;
        }

        const resolvedTotal = total ?? subdomains.length + hiddenCount;
        // Keep the counter consistent if the summary was missing but a remainder
        // line was present.
        const finalTotal = Math.max(resolvedTotal, subdomains.length + hiddenCount);

        return {
            domain: resolvedDomain,
            total: finalTotal,
            subdomains,
            hiddenCount,
        };
    }
}
