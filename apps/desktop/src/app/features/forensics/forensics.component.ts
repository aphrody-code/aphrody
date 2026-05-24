// SPDX-License-Identifier: Apache-2.0
import { Component } from "@angular/core";
import { ToolAction, ToolRunnerComponent } from "../../shared/tool-runner/tool-runner.component";

/** Forensics view — Chromium forensics + safe SQLite schema dump (aphrody chromium / forensics). */
@Component({
  selector: "app-forensics",
  imports: [ToolRunnerComponent],
  template: `<app-tool-runner
    title="Forensics"
    subtitle="Extraction forensique reproductible (lecture seule, jamais de secrets en clair)"
    icon="travel_explore"
    [actions]="actions"
  />`,
})
export class ForensicsComponent {
  readonly actions: ToolAction[] = [
    {
      label: "Carte du système de fichiers",
      icon: "account_tree",
      args: ["forensics", "map"],
      prompt: { placeholder: "Racine à cartographier (chemin)" },
      hint: "Cartographie + classification, sans copier de valeurs secrètes.",
    },
    {
      label: "Schéma SQLite (lecture seule)",
      icon: "database",
      args: ["forensics", "sqlite"],
      prompt: { placeholder: "Chemin de la base .db / .vscdb", flag: "--db" },
      hint: "Lit sqlite_master uniquement (noms + CREATE), jamais les valeurs.",
    },
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
