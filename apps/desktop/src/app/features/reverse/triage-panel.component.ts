// SPDX-License-Identifier: Apache-2.0
import { Component, computed, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatButtonModule } from "@angular/material/button";
import { MatCardModule } from "@angular/material/card";
import { MatChipsModule } from "@angular/material/chips";
import { MatIconModule } from "@angular/material/icon";
import { MatProgressBarModule } from "@angular/material/progress-bar";
import { MatProgressSpinnerModule } from "@angular/material/progress-spinner";
import { MatTableModule } from "@angular/material/table";
import { MatTooltipModule } from "@angular/material/tooltip";

import { AphrodyService } from "../../core/aphrody.service";

/**
 * Detected binary format. Mirrors EXACTLY the Rust `aphrody_re::Format` enum,
 * serialised with `#[serde(rename_all = "lowercase")]` — so the wire values are
 * the lowercase variant names. `Format::Unknown` is always populated when no
 * magic matched (never null).
 */
export type ReFormat = "pe32" | "pe64" | "elf32" | "elf64" | "unknown";

/**
 * One section row. Matches EXACTLY `aphrody_re::Section`:
 * `{ name: String, vaddr: u64, size: u64, entropy: Option<f64> }`.
 * `entropy` is Shannon entropy in bits/byte (range [0.0, 8.0]); `null` for
 * sections with no on-disk content (e.g. `.bss`).
 */
export interface ReSection {
  name: string;
  vaddr: number;
  size: number;
  entropy: number | null;
}

/**
 * Full triage report. Matches EXACTLY `aphrody_re::TriageReport`:
 * `{ format, size: usize, sha256: String, arch: Option<String>,
 *    entry_point: Option<u64>, sections: Vec<Section>, imports: Vec<String>,
 *    exports: Vec<String>, strings_sample: Vec<String> }`.
 * `sections`/`imports`/`exports`/`strings_sample` are always arrays (never
 * null) per the Rust DTO; `arch`/`entry_point` are `Option` -> nullable.
 */
export interface TriageReport {
  format: ReFormat;
  size: number;
  sha256: string;
  arch: string | null;
  entry_point: number | null;
  sections: ReSection[];
  imports: string[];
  exports: string[];
  strings_sample: string[];
}

/** Entropy threshold (bits/byte) above which a section is flagged as packed/encrypted. */
const ENTROPY_HIGH = 7.2;
/** Entropy threshold (bits/byte) above which a section is merely "dense". */
const ENTROPY_MED = 6.0;

/**
 * Typed M3 panel for `aphrody re triage`. Takes a binary path, runs
 * `re triage <path> --pretty` in-process via {@link AphrodyService}, parses the
 * JSON into a strongly-typed {@link TriageReport}, and renders a header card
 * (format / arch / size / entry point / SHA-256), a sections MatTable with
 * per-section entropy bars, import/export chip lists, and a collapsible strings
 * sample. Falls back to a readable error pane on non-zero exit or JSON parse
 * failure.
 */
