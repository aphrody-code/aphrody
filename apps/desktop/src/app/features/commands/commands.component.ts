// SPDX-License-Identifier: Apache-2.0
import {
  Component,
  OnInit,
  computed,
  inject,
  signal,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { RouterLink } from "@angular/router";
import { MatButtonModule } from "@angular/material/button";
import { MatDividerModule } from "@angular/material/divider";
import { MatIconModule } from "@angular/material/icon";
import { MatProgressSpinnerModule } from "@angular/material/progress-spinner";
import { MatTooltipModule } from "@angular/material/tooltip";

import { AphrodyService, ExecResult } from "../../core/aphrody.service";

// ---------------------------------------------------------------------------
// Command definitions
// ---------------------------------------------------------------------------

/** (F) = feature-gated — may be absent from the default in-process binary. */
export type FeatureGate = "forensics" | "image" | "firefly" | "index" | null;

export interface CliGroup {
  label: string;
  icon: string;
  commands: CliCommand[];
}

export interface CliCommand {
  name: string;
  icon: string;
  desc: string;
  gate: FeatureGate;
  /** Deep-link to a dedicated rich view (optional). */
  deepLink?: string;
  /** Extra argv tokens prepended before user-supplied extra args. */
  baseArgs?: string[];
  /** Placeholder shown in the extra-args input field. */
  argsHint?: string;
  /** When true, skip the live `<cmd> --help` exec and show `staticHelp`. */
  noHelp?: boolean;
  /** Static explanation shown in place of `--help` (clap external_subcommand). */
  staticHelp?: string;
}

/** All 37 top-level aphrody commands, mirroring crates/cli/src/lib.rs enum Commands. */
const COMMAND_GROUPS: CliGroup[] = [
  {
    label: "Assistant et chat",
    icon: "chat_bubble",
    commands: [
      {
        name: "chat",
        icon: "forum",
        desc: "Agent de chat turn-loop (Gemini 3.5 Flash)",
        gate: null,
        argsHint: '--prompt "Bonjour"',
      },
      {
        name: "hermes",
        icon: "graphic_eq",
        desc: "Agent multi-canaux voice-to-voice (Discord + X)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "a2a",
        icon: "swap_horiz",
        desc: "Client natif A2A (Agent-to-Agent protocol)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "auto",
        icon: "bolt",
        desc: "Exécution auto : prompt en langage naturel (agent A2A) ou script Bun / Uv / cargo",
        gate: null,
        // external_subcommand: the user types no literal "auto" token. The extra
        // args ARE the argv (e.g. a natural-language prompt) -> baseArgs empty.
        baseArgs: [],
        argsHint: "résume les changements récents du dépôt",
        noHelp: true,
        staticHelp:
          "auto est le routeur par défaut de clap (external_subcommand) : « aphrody <texte> » " +
          "sans sous-commande connue arrive ici. Un prompt en langage naturel est routé vers le " +
          "client A2A (agent) ; sinon les arguments sont exécutés via le moteur Bun / Uv / cargo / " +
          "script. Il n'a pas d'aide --help propre : les arguments saisis ci-dessous sont transmis " +
          "verbatim (aucune sous-commande « auto » littérale n'est ajoutée).",
      },
    ],
  },
  {
    label: "Recherche et web",
    icon: "search",
    commands: [
      {
        name: "search",
        icon: "search",
        desc: "Recherche Google native",
        gate: null,
        argsHint: '"requête de recherche"',
      },
      {
        name: "dns",
        icon: "dns",
        desc: "Résolution DNS et OSINT réseau",
        gate: null,
        deepLink: "/network",
        argsHint: "example.com",
      },
      {
        name: "gemini",
        icon: "auto_awesome",
        desc: "Binaire natif Gemini CLI (forward)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "notebooklm",
        icon: "menu_book",
        desc: "Client RPC Google NotebookLM",
        gate: null,
        argsHint: "--help",
      },
    ],
  },
  {
    label: "Reverse engineering et forensics",
    icon: "biotech",
    commands: [
      {
        name: "re",
        icon: "biotech",
        desc: "Reverse engineering de binaires (sections, strings, triage)",
        gate: null,
        deepLink: "/reverse",
        argsHint: "triage chemin/vers/binaire",
      },
      {
        name: "forensics",
        icon: "travel_explore",
        desc: "Extraction forensique reproductible",
        gate: "forensics",
        deepLink: "/forensics",
        argsHint: "--help",
      },
      {
        name: "chromium",
        icon: "cookie",
        desc: "Forensics Chromium (cookies, profils ABE)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "scan",
        icon: "analytics",
        desc: "Analyse du repo (arbre + manifestes)",
        gate: null,
        argsHint: "--help",
      },
    ],
  },
  {
    label: "Création et médias",
    icon: "palette",
    commands: [
      {
        name: "image",
        icon: "image",
        desc: "Génération d'images Gemini (Nano Banana)",
        gate: "image",
        argsHint: '--prompt "une forêt lumineuse"',
      },
      {
        name: "firefly",
        icon: "local_fire_department",
        desc: "Adobe Firefly Services (génération, expand, fill)",
        gate: "firefly",
        argsHint: 'generate --prompt "motif abstrait"',
      },
      {
        name: "design",
        icon: "palette",
        desc: "Tokens Material 3 (export CSS / JSON)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "logo",
        icon: "stars",
        desc: "Rendu et export du logo aphrody",
        gate: null,
        argsHint: "--help",
      },
    ],
  },
  {
    label: "Mémoire et agents",
    icon: "psychology",
    commands: [
      {
        name: "memory",
        icon: "psychology",
        desc: "Fournisseurs de mémoire (Mem0 / Honcho)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "oc-onboard",
        icon: "person_add",
        desc: "Onboarding d'un nouvel agent (openclaw)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "oc-reset",
        icon: "restart_alt",
        desc: "Réinitialisation de l'état agent (openclaw)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "oc-uninstall",
        icon: "delete_sweep",
        desc: "Désinstallation de l'agent (openclaw)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "oc-pairing",
        icon: "cable",
        desc: "Appairage de dispositifs openclaw",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "oc-docs",
        icon: "description",
        desc: "Documentation de l'API openclaw",
        gate: null,
        argsHint: "--help",
      },
    ],
  },
  {
    label: "Canaux et notifications",
    icon: "notifications",
    commands: [
      {
        name: "notify",
        icon: "notifications",
        desc: "Notifier via Slack / Telegram / Matrix",
        gate: null,
        argsHint: '--slack "message"',
      },
    ],
  },
  {
    label: "Modèles et comptes IA",
    icon: "rocket_launch",
    commands: [
      {
        name: "antigravity",
        icon: "rocket",
        desc: "Client natif Google AI Ultra / Gemini (Antigravity SDK)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "agy",
        icon: "rocket_launch",
        desc: "Forward vers le CLI Antigravity (agy)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "agy-loop",
        icon: "loop",
        desc: "Boucle autonome agy (hook AfterAgent, no-human-in-loop)",
        gate: null,
        argsHint: "--help",
      },
    ],
  },
  {
    label: "Système et installation",
    icon: "settings",
    commands: [
      {
        name: "doctor",
        icon: "monitor_heart",
        desc: "Diagnostic env + A2A + supply-chain",
        gate: null,
        deepLink: "/diagnostic",
        argsHint: "--help",
      },
      {
        name: "version",
        icon: "info",
        desc: "Version et état du système",
        gate: null,
      },
      {
        name: "self",
        icon: "settings_suggest",
        desc: "Installer / mettre à jour / bootstrapper aphrody",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "completions",
        icon: "keyboard",
        desc: "Génération de completions shell (bash / zsh / fish)",
        gate: null,
        argsHint: "--shell bash",
      },
      {
        name: "mirror",
        icon: "sync_alt",
        desc: "Mirroring des assets Material Design 3",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "auth",
        icon: "key",
        desc: "Authentification Google (OAuth2)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "cros",
        icon: "memory",
        desc: "Compilation hyper-optimisée de ChromeOS (CrOS toolchain)",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "term",
        icon: "dvr",
        desc: "Pont WebSocket-PTY pour le frontend WASM",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "ide",
        icon: "terminal",
        desc: "Inspecteur d'installation Antigravity IDE",
        gate: null,
        argsHint: "--help",
      },
      {
        name: "index",
        icon: "manage_search",
        desc: "Index de recherche local FTS5 (sqlite)",
        gate: "index",
        argsHint: "--help",
      },
      {
        name: "mcp",
        icon: "hub",
        desc: "Serveur Model Context Protocol (outils, list, call)",
        gate: null,
        deepLink: "/mcp",
        argsHint: "list",
      },
    ],
  },
];

// ---------------------------------------------------------------------------
// Per-card run state
// ---------------------------------------------------------------------------

export interface CardState {
  running: boolean;
  result: ExecResult | null;
  ranLabel: string;
  /** Live --help text fetched at load; null = not yet loaded. */
  helpText: string | null;
  helpLoading: boolean;
  /** Extra arg string typed by the user in this card's input. */
  extraArgs: string;
  /** True when card is expanded (showing help + run form). */
  expanded: boolean;
  /** Whether the command is absent from `aphrody --help` (feature-gated). */
  absent: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/**
 * Commandes palette — the exhaustive index of every top-level `aphrody`
 * command (all 37, mirroring the real clap `Commands` enum in
 * `crates/cli/src/lib.rs`).
 *
 * At load it runs `aphrody --help` once to detect which feature-gated commands
 * are absent from the in-process binary. Each card fetches real `--help` output
 * on expand. Every card is runnable via `aphrody_exec`. Feature-gated commands
 * that are absent show an honest degradation note — they are never hidden.
 *
 * Dedicated rich views (reverse, dns, forensics, diagnostic) are deep-linked
 * from the relevant cards; they are never deleted.
 */
@Component({
  selector: "app-commands",
  imports: [
    FormsModule,
    MatButtonModule,
    MatDividerModule,
    MatIconModule,
    MatProgressSpinnerModule,
    MatTooltipModule,
    RouterLink,
  ],
  templateUrl: "./commands.component.html",
  styleUrl: "./commands.component.scss",
})
export class CommandsComponent implements OnInit {
  private readonly aphrody = inject(AphrodyService);

  readonly query = signal("");
  /** Set of command names absent from `aphrody --help` (feature-gated). */
  readonly absentCommands = signal<Set<string>>(new Set());
  /** Whether the initial `aphrody --help` probe is still in flight. */
  readonly probing = signal(true);

  readonly groups = COMMAND_GROUPS;
  readonly allCommands: CliCommand[] = COMMAND_GROUPS.flatMap((g) => g.commands);

  /** Per-command reactive state. */
  readonly cardState = signal<Record<string, CardState>>({});

  readonly filteredGroups = computed(() => {
    const q = this.query().trim().toLowerCase();
    if (!q) return this.groups;
    return this.groups
      .map((g) => ({
        ...g,
        commands: g.commands.filter(
          (c) =>
            c.name.includes(q) ||
            c.desc.toLowerCase().includes(q) ||
            g.label.toLowerCase().includes(q),
        ),
      }))
      .filter((g) => g.commands.length > 0);
  });

  async ngOnInit(): Promise<void> {
    // Initialise per-card state.
    const initial: Record<string, CardState> = {};
    for (const cmd of this.allCommands) {
      initial[cmd.name] = {
        running: false,
        result: null,
        ranLabel: "",
        helpText: null,
        helpLoading: false,
        extraArgs: "",
        expanded: false,
        absent: false,
      };
    }
    this.cardState.set(initial);

    // Probe the binary for feature-gated commands.
    await this.probeAvailability();
  }

  /** Run `aphrody --help` once and mark absent feature-gated commands. */
  private async probeAvailability(): Promise<void> {
    this.probing.set(true);
    try {
      const res = await this.aphrody.exec(["--help"]);
      const output = (res.stdout + res.stderr).toLowerCase();
      const absent = new Set<string>();
      for (const cmd of this.allCommands) {
        if (cmd.gate !== null && !output.includes(cmd.name)) {
          absent.add(cmd.name);
          this.updateCard(cmd.name, { absent: true });
        }
      }
      this.absentCommands.set(absent);
    } catch {
      // Could not probe (browser / no host) — mark nothing absent.
    } finally {
      this.probing.set(false);
    }
  }

  private updateCard(name: string, patch: Partial<CardState>): void {
    this.cardState.update((m) => ({
      ...m,
      [name]: { ...m[name], ...patch } as CardState,
    }));
  }

  card(name: string): CardState {
    return (
      this.cardState()[name] ?? {
        running: false,
        result: null,
        ranLabel: "",
        helpText: null,
        helpLoading: false,
        extraArgs: "",
        expanded: false,
        absent: false,
      }
    );
  }

  /** Toggle the card's expanded state; load --help on first expand. */
  async toggle(cmd: CliCommand): Promise<void> {
    const c = this.card(cmd.name);
    const next = !c.expanded;
    this.updateCard(cmd.name, { expanded: next, result: null });

    if (next && c.helpText === null && !c.helpLoading) {
      await this.loadHelp(cmd);
    }
  }

  /** Fetch `aphrody <cmd> --help` and cache it in the card state. */
  private async loadHelp(cmd: CliCommand): Promise<void> {
    // external_subcommand (auto) has no clap --help: show the static note.
    if (cmd.noHelp) {
      this.updateCard(cmd.name, { helpText: cmd.staticHelp ?? "", helpLoading: false });
      return;
    }
    this.updateCard(cmd.name, { helpLoading: true });
    try {
      const res = await this.aphrody.exec([cmd.name, "--help"]);
      const text = (res.stdout || res.stderr || "(aucune aide disponible)").trim();
      this.updateCard(cmd.name, { helpText: text, helpLoading: false });
    } catch (err) {
      this.updateCard(cmd.name, {
        helpText: `Aide non disponible : ${String(err)}`,
        helpLoading: false,
      });
    }
  }

  /** Run the command with baseArgs + user-supplied extra args. */
  async run(cmd: CliCommand): Promise<void> {
    const c = this.card(cmd.name);
    if (c.running || c.absent) return;

    // Parse extra args (simple whitespace split; quotes not handled).
    const extra = c.extraArgs
      .trim()
      .split(/\s+/)
      .filter((a) => a.length > 0);
    const argv = [...(cmd.baseArgs ?? [cmd.name]), ...extra];

    this.updateCard(cmd.name, {
      running: true,
      result: null,
      ranLabel: ["aphrody", ...argv].join(" "),
    });
    try {
      const result = await this.aphrody.exec(argv);
      this.updateCard(cmd.name, { result });
    } catch (err) {
      this.updateCard(cmd.name, {
        result: { code: -1, stdout: "", stderr: String(err) },
      });
    } finally {
      this.updateCard(cmd.name, { running: false });
    }
  }

  setExtraArgs(name: string, value: string): void {
    this.updateCard(name, { extraArgs: value });
  }

  featureNote(gate: FeatureGate): string {
    if (!gate) return "";
    const map: Record<string, string> = {
      forensics: "forensics",
      image: "image",
      firefly: "firefly",
      index: "index",
    };
    return `Nécessite un build avec --features ${map[gate] ?? gate}`;
  }
}
