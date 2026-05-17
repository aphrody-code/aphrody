# n2b — Node.js → Bun-native migration toolkit

Complete documentation bundle migrated from `~/vps/rust/n2b/` (upstream source).

## Contenu

### Core (racine du projet)

| Fichier | Description |
|---|---|
| `README.md` | Présentation, install, usage CLI rapide |
| `CLAUDE.md` | Guide Claude Code pour travailler sur le source n2b (Rust workspace) |
| `STRUCTURE.md` | Architecture crates (`n2b-core`, `n2b-cli`, `n2b-native`) |
| `CHANGELOG.md` | Historique versions |
| `build-your-own-x.md` | Tutoriel "how n2b was built" |

### Sous-sections (`docs/`)

| Dossier | Fichiers | Usage |
|---|---|---|
| `bun/` | `bun-roadmap-159.md`, `bun-roadmap-mapping.md` | Mapping roadmap Bun → règles n2b, coverage des 159 APIs Bun |
| `research/` | `rust-starred-libs-2026.md`, `rust-web-stack-2026.md`, `monorepo-architecture-2026.md`, `wasm-bindgen-study.md`, `discord-stack-research.md` | Audits techniques guidant le design n2b |
| `wasm/` | `WASM_BINDGEN_BUN_PATCH.md`, `WASM_BINDGEN_PERF.md`, `WASM_PACK_BUN_PATCH.md` + `n2b-reports/*` (binaryen, wabt, wasm-bindgen, wasm-pack) | Patches wasm-bindgen/wasm-pack pour Bun + rapports d'audit |
| `reports/` | `bun-bench-baseline.md` | Benchmarks baseline |

## Agent + command dans le plugin

- **Agent** : `bun-agent/agents/n2b.md` — migration specialist (invoqué via `@n2b`)
- **Command** : `bun-agent/commands/n2b.md` — `/n2b audit`, `/n2b phase N`, `/n2b fix <path>`, `/n2b migrate`, `/n2b rollback`

## Règles de mise à jour

- **Upstream** : `~/vps/rust/n2b/` reste la source de vérité du code Rust
- **Docs** : chaque changement docs upstream doit être re-synchronisé ici via :
  ```bash
  rsync -a --delete ~/vps/rust/n2b/docs/ ~/vps/agents/bun-agent/docs/n2b/
  cp ~/vps/rust/n2b/{README,CLAUDE,STRUCTURE,CHANGELOG,CONTRIBUTING,build-your-own-x}.md ~/vps/agents/bun-agent/docs/n2b/
  ```
- **Ne pas éditer** les fichiers ici — éditer l'upstream puis re-sync
