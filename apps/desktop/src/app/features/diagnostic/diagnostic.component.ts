// SPDX-License-Identifier: Apache-2.0
import { Component, inject, OnInit, signal } from "@angular/core";
import { ToolAction, ToolRunnerComponent } from "../../shared/tool-runner/tool-runner.component";
import { AphrodyService, Meta } from "../../core/aphrody.service";

/** Diagnostic view — version, doctor, host metadata (aphrody version / doctor). */
@Component({
  selector: "app-diagnostic",
  imports: [ToolRunnerComponent],
  template: `
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
    <app-tool-runner
      title="Diagnostic"
      subtitle="État du système, supply-chain et intégration A2A (aphrody doctor / version)"
      icon="monitor_heart"
      [actions]="actions"
    />
  `,
  styles: [
    `
      .meta-strip {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        max-width: 920px;
        margin: 20px auto -8px;
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
    `,
  ],
})
export class DiagnosticComponent implements OnInit {
  private readonly aphrody = inject(AphrodyService);
  readonly meta = signal<Meta | null>(null);
  readonly isTauri = this.aphrody.isTauri;

  readonly actions: ToolAction[] = [
    {
      label: "Diagnostic complet (doctor)",
      icon: "monitor_heart",
      args: ["doctor"],
      hint: "Environnement + A2A + supply-chain (première impression).",
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
