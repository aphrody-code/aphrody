// SPDX-License-Identifier: Apache-2.0
import { Component } from "@angular/core";
import { ToolAction, ToolRunnerComponent } from "../../shared/tool-runner/tool-runner.component";

/** Network view — DNS OSINT reconnaissance + Google search (aphrody dns / search). */
@Component({
  selector: "app-network",
  imports: [ToolRunnerComponent],
  template: `<app-tool-runner
    title="Réseau"
    subtitle="Reconnaissance DNS et recherche Google native (aphrody dns / search)"
    icon="dns"
    [actions]="actions"
  />`,
})
export class NetworkComponent {
  readonly actions: ToolAction[] = [
    {
      label: "Reconnaissance DNS",
      icon: "dns",
      args: ["dns"],
      prompt: { placeholder: "Domaine (ex. example.com)" },
      hint: "Résolution DNS OSINT agressive d'un domaine.",
    },
    {
      label: "Recherche Google native",
      icon: "search",
      args: ["search"],
      prompt: { placeholder: "Requête de recherche" },
      hint: "Recherche Google native via le compte connecté.",
    },
  ];
}
