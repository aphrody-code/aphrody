// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, inject, signal } from "@angular/core";
import { MatIconModule } from "@angular/material/icon";

import { AgentService, HostUnavailableError, StatResult } from "../../../core/agent.service";
import { AphrodyService } from "../../../core/aphrody.service";
import { formatBytes } from "../../assistant/attachment";

/**
 * Mémoire tab: surfaces the agent's local memory store
 * (`~/.aphrody/aphrody-memory.sqlite`) — real presence + size + mtime via the
 * scoped `aphrody_stat` command — and the memory providers discovered from
 * `aphrody memory --help`. Read-only and honest: no fabricated record counts.
 */
@Component({
    selector: "app-memory-tab",
    imports: [MatIconModule],
    template: `
        <section class="card">
            <h2>Magasin de mémoire local</h2>
            <p class="hint">
                L'agent conserve sa mémoire à long terme dans une base SQLite locale. Elle est lue à
                chaque conversation et alimente le contexte.
            </p>
            @if (stat(); as s) {
                <div class="grid">
                    <div class="kv">
                        <span>Fichier</span><b class="mono">{{ s.path }}</b>
                    </div>
                    <div class="kv">
                        <span>État</span>
                        @if (s.exists) {
                            <b class="ok">présent</b>
                        } @else {
                            <b class="muted">absent (sera créé au besoin)</b>
                        }
                    </div>
                    @if (s.exists) {
                        <div class="kv">
                            <span>Taille</span><b>{{ size(s) }}</b>
                        </div>
                        @if (s.modified_secs > 0) {
                            <div class="kv">
                                <span>Modifié</span><b>{{ modified(s) }}</b>
                            </div>
                        }
                    }
                </div>
            } @else if (statError()) {
                <p class="warn">{{ statError() }}</p>
            } @else {
                <p class="hint">Lecture de l'état…</p>
            }
        </section>

        <section class="card">
            <h2>Fournisseurs de mémoire</h2>
            <p class="hint">
                Le trait <code>MemoryProvider</code> permet de copier des enregistrements entre
                fournisseurs (<code>aphrody memory migrate</code>). Les fournisseurs HTTP lisent
                leurs identifiants dans l'environnement.
            </p>
            <ul class="providers">
                <li>
                    <b>SqliteLocal</b><span>Base locale hors-ligne (par défaut, ci-dessus).</span>
                </li>
                <li>
                    <b>Mem0</b><span>Service HTTP — clé <code>MEM0_API_KEY</code>.</span>
                </li>
                <li>
                    <b>Honcho</b
                    ><span>Service HTTP (dialectic v3) — clé <code>HONCHO_API_KEY</code>.</span>
                </li>
            </ul>
            @if (helpText()) {
                <pre class="help">{{ helpText() }}</pre>
            }
        </section>
    `,
    styles: [
        `
            .card {
                background: var(--mat-sys-surface-container);
                border-radius: 20px;
                padding: 20px 22px;
                margin-bottom: 16px;
            }
            .card h2 {
                font-size: 16px;
                font-weight: 500;
                margin: 0 0 12px;
            }
            .hint {
                margin: 0 0 14px;
                font-size: 13px;
                line-height: 1.5;
                color: var(--mat-sys-on-surface-variant);
            }
            code {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 0.92em;
            }
            .grid {
                display: flex;
                flex-direction: column;
                gap: 1px;
                border-radius: 12px;
                overflow: hidden;
                background: var(--mat-sys-outline-variant);
            }
            .kv {
                display: flex;
                justify-content: space-between;
                gap: 16px;
                padding: 12px 16px;
                background: var(--mat-sys-surface-container-low);
                font-size: 13px;
            }
            .kv span {
                color: var(--mat-sys-on-surface-variant);
                flex: 0 0 auto;
            }
            .kv b {
                text-align: right;
                min-width: 0;
                overflow: hidden;
                text-overflow: ellipsis;
            }
            .mono {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 12px;
            }
            .ok {
                color: var(--mat-sys-tertiary);
            }
            .muted {
                color: var(--mat-sys-on-surface-variant);
                font-weight: 400;
            }
            .warn {
                margin: 0;
                font-size: 13px;
                color: var(--mat-sys-error);
            }
            .providers {
                list-style: none;
                margin: 0;
                padding: 0;
                display: flex;
                flex-direction: column;
                gap: 8px;
            }
            .providers li {
                display: flex;
                flex-direction: column;
                padding: 12px 14px;
                border-radius: 12px;
                background: var(--mat-sys-surface-container-low);
            }
            .providers b {
                font-size: 13px;
            }
            .providers span {
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
            }
            .help {
                margin: 14px 0 0;
                padding: 12px 14px;
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 11px;
                line-height: 1.5;
                color: var(--mat-sys-on-surface-variant);
                background: var(--mat-sys-surface-container-low);
                border-radius: 10px;
                white-space: pre-wrap;
                max-height: 200px;
                overflow: auto;
            }
        `,
    ],
})
export class MemoryTabComponent implements OnInit {
    private readonly agent = inject(AgentService);
    private readonly aphrody = inject(AphrodyService);

    readonly stat = signal<StatResult | null>(null);
    readonly statError = signal("");
    readonly helpText = signal("");

    async ngOnInit(): Promise<void> {
        try {
            this.stat.set(await this.agent.stat("memory-sqlite"));
        } catch (err) {
            this.statError.set(
                err instanceof HostUnavailableError
                    ? "L'état du magasin nécessite l'application de bureau."
                    : `Lecture impossible : ${String(err)}`,
            );
        }
        try {
            const res = await this.aphrody.exec(["memory", "--help"]);
            const out = (res.stdout || res.stderr).trim();
            if (out) this.helpText.set(out.slice(0, 1200));
        } catch {
            // non-fatal: providers list above is still informative
        }
    }

    size(s: StatResult): string {
        return formatBytes(s.size);
    }

    modified(s: StatResult): string {
        return new Date(s.modified_secs * 1000).toLocaleString("fr-FR");
    }
}
