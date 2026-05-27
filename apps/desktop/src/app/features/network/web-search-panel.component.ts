// SPDX-License-Identifier: Apache-2.0
import { Component, computed, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatButtonModule } from "@angular/material/button";
import { MatCardModule } from "@angular/material/card";
import { MatIconModule } from "@angular/material/icon";
import { MatProgressSpinnerModule } from "@angular/material/progress-spinner";

import { AphrodyService } from "../../core/aphrody.service";

/**
 * One typed web search result.
 *
 * `aphrody search <requête…>` emits PLAIN TEXT (no `--json` flag exists). The
 * Rust `SearchCommand` (crates/cli/src/commands.rs:923) scrapes
 * `html.duckduckgo.com`, unwraps the `uddg=` redirect to the real target URL,
 * and prints up to 5 results, each as:
 *
 * ```text
 * Recherche : <requête>          (prefixed by a U+1F50D search glyph)
 *
 * <title>            (bold green)
 * <url>              (cyan, underlined)
 * <snippet>          (optional — omitted when empty)
 * ```
 *
 * Results are separated by a leading blank line. When nothing is found it
 * prints a single line containing "Aucun résultat trouvé".
 *
 * Because the output is line-oriented and the only stable structural cue is
 * "a result block starts with a URL line", we reconstruct blocks by detecting
 * URL lines and attaching the preceding non-URL line as the title and any
 * following non-URL line as the snippet.
 */
export interface SearchResult {
    /** Result title (line preceding the URL). */
    title: string;
    /** Absolute result URL (already un-wrapped from the DDG redirect by the CLI). */
    url: string;
    /** Optional snippet/description (line following the URL). */
    snippet: string;
}

/** Matches an http(s) URL line emitted by the CLI for a result. */
const URL_LINE = /^https?:\/\/\S+$/i;

/**
 * Typed M3 panel for `aphrody search`. Takes a free-text query, runs
 * `search <requête…>` in-process via {@link AphrodyService}, parses the
 * plain-text DuckDuckGo results into typed {@link SearchResult} blocks, and
 * renders them as Material cards with clickable URLs (opened via the Tauri
 * opener plugin, with a `window.open` web fallback). Falls back to a readable
 * raw-text pane when the output can't be structured, and an error pane on
 * non-zero exit.
 */
