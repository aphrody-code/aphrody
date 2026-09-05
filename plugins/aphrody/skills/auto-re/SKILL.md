---
name: auto-re
description: Orchestrates automated reverse engineering on a binary or directory. Use when you need a complete RE report on a PE/ELF binary without specifying individual passes. Chains triage, strings, Google endpoint extraction, Go detection, and entrypoint disasm in a single command. Pairs with deep-analysis (Ghidra) for decompilation.
version: "1.0.0"
metadata:
  source: aphrody native — crates/aphrody-re + crates/cli/src/re_auto.rs
  since: "2026-05-22"
---

# auto-re — Workflow de reverse engineering automatise

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant rapport RE complet.

Multiplateforme : `aphrody re` analyse PE/ELF/Mach-O sur Linux/Windows/macOS. Chemins relatifs ou `$VAR` ; ne jamais coder en dur un home utilisateur.

## Vue d'ensemble

`aphrody re auto` orchestre en un seul appel toutes les passes RE pure-Rust :

1. **Triage** : format (PE32/PE64/ELF32/ELF64), arch, sections + entropie Shannon, SHA-256.
2. **Strings** : extraction ASCII + UTF-16LE, borné (`--limit`, defaut 2000).
3. **Google endpoints** : OAuth client IDs, API endpoints, famille binaire (Electron/WebView2/GoBinary/Chromium...).
4. **Go analysis** : version Go (buildinfo), pclntab (fonctions, packages), typelinks.
5. **Disasm entrypoint** : 32 instructions depuis l'entrypoint (x86_64 uniquement).
6. **Ghidra headless** (opt-in `--deep`) : decompilation + symboles via `analyzeHeadless`.

Toutes les passes rapides sont pure-Rust, instantanees (ms). Ghidra est strictement opt-in.

## Quand utiliser auto vs les commandes individuelles

| Besoin | Commande |
|---|---|
| Vue d'ensemble complete d'un binaire inconnu | `re auto <path>` |
| Binaire Go specifiquement (fonction table, version) | `re go <path>` |
| Sections + entropie seulement | `re sections <path>` |
| Strings seules | `re strings <path> --limit N` |
| Endpoints Google uniquement | `re google <path>` |
| Decompilation C profonde | `re auto <path> --deep` (si Ghidra dispo) |
| Batch dossier | `re auto <dossier>/ --json` |

## Commandes

### Binaire unique

```bash
# JSON compact sur stdout (pour scripting/jq)
aphrody re auto /path/to/binary --json

# Avec resume markdown sur stderr + JSON stdout
aphrody re auto /path/to/binary

# Limiter les strings (defaut : 2000)
aphrody re auto /path/to/binary --json --limit 500

# Avec decompilation Ghidra (requiert $GHIDRA_INSTALL_DIR ou $GHIDRA_HOME)
aphrody re auto /path/to/binary --deep --json
```

### Binaire Go

```bash
# Analyse Go specifique : version, fonctions, packages, types
aphrody re go /path/to/binary
aphrody re go /path/to/binary --pretty

# Exemple sur un language server Go strippé (google3) — chemin via variable
aphrody re go "$LS_BIN"   # ex. un language_server_*.exe (Windows) ou ELF (Linux)
```

### Batch dossier

```bash
# Scanne tous les PE/ELF dans un dossier (recursif, profondeur 8 max)
aphrody re auto ./bin/ --json
aphrody re auto ./bin/ --json --limit 200
```

## Structure du rapport JSON

```json
{
  "target": "/path/to/binary",
  "format": "pe64",
  "size": 133599744,
  "sha256": "...",
  "arch": "x86_64",
  "entry_point": 4198400,
  "sections": [
    { "name": ".text", "vaddr": 4096, "size": 15360000, "entropy": 6.28 }
  ],
  "strings_sample": ["go1.27", "cortex", "cloudcode-pa.googleapis.com"],
  "endpoints": {
    "family": "GoBinary",
    "oauth_client_ids": [],
    "api_endpoints": ["cloudcode-pa.googleapis.com"],
    "updater_urls": []
  },
  "go": {
    "go_version": "go1.27",
    "build_flags": "",
    "module_path": "",
    "func_count": 45000,
    "funcs_sample": [
      { "name": "main.main", "addr": 4198400 }
    ],
    "packages": ["main", "net/http", "google.golang.org/grpc"],
    "types_sample": ["*http.Client", "[]byte"]
  },
  "disasm_entry": [
    { "address": 4198400, "text": "push rbp" }
  ],
  "deep": null,
  "warnings": []
}
```

