// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, computed, inject, signal } from "@angular/core";
import { MatIconModule } from "@angular/material/icon";

import { AphrodyService } from "../../core/aphrody.service";

/** Runtime section of `aphrody doctor --json`. */
interface DoctorRuntime {
    binary_version: string;
    rustls_provider: string;
    reqwest_tls: string;
    allocator: string;
    tokio_runtime: string;
    ai_json: string;
    well_known_ai_json: string;
}

/** Peer A2A coordination section of `aphrody doctor --json`. */
interface DoctorPeerA2a {
    heartbeat_age_s: number | null;
    is_stale: boolean;
    heartbeat_detail: string;
    http_listener_status: number | null;
    http_listener_detail: string;
    inbox_detail: string;
}

/** Supply-chain section of `aphrody doctor --json`. */
interface DoctorSupplyChain {
    cargo_deny: string;
    cargo_vet_audits: string;
}

/** Agent-workspace section of `aphrody doctor --json`. */
interface DoctorAgentWorkspace {
    detail: string;
}

/** Verdict emitted by `DoctorReport::verdict()` (Rust `&'static str`). */
type DoctorVerdict = "HEALTHY" | "DEGRADED" | "UNHEALTHY";

/** Full envelope of `aphrody doctor --json`, matching `commands.rs:1270`. */
interface DoctorReport {
    runtime: DoctorRuntime;
    peer_a2a: DoctorPeerA2a;
    supply_chain: DoctorSupplyChain;
    agent_workspace: DoctorAgentWorkspace;
    environment: Record<string, string>;
    verdict: DoctorVerdict;
}

/** One key/value row rendered inside a card. */
interface KvRow {
    label: string;
    value: string;
}

/** Health tone driving chip/dot colours, mapped from the Rust verdict. */
type Health = "ok" | "warn" | "err";

/**
 * Doctor dashboard — turns `aphrody doctor --json` into typed Material 3 cards.
 *
 * The `verdict` field is a Rust `&'static str` with exactly three values
 * (`HEALTHY` / `DEGRADED` / `UNHEALTHY`, see `DoctorReport::verdict()` at
 * `crates/cli/src/commands.rs:1069`); it is mapped to a coloured health chip:
 * green (ok) / amber (warn) / red (err). Parsing is defensive: if the output
 * is not the expected JSON, the raw stdout/stderr + exit code are surfaced.
 */