@Component({
    selector: "app-web-search-panel",
    imports: [FormsModule, MatButtonModule, MatCardModule, MatIconModule, MatProgressSpinnerModule],
    template: `
        <mat-card class="run-card" appearance="outlined">
            <div class="run-row">
                <mat-icon class="material-symbols-outlined run-glyph">search</mat-icon>
                <div class="run-text">
                    <span class="run-title">Recherche web typée</span>
                    <span class="run-hint"
                        >Résultats web sans clé ni navigateur, via DuckDuckGo (aphrody
                        search).</span
                    >
                </div>
            </div>
            <div class="run-input">
                <input
                    class="text-input"
                    placeholder="Requête de recherche"
                    [value]="query()"
                    (input)="query.set($any($event.target).value)"
                    (keydown.enter)="run()"
                    [disabled]="running()"
                    aria-label="Requête de recherche"
                />
                <button
                    mat-raised-button
                    color="primary"
                    (click)="run()"
                    [disabled]="running() || !query().trim()"
                >
                    <span class="btn-inner">
                        @if (running()) {
                            <mat-spinner [diameter]="18" />
                            <span>Recherche…</span>
                        } @else {
                            <mat-icon class="material-symbols-outlined">search</mat-icon>
                            <span>Rechercher</span>
                        }
                    </span>
                </button>
            </div>
        </mat-card>

        @if (error(); as e) {
            <mat-card class="err-card" appearance="outlined">
                <div class="err-head">
                    <mat-icon class="material-symbols-outlined">error</mat-icon>
                    <b>Échec de la recherche</b>
                </div>
                <pre class="err-body">{{ e }}</pre>
            </mat-card>
        }

        @if (results(); as list) {
            <div class="res-head">
                <mat-icon class="material-symbols-outlined">travel_explore</mat-icon>
                <span>Résultats pour « {{ ranQuery() }} »</span>
                <span class="count">{{ list.length }}</span>
            </div>
            @if (list.length === 0) {
                <mat-card class="empty-card" appearance="outlined">
                    <p class="empty">
                        <mat-icon class="material-symbols-outlined">search_off</mat-icon>
                        Aucun résultat trouvé (DuckDuckGo a renvoyé une page vide — réessayer plus
                        tard).
                    </p>
                </mat-card>
            } @else {
                @for (r of list; track r.url + r.title) {
                    <mat-card class="res-card" appearance="outlined">
                        <a
                            class="res-title"
                            [href]="r.url"
                            target="_blank"
                            rel="noopener noreferrer"
                            (click)="open($event, r.url)"
                            >{{ r.title }}</a
                        >
                        <a
                            class="res-url"
                            [href]="r.url"
                            target="_blank"
                            rel="noopener noreferrer"
                            (click)="open($event, r.url)"
                        >
                            <mat-icon class="material-symbols-outlined link-glyph">link</mat-icon>
                            <span class="url-text">{{ r.url }}</span>
                        </a>
                        @if (r.snippet) {
                            <p class="res-snippet">{{ r.snippet }}</p>
                        }
                    </mat-card>
                }
            }
        } @else if (rawFallback(); as raw) {
            <mat-card class="raw-card" appearance="outlined">
                <div class="block-head">
                    <mat-icon class="material-symbols-outlined">notes</mat-icon>
                    <span>Sortie brute</span>
                </div>
                <pre class="raw-body">{{ raw }}</pre>
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
            .err-body,
            .raw-body {
                margin: 0;
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 12px;
                line-height: 1.5;
                white-space: pre-wrap;
                word-break: break-word;
                color: var(--mat-sys-on-surface-variant);
                max-height: 320px;
                overflow: auto;
            }
            .res-head {
                display: flex;
                align-items: center;
                gap: 8px;
                font-size: 14px;
                font-weight: 500;
                margin-bottom: -2px;
            }
            .res-head > mat-icon {
                color: var(--mat-sys-primary);
                font-size: 20px;
                width: 20px;
                height: 20px;
            }
            .res-head .count,
            .block-head .count {
                font-size: 12px;
                font-weight: 600;
                padding: 2px 9px;
                border-radius: 999px;
                background: var(--mat-sys-secondary-container);
                color: var(--mat-sys-on-secondary-container);
            }
            .res-card {
                display: flex;
                flex-direction: column;
                gap: 6px;
                padding: 14px 16px;
            }
            .res-title {
                font-size: 16px;
                font-weight: 500;
                color: var(--mat-sys-primary);
                text-decoration: none;
                word-break: break-word;
                line-height: 1.35;
            }
            .res-title:hover {
                text-decoration: underline;
            }
            .res-url {
                display: inline-flex;
                align-items: center;
                gap: 5px;
                font-size: 12.5px;
                color: var(--mat-sys-on-surface-variant);
                text-decoration: none;
                min-width: 0;
            }
            .res-url:hover .url-text {
                text-decoration: underline;
            }
            .link-glyph {
                font-size: 15px;
                width: 15px;
                height: 15px;
                flex: 0 0 auto;
            }
            .url-text {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                word-break: break-all;
            }
            .res-snippet {
                margin: 2px 0 0;
                font-size: 13.5px;
                line-height: 1.5;
                color: var(--mat-sys-on-surface);
                word-break: break-word;
            }
            .block-head {
                display: flex;
                align-items: center;
                gap: 8px;
                font-size: 14px;
                font-weight: 500;
                margin-bottom: 12px;
            }
            .block-head > mat-icon {
                color: var(--mat-sys-primary);
                font-size: 20px;
                width: 20px;
                height: 20px;
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
export class WebSearchPanelComponent {
    private readonly aphrody = inject(AphrodyService);

    /** Query entered by the user. */
    readonly query = signal("");
    /** True while a run is in flight. */
    readonly running = signal(false);
    /** Parsed result list, or null before the first/failed run. */
    readonly results = signal<SearchResult[] | null>(null);
    /** Raw stdout shown when parsing yields no structured results (null = none). */
    readonly rawFallbackText = signal<string | null>(null);
    /** Human-readable error text on failure (null = no error). */
    readonly error = signal<string | null>(null);
    /** Query echoed in the results header (snapshot at run time). */
    readonly ranQuery = signal("");

    /** Raw fallback text, suppressed once structured results exist. */
    readonly rawFallback = computed(() => (this.results() ? null : this.rawFallbackText()));

    /** Run `aphrody search <requête…>` and parse the plain-text results. */
    async run(): Promise<void> {
        const q = this.query().trim();
        if (!q || this.running()) {
            return;
        }
        this.running.set(true);
        this.error.set(null);
        this.results.set(null);
        this.rawFallbackText.set(null);
        this.ranQuery.set(q);
        try {
            // `query` is a trailing var-arg (Vec<String>): split on whitespace so a
            // multi-word query maps to distinct argv tokens like the shell would.
            const res = await this.aphrody.exec(["search", ...q.split(/\s+/).filter(Boolean)]);
            if (res.code !== 0) {
                this.error.set(
                    (res.stderr || res.stdout || `La commande a renvoyé le code ${res.code}.`)
                        .trim()
                        .slice(0, 2000),
                );
                return;
            }
            const out = res.stdout ?? "";
            if (/Aucun résultat trouvé/i.test(out)) {
                // Explicit empty result set — show the typed (empty) results UI.
                this.results.set([]);
                return;
            }
            const parsed = this.parse(out);
            if (parsed.length > 0) {
                this.results.set(parsed);
            } else {
                // Unstructured but non-empty output: keep it readable rather than drop it.
                const trimmed = out.trim();
                this.rawFallbackText.set(trimmed || (res.stderr || "").trim() || "Aucune sortie.");
            }
        } catch (err) {
            this.error.set(`Recherche impossible : ${String(err)}`);
        } finally {
            this.running.set(false);
        }
    }

    /**
     * Reconstruct result blocks from the line-oriented CLI output. A URL line is
     * the structural anchor: the nearest preceding non-empty, non-URL line is the
     * title; the immediately following non-empty, non-URL line (if any) is the
     * snippet. The CLI's leading "Recherche :" header line (emitted with a
     * U+1F50D search glyph prefix) is ignored.
     */
    private parse(stdout: string): SearchResult[] {
        // The CLI header is `<U+1F50D> Recherche : <query>`; match the code point
        // by escape (no emoji glyph in source) rather than embedding the literal.
        const SEARCH_HEADER = "\u{1F50D}";
        const lines = stdout
            .split(/\r?\n/)
            .map((l) => l.trim())
            .filter((l) => l.length > 0 && !l.startsWith(SEARCH_HEADER));

        const out: SearchResult[] = [];
        for (let i = 0; i < lines.length; i++) {
            if (!URL_LINE.test(lines[i])) {
                continue;
            }
            const url = lines[i];
            // Title = previous line, only if it is not itself a URL.
            const prev = i > 0 ? lines[i - 1] : "";
            const title = prev && !URL_LINE.test(prev) ? prev : url;
            // Snippet = next line, only if present and not a URL (which would start
            // the next result block).
            const next = i + 1 < lines.length ? lines[i + 1] : "";
            const snippet = next && !URL_LINE.test(next) ? next : "";
            out.push({ title, url, snippet });
        }
        return out;
    }

    /**
     * Open a result URL. In the Tauri shell this routes through the opener plugin
     * (guarded dynamic import); on the web it falls back to the anchor's native
     * navigation (we only preventDefault when we successfully handle it).
     */
    async open(event: Event, url: string): Promise<void> {
        if (!this.aphrody.isTauri) {
            // Plain browser: let the native <a target="_blank"> navigation happen.
            return;
        }
        event.preventDefault();
        try {
            const mod = (await import("@tauri-apps/plugin-opener")) as {
                openUrl: (u: string) => Promise<void>;
            };
            await mod.openUrl(url);
        } catch {
            // Plugin unavailable: fall back to a new tab.
            globalThis.open?.(url, "_blank", "noopener");
        }
    }
}
