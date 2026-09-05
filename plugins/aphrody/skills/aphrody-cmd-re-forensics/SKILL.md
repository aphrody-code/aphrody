---
name: aphrody-cmd-re-forensics
description: Contexte direct des commandes de reverse engineering, forensics et analyse repo du CLI aphrody — re (triage/strings/sections/auto), forensics, scan, chromium, auto. Use pour savoir ce que fait une de ces commandes, ses sous-commandes, son feature-gate et ses garanties de sécurité avant de l'invoquer.
version: "1.0.0"
metadata:
  source: aphrody native — crates/cli/src/main.rs + crates/aphrody-re
  since: "2026-05-23"
---

# aphrody — commandes reverse engineering / forensics / analyse

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant complétion.
Cross-platform (Linux #1, Windows, macOS) ; pur Rust, aucune dépendance GPL.
Vérité de référence : `aphrody <cmd> --help`.

## Reverse engineering

- **`aphrody re triage <bin>`** — format (PE32/PE64/ELF32/ELF64), arch, sections + entropie Shannon par section, imports/exports, échantillon strings ASCII/UTF-16LE, empreinte SHA-256. Détection via magic bytes (goblin).
- **`aphrody re strings <bin>`** — extraction strings ASCII/UTF-16LE.
- **`aphrody re sections <bin>`** — table des sections + entropie.
- **`aphrody re auto <bin|dir> [--json] [--limit N]`** — orchestre toutes les passes en un appel : triage + strings + extraction endpoints Google + détection Go + désassemblage entrypoint. Batch sur un dossier. Ex. : `aphrody re auto language_server.exe --json | jq '.go.func_count'`. (Gotcha go1.27 : `func_count=0` ⇒ voir memory auto-re-setup.)
  Pour la décompilation (Ghidra), enchaîner avec le skill `deep-analysis`.

## Forensics

- **`aphrody forensics sqlite --db <fichier>`** — extraction forensique reproductible : map filesystem + dump de schéma SQLite. **SÉCURITÉ** : mappe/classe/dump le schéma UNIQUEMENT — ne lit ni n'imprime JAMAIS de valeurs secrètes (tokens, cookies, lignes `secret://`). Ouvre les bases en read-only et ne lit que `sqlite_master` (noms + CREATE). Feature-gate `--features forensics`. Ex. : `… --db state.vscdb | jq '.tables[].name'`.
- **`aphrody chromium <action>`** — forensics Chromium (autopsy navigateur). Voir aussi le MCP `chrome_autopsy` / la memory chrome-abe-cookie-extraction.

## Analyse repo

- **`aphrody scan <action>`** — analytics repo : scan de l'arbre (taille / nombre de fichiers) + scan des manifestes (Cargo / JSON / TOML).
- **`aphrody auto <prompt|cmd…>`** — commande par défaut : un prompt en langage naturel est routé vers le client A2A JSON-RPC natif ; un token ressemblant à une commande (nom de moteur connu, sous-commande standard, fichier script) part au dispatcher bun/uv/cargo.

## Garde-fous

- Forensics : jamais d'exfiltration de valeurs secrètes ; read-only strict.
- Feature-gates `forensics`/`index` host-only (absents du build par défaut et du wasm).