@Component({
  selector: "app-triage-panel",
  imports: [
    FormsModule,
    MatButtonModule,
    MatCardModule,
    MatChipsModule,
    MatIconModule,
    MatProgressBarModule,
    MatProgressSpinnerModule,
    MatTableModule,
    MatTooltipModule,
  ],
  template: `
    <mat-card class="run-card" appearance="outlined">
      <div class="run-row">
        <mat-icon class="material-symbols-outlined run-glyph">biotech</mat-icon>
        <div class="run-text">
          <span class="run-title">Triage typé d'un binaire</span>
          <span class="run-hint">Format, sections + entropie, imports/exports, SHA-256 (aphrody re triage).</span>
        </div>
      </div>
      <div class="run-input">
        <input
          class="path-input"
          placeholder="Chemin du binaire (PE / ELF)"
          [value]="path()"
          (input)="path.set($any($event.target).value)"
          (keydown.enter)="analyze()"
          [disabled]="running()"
          aria-label="Chemin du binaire à analyser"
        />
        <button mat-raised-button color="primary" (click)="analyze()" [disabled]="running() || !path().trim()">
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
          <b>Échec du triage</b>
        </div>
        <pre class="err-body">{{ e }}</pre>
      </mat-card>
    }

    @if (report(); as r) {
      <!-- Header card -->
      <mat-card class="hdr-card" appearance="outlined">
        <div class="hdr-top">
          <span class="fmt-chip" [attr.data-fmt]="formatBucket()">
            <mat-icon class="material-symbols-outlined">{{ formatIcon() }}</mat-icon>
            {{ formatLabel() }}
          </span>
          @if (r.arch) {
            <span class="kv"><span class="k">Architecture</span><span class="v">{{ r.arch }}</span></span>
          }
          <span class="kv"><span class="k">Taille</span><span class="v">{{ humanSize(r.size) }}</span></span>
          @if (r.entry_point !== null) {
            <span class="kv"><span class="k">Point d'entrée</span><span class="v mono">{{ hex(r.entry_point) }}</span></span>
          }
        </div>
        <div class="sha-row">
          <span class="k">SHA-256</span>
          <code class="sha mono" [matTooltip]="r.sha256" matTooltipClass="sha-tip">{{ shaShort() }}</code>
        </div>
      </mat-card>

      <!-- Sections table -->
      @if (r.sections.length > 0) {
        <mat-card class="tbl-card" appearance="outlined">
          <div class="block-head">
            <mat-icon class="material-symbols-outlined">view_module</mat-icon>
            <span>Sections</span>
            <span class="count">{{ r.sections.length }}</span>
          </div>
          <table mat-table [dataSource]="r.sections" class="sec-table">
            <ng-container matColumnDef="name">
              <th mat-header-cell *matHeaderCellDef>Nom</th>
              <td mat-cell *matCellDef="let s"><code class="mono">{{ s.name }}</code></td>
            </ng-container>

            <ng-container matColumnDef="vaddr">
              <th mat-header-cell *matHeaderCellDef>Adresse</th>
              <td mat-cell *matCellDef="let s"><code class="mono dim">{{ hex(s.vaddr) }}</code></td>
            </ng-container>

            <ng-container matColumnDef="size">
              <th mat-header-cell *matHeaderCellDef>Taille</th>
              <td mat-cell *matCellDef="let s">{{ humanSize(s.size) }}</td>
            </ng-container>

            <ng-container matColumnDef="entropy">
              <th mat-header-cell *matHeaderCellDef>Entropie (bits/octet)</th>
              <td mat-cell *matCellDef="let s">
                @if (s.entropy === null) {
                  <span class="dim">—</span>
                } @else {
                  <div class="entropy-cell">
                    <span class="entropy-num" [attr.data-lvl]="entropyLevel(s.entropy)">{{ s.entropy.toFixed(2) }}</span>
                    <mat-progress-bar
                      class="entropy-bar"
                      [attr.data-lvl]="entropyLevel(s.entropy)"
                      mode="determinate"
                      [value]="entropyPct(s.entropy)"
                      [attr.aria-label]="entropyAria(s.name, s.entropy)"
                    />
                  </div>
                }
              </td>
            </ng-container>

            <tr mat-header-row *matHeaderRowDef="sectionColumns"></tr>
            <tr mat-row *matRowDef="let row; columns: sectionColumns"></tr>
          </table>
          <p class="legend">
            <span class="lg high"></span> ≥ {{ entropyHigh }} : probablement packé / chiffré
            <span class="lg med"></span> ≥ {{ entropyMed }} : dense
            <span class="lg low"></span> faible
          </p>
        </mat-card>
      }

      <!-- Imports / Exports -->
      @if (r.imports.length > 0 || r.exports.length > 0) {
        <div class="sym-grid">
          @if (r.imports.length > 0) {
            <mat-card class="sym-card" appearance="outlined">
              <div class="block-head">
                <mat-icon class="material-symbols-outlined">south_west</mat-icon>
                <span>Imports</span>
                <span class="count">{{ r.imports.length }}</span>
              </div>
              <mat-chip-set class="sym-chips" aria-label="Symboles importés">
                @for (sym of r.imports; track $index) {
                  <mat-chip disableRipple><code class="mono">{{ sym }}</code></mat-chip>
                }
              </mat-chip-set>
            </mat-card>
          }
          @if (r.exports.length > 0) {
            <mat-card class="sym-card" appearance="outlined">
              <div class="block-head">
                <mat-icon class="material-symbols-outlined">north_east</mat-icon>
                <span>Exports</span>
                <span class="count">{{ r.exports.length }}</span>
              </div>
              <mat-chip-set class="sym-chips" aria-label="Symboles exportés">
                @for (sym of r.exports; track $index) {
                  <mat-chip disableRipple><code class="mono">{{ sym }}</code></mat-chip>
                }
              </mat-chip-set>
            </mat-card>
          }
        </div>
      }

      <!-- Strings sample -->
      @if (r.strings_sample.length > 0) {
        <mat-card class="str-card" appearance="outlined">
          <button class="block-head as-button" (click)="stringsOpen.set(!stringsOpen())" [attr.aria-expanded]="stringsOpen()">
            <mat-icon class="material-symbols-outlined">data_array</mat-icon>
            <span>Échantillon de chaînes</span>
            <span class="count">{{ r.strings_sample.length }}</span>
            <mat-icon class="material-symbols-outlined chevron">{{ stringsOpen() ? "expand_less" : "expand_more" }}</mat-icon>
          </button>
          @if (stringsOpen()) {
            <ul class="str-list">
              @for (str of r.strings_sample; track $index) {
                <li class="mono">{{ str }}</li>
              }
            </ul>
          }
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
        background: var(--mat-sys-surface-container-high);
        color: var(--mat-sys-on-surface);
      }
      .fmt-chip mat-icon {
        font-size: 18px;
        width: 18px;
        height: 18px;
      }
      .fmt-chip[data-fmt="pe"] {
        background: color-mix(in srgb, #4285f4 22%, transparent);
        color: var(--mat-sys-on-surface);
      }
      .fmt-chip[data-fmt="elf"] {
        background: color-mix(in srgb, #34a853 22%, transparent);
        color: var(--mat-sys-on-surface);
      }
      .fmt-chip[data-fmt="unknown"] {
        background: var(--mat-sys-surface-container-highest);
        color: var(--mat-sys-on-surface-variant);
      }
      .kv {
        display: flex;
        flex-direction: column;
        gap: 2px;
      }
      .kv .k,
      .sha-row .k {
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--mat-sys-on-surface-variant);
      }
      .kv .v {
        font-size: 14px;
        font-weight: 500;
      }
      .sha-row {
        display: flex;
        align-items: center;
        gap: 12px;
        padding-top: 12px;
        border-top: 1px solid var(--mat-sys-outline-variant);
      }
      .sha {
        font-size: 13px;
        color: var(--mat-sys-on-surface);
        cursor: help;
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
      .block-head.as-button {
        width: 100%;
        border: none;
        background: transparent;
        color: inherit;
        cursor: pointer;
        font: inherit;
        font-weight: 500;
        padding: 0;
        margin: 0;
        text-align: left;
      }
      .block-head .chevron {
        margin-left: auto;
        color: var(--mat-sys-on-surface-variant);
      }
      /* Sections table */
      .sec-table {
        width: 100%;
        background: transparent;
      }
      .sec-table th.mat-mdc-header-cell,
      .sec-table td.mat-mdc-cell {
        color: var(--mat-sys-on-surface);
        border-bottom-color: var(--mat-sys-outline-variant);
        padding: 8px 12px 8px 0;
      }
      .sec-table th.mat-mdc-header-cell {
        color: var(--mat-sys-on-surface-variant);
        font-size: 12px;
      }
      .entropy-cell {
        display: flex;
        align-items: center;
        gap: 10px;
        min-width: 160px;
      }
      .entropy-num {
        font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
        font-size: 13px;
        font-variant-numeric: tabular-nums;
        min-width: 38px;
      }
      .entropy-num[data-lvl="high"] {
        color: var(--mat-sys-error);
        font-weight: 600;
      }
      .entropy-num[data-lvl="med"] {
        color: #b8860b;
      }
      .entropy-bar {
        flex: 1 1 auto;
        max-width: 140px;
        border-radius: 999px;
      }
      .entropy-bar[data-lvl="high"] {
        --mat-progress-bar-active-indicator-color: var(--mat-sys-error);
      }
      .entropy-bar[data-lvl="med"] {
        --mat-progress-bar-active-indicator-color: #c9920a;
      }
      .entropy-bar[data-lvl="low"] {
        --mat-progress-bar-active-indicator-color: var(--mat-sys-primary);
      }
      .legend {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 6px 14px;
        margin: 12px 0 0;
        font-size: 11px;
        color: var(--mat-sys-on-surface-variant);
      }
      .lg {
        display: inline-block;
        width: 12px;
        height: 12px;
        border-radius: 3px;
        margin-right: 2px;
        vertical-align: middle;
      }
      .lg.high {
        background: var(--mat-sys-error);
      }
      .lg.med {
        background: #c9920a;
      }
      .lg.low {
        background: var(--mat-sys-primary);
      }
      /* Symbols */
      .sym-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: 14px;
      }
      .sym-chips {
        max-height: 220px;
        overflow: auto;
      }
      .sym-chips code {
        font-size: 12px;
      }
      /* Strings */
      .str-list {
        list-style: none;
        margin: 12px 0 0;
        padding: 0;
        max-height: 320px;
        overflow: auto;
        border-top: 1px solid var(--mat-sys-outline-variant);
      }
      .str-list li {
        font-size: 12px;
        line-height: 1.6;
        padding: 3px 4px;
        color: var(--mat-sys-on-surface);
        white-space: pre-wrap;
        word-break: break-all;
        border-bottom: 1px solid color-mix(in srgb, var(--mat-sys-outline-variant) 50%, transparent);
      }
    `,
  ],
})
export class TriagePanelComponent {
  private readonly aphrody = inject(AphrodyService);

