// SPDX-License-Identifier: Apache-2.0
import { Routes } from "@angular/router";

export const routes: Routes = [
    { path: "", pathMatch: "full", redirectTo: "assistant" },
    {
        path: "assistant",
        loadComponent: () =>
            import("./features/assistant/assistant.component").then((m) => m.AssistantComponent),
    },
    {
        path: "dashboard",
        loadComponent: () =>
            import("./features/dashboard/dashboard.component").then((m) => m.DashboardComponent),
    },
    {
        path: "skills",
        loadComponent: () =>
            import("./features/skills/skills.component").then((m) => m.SkillsComponent),
    },
    {
        path: "mcp",
        loadComponent: () => import("./features/mcp/mcp.component").then((m) => m.McpComponent),
    },
    {
        path: "reverse",
        loadComponent: () =>
            import("./features/reverse/reverse.component").then((m) => m.ReverseComponent),
    },
    {
        path: "forensics",
        loadComponent: () =>
            import("./features/forensics/forensics.component").then((m) => m.ForensicsComponent),
    },
    {
        path: "network",
        loadComponent: () =>
            import("./features/network/network.component").then((m) => m.NetworkComponent),
    },
    {
        path: "diagnostic",
        loadComponent: () =>
            import("./features/diagnostic/diagnostic.component").then((m) => m.DiagnosticComponent),
    },
    {
        path: "commands",
        loadComponent: () =>
            import("./features/commands/commands.component").then((m) => m.CommandsComponent),
    },
    {
        path: "settings",
        loadComponent: () =>
            import("./features/settings/settings.component").then((m) => m.SettingsComponent),
    },
    {
        path: "about",
        loadComponent: () =>
            import("./features/about/about.component").then((m) => m.AboutComponent),
    },
    { path: "**", redirectTo: "assistant" },
];
