<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — North Star : Antigravity, réécrit en Rust

> Statut : **vision canonique** (2026-05-27). Ce document fixe le cap d'aphrody
> et prime sur les plans plus étroits (`docs/plans/gemini-codex.md` devient le
> sous-plan « moteur »). Aligné avec `CLAUDE.md` §0 (cap projet) et §0.1
> (autonomie totale).

## 0. Le problème : un CLI disloqué

Aujourd'hui `aphrody` est un **couteau-suisse de ~40 sous-commandes hétérogènes**
sans fil conducteur agentique :

```
auth  mirror  dns  version  doctor  chromium  cros  search  gemini  gui
agy  agy-loop  term  mcp  re  forensics  index  image  firefly  memory
chat  hermes  notify  completions  self  scan  oc-*  notebooklm  antigravity
logo  design  ide  tokens  …
```

Chaque commande est un outil isolé. Il n'y a pas de **moteur d'agent** central,
pas de **plan de contrôle** des agents, pas d'expérience unifiée. C'est riche
mais fragmenté.

## 1. La vision

**aphrody devient une plateforme agentique unifiée — une réécriture
d'Antigravity en Rust** — tout-en-un, **axée sur l'autonomie totale et le
contrôle total des agents.**

Un **moteur d'agent unique** (le cœur), exposé par **plusieurs surfaces minces**
(CLI, TUI, app desktop, serveur MCP, réseau A2A, FFI). Plus de commandes
disloquées : tout converge vers *piloter, superviser et automatiser des agents*.

Inspirations, et ce qu'on prend de chacune :

