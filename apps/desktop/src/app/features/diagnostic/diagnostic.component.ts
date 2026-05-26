// SPDX-License-Identifier: Apache-2.0
import { Component, inject, OnInit, signal } from "@angular/core";

import { ToolAction, ToolRunnerComponent } from "../../shared/tool-runner/tool-runner.component";
import { AphrodyService, Meta } from "../../core/aphrody.service";
import { DoctorDashboardComponent } from "./doctor-dashboard.component";
import { VersionCardComponent } from "./version-card.component";

/**
 * Diagnostic view — the primary render is a typed Material 3 dashboard built
 * from `aphrody doctor --json` (verdict banner + runtime / A2A / supply-chain /
 * workspace / environment cards) plus a version-and-system card from
 * `aphrody version --json`. A meta strip of host pills sits on top. The raw
 * text output of the underlying commands stays one click away via a collapsible
 * generic tool runner for power users.
 */
@Component({
  selector: "app-diagnostic",
  imports: [DoctorDashboardComponent, VersionCardComponent, ToolRunnerComponent],
  template: `
    <div class="diag">
      <div class="meta-strip">
        @if (meta(); as m) {
          <span class="pill">app {{ m.app_version }}</span>
          <span class="pill">{{ m.target_os }}</span>
          <span class="pill">{{ m.target_arch }}</span>
          <span class="pill">{{ m.family }}</span>
          @if (!isTauri) {
            <span class="pill warn">mode navigateur</span>
          }
        }
      </div>

      <app-doctor-dashboard />

      <div class="section">
        <app-version-card />
      </div>

      <div class="section raw-section">
        <button class="raw-toggle" (click)="showRaw.set(!showRaw())" [attr.aria-expanded]="showRaw()">
          <span>Sortie texte brute (doctor / version)</span>
          <span class="raw-chevron" [class.open]="showRaw()">⌄</span>
        </button>
        @if (showRaw()) {
          <app-tool-runner
            title="Sortie texte"
            subtitle="Diagnostic et version au format texte / JSON, tels que produits par le binaire aphrody."
            icon="terminal"
            [actions]="actions"
          />
        }
      </div>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
        height: 100%;
        overflow-y: auto;
      }
      .diag {
        padding-top: 20px;
      }
      .meta-strip {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        max-width: 920px;
        margin: 0 auto 8px;
        padding: 0 28px;
      }
      .pill {
        font-size: 12px;
        padding: 4px 12px;
        border-radius: 10px;
        background: var(--mat-sys-surface-container-high);
        color: var(--mat-sys-on-surface-variant);
      }
      .pill.warn {
        background: var(--mat-sys-secondary-container);
        color: var(--mat-sys-on-secondary-container);
      }
      .section {
        max-width: 920px;
        margin: 0 auto;
        padding: 0 28px;
      }
      .raw-section {
        margin-top: 20px;
        padding-bottom: 48px;
      }
      .raw-toggle {
        display: flex;
        align-items: center;
        justify-content: space-between;
        width: 100%;
        gap: 8px;
        padding: 12px 16px;
        border: none;
        border-radius: 14px;
        background: var(--mat-sys-surface-container-low);
        color: var(--mat-sys-on-surface-variant);
        font: inherit;
        font-size: 13px;
        cursor: pointer;
      }
      .raw-toggle:hover {
        background: var(--mat-sys-surface-container);
      }
      .raw-chevron {
        transition: transform 0.15s ease;
      }
      .raw-chevron.open {
        transform: rotate(180deg);
      }
    `,
  ],
})
export class DiagnosticComponent implements OnInit {
  private readonly aphrody = inject(AphrodyService);
  readonly meta = signal<Meta | null>(null);
  readonly isTauri = this.aphrody.isTauri;
  readonly showRaw = signal(false);

  readonly actions: ToolAction[] = [
    {
      label: "Diagnostic complet (doctor)",
      icon: "monitor_heart",
      args: ["doctor"],
      hint: "Environnement + A2A + supply-chain (sortie texte).",
    },
    {
      label: "Diagnostic JSON",
      icon: "data_object",
      args: ["doctor", "--json"],
      hint: "Même diagnostic, au format JSON structuré.",
    },
    {
      label: "Version et état",
      icon: "info",
      args: ["version"],
      hint: "Version du binaire et état du système.",
    },
    {
      label: "Version JSON",
      icon: "data_object",
      args: ["version", "--json"],
      hint: "Objet JSON unique avec la version.",
    },
  ];

  async ngOnInit(): Promise<void> {
    try {
      this.meta.set(await this.aphrody.meta());
    } catch {
      // non-fatal: header strip simply stays empty
    }
  }
}