  /** Path entered by the user. */
  readonly path = signal("");
  /** Action label while a run is in flight (null = idle). */
  readonly running = signal(false);
  /** Parsed report, or null before the first/failed run. */
  readonly report = signal<TriageReport | null>(null);
  /** Human-readable error text on failure (null = no error). */
  readonly error = signal<string | null>(null);
  /** Whether the strings sample list is expanded. */
  readonly stringsOpen = signal(false);

  readonly entropyHigh = ENTROPY_HIGH;
  readonly entropyMed = ENTROPY_MED;
  readonly sectionColumns = ["name", "vaddr", "size", "entropy"] as const;

  /** Coarse format bucket used to colour the header chip. */
  readonly formatBucket = computed<"pe" | "elf" | "unknown">(() => {
    const f = this.report()?.format ?? "unknown";
    if (f === "pe32" || f === "pe64") return "pe";
    if (f === "elf32" || f === "elf64") return "elf";
    return "unknown";
  });

  /** Human label for the detected format. */
  readonly formatLabel = computed(() => {
    switch (this.report()?.format) {
      case "pe32":
        return "PE32 (Windows)";
      case "pe64":
        return "PE32+ (Windows 64-bit)";
      case "elf32":
        return "ELF32 (Linux/Unix)";
      case "elf64":
        return "ELF64 (Linux/Unix)";
      default:
        return "Format inconnu";
    }
  });