@Component({
    selector: "app-doctor-dashboard",
    imports: [MatIconModule],
    template: `
        <div class="page">
            <header class="head">
                <div class="head-icon">
                    <mat-icon class="material-symbols-outlined">monitor_heart</mat-icon>
                </div>
                <div>
                    <h1>Diagnostic</h1>
                    <p>
                        État du système, supply-chain et intégration A2A (<code>aphrody doctor</code
                        >).
                    </p>
                </div>
                <button
                    class="refresh"
                    (click)="load()"
                    [disabled]="loading()"
                    aria-label="Rafraîchir le diagnostic"
                >
                    <mat-icon class="material-symbols-outlined" [class.spin]="loading()"
                        >refresh</mat-icon
                    >
                </button>
            </header>

            @if (loading()) {
                <div class="state">
                    <mat-icon class="material-symbols-outlined spin">progress_activity</mat-icon>
                    Exécution de <code>aphrody doctor</code>…
                </div>
            } @else if (parseError()) {
                <div class="state err">
                    <mat-icon class="material-symbols-outlined">error</mat-icon>
                    <div class="state-body">
                        <b>Diagnostic illisible.</b>
                        <p>{{ parseError() }}</p>
                        <p class="muted">Code de sortie : {{ rawCode() }}</p>
                        @if (rawStdout()) {
                            <pre class="raw">{{ rawStdout() }}</pre>
                        }
                        @if (rawStderr()) {
                            <pre class="raw err-raw">{{ rawStderr() }}</pre>
                        }
                    </div>
                </div>
            } @else if (report(); as r) {
                <!-- Verdict banner -->
                <section class="verdict" [attr.data-h]="verdictHealth()">
                    <mat-icon class="material-symbols-outlined verdict-glyph">{{
                        verdictIcon()
                    }}</mat-icon>
                    <div class="verdict-text">
                        <b>{{ verdictLabel() }}</b>
                        <span>{{ verdictHint() }}</span>
                    </div>
                    <span class="chip" [attr.data-h]="verdictHealth()">{{ r.verdict }}</span>
                </section>

                <!-- Runtime -->
                <div class="card">
                    <div class="card-head">
                        <mat-icon class="material-symbols-outlined">settings_suggest</mat-icon>
                        <span>Runtime</span>
                    </div>
                    <div class="kv">
                        @for (row of runtimeRows(); track row.label) {
                            <div class="kv-row">
                                <span class="kv-key">{{ row.label }}</span>
                                <span class="kv-val">{{ row.value }}</span>
                            </div>
                        }
                    </div>
                </div>

                <!-- Peer A2A -->
                <div class="card">
                    <div class="card-head">
                        <mat-icon class="material-symbols-outlined">sync_alt</mat-icon>
                        <span>Coordination A2A (pair)</span>
                        <span class="dot" [attr.data-h]="peerHealth()"></span>
                    </div>
                    <div class="kv">
                        <div class="kv-row">
                            <span class="kv-key">Statut du heartbeat</span>
                            <span class="kv-val">
                                <span class="inline-chip" [attr.data-h]="peerHealth()">{{
                                    peerStatusLabel()
                                }}</span>
                            </span>
                        </div>
                        <div class="kv-row">
                            <span class="kv-key">Âge du heartbeat</span>
                            <span class="kv-val">{{ heartbeatAge() }}</span>
                        </div>
                        <div class="kv-row">
                            <span class="kv-key">Détail heartbeat</span>
                            <span class="kv-val">{{ r.peer_a2a.heartbeat_detail }}</span>
                        </div>
                        <div class="kv-row">
                            <span class="kv-key">Écouteur HTTP</span>
                            <span class="kv-val">{{ httpListener() }}</span>
                        </div>
                        <div class="kv-row">
                            <span class="kv-key">Détail écouteur</span>
                            <span class="kv-val">{{ r.peer_a2a.http_listener_detail }}</span>
                        </div>
                        <div class="kv-row">
                            <span class="kv-key">Boîte de réception</span>
                            <span class="kv-val">{{ r.peer_a2a.inbox_detail }}</span>
                        </div>
                    </div>
                </div>

                <!-- Supply chain + agent workspace, side by side -->
                <div class="card-grid">
                    <div class="card">
                        <div class="card-head">
                            <mat-icon class="material-symbols-outlined">verified_user</mat-icon>
                            <span>Supply-chain</span>
                        </div>
                        <div class="kv">
                            <div class="kv-row">
                                <span class="kv-key">cargo-deny</span>
                                <span class="kv-val">{{ r.supply_chain.cargo_deny }}</span>
                            </div>
                            <div class="kv-row">
                                <span class="kv-key">audits cargo-vet</span>
                                <span class="kv-val">{{ r.supply_chain.cargo_vet_audits }}</span>
                            </div>
                        </div>
                    </div>

                    <div class="card">
                        <div class="card-head">
                            <mat-icon class="material-symbols-outlined">home_storage</mat-icon>
                            <span>Espace de travail de l'agent</span>
                        </div>
                        <div class="kv">
                            <div class="kv-row">
                                <span class="kv-key">Détail</span>
                                <span class="kv-val">{{ r.agent_workspace.detail }}</span>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Environment -->
                <div class="card">
                    <div class="card-head">
                        <mat-icon class="material-symbols-outlined">terminal</mat-icon>
                        <span>Environnement</span>
                        <span class="count">{{ envRows().length }} variable(s)</span>
                    </div>
                    @if (envRows().length === 0) {
                        <p class="empty">Aucune variable d'environnement signalée.</p>
                    } @else {
                        <table class="env-table">
                            <thead>
                                <tr>
                                    <th>Variable</th>
                                    <th>Valeur</th>
                                </tr>
                            </thead>
                            <tbody>
                                @for (row of envRows(); track row.label) {
                                    <tr>
                                        <td class="env-key">{{ row.label }}</td>
                                        <td class="env-val">{{ row.value }}</td>
                                    </tr>
                                }
                            </tbody>
                        </table>
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
            .page {
                max-width: 920px;
                margin: 0 auto;
                padding: 8px 28px 64px;
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
            code {
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
                animation: dr-spin 1s linear infinite;
            }
            @keyframes dr-spin {
                to {
                    transform: rotate(360deg);
                }
            }
            .verdict {
                display: flex;
                align-items: center;
                gap: 14px;
                padding: 16px 18px;
                border-radius: 18px;
                margin-bottom: 16px;
                background: var(--mat-sys-surface-container);
                border-left: 4px solid var(--mat-sys-outline);
            }
            .verdict[data-h="ok"] {
                border-left-color: #34a853;
            }
            .verdict[data-h="warn"] {
                border-left-color: #fbbc04;
            }
            .verdict[data-h="err"] {
                border-left-color: var(--mat-sys-error);
            }
            .verdict-glyph {
                font-size: 30px;
                width: 30px;
                height: 30px;
            }
            .verdict[data-h="ok"] .verdict-glyph {
                color: #34a853;
            }
            .verdict[data-h="warn"] .verdict-glyph {
                color: #fbbc04;
            }
            .verdict[data-h="err"] .verdict-glyph {
                color: var(--mat-sys-error);
            }
            .verdict-text {
                display: flex;
                flex-direction: column;
                flex: 1 1 auto;
                min-width: 0;
            }
            .verdict-text b {
                font-size: 16px;
            }
            .verdict-text span {
                font-size: 13px;
                color: var(--mat-sys-on-surface-variant);
            }
            .chip {
                font-size: 12px;
                font-weight: 600;
                letter-spacing: 0.4px;
                padding: 5px 12px;
                border-radius: 999px;
                flex: 0 0 auto;
                background: var(--mat-sys-surface-container-highest);
                color: var(--mat-sys-on-surface-variant);
            }
            .chip[data-h="ok"] {
                color: #1b5e20;
                background: color-mix(in srgb, #34a853 22%, transparent);
            }
            .chip[data-h="warn"] {
                color: var(--mat-sys-on-surface);
                background: color-mix(in srgb, #fbbc04 28%, transparent);
            }
            .chip[data-h="err"] {
                color: var(--mat-sys-on-error-container);
                background: var(--mat-sys-error-container);
            }
            .card-grid {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
                gap: 12px;
                margin-bottom: 12px;
            }
            .card {
                border-radius: 16px;
                background: var(--mat-sys-surface-container-low);
                padding: 16px 18px;
                margin-bottom: 12px;
            }
            .card-grid .card {
                margin-bottom: 0;
            }
            .card-head {
                display: flex;
                align-items: center;
                gap: 8px;
                margin-bottom: 12px;
            }
            .card-head > mat-icon {
                color: var(--mat-sys-primary);
                font-size: 20px;
                width: 20px;
                height: 20px;
            }
            .card-head > span:not(.dot):not(.count) {
                font-size: 15px;
                font-weight: 500;
                flex: 1 1 auto;
            }
            .count {
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
            }
            .dot {
                width: 10px;
                height: 10px;
                border-radius: 50%;
                flex: 0 0 auto;
                background: var(--mat-sys-outline);
            }
            .dot[data-h="ok"] {
                background: #34a853;
            }
            .dot[data-h="warn"] {
                background: #fbbc04;
            }
            .dot[data-h="err"] {
                background: var(--mat-sys-error);
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
                flex: 0 0 40%;
                color: var(--mat-sys-on-surface-variant);
            }
            .kv-val {
                flex: 1 1 60%;
                color: var(--mat-sys-on-surface);
                word-break: break-word;
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
            }
            .inline-chip {
                font-size: 11px;
                font-weight: 600;
                padding: 2px 10px;
                border-radius: 999px;
                background: var(--mat-sys-surface-container-highest);
                color: var(--mat-sys-on-surface-variant);
                font-family: inherit;
            }
            .inline-chip[data-h="ok"] {
                color: #1b5e20;
                background: color-mix(in srgb, #34a853 22%, transparent);
            }
            .inline-chip[data-h="warn"] {
                color: var(--mat-sys-on-surface);
                background: color-mix(in srgb, #fbbc04 28%, transparent);
            }
            .inline-chip[data-h="err"] {
                color: var(--mat-sys-on-error-container);
                background: var(--mat-sys-error-container);
            }
            .env-table {
                width: 100%;
                border-collapse: collapse;
                font-size: 13px;
            }
            .env-table th {
                text-align: left;
                padding: 8px 12px;
                font-weight: 500;
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
                border-bottom: 1px solid var(--mat-sys-outline-variant);
            }
            .env-table td {
                padding: 9px 12px;
                border-bottom: 1px solid var(--mat-sys-outline-variant);
                vertical-align: top;
            }
            .env-table tbody tr:last-child td {
                border-bottom: none;
            }
            .env-key {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                color: var(--mat-sys-on-surface-variant);
                white-space: nowrap;
            }
            .env-val {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                color: var(--mat-sys-on-surface);
                word-break: break-word;
            }
            .empty {
                margin: 0;
                font-size: 13px;
                color: var(--mat-sys-on-surface-variant);
            }
            .state {
                display: flex;
                align-items: center;
                gap: 12px;
                padding: 28px 20px;
                border-radius: 16px;
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
                flex: 0 0 auto;
            }
            .state-body {
                min-width: 0;
                flex: 1 1 auto;
            }
            .state-body p {
                margin: 4px 0 0;
                font-size: 13px;
                color: var(--mat-sys-on-surface-variant);
            }
            .state-body .muted {
                font-size: 12px;
            }
            .raw {
                margin: 10px 0 0;
                padding: 12px 14px;
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 12px;
                line-height: 1.5;
                color: var(--mat-sys-on-surface-variant);
                background: var(--mat-sys-surface-container);
                border-radius: 12px;
                white-space: pre-wrap;
                word-break: break-word;
                max-height: 280px;
                overflow: auto;
            }
            .raw.err-raw {
                color: var(--mat-sys-error);
            }
        `,
    ],
})
export class DoctorDashboardComponent implements OnInit {
    private readonly aphrody = inject(AphrodyService);

