// SPDX-License-Identifier: Apache-2.0
import { Component, signal } from "@angular/core";
import { MatIconModule } from "@angular/material/icon";

import { ToolAction, ToolRunnerComponent } from "../../shared/tool-runner/tool-runner.component";
import { DnsReconPanelComponent } from "./dns-recon-panel.component";
import { WebSearchPanelComponent } from "./web-search-panel.component";

/**
 * Network view — typed M3 panels for DNS OSINT reconnaissance (`aphrody dns`)
 * and keyless web search (`aphrody search`). Both surfaces emit plain text (no
 * `--json` flag), so each panel parses the CLI output defensively into a typed
 * shape and renders structured Material cards/grids. The generic
 * `<app-tool-runner>` is kept as a collapsible secondary view exposing the raw
 * stdout/stderr for both commands.
 */
@Component({
    selector: "app-network",
    imports: [MatIconModule, ToolRunnerComponent, DnsReconPanelComponent, WebSearchPanelComponent],
    template: `
        <div class="page">
            <header class="head">
                <div class="head-icon">
                    <mat-icon class="material-symbols-outlined">dns</mat-icon>
                </div>
                <div>
                    <h1>Réseau</h1>
                    <p>
                        Reconnaissance DNS OSINT et recherche web native (<code
                            >aphrody dns / search</code
                        >).
                    </p>
                </div>
            </header>

            <section class="panel-block">
                <app-dns-recon-panel />
            </section>

            <section class="panel-block">
                <app-web-search-panel />
            </section>

            <section class="raw-block">
                <button
                    class="raw-toggle"
                    (click)="rawOpen.set(!rawOpen())"
                    [attr.aria-expanded]="rawOpen()"
                >
                    <mat-icon class="material-symbols-outlined">terminal</mat-icon>
                    <span>Vue brute (sortie CLI)</span>
                    <mat-icon class="material-symbols-outlined chevron">{{
                        rawOpen() ? "expand_less" : "expand_more"
                    }}</mat-icon>
                </button>
                @if (rawOpen()) {
                    <app-tool-runner
                        title="Réseau — sortie brute"
                        subtitle="Sortie texte non formatée des commandes dns / search."
                        icon="dns"
                        [actions]="actions"
                    />
                }
            </section>
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
            .panel-block {
                margin-bottom: 22px;
            }
            .raw-block {
                margin-top: 8px;
            }
            .raw-toggle {
                display: flex;
                align-items: center;
                gap: 8px;
                width: 100%;
                padding: 12px 16px;
                border: 1px solid var(--mat-sys-outline-variant);
                border-radius: 14px;
                background: var(--mat-sys-surface-container-low);
                color: var(--mat-sys-on-surface);
                font: inherit;
                font-size: 14px;
                font-weight: 500;
                cursor: pointer;
                text-align: left;
            }
            .raw-toggle:hover {
                background: var(--mat-sys-surface-container);
            }
            .raw-toggle > mat-icon:first-child {
                color: var(--mat-sys-on-surface-variant);
                font-size: 20px;
                width: 20px;
                height: 20px;
            }
            .raw-toggle .chevron {
                margin-left: auto;
                color: var(--mat-sys-on-surface-variant);
            }
            app-tool-runner {
                display: block;
                margin-top: 12px;
            }
        `,
    ],
})
export class NetworkComponent {
    /** Whether the raw CLI output view is expanded. */
    readonly rawOpen = signal(false);

    readonly actions: ToolAction[] = [
        {
            label: "Reconnaissance DNS",
            icon: "dns",
            args: ["dns"],
            prompt: { placeholder: "Domaine (ex. example.com)" },
            hint: "Résolution DNS OSINT agressive d'un domaine.",
        },
        {
            label: "Recherche web native",
            icon: "search",
            args: ["search"],
            prompt: { placeholder: "Requête de recherche" },
            hint: "Recherche web native sans clé ni navigateur (DuckDuckGo).",
        },
    ];
}
