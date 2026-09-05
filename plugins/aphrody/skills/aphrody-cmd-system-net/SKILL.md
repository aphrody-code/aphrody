---
name: aphrody-cmd-system-net
description: Contexte direct des commandes système, cycle de vie, réseau et intégration du CLI aphrody — auth, doctor, version, completions, self, oc-onboard/reset/uninstall/pairing/docs, mirror, cros, index, search, dns, a2a, mcp, notify, ide, term. Use pour savoir ce que fait une de ces commandes, ses flags et ses effets (dont les opérations destructives) avant de l'invoquer.
version: "1.0.0"
metadata:
  source: aphrody native — crates/cli/src/main.rs (enum Commands)
  since: "2026-05-23"
---

# aphrody — commandes système / cycle de vie / réseau / intégration

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant complétion.
Cross-platform (Linux #1, Windows, macOS). Vérité de référence : `aphrody <cmd> --help`.

## Diagnostic & cycle de vie

- **`aphrody version [--json]`** — version + état système.
- **`aphrody doctor [--json]`** — diagnostic env + intégration A2A + supply-chain (first-impression).
- **`aphrody auth [--force]`** — authentification Google (God Mode / OAuth2).
- **`aphrody completions <bash|zsh|fish|pwsh|elvish>`** — génère les completions shell.
- **`aphrody self <action>`** — installer / bootstrap natif (remplace les scripts `.ps1`/`.sh`).
- **`aphrody mirror [--action start]`** — gère le mirroring des assets MD3.
- **`aphrody cros <action>`** — compilation hyper-optimisée de ChromeOS.

## openclaw (état local) — ⚠ certaines opérations sont destructives

- **`aphrody oc-onboard [--workspace D] [--non-interactive] [--accept-risk] [--force]`** — bootstrap état local + seed config. `--non-interactive` exige `--accept-risk`.
- **`aphrody oc-reset --scope <config|config-creds-sessions|full> [--yes] [--dry-run]`** — ⚠ **destructif** : reset état local. `--yes` requis (sauf `--dry-run`). Toujours proposer `--dry-run` d'abord.
- **`aphrody oc-uninstall [--service|--state|--workspace|--app|--all] [--yes] [--dry-run]`** — ⚠ **destructif** : désinstalle des scopes.
- **`aphrody oc-pairing <action>`** — pairing DM sécurisé (`~/.aphrody/pairing.json`) : list / approve / inject.
- **`aphrody oc-docs <query> [--url-only]`** — ouvre / cherche le site de doc.

## Réseau / recon / intégration

- **`aphrody dns <domaine>`** — résolution DNS OSINT (reconnaissance agressive).
- **`aphrody search <termes…>`** — recherche Google native.
- **`aphrody a2a <prompt>`** — client A2A natif (JSON-RPC).
- **`aphrody mcp <action>`** — Model Context Protocol : list servers, call tools (`~/.config/aphrody/mcp.json`).
- **`aphrody notify --channel <slack|telegram|matrix> --message <txt> [--room R]`** — envoie un message. Env : `SLACK_CHANNEL` / `TELEGRAM_CHAT_ID` / `MATRIX_ROOM_ID` si `--room` absent.
- **`aphrody ide <action>`** — info / intégration IDE. Ex. : `aphrody ide info --json | jq '.ideVersion'`.
- **`aphrody term [--addr 127.0.0.1:8788] [--shell …] [--cwd D]`** — pont WebSocket-PTY pour le frontend WASM (localhost uniquement).

## Index recherche locale

- **`aphrody index build --root <dir>`** / **`aphrody index search "<q>" [--limit N] [--json]`** — index FTS5 metadata-only (mirror du `local_files.db` Google). Feature-gate `--features index`.

## Garde-fous

- Opérations destructives (`oc-reset`, `oc-uninstall`) : exiger confirmation explicite (`--yes`) ou proposer `--dry-run` ; jamais en aveugle.
- `term` : bind localhost strict, jamais exposé publiquement.
