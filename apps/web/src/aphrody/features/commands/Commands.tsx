// Commands palette: the exhaustive index of every top-level aphrody command (8 groups), each runnable via run([...]) with extra args.

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  MdCircularProgress,
  MdFilledButton,
  MdFilterChip,
  MdIcon,
  MdIconButton,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { run } from "../../client.ts";
import type { ExecResult } from "../../types.ts";
import { Hint, PageHead } from "../../ui.tsx";

/** (F) = feature-gated — may be absent from the default in-process binary. */
type FeatureGate = "forensics" | "image" | "firefly" | "index" | null;

interface CliCommand {
  name: string;
  icon: string;
  desc: string;
  gate: FeatureGate;
  /** Deep-link section to a dedicated rich view (optional). */
  deepLink?: string;
  /** Extra argv tokens prepended before user-supplied extra args. */
  baseArgs?: string[];
  /** Placeholder shown in the extra-args input field. */
  argsHint?: string;
  /** When true, show staticHelp instead of fetching `<cmd> --help`. */
  noHelp?: boolean;
  /** Static explanation shown in place of `--help` (clap external_subcommand). */
  staticHelp?: string;
}

interface CliGroup {
  label: string;
  icon: string;
  commands: CliCommand[];
}

/** All top-level aphrody commands, mirroring crates/cli/src/lib.rs enum Commands. */
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
        // args ARE the argv -> baseArgs empty.
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
        deepLink: "reverse",
        argsHint: "triage chemin/vers/binaire",
      },
      {
        name: "forensics",
        icon: "travel_explore",
        desc: "Extraction forensique reproductible",
        gate: "forensics",
        deepLink: "forensics",
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
        deepLink: "diagnostic",
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
        deepLink: "mcp",
        argsHint: "list",
      },
    ],
  },
];

const ALL_COMMANDS: CliCommand[] = COMMAND_GROUPS.flatMap((g) => g.commands);

const GATE_LABEL: Record<string, string> = {
  forensics: "forensics",
  image: "image",
  firefly: "firefly",
  index: "index",
};

function featureNote(gate: FeatureGate): string {
  if (!gate) return "";
  return `Nécessite un build avec --features ${GATE_LABEL[gate] ?? gate}`;
}

/** Per-card run state, keyed by command name. */
interface CardState {
  expanded: boolean;
  extraArgs: string;
  running: boolean;
  result: ExecResult | null;
  ranLabel: string;
}

const EMPTY_CARD: CardState = {
  expanded: false,
  extraArgs: "",
  running: false,
  result: null,
  ranLabel: "",
};

