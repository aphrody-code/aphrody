# STRUCTURE.md — Cartographie du workspace `rsbun`

> **Rôle** : hub de recherche + projets Rust/Bun. Pas de `Cargo.toml` workspace unique — chaque sous-dossier a son propre système de build.

---

## Catégorisation stricte

Chaque sous-dossier appartient à **exactement une** des 3 catégories :

| Icône | Catégorie | Règle |
|---|---|---|
| 🔨 | **Own** (production) | Code qu'on écrit et qu'on publie/déploie |
| 🔬 | **Study** (R&D) | Notes, POCs, benchmarks, patches locaux |
| 📦 | **Vendored** (upstream clones) | Clones read-only avec `.git/` intact |

---

## Arborescence

```
rsbun/
├── 📄 README.md                     Entrée utilisateur
├── 📄 STRUCTURE.md                  Ce fichier (cartographie)
├── 📄 CLAUDE.md                     Guide pour Claude Code
├── 📄 .gitignore · .editorconfig · rustfmt.toml    Standards root
├── 📄 justfile                      Tâches du workspace
│
├── 🔨 n2b/                          CLI Rust Node→Bun migration
│   ├── crates/{n2b-cli, n2b-core, n2b-native}
│   └── packages/{n2b, n2b-cli}      (façade TS)
│
├── 🔨 bun++/                        Extensions/POCs au-dessus de Bun
│   ├── packages/                    @bun++/zstd, @bun++/libmagic, @bun++/hash-cc…
│   └── upstream-patches/            Patches prêts pour PR sur oven-sh/bun
│       └── bun-zig-perf/            3 patches `perf(zig)` + bundle
│
├── 🔨 md3-ui/                       Material Design 3 UI (Base UI + Tailwind)
│
├── 🔨 scripts/                      Scripts utilitaires du workspace
│   ├── rename-refs.sh               Renommage global via rg + sd
│   ├── audit-sizes.sh               Rapport disque par catégorie
│   └── git-status-all.sh            Status git de tous les sous-repos
│
├── 🔬 patches/                      Sauvegardes hors-upstream
│   ├── README.md
│   ├── wasm-bindgen/                3 commits + bundle (poussé sur aphrody-code/wasm-bindgen@perf)
│   ├── wasm-pack/                   3 commits + bundle (bundle local)
│   └── wasm-tools/                  Code maison Bun-native
│       ├── binaryen-bun/            Wrappers TS pour wasm-opt
│       └── wabt-bun/                Wrappers TS + shim .so pour wabt
│
├── 📦 discordx/                     Clone discordx (framework Discord TS)
│
│
├── 📦 material-ui/                  Clone MUI (référence md3-ui)
│
└── 📁 docs/                         Documentation et recherche
    ├── README.md                    Index docs
    ├── research/                    Études de l'assistant
    │   ├── discord-stack-research.md
    │   ├── monorepo-architecture-2026.md
    │   ├── rust-starred-libs-2026.md
    │   ├── rust-web-stack-2026.md
    │   └── wasm-bindgen-study.md
    ├── bun/                         Notes Bun-spécifiques
    │   ├── bun-roadmap-159.md
    │   └── bun-roadmap-mapping.md
    ├── bun-upstream/                Miroir docs Bun (329 `.md`) + llms/
    │   └── llms/                    llms-full.txt + sitemap.txt + sitemap.xml
    ├── wasm/                        Docs des patches (extraites de feu wasm/)
    │   ├── WASM_BINDGEN_BUN_PATCH.md
    │   ├── WASM_BINDGEN_PERF.md
    │   ├── WASM_PACK_BUN_PATCH.md
    │   └── n2b-reports/             Rapports n2b (binaryen, wabt, wasm-bindgen, wasm-pack)
    ├── reports/                     Rapports benchmark/audit
    │   └── bun-bench-baseline.md
    ├── awesome-rust/                Liste curée Rust (ex-clone)
    │   ├── README.md                307 Ko de liens upstream
    │   ├── CURATED.md               Extrait rsbun-filtré (16 Ko)
    └── build-your-own-x.md          Référence externe
```

---

## Tailles disque (post-cleanup)

| Dossier | Taille | Catégorie |
|---|---|---|
| `n2b/` | 5.9 GB | 🔨 Own (target/ + node_modules/) |
| `md3-ui/` | 1.6 GB | 🔨 Own |
| `discordx/` | 842 MB | 📦 Clone |
| `material-ui/` | 615 MB | 📦 Clone |
| `bun++/` | 150 MB | 🔨 Own |
| `docs/` | 4.3 MB | 📁 Docs (inclut awesome-rust) |
| `patches/` | 2.3 MB | 🔬 Sauvegardes |
| `scripts/` | 24 KB | 🔨 Own |
| **Total** | **~8.9 GB** | |

**Supprimés le 2026-04-18** :
- `bun/` (7.0 GB) → clone oven-sh/bun ; 3 commits locaux `claude/zig-perf` préservés dans `bun++/upstream-patches/bun-zig-perf/`
- `wasm/` (2.8 GB) → contenu unique préservé dans `patches/` + `docs/wasm/`
- `mui-x/` (83 MB) → pur clone sans modifications

---

## Conventions

### Git
- **Projets own** (`n2b`, `bun++`, `md3-ui`, `scripts`) → chacun son repo distant
- **Clones** (`discordx`, `material-ui`) → `.git/` pointe vers upstream, **ne pas committer** du code local sans branche séparée
- **Références** (`awesome-rust`) → markdown seulement, pas de `.git/`
- **`rsbun/` lui-même** → pas de repo git global (chaque sous-projet est indépendant)

### Versions
- **Rust** : 1.95+ stable
- **Bun** : 1.3.13+ (canary accepté sur ce VPS)
- **Node** : interdit (voir `CLAUDE.md`)

### Style
- `rustfmt.toml` root → `edition = "2024"`, `max_width = 100`
- `.editorconfig` → LF, UTF-8, 2 espaces (4 pour Rust/TOML)
- `.gitignore` root → `target/`, `node_modules/`, `.next/`, `.turbo/`, `dist/`

### Tâches
Toujours passer par `just <task>` depuis `/home/ubuntu/rsbun/` :
- `just sizes` — taille par dossier
- `just tree` — arbre L2
- `just update-clones` — pull tous les clones
- `just n2b-build` / `just n2b-test` / `just n2b-install`
- `just clean-rust` / `just clean-js` — nettoyage
- `just fmt` / `just lint` / `just audit`

---

## Relation entre les projets

```
           ┌────────────────────────────┐
           │ 🔨 n2b                     │ ← CLI utilisée par rpb-dashboard
           │  - Rust+TS monorepo        │
           │  - wrap wasm-pack (externe)│
           └────────┬───────────────────┘
                    │
          ┌─────────┬──────────────┐
          ▼         ▼              ▼
      🔨 bun++/  🔬 patches/  📁 docs/bun-upstream/
      (ext + PR) (upstream work) (doc miroir)
```

---

## Ajouter un nouveau projet

1. Créer le dossier à la racine : `rsbun/<nom>/`
2. Catégoriser (🔨 / 🔬 / 📦) — mettre à jour ce fichier
3. Ajouter entry dans `README.md` (table des projets)
4. Ajouter tâches dans `justfile` si pertinent
5. Respecter `.gitignore` / `.editorconfig` / `rustfmt.toml` root