    readonly loading = signal(false);
    readonly report = signal<DoctorReport | null>(null);
    readonly parseError = signal("");
    readonly rawCode = signal(0);
    readonly rawStdout = signal("");
    readonly rawStderr = signal("");

    /** Verdict mapped to a health tone: HEALTHY -> ok, DEGRADED -> warn, else err. */
    readonly verdictHealth = computed<Health>(() => {
        const v = this.report()?.verdict;
        if (v === "HEALTHY") return "ok";
        if (v === "DEGRADED") return "warn";
        return "err";
    });

    readonly verdictIcon = computed(() => {
        switch (this.verdictHealth()) {
            case "ok":
                return "check_circle";
            case "warn":
                return "warning";
            default:
                return "error";
        }
    });

    readonly verdictLabel = computed(() => {
        switch (this.report()?.verdict) {
            case "HEALTHY":
                return "Système sain";
            case "DEGRADED":
                return "Système dégradé";
            case "UNHEALTHY":
                return "Système en défaut";
            default:
                return "Verdict inconnu";
        }
    });

    readonly verdictHint = computed(() => {
        switch (this.report()?.verdict) {
            case "HEALTHY":
                return "Toutes les vérifications critiques passent.";
            case "DEGRADED":
                return "Pair hors-ligne ou ressource non critique manquante.";
            case "UNHEALTHY":
                return "Une ou plusieurs vérifications critiques ont échoué.";
            default:
                return "";
        }
    });

