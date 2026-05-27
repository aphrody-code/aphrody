// SPDX-License-Identifier: Apache-2.0
import { Component, signal } from "@angular/core";
import { MatIconModule } from "@angular/material/icon";

import { ToolAction, ToolRunnerComponent } from "../../shared/tool-runner/tool-runner.component";
import { FsMapPanelComponent } from "./fs-map-panel.component";
import { SqliteSchemaPanelComponent } from "./sqlite-schema-panel.component";

/** The forensics sub-views the operator can switch between. */
type ForensicsTab = "map" | "sqlite" | "chromium";

/** One entry in the sub-command selector. */
interface TabDef {
    id: ForensicsTab;
    label: string;
    icon: string;
}

/**
 * Forensics view — reproducible, read-only forensic extraction
 * (<code>aphrody forensics</code> + <code>aphrody chromium</code>).
 *
 * The two native <code>forensics</code> sub-commands emit structured JSON, so
 * each gets a strongly-typed M3 panel:
 *   - <b>map</b> -> {@link FsMapPanelComponent} (classified filesystem map; the
 *     stdout summary is typed, the full per-file map is written to map.json);
 *   - <b>sqlite</b> -> {@link SqliteSchemaPanelComponent} (read-only
 *     sqlite_master schema dump — names + CREATE statements only).
 * The Chromium artefact commands (free-text output) keep the generic
 * {@link ToolRunnerComponent}. A segmented selector switches between them.
 *
 * SECURITY: every path here is read-only and the CLI never emits secret VALUES
 * (cookies / tokens / row data are never read). When a command is unavailable
 * on the host (e.g. a Chromium artefact path missing), the CLI returns a
 * structured error that the typed panels render as a readable error card.
 */
@Component({
    selector: "app-forensics",
    imports: [MatIconModule, ToolRunnerComponent, FsMapPanelComponent, SqliteSchemaPanelComponent],
    template: `
        <div class="page">
            <header class="head">
                <div class="head-icon">
                    <mat-icon class="material-symbols-outlined">travel_explore</mat-icon>
                </div>
                <div>
                    <h1>Forensics</h1>
                    <p>
                        Extraction forensique reproductible (lecture seule, jamais de secrets en
                        clair).
                    </p>
                </div>
            </header>

            <div class="seg" role="tablist" aria-label="Sous-commande forensics">
                @for (t of tabs; track t.id) {
                    <button
                        class="seg-btn"
                        role="tab"
                        type="button"
                        [class.active]="tab() === t.id"
                        [attr.aria-selected]="tab() === t.id"
                        (click)="tab.set(t.id)"
                    >
                        <mat-icon class="material-symbols-outlined">{{ t.icon }}</mat-icon>
                        <span>{{ t.label }}</span>
                    </button>
                }
            </div>

            <section class="panel">
                @switch (tab()) {
                    @case ("map") {
                        <app-fs-map-panel />
                    }
                    @case ("sqlite") {
                        <app-sqlite-schema-panel />
                    }
                    @case ("chromium") {
                        <app-tool-runner
                            title="Artefacts Chromium"
                            subtitle="Export / synchronisation forensique des artefacts Chromium (sortie texte brute)"
                            icon="cookie"
                            [actions]="chromiumActions"
                        />
                    }
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
            .seg {
                display: inline-flex;
                gap: 4px;
                padding: 4px;
                border-radius: 999px;
                background: var(--mat-sys-surface-container);
                margin-bottom: 18px;
                flex-wrap: wrap;
            }
            .seg-btn {
                display: inline-flex;
                align-items: center;
                gap: 7px;
                border: none;
                background: transparent;
                color: var(--mat-sys-on-surface-variant);
                font: inherit;
                font-size: 13px;
                font-weight: 500;
                padding: 8px 16px;
                border-radius: 999px;
                cursor: pointer;
            }
            .seg-btn mat-icon {
                font-size: 18px;
                width: 18px;
                height: 18px;
            }
            .seg-btn:hover:not(.active) {
                background: var(--mat-sys-surface-container-high);
            }
            .seg-btn.active {
                background: var(--mat-sys-secondary-container);
                color: var(--mat-sys-on-secondary-container);
            }
            .panel {
                display: block;
            }
        `,
    ],
})
export class ForensicsComponent {
    /** Active sub-view. */
    readonly tab = signal<ForensicsTab>("map");

    /** Sub-command selector entries. */
    readonly tabs: TabDef[] = [
        { id: "map", label: "Carte du système de fichiers", icon: "account_tree" },
        { id: "sqlite", label: "Schéma SQLite", icon: "database" },
        { id: "chromium", label: "Artefacts Chromium", icon: "cookie" },
    ];

    /**
     * Chromium artefact commands. These produce free-text output (no documented
     * JSON contract in the forensics module), so they stay on the generic
     * tool-runner rather than a typed panel.
     */
    readonly chromiumActions: ToolAction[] = [
        {
            label: "Export de session Chromium",
            icon: "cookie",
            args: ["chromium", "export-session"],
            hint: "Exporte l'état de session Chromium de manière reproductible.",
        },
        {
            label: "Synchronisation Chromium",
            icon: "sync",
            args: ["chromium", "sync"],
            hint: "Synchronisation forensique des artefacts Chromium.",
        },
    ];
}
