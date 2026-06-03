<!-- SPDX-License-Identifier: Apache-2.0 -->
# Antigravity 2.0 & `agy` CLI — référence aphrody

Référence distillée (Google I/O 2026, 19 mai 2026) servant de base au plugin
`extensions/aphrody-agy/`. Sources en bas de page.

## 1. Antigravity 2.0

Plateforme de développement **agent-first** autonome (anciennement IDE unique,
fork Windsurf/Codeium — cf. memory `antigravity-re-findings`). Composants
livrés à I/O 2026 :

- **App desktop** : orchestration multi-agents, exécution parallèle, sous-agents
  custom, tâches planifiées en arrière-plan, intégrations AI Studio / Android /
  Firebase, commandes vocales natives.
- **CLI `agy`** : agent de codage terminal (TUI + headless).
- **SDK** : construction d'agents custom.
- **Managed execution** + support entreprise.
- Moteur : **Gemini 3.5 Flash** (co-développé avec Antigravity). Modèles
  accessibles via le CLI : Gemini 3.5 Flash, Gemini 3.1 Pro, Claude Sonnet,
  Claude Opus, GPT-OSS 120B (selon le plan ; tier AI Ultra à 100 $/mois = 5× Pro).
- **Gemini CLI déprécié** : transition vers `agy`, deadline migration **18 juin 2026**.

## 2. Installation du CLI `agy`

| OS | Commande |
|----|----------|
| macOS / Linux | `curl -fsSL https://antigravity.google/cli/install.sh \| bash` |
| Windows (PowerShell) | `irm https://antigravity.google/cli/install.ps1 \| iex` |
| Windows (CMD) | `curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd` |