    readonly runtimeRows = computed<KvRow[]>(() => {
        const rt = this.report()?.runtime;
        if (!rt) return [];
        return [
            { label: "Version du binaire", value: rt.binary_version },
            { label: "Fournisseur rustls", value: rt.rustls_provider },
            { label: "TLS reqwest", value: rt.reqwest_tls },
            { label: "Allocateur", value: rt.allocator },
            { label: "Runtime tokio", value: rt.tokio_runtime },
            { label: "ai.json", value: rt.ai_json },
            { label: ".well-known/ai.json", value: rt.well_known_ai_json },
        ];
    });

    readonly envRows = computed<KvRow[]>(() => {
        const env = this.report()?.environment;
        if (!env) return [];
        return Object.keys(env)
            .sort((a, b) => a.localeCompare(b))
            .map((label) => ({ label, value: env[label] }));
    });

    /** Peer heartbeat health: stale -> warn, fresh -> ok, unknown age -> warn. */
    readonly peerHealth = computed<Health>(() => {
        const p = this.report()?.peer_a2a;
        if (!p) return "warn";
        if (p.is_stale) return "warn";
        if (p.heartbeat_age_s === null) return "warn";
        return "ok";
    });

    readonly peerStatusLabel = computed(() => {
        const p = this.report()?.peer_a2a;
        if (!p) return "inconnu";
        if (p.heartbeat_age_s === null) return "pair hors-ligne";
        return p.is_stale ? "périmé" : "actif";
    });