  /** Material symbol for the format chip. */
  readonly formatIcon = computed(() => {
    switch (this.formatBucket()) {
      case "pe":
        return "window";
      case "elf":
        return "terminal";
      default:
        return "help";
    }
  });

  /** SHA-256 truncated for display (full value lives in the tooltip). */
  readonly shaShort = computed(() => {
    const sha = this.report()?.sha256 ?? "";
    return sha.length > 24 ? `${sha.slice(0, 12)}…${sha.slice(-12)}` : sha;
  });

  /** Run `aphrody re triage <path> --pretty` and parse the JSON report. */
  async analyze(): Promise<void> {
    const p = this.path().trim();
    if (!p || this.running()) {
      return;
    }
    this.running.set(true);
    this.error.set(null);
    this.report.set(null);
    this.stringsOpen.set(false);
    try {
      // `re triage` ALWAYS emits JSON (compact by default, pretty with
      // `--pretty`); there is no `--json` flag. We request `--pretty` for a
      // deterministic shape; `JSON.parse` handles both.
      const res = await this.aphrody.exec(["re", "triage", p, "--pretty"]);
      if (res.code !== 0) {
        this.error.set(
          (res.stderr || res.stdout || `Le binaire a renvoyé le code ${res.code}.`).trim().slice(0, 2000),
        );
        return;
      }
      const out = res.stdout.trim();
      if (!out) {
        this.error.set("La commande n'a renvoyé aucune sortie.");
        return;
      }
      const parsed = JSON.parse(out) as TriageReport;
      // Defensive normalisation: the Rust DTO guarantees arrays, but a web
      // fallback / unexpected payload might not — coerce so the template is safe.
      this.report.set({
        format: parsed.format ?? "unknown",
        size: parsed.size ?? 0,
        sha256: parsed.sha256 ?? "",
        arch: parsed.arch ?? null,
        entry_point: parsed.entry_point ?? null,
        sections: Array.isArray(parsed.sections) ? parsed.sections : [],
        imports: Array.isArray(parsed.imports) ? parsed.imports : [],
        exports: Array.isArray(parsed.exports) ? parsed.exports : [],
        strings_sample: Array.isArray(parsed.strings_sample) ? parsed.strings_sample : [],
      });
    } catch (err) {
      this.error.set(`Sortie illisible (JSON invalide) : ${String(err)}`);
    } finally {
      this.running.set(false);
    }
  }

