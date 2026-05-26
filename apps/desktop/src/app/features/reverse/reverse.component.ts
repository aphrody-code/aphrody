// SPDX-License-Identifier: Apache-2.0
import { Component } from "@angular/core";
import { MatIconModule } from "@angular/material/icon";

import { ToolAction, ToolRunnerComponent } from "../../shared/tool-runner/tool-runner.component";
import { TriagePanelComponent } from "./triage-panel.component";

/**
 * Reverse-engineering view. The `triage` pillar (R5) is rendered through a
 * dedicated typed M3 panel ({@link TriagePanelComponent}) — header card,
 * sections table with entropy bars, import/export chips, strings sample. The
 * remaining `aphrody re` sub-commands (strings / sections / google / go / auto)
 * keep the generic raw-output {@link ToolRunnerComponent}.
 */
@Component({
  selector: "app-reverse",
  imports: [MatIconModule, ToolRunnerComponent, TriagePanelComponent],
  template: `
    <div class="page">
      <header class="head">
        <div class="head-icon"><mat-icon class="material-symbols-outlined">biotech</mat-icon></div>
        <div>
          <h1>Reverse engineering</h1>
          <p>Triage et analyse de binaires PE / ELF (<code>aphrody re</code>).</p>
        </div>
      </header>

      <app-triage-panel />

      <div class="more">
        <h2 class="section-title">Autres analyses</h2>
        <app-tool-runner
          title="Outils complémentaires"
          subtitle="Chaînes, sections brutes, endpoints Google, détection Go, analyse complète."
          icon="construction"
          [actions]="actions"
        />
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
      .page {
        max-width: 920px;
        margin: 0 auto;
        padding: 28px 28px 64px;
      }
      .head {
        display: flex;
        align-items: center;
        gap: 16px;
        margin-bottom: 20px;
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
      .head p code {
        font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
      }
      .more {
        margin-top: 28px;
      }
      .section-title {
        font-size: 14px;
        font-weight: 500;
        color: var(--mat-sys-on-surface-variant);
        margin: 0 0 12px;
      }
    `,
  ],
})
export class ReverseComponent {
  readonly actions: ToolAction[] = [
    {
      label: "Extraire les chaînes",
      icon: "data_array",
      args: ["re", "strings"],
      prompt: { placeholder: "Chemin du binaire" },
      hint: "Chaînes ASCII et UTF-16LE imprimables.",
    },
    {
      label: "Lister les sections",
      icon: "view_module",
      args: ["re", "sections"],
      prompt: { placeholder: "Chemin du binaire" },
      hint: "Table des sections avec entropie de Shannon.",
    },
    {
      label: "Endpoints Google",
      icon: "travel_explore",
      args: ["re", "google"],
      prompt: { placeholder: "Chemin du binaire" },
      hint: "Extraction des endpoints/URLs Google embarqués.",
    },
    {
      label: "Détection Go",
      icon: "code",
      args: ["re", "go"],
      prompt: { placeholder: "Chemin du binaire" },
      hint: "Détecte un binaire Go et son buildinfo.",
    },
    {
      label: "Analyse automatique complète",
      icon: "auto_awesome",
      args: ["re", "auto"],
      prompt: { placeholder: "Chemin du binaire ou dossier" },
      hint: "Enchaîne triage + strings + endpoints + Go + désassemblage.",
    },
  ];
}
