// SPDX-License-Identifier: Apache-2.0
import { Component } from "@angular/core";
import { ToolAction, ToolRunnerComponent } from "../../shared/tool-runner/tool-runner.component";

/** Reverse-engineering view — wraps the real `aphrody re` subcommands. */
@Component({
  selector: "app-reverse",
  imports: [ToolRunnerComponent],
  template: `<app-tool-runner
    title="Reverse engineering"
    subtitle="Triage et analyse de binaires PE / ELF (aphrody re)"
    icon="biotech"
    [actions]="actions"
  />`,
})
export class ReverseComponent {
  readonly actions: ToolAction[] = [
    {
      label: "Triage d'un binaire",
      icon: "biotech",
      args: ["re", "triage"],
      prompt: { placeholder: "Chemin du binaire (PE/ELF)" },
      hint: "Format, sections + entropie, imports/exports, SHA-256.",
    },
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