  /** Format a byte count as B / Ko / Mo / Go. */
  humanSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} o`;
    const units = ["Ko", "Mo", "Go", "To"];
    let value = bytes / 1024;
    let i = 0;
    while (value >= 1024 && i < units.length - 1) {
      value /= 1024;
      i++;
    }
    return `${value.toFixed(value >= 10 || Number.isInteger(value) ? 0 : 1)} ${units[i]}`;
  }

  /** Render a 64-bit virtual address as 0x-prefixed hex. */
  hex(value: number): string {
    return `0x${value.toString(16)}`;
  }

  /** Map an entropy value to a severity bucket. */
  entropyLevel(entropy: number): "high" | "med" | "low" {
    if (entropy >= ENTROPY_HIGH) return "high";
    if (entropy >= ENTROPY_MED) return "med";
    return "low";
  }

  /** Entropy as a 0..100 percentage of the 8 bits/byte maximum. */
  entropyPct(entropy: number): number {
    return Math.max(0, Math.min(100, (entropy / 8) * 100));
  }

  /** Accessible label for an entropy progress bar. */
  entropyAria(name: string, entropy: number): string {
    const lvl =
      this.entropyLevel(entropy) === "high"
        ? "entropie élevée, probablement packé ou chiffré"
        : this.entropyLevel(entropy) === "med"
          ? "entropie modérée"
          : "entropie faible";
    return `Section ${name} : entropie ${entropy.toFixed(2)} sur 8 bits par octet, ${lvl}`;
  }
}
