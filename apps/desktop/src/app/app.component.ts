// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, inject } from "@angular/core";
import { RouterLink, RouterLinkActive, RouterOutlet } from "@angular/router";
import { MatButtonModule } from "@angular/material/button";
import { MatIconModule } from "@angular/material/icon";
import { MatTooltipModule } from "@angular/material/tooltip";

import { AccountService } from "./core/account.service";
import { ThemeService } from "./core/theme.service";

interface NavItem {
    path: string;
    icon: string;
    label: string;
}

/**
 * App shell: the narrow (~72px) icon-only left rail on the deep-black glow
 * background, plus the routed content. aphrody is a general-purpose autonomous
 * agent (not a reverse-engineering tool), so the rail leads with the Assistant
 * and the full Commandes palette; power tools (re, dns, forensics, doctor, …)
 * are reachable from there. Bottom: theme toggle, signed-in account, settings.
 */
@Component({
    selector: "app-root",
    imports: [
        RouterOutlet,
        RouterLink,
        RouterLinkActive,
        MatButtonModule,
        MatIconModule,
        MatTooltipModule,
    ],
    templateUrl: "./app.component.html",
    styleUrl: "./app.component.scss",
})
export class AppComponent implements OnInit {
    readonly theme = inject(ThemeService);
    readonly account = inject(AccountService);

    readonly nav: NavItem[] = [
        { path: "/assistant", icon: "chat_bubble", label: "Assistant" },
        { path: "/dashboard", icon: "dashboard", label: "Accueil" },
        { path: "/skills", icon: "extension", label: "Skills" },
        { path: "/mcp", icon: "hub", label: "MCP" },
        { path: "/commands", icon: "apps", label: "Commandes" },
    ];

    ngOnInit(): void {
        // Resolve the signed-in Google account (aphrody auth) for the rail avatar.
        void this.account.refresh();
    }
}