Binaire installé : **`agy`** (pas `antigravity`). Unix → `~/.local/bin/` ;
Windows → `%LOCALAPPDATA%\Antigravity\` (côté aphrody, résolution
`$APHRODY_AGY_BIN > %LOCALAPPDATA%\agy\bin\agy.exe > PATH`, cf. `aphrody agy`).

## 3. Commandes & flags

| Commande | Rôle |
|----------|------|
| `agy` | Mode agent interactif (TUI) |
| `agy --version` | Version |
| `agy -p "prompt"` | Mode commande — complétion headless one-shot |
| `agy -p "prompt" --output-format json` | Sortie JSON structurée (pipe) |
| `agy -m <model> -p "prompt"` | Modèle spécifique en headless |
| `agy inspect` | Affiche contexte projet, skills, plugins, hooks, serveurs MCP |
| `agy plugin import gemini` | Migre les extensions Gemini CLI → Plugins agy |

Slash-commands TUI : `/help`, `/context`, `/usage`, `/export` (vers GUI 2.0),
`/model <id>`, `/agent <nom> "tâche"` (sous-agent async), `/logout`.
Référence fichiers en contexte : `@fichier`, `@dir/`, `@**/*.ts`.

## 4. Authentification

Source officielle : [Antigravity CLI reference](https://antigravity.google/docs/cli-reference).

| Plateforme | Stockage OAuth `agy` | Lecture `aphrody antigravity` / `AgyBackend` |
|------------|----------------------|-----------------------------------------------|
| **Linux** | Fichier `~/.gemini/antigravity-cli/antigravity-oauth-token` (JSON `{"token":{…},"auth_method":"consumer"}`) | `antigravity-sdk` lit ce fichier en premier |
| **Linux** (fallback) | `~/.config/aphrody/antigravity-token.json` après `aphrody antigravity login` | même SDK |
| **Windows** | Credential Manager `gemini:antigravity` | `CredReadW` |
| **macOS** | Keychain (équivalent desktop) | idem Windows via store OS |

Variables shell recommandées (voir `awesome-grok-build/scripts/rust-nightly-env.sh`) :

```bash
export APHRODY_AGY_OAUTH_FILE="$HOME/.gemini/antigravity-cli/antigravity-oauth-token"
export APHRODY_ANTIGRAVITY_CONFIG="$HOME/.config/antigravity/config.toml"
```

- **Remote / SSH** : URL d'autorisation + code one-time à coller en local.
- **Clé API** (optionnel, facturé) : `ANTIGRAVITY_API_KEY` dans `~/.bashrc` — préférer le token OAuth `agy` (keyless Code Assist).
- **Refresh** : ne pas compter sur `aphrody antigravity refresh` seul (OAuth public sans `client_secret`) ; relancer `agy` ou `aphrody antigravity login` pour re-minter.

## 5. Configuration & extensibilité

| Élément | Emplacement | Rôle |
|---------|-------------|------|
| `config.toml` | `~/.config/antigravity/config.toml` | model, base_url, var d'env de la clé API |
| `AGENTS.md` | racine projet | Instructions projet en langage naturel |
| `.agents/skills/*.md` | projet | Skills réutilisables → slash-commands (`lint.md` ⇒ `/lint`) |
| `~/.gemini/antigravity-cli/skills/` | home | Skills globaux |
| `mcp_config.json` | racine projet | Serveurs MCP (stdio local / HTTP distant `serverUrl`) |
| `hooks/hooks.json` | dans l'extension | Intercepteurs de cycle de vie (JSON) |

**Plugins** = extensions **Gemini CLI rebrandées** (même format, migration
`agy plugin import gemini`). Manifeste `gemini-extension.json` à la racine de
l'extension.

### 5.1. Manifeste `gemini-extension.json`

```json
{
  "name": "lowercase-dashes-only",
  "version": "semver",
  "description": "…",
  "mcpServers": { "srv": { "command": "bin", "args": [], "cwd": "${extensionPath}" } },
  "contextFileName": "AGENTS.md",
  "excludeTools": ["run_shell_command(rm -rf)"],
  "settings": [{ "name": "…", "description": "…", "envVar": "API_KEY", "sensitive": true }],
  "migratedTo": "https://…"
}
```

Variables de substitution : `${extensionPath}`, `${workspacePath}`, `${/}`
(séparateur de chemin selon l'OS). Commandes custom = TOML dans `commands/`
(`commands/deploy.toml` ⇒ `/deploy`, `commands/gcs/sync.toml` ⇒ `/gcs:sync`).

### 5.2. Hooks (`hooks/hooks.json`)

Events : `SessionStart`, `SessionEnd`, `BeforeAgent`, `AfterAgent`,
`BeforeModel`, `AfterModel`, `BeforeToolSelection`, `BeforeTool`, `AfterTool`,
`Notification`, `PreCompress`.
(⚠ noms Gemini CLI : `BeforeTool`/`AfterTool`, **pas** `PreToolUse`/`PostToolUse`
de Claude Code — le migrateur mappe `PostToolUse → AfterTool`.)

Structure :
```json
{ "hooks": { "AfterAgent": [ { "matcher": "", "hooks": [
  { "type": "command", "command": "…", "name": "…", "timeout": 60000 } ] } ] } }
```

**Protocole** : le hook reçoit un JSON sur **stdin** (`session_id`,
`transcript_path`, `cwd`, `hook_event_name`, `timestamp`) et émet un JSON sur
**stdout** (exit 0). Champs de contrôle communs : `systemMessage`,
`suppressOutput`, `continue` (false ⇒ stoppe la boucle agent), `stopReason`,
`decision` (`allow`/`deny`/`block`), `reason`, `hookSpecificOutput`.

`AfterAgent` (post-réponse) reçoit en plus `prompt`, `prompt_response`,
`stop_hook_active`. **`decision: "deny"` + `reason` force l'agent à continuer**
(nouveau tour, `reason` injecté comme prompt système). Exit code 2 ⇒ retry avec
stderr comme feedback. C'est le levier exploité par `aphrody agy-loop`.

## 6. Mode boucle autonome — il n'y en a PAS nativement

`agy` n'expose aucun mode « continue jusqu'à terminé » : tout est initié par
l'utilisateur ou borné au sous-agent. Le plugin `aphrody-agy` comble ce manque
via un hook `AfterAgent` (`aphrody agy-loop hook`) qui réinjecte une directive
tant que le jeton `APHRODY_LOOP_DONE` n'a pas été émis. Voir
`extensions/aphrody-agy/README.md`.

## Sources

- [TechCrunch — Antigravity 2.0 desktop + CLI (I/O 2026)](https://techcrunch.com/2026/05/19/google-launches-antigravity-2-0-with-an-updated-desktop-app-and-cli-tool-at-io-2026/)
- [MarkTechPost — Antigravity 2.0 CLI/SDK/managed execution](https://www.marktechpost.com/2026/05/19/google-launches-antigravity-2-0-at-i-o-2026-a-standalone-agent-first-platform-with-cli-sdk-managed-execution-and-enterprise-support/)
- [antigravity.google/docs/cli-using](https://antigravity.google/docs/cli-using) · [docs/command](https://antigravity.google/docs/command)
- [DEV — Antigravity CLI hands-on](https://dev.to/arindam_1729/antigravity-cli-a-hands-on-guide-to-googles-terminal-coding-agent-5bc7)
- [Gemini CLI — Extension reference](https://geminicli.com/docs/extensions/reference/) · [Hooks reference](https://geminicli.com/docs/hooks/reference/)
- [Google Developers Blog — Transitioning Gemini CLI to Antigravity CLI](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)