    readonly heartbeatAge = computed(() => {
        const p = this.report()?.peer_a2a;
        if (!p || p.heartbeat_age_s === null) return "—";
        return this.formatAge(p.heartbeat_age_s);
    });

    readonly httpListener = computed(() => {
        const p = this.report()?.peer_a2a;
        if (!p || p.http_listener_status === null) return "aucun (hors-ligne)";
        return `HTTP ${p.http_listener_status}`;
    });

    ngOnInit(): void {
        void this.load();
    }

    /** Format an age in seconds as a compact French duration. */
    private formatAge(seconds: number): string {
        if (seconds < 60) return `${seconds} s`;
        if (seconds < 3600) {
            const m = Math.floor(seconds / 60);
            const s = seconds % 60;
            return s > 0 ? `${m} min ${s} s` : `${m} min`;
        }
        const h = Math.floor(seconds / 3600);
        const m = Math.floor((seconds % 3600) / 60);
        return m > 0 ? `${h} h ${m} min` : `${h} h`;
    }

    async load(): Promise<void> {
        this.loading.set(true);
        this.parseError.set("");
        try {
            const res = await this.aphrody.exec(["doctor", "--json"]);
            this.rawCode.set(res.code);
            this.rawStdout.set(res.stdout);
            this.rawStderr.set(res.stderr);

            const out = res.stdout.trim();
            const start = out.indexOf("{");
            if (start < 0) {
                this.report.set(null);
                this.parseError.set(
                    res.stderr.trim() ||
                        "La commande doctor n'a renvoyé aucun objet JSON exploitable.",
                );
                return;
            }

            const parsed = JSON.parse(out.slice(start)) as DoctorReport;
            if (!parsed.runtime || !parsed.verdict) {
                this.report.set(null);
                this.parseError.set("Le JSON du diagnostic ne contient pas les champs attendus.");
                return;
            }
            this.report.set(parsed);
        } catch (err) {
            this.report.set(null);
            this.parseError.set(`Lecture impossible : ${String(err)}`);
        } finally {
            this.loading.set(false);
        }
    }
}
