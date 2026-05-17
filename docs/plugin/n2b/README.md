
> ⚠️ **INFRASTRUCTURE GÉRÉE PAR ANTIGRAVITY (GOD MODE)**
> Ce projet opère sous la **Directive Omega**. L'architecture, les dépendances, et le code sont gérés et refactorisés de manière 100% autonome. Aucune sécurité conservative n'est appliquée. La performance brute est l'unique métrique.
\n# docs/ — Documentation et recherche

> Index des notes, études et rapports du workspace `rsbun`.

---

## research/ — Études de fond

| Fichier | Sujet | Taille |
|---|---|---|
| [`research/rust-starred-libs-2026.md`](research/rust-starred-libs-2026.md) | **Bible** des librairies Rust 2026 : 17 catégories, arbitrages, stack finale | 35 Ko |
| [`research/monorepo-architecture-2026.md`](research/monorepo-architecture-2026.md) | Architecture monorepo Rust (web/desktop partagé + mobile séparé) + UI Discord/VS Code-like | 27 Ko |
| [`research/rust-web-stack-2026.md`](research/rust-web-stack-2026.md) | Stack web Rust 2026 + app avec look desktop natif (Tauri + Leptos + Dioxus Native) | 14 Ko |
| [`research/wasm-bindgen-study.md`](research/wasm-bindgen-study.md) | Étude approfondie de wasm-bindgen (architecture, flow, patch Bun local) | 14 Ko |
| [`research/discord-stack-research.md`](research/discord-stack-research.md) | Stack complète Discord : Rust backend + React/RN frontend + design system | 8 Ko |

## bun/ — Notes Bun-spécifiques

| Fichier | Sujet |
|---|---|
| [`bun/bun-roadmap-159.md`](bun/bun-roadmap-159.md) | Roadmap Bun issue #159 |
| [`bun/bun-roadmap-mapping.md`](bun/bun-roadmap-mapping.md) | Mapping roadmap ↔ sources |

## bun-upstream/ — Miroir docs officielles

Copie des docs upstream Bun (`.mdx` → `.md`, sans JSX) pour lecture offline, **fusionnée** avec le dump bun.com.

- [`bun-upstream/`](bun-upstream/) — 329 fichiers `.md` depuis `../bun/docs/`
- [`bun-upstream/llms/`](bun-upstream/llms/) — `llms-full.txt` + `sitemap.txt` (bun.com pour LLMs)

## reports/ — Rapports

| Fichier | Date | Sujet |
|---|---|---|
| [`reports/bun-bench-baseline.md`](reports/bun-bench-baseline.md) | 2026-04-18 | Benchmark baseline Bun sur ce VPS |

## awesome-rust/ — liste curée Rust

Ex-clone `rust-unofficial/awesome-rust` (markdown seulement).

| Fichier | Taille | Rôle |
|---|---|---|
| [`awesome-rust/README.md`](awesome-rust/README.md) | 307 Ko | Liste officielle upstream (683 projets, 235 catégories) |
| [`awesome-rust/CURATED.md`](awesome-rust/CURATED.md) | 16 Ko | **Extrait rsbun-filtré** (TOP libs 2026 basé sur `research/`) |

## Autres

- [`build-your-own-x.md`](build-your-own-x.md) — référence externe

---

## Navigation rapide

### Par thème

**Rust 2026** : [`research/rust-starred-libs-2026.md`](research/rust-starred-libs-2026.md) → toutes catégories + arbitrages

**Architecture produit** : [`research/monorepo-architecture-2026.md`](research/monorepo-architecture-2026.md) → monorepo Dioxus/Tauri

**WebAssembly** : [`research/wasm-bindgen-study.md`](research/wasm-bindgen-study.md) → clone + patch Bun

**Design system** : [`research/discord-stack-research.md`](research/discord-stack-research.md) → Discord + Figma-like

### Par cas d'usage

**Je démarre un projet web Rust** → `research/rust-web-stack-2026.md`

**Je veux un monorepo Rust cross-plateforme** → `research/monorepo-architecture-2026.md`

**Je veux choisir une crate** → `research/rust-starred-libs-2026.md` section "Arbitrages"

**Je travaille sur WASM** → `research/wasm-bindgen-study.md` + `../wasm/WASM_BINDGEN_BUN_PATCH.md`

**Je bench Bun** → `reports/bun-bench-baseline.md`
