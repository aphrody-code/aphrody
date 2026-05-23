<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-agy — plugin de boucle de codage autonome pour le CLI Antigravity (`agy`)

Extension `agy` (format Gemini CLI, cf. [`docs/agy-cli/README.md`](../../docs/agy-cli/README.md))
qui **force `agy` à coder en boucle** jusqu'à complétion d'un objectif, sans
humain dans la boucle, adossée à la surface MCP Rust **aphrody** (`aphrody-mcp`).

## Pourquoi

`agy` n'a aucun mode « continue jusqu'à terminé » natif (tout est borné à un
tour ou un sous-agent). Ce plugin comble le manque via un hook `AfterAgent` :
après chaque réponse, `aphrody agy-loop hook` lit le JSON du hook sur stdin et,
tant que la boucle est armée et que le jeton `APHRODY_LOOP_DONE` n'a pas été
émis, renvoie `{"decision":"deny","reason":…}` — ce qu'`agy` interprète comme
« rejette cet arrêt, génère un nouveau tour », relançant l'agent. Garde
anti-emballement par plafond d'itérations (défaut 50).

## Contenu

| Fichier | Rôle |
|---------|------|
| `gemini-extension.json` | Manifeste : monte le serveur MCP `aphrody-mcp`, contexte `AGENTS.md` |
| `AGENTS.md` | Règles de codage autonome injectées à chaque session |
| `hooks/hooks.json` | Hook `AfterAgent` → `aphrody agy-loop hook` (le moteur de boucle) |
| `commands/grind.toml` | `/grind <objectif>` — arme la boucle et démarre |
| `commands/ship.toml` | `/ship` — désarme et finalise (verify + commit) |

## Prérequis

Le binaire **`aphrody`** (qui porte `aphrody agy-loop`) et **`aphrody-mcp`**
doivent être sur le `PATH`. Installer via `scripts/deploy.{ps1,sh}` (copie vers
`~/.local/bin`, déjà dans le `PATH`).

## Installation

```bash
# Copier l'extension dans le dossier d'extensions agy, ou pointer agy dessus.
agy plugin import gemini   # si migration depuis une extension Gemini CLI
agy inspect                # vérifie que l'extension, le hook et le MCP sont chargés
```

## Usage

```bash
agy                        # ouvre le TUI ; l'extension est active
# puis dans le TUI :
/grind implémente l'auth OAuth de bout en bout, tests verts, committé
# … agy code, est relancé automatiquement à chaque tour …
/ship                      # finalise et désarme
```

Headless / scripté :

```bash
aphrody agy-loop start --goal "refactor le module X, zéro régression" --max 30
agy -p "Commence l'objectif courant de la boucle aphrody."
# le hook AfterAgent relance jusqu'à APHRODY_LOOP_DONE ou le plafond
aphrody agy-loop status
aphrody agy-loop stop      # arrêt d'urgence
```

## Mécanique du hook (`aphrody agy-loop hook`)

1. Lit le JSON `AfterAgent` sur stdin (`cwd`, `prompt_response`, `stop_hook_active`).
2. Boucle inactive (pas de `.agents/aphrody-loop.json`) ⇒ `{}` (no-op, arrêt normal).
3. `APHRODY_LOOP_DONE` présent dans la réponse, ou `.agents/aphrody-loop.stop`
   présent ⇒ efface l'état, laisse `agy` s'arrêter (`continue:true`).
4. Plafond d'itérations atteint ⇒ arrêt + message (garde anti-emballement).
5. Sinon ⇒ incrémente, persiste, renvoie `decision:deny` + directive de relance.

État par workspace : `.agents/aphrody-loop.json` (objectif, itération, plafond).