## Chainage recommande

### Workflow initial sur un binaire inconnu

```bash
# 1. Triage rapide
aphrody re auto /path/to/binary --json > report.json

# 2. Identifier la famille
jq '.format, .arch, .endpoints.family' report.json

# 3. Si Go : approfondir
jq '.go.packages[:20]' report.json
jq '[.go.funcs_sample[] | select(.name | contains("grpc"))]' report.json

# 4. Endpoints reseau
jq '.endpoints.api_endpoints' report.json

# 5. Strings suspectes
jq '.strings_sample[] | select(test("password|token|secret|key"; "i"))' report.json

# 6. Si decompilation requise (lent)
aphrody re auto /path/to/binary --deep --json | jq '.deep'
```

### Exemple complet sur un language server Go strippé

```bash
LS="$LS_BIN"   # chemin du binaire cible (variable, pas de home codé en dur)

# Auto complet
aphrody re auto "$LS" --json > ls-report.json

# Go specifique (fonctions, packages)
aphrody re go "$LS" --pretty > ls-go.json

# Endpoints gRPC
jq '.endpoints.api_endpoints' ls-report.json

# Packages Go
jq '.go.packages' ls-go.json

# Fonctions contenant "cortex" ou "cascade"
jq '[.go.funcs_sample[] | select(.name | test("cortex|cascade|Cascade"; "i"))]' ls-go.json
```

## Flags

| Flag | Type | Defaut | Description |
|---|---|---|---|
| `--deep` | bool | false | Invoke Ghidra headless (lent, opt-in) |
| `--json` | bool | false | JSON only sur stdout, pas de markdown stderr |
| `--limit N` | usize | 2000 | Max strings par fichier |
| `--pretty` | bool | false | Pretty-print JSON (re go seulement) |

## Detection Go

Le detecteur Go utilise deux heuristiques complementaires :

1. **buildinfo magic** (`\xff Go buildinf:`) : donne la version exacte, les flags de build, et le module path. Absent sur les builds Blaze/google3.

2. **pclntab magic** (`0xfffffff1` Go 1.18 / `0xfffffff2` Go 1.20+) : table des fonctions. Present meme sur les binaires strippes. Layouts supportes : Go 1.2, 1.16, 1.18, 1.20+. Le parsing degrade proprement sur les layouts inconnus.

3. **typelinks heuristique** : scan des prefixes de types Go (`*`, `[]`, `map[`, etc.) pour un echantillon de noms de types, sans reconstruction complete du moduledata.

## Limites connues

- **func_count=0 sur les binaires Go strippés google3** : le pclntab est detecte (is_go=true) mais le layout interne du binaire go1.27 interne Google n'est pas entierement parse. La detection reste correcte.
- **disasm_entry** : requiert x86_64. Les binaires arm64/riscv/etc. n'ont pas de disasm d'entrypoint.
- **Ghidra** : requiert `$GHIDRA_INSTALL_DIR` ou `$GHIDRA_HOME`. Si absent, le champ `deep` contient `{ "ghidra_available": false, "ghidra_unavailable_reason": "..." }`.
- **Batch dossier** : recursivite limitee a 8 niveaux. Concurrence = 4 threads natifs.
- **Binaires > 1 GB** : strings_limit borne a --limit (defaut 2000) pour eviter l'OOM.

## Integration avec deep-analysis

`auto-re` produit le triage initial. Pour une investigation approfondie :

1. Lancer `auto-re` pour identifier les zones suspectes.
2. Charger le binaire dans Ghidra (GUI ou headless via `--deep`).
3. Utiliser le skill `deep-analysis` pour l'investigation fonction par fonction.
4. Utiliser `protocol-reverse-engineering` pour les endpoints reseau identifies.