export function Commands() {
  const [query, setQuery] = useState("");
  const [activeGroup, setActiveGroup] = useState<string | null>(null);
  const [cards, setCards] = useState<Record<string, CardState>>({});

  useEffect(() => {
    const initial: Record<string, CardState> = {};
    for (const cmd of ALL_COMMANDS) initial[cmd.name] = { ...EMPTY_CARD };
    setCards(initial);
  }, []);

  const card = useCallback((name: string): CardState => cards[name] ?? EMPTY_CARD, [cards]);

  const updateCard = useCallback((name: string, patch: Partial<CardState>) => {
    setCards((m) => ({ ...m, [name]: { ...(m[name] ?? EMPTY_CARD), ...patch } }));
  }, []);

  const toggle = useCallback(
    (cmd: CliCommand) => {
      const c = cards[cmd.name] ?? EMPTY_CARD;
      updateCard(cmd.name, { expanded: !c.expanded, result: null });
    },
    [cards, updateCard],
  );

  const runCmd = useCallback(
    async (cmd: CliCommand) => {
      const c = cards[cmd.name] ?? EMPTY_CARD;
      if (c.running) return;
      // Parse extra args (simple whitespace split; quotes not handled).
      const extra = c.extraArgs
        .trim()
        .split(/\s+/)
        .filter((a) => a.length > 0);
      const argv = [...(cmd.baseArgs ?? [cmd.name]), ...extra];
      updateCard(cmd.name, {
        running: true,
        result: null,
        ranLabel: ["aphrody", ...argv].join(" "),
      });
      const result = await run(argv);
      updateCard(cmd.name, { running: false, result });
    },
    [cards, updateCard],
  );

  const filteredGroups = useMemo(() => {
    const q = query.trim().toLowerCase();
    return COMMAND_GROUPS.map((g) => {
      const commands = g.commands.filter((c) => {
        if (activeGroup && g.label !== activeGroup) return false;
        if (!q) return true;
        return (
          c.name.includes(q) ||
          c.desc.toLowerCase().includes(q) ||
          g.label.toLowerCase().includes(q)
        );
      });
      return { ...g, commands };
    }).filter((g) => g.commands.length > 0);
  }, [query, activeGroup]);

  return (
    <div className="aph-commands">
      <PageHead
        title="Commandes"
        subtitle={`Toute la surface du CLI aphrody — ${ALL_COMMANDS.length} commandes, chacune exécutable.`}
      />

      <div className="aph-row" style={{ marginBottom: 12 }}>
        <MdOutlinedTextField
          style={{ flex: "1 1 auto" }}
          label="Filtrer par nom, description ou groupe"
          value={query}
          onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
        >
          <MdIcon slot="leading-icon">search</MdIcon>
          {query ? (
            <MdIconButton slot="trailing-icon" aria-label="Effacer" onClick={() => setQuery("")}>
              <MdIcon>close</MdIcon>
            </MdIconButton>
          ) : null}
        </MdOutlinedTextField>
      </div>

      <div className="aph-chips" style={{ marginBottom: 16 }}>
        {COMMAND_GROUPS.map((g) => (
          <MdFilterChip
            key={g.label}
            label={g.label}
            selected={activeGroup === g.label}
            onClick={() => setActiveGroup((cur) => (cur === g.label ? null : g.label))}
          />
        ))}
      </div>

      {filteredGroups.length === 0 ? (
        <Hint icon="search_off" title={`Aucune commande ne correspond à « ${query} ».`} />
      ) : (
        filteredGroups.map((group) => (
          <section key={group.label} style={{ marginBottom: 24 }}>
            <div className="aph-row" style={{ gap: 8, marginBottom: 10 }}>
              <MdIcon style={{ color: "var(--md-sys-color-primary)" }}>{group.icon}</MdIcon>
              <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>{group.label}</h2>
              <span className="aph-tag">{group.commands.length}</span>
            </div>

            <div className="aph-grid">
              {group.commands.map((cmd) => {
                const c = card(cmd.name);
                return (
                  <div key={cmd.name} className="aph-cmd-card">
                    <div
                      className="aph-row"
                      style={{ gap: 12, cursor: "pointer" }}
                      onClick={() => toggle(cmd)}
                    >
                      <MdIcon style={{ color: "var(--md-sys-color-primary)" }}>{cmd.icon}</MdIcon>
                      <div style={{ flex: "1 1 auto", minWidth: 0 }}>
                        <div className="aph-row" style={{ gap: 6, flexWrap: "wrap" }}>
                          <code style={{ fontFamily: '"Roboto Mono", ui-monospace, monospace' }}>
                            aphrody {cmd.name}
                          </code>
                          {cmd.gate !== null && (
                            <span className="aph-tag aph-tag--accent" title={featureNote(cmd.gate)}>
                              (F)
                            </span>
                          )}
                        </div>
                        <span className="aph-muted" style={{ fontSize: 12 }}>
                          {cmd.desc}
                        </span>
                      </div>
                      <MdIconButton aria-label={c.expanded ? "Réduire" : "Développer"}>
                        <MdIcon>{c.expanded ? "expand_less" : "expand_more"}</MdIcon>
                      </MdIconButton>
                    </div>

                    {c.expanded && (
                      <div style={{ marginTop: 12 }}>
                        {cmd.noHelp && cmd.staticHelp && (
                          <p className="aph-muted" style={{ fontSize: 12, margin: "0 0 10px" }}>
                            {cmd.staticHelp}
                          </p>
                        )}
                        {cmd.gate !== null && (
                          <p
                            className="aph-row aph-muted"
                            style={{ fontSize: 12, gap: 6, margin: "0 0 10px" }}
                          >
                            <MdIcon style={{ fontSize: 16 }}>construction</MdIcon>
                            {featureNote(cmd.gate)} — feature-gate.
                          </p>
                        )}

                        <div className="aph-action__controls">
                          <MdOutlinedTextField
                            style={{ flex: "1 1 auto" }}
                            label="Arguments"
                            placeholder={cmd.argsHint ?? "--help"}
                            value={c.extraArgs}
                            onInput={(e) =>
                              updateCard(cmd.name, {
                                extraArgs: (e.target as HTMLInputElement).value,
                              })
                            }
                          />
                          <MdFilledButton disabled={c.running} onClick={() => void runCmd(cmd)}>
                            {c.running ? (
                              <MdCircularProgress indeterminate slot="icon" />
                            ) : (
                              <MdIcon slot="icon">play_arrow</MdIcon>
                            )}
                            {c.running ? "Exécution…" : "Exécuter"}
                          </MdFilledButton>
                        </div>

                        {c.ranLabel && (
                          <div className="aph-output" style={{ marginTop: 12 }}>
                            <div className="aph-output__head">
                              <MdIcon>terminal</MdIcon>
                              <span className="aph-output__cmd">{c.ranLabel}</span>
                              {c.result && (
                                <span
                                  className={`aph-code ${
                                    c.result.code === 0 ? "aph-code--ok" : "aph-code--err"
                                  }`}
                                >
                                  code {c.result.code}
                                </span>
                              )}
                            </div>
                            <pre className="aph-output__body">
                              {c.running
                                ? "Exécution en cours…"
                                : c.result
                                  ? `${c.result.stdout}${
                                      c.result.stderr ? `\n${c.result.stderr}` : ""
                                    }`
                                  : ""}
                            </pre>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </section>
        ))
      )}
    </div>
  );
}