| Source | Ce qu'aphrody en reprend |
|---|---|
| **Antigravity** (cible : IDE agentique Google, fork Windsurf/Codeium, sidecar Go `jetski`/`cortex`/`cascade`) | Le modèle **Manager/Cascade** (orchestration multi-agents), les 3 surfaces (éditeur + panneau d'agents + navigateur), l'auth Google AI Ultra / Gemini (token agy via Credential Manager — `antigravity-sdk`) |
| **OpenAI Codex** (Apache-2.0, `var/codex`) | La **boucle de turn agentique** streaming, `apply-patch`, le catalogue d'outils, le sandboxing, le **protocole NDJSON**, le **TUI Ratatui** |
| **agy CLI** (Antigravity CLI) | Backend Gemini via token agy, la boucle no-human-in-the-loop (`agy-loop`) |
| **Gemini CLI** (Google, open source) | UX CLI agentique (ReAct loop, slash-commands, écosystème d'outils, multimodal) |
| **Google desktop app** | L'expérience desktop **Material 3** (l'app Tauri + Angular Material déjà dans `apps/desktop`) |

## 2. Architecture unifiée : un moteur, des surfaces

```
                    ┌─────────────────────────────────────────────┐
   SURFACES         │  CLI    TUI    Desktop(Tauri)   MCP   A2A   FFI │
   (clients minces) └───────────────────────┬─────────────────────┘
                                             │  (protocole NDJSON / JSON-RPC)
                    ┌────────────────────────▼─────────────────────┐
   CONTROL PLANE    │  Superviseur multi-agents · politiques/garde-  │
   (autonomie +     │  fous (aphrody-guard) · approbations/steering ·│
    contrôle)       │  parallélisme · A2A · agy-loop/autopilot ·     │
                    │  scheduling (cron) · observabilité (rollout)   │
                    └────────────────────────┬─────────────────────┘
                    ┌────────────────────────▼─────────────────────┐
   ENGINE           │  Turn loop · Tools · Skills · Memory · Session·│
   (moteur d'agent) │  Providers (Gemini REST / gemini-web / agy)    │
                    └────────────────────────┬─────────────────────┘
                    ┌────────────────────────▼─────────────────────┐
   CAPABILITIES     │  apply-patch · shell sandboxé · fs-search · RE/│
   (outils réels)   │  forensics · web/browser · image/firefly · …   │
                    └───────────────────────────────────────────────┘
```

### 2.1 ENGINE — le moteur d'agent (cœur, Rust pur)
Boucle de turn unique partagée par toutes les surfaces (cf. `gemini-codex.md`) :
- `run_turn` streaming → tool-calls → ré-injection multi-tour → `TurnComplete` ;
- trait **`ModelClient`** abstrayant les providers : Gemini REST (`gemini-runtime`),
  gemini-web (cookies), token agy (`antigravity-sdk`) ;
- **protocole** SQ/EQ NDJSON (`Submission`/`Event`) — le même fil pour CLI, TUI,
  desktop, MCP ;
- **tools** (`aphrody-tools` + `apply-patch` à porter), **skills**
  (`aphrody-skills`), **memory** (`aphrody-memory`/`aphrody-agent-home`),
  **session** (`aphrody-session` + rollout JSONL).

### 2.2 CONTROL PLANE — autonomie + contrôle total des agents (le différenciateur)
C'est ce qui fait d'aphrody « Antigravity en Rust » plutôt qu'un simple agent :
- **superviseur multi-agents** : spawn / observe / steer / interrompt / tue des
  agents concurrents (locaux et distants) ;
- **modes d'autonomie** : du *gated* (approbation par tool-call) au *full
  autonomous* (no-human-in-the-loop, §0.1) — défaut autonome, garde-fous
  **opt-in** (`aphrody-guard`, `APHRODY_GUARD=1`) ;
- **steering live** : injecter des consignes en cours de turn (`steer_input`) ;
- **politiques** : command-safety, sandbox, limites de ressources, hooks de
  cycle de vie (`PreToolUse`/`PostToolUse`/`Stop`) ;
- **coordination A2A** : duels/collaboration multi-Claude/Gemini (`a2a-*`,
  `.coord`), peer winclean C# ;
- **boucles autonomes** : `agy-loop`, autopilot, cron (`aphrody-cron`) ;
- **observabilité** : rollout/trace rejouable, télémétrie (`aphrody-telemetry`).

### 2.3 SURFACES — clients minces sur le moteur (« tout-en-un »)
- **CLI** : verbes réorganisés autour de l'agent (cf. §4), scriptable, sans TTY.
- **TUI** : agent interactif plein écran (Ratatui 0.30 upstream — jalon GC-8 ;
  les 4 features instables nécessaires sont dispo en 0.30, fork nornagon inutile).
- **Desktop (Tauri + Angular Material 3)** : clone Antigravity IDE — éditeur +
  panneau d'agents + navigateur ; `apps/desktop` existe déjà, appelle le moteur
  in-process.
- **Serveur MCP** : expose l'agent comme outil MCP (`gemini`/`gemini-reply`).
- **A2A** : l'agent comme nœud d'un réseau agent-à-agent.
- **FFI** : `aphrody-ffi` cdylib pour embarquer le moteur (Bun / C-ABI).

## 3. Le contrôle total des agents (détail)

| Capacité | Brique aphrody | État |
|---|---|---|
| Lancer/superviser/tuer des agents | `aphrody-supervisor` (fan-in + lifecycle) sur `aphrody-engine::spawn_session` | En cours (Phase 3a) |
| Approbations exec/patch | protocole `ExecApprovalRequest` + `ApprovalGate` (mode `Gated`) | **Livré** (engine) |
| Steering en cours de turn | `Op::Interrupt` + `InterruptFlag` coopératif | **Livré** (interrupt) ; steer_input à porter |
| Garde-fous (sandbox, command-safety) | `aphrody-guard` (opt-in) | **Livré** |
| Hooks de cycle de vie | `aphrody-skills/hooks` | Présent |
| Coordination multi-agents | `a2a-*`, `.coord` | Présent |
| Boucles autonomes | `agy-loop`, autopilot | Présent |
| Persistence/replay de session | rollout JSONL (`aphrody-rollout`, miroir des events) | **Livré** |
| Scheduling | `aphrody-cron` | Présent |
| Observabilité | `aphrody-telemetry`, rollout-trace | Partiel |

## 4. Réorganiser le CLI disloqué → surface agent-centrique

Regrouper les ~40 commandes en familles cohérentes, avec alias rétro-compatibles :

- **`aphrody agent`** (cœur) : unifie `chat`, `exec`, `agy`, `agy-loop`,
  `hermes`, `antigravity` → un seul agent multi-provider, multi-mode
  (interactif/exec/loop), multi-canal.
- **`aphrody tui`** / **`aphrody desktop`** : surfaces interactives.
- **capacités d'agent** (outils, plus des verbes top-level) : `re`, `forensics`,
  `scan`, `chromium`, `index`/`search`, `notebooklm` deviennent des **tools**
  exposés au moteur, accessibles aussi en direct.
- **créatif** : `image`, `firefly`, `logo`, `design`, `tokens`.
- **système** : `auth`, `doctor`, `version`, `completions`, `self`, `oc-*`.
- **intégrations** : `mcp`, `a2a`, `ide`, `notify`, `term`, `mirror`, `dns`,
  `cros`.

Objectif : qu'un nouvel utilisateur tape `aphrody` (→ TUI agent) ou
`aphrody agent "..."` et obtienne immédiatement un agent autonome, pas une liste
de 40 outils.

## 5. Mapping des assets existants → architecture unifiée

| Brique existante | Rôle dans la plateforme unifiée |
|---|---|
| `gemini-runtime`, `gemini-web` | Providers du moteur (trait `ModelClient`) |
| `antigravity-sdk` | Provider + auth Google AI Ultra (token agy) |
| `aphrody-chat`, `hermes` | Fusionnés dans `aphrody agent` |
| `aphrody-session`, `aphrody-events` | Protocole + boucle de turn + rollout |
| `aphrody-tools` | Catalogue d'outils du moteur (+ `apply-patch` à porter) |
| `aphrody-skills` (+ hooks) | Skills + hooks de cycle de vie |
| `aphrody-memory`, `aphrody-agent-home` | Mémoire / soul / workspace de l'agent |
| `aphrody-guard` | Plan de contrôle : garde-fous (opt-in) |
| `a2a-*`, `aphrody-cron` | Plan de contrôle : coordination + scheduling |
| `google_mcp` (`aphrody-mcp`) | Surface MCP |
| `aphrody-ffi` | Surface FFI |
| `apps/desktop` (Tauri + Angular Material) | Surface desktop (clone Antigravity) |
| `cli` | Surface CLI (à réorganiser, §4) |
| `aphrody-re`, `forensics`, `mrx`, `aphrody-fsindex` | Capacités/outils de l'agent |
| `aphrody-images`, `aphrody-firefly`, `aphrody-logo`, `aphrody-design`, `m3-tokens` | Capacités créatives |

## 6. Roadmap par phases

- **Phase 0 — Fondations** : **Livré** — `aphrody-guard` ; cartographies
  Codex + Ratatui ; plans `gemini-codex.md` + ce `VISION.md`.
- **Phase 1 — ENGINE** : **Livré** — moteur d'agent unifié. Substrat :
  `aphrody-agent-proto` (protocole SQ/EQ NDJSON), `aphrody-model-client`
  (trait `ModelClient` + `GeminiClient`), `aphrody-toolcall` (`ToolRegistry`),
  `aphrody-patch` (apply-patch), `aphrody-rollout` (JSONL). Cœur :
  `aphrody-engine` (`run_turn` : stream → `EventMsg` → tool calls → réinjection
  multi-tour → rollout ; modes `FullAuto`/`Gated` ; `spawn_session` actor) +
  `aphrody-agent-tools` (shell-exec streaming + apply-patch). Jalons GC-1..GC-7.
- **Phase 2/3a — CONTROL PLANE + câblage** : **en cours** —
  `aphrody-agent-runtime` (factory : assemble `ModelClient` + `ToolRegistry` +
  rollout → session) et `aphrody-supervisor` (N agents nommés, fan-in d'events
  taggés, lifecycle). Restent : intégration `aphrody-guard` + A2A + agy-loop,
  steer_input.
- **Phase 3b — SURFACES** : `aphrody-tui` (Ratatui 0.30, GC-8) **livré** ;
  restent la réorganisation CLI agent-centrique (`aphrody agent`, §4) et les
  surfaces MCP/A2A unifiées sur le moteur.
- **Phase 4 — DESKTOP** : clone Antigravity IDE sur `apps/desktop` (éditeur +
  panneau d'agents + navigateur), branché in-process sur le moteur.

## 7. Principes directeurs

1. **Un moteur, des surfaces** — jamais de logique d'agent dupliquée dans une
   surface ; tout passe par le moteur via le protocole.
2. **Autonomie par défaut, contrôle à la demande** — full no-human-in-the-loop
   (§0.1) ; garde-fous opt-in (`APHRODY_GUARD`).
3. **Rust primaire, cross-platform** — Linux #1, Windows #2, wasm #3 ; la seule
   surface non-Rust reste l'app desktop Angular.
4. **Latence minimale** — streaming dès le premier delta, client réutilisé.
5. **Zéro stub** — chaque brique livrée est réelle et vérifiée.

## 8. Voir aussi
- `docs/plans/gemini-codex.md` — le sous-plan ENGINE (boucle de turn + provider).
- `docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`.
- Mémoire : `codex-inspiration-port`, `antigravity-re-findings`,
  `antigravity-integration-landed`, `tauri-desktop-app`, `chat-default-backend-agy`.
- Source de référence : `var/codex` (Codex, Apache-2.0), `var/ratatui` (TUI).
