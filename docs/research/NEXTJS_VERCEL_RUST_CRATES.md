<!-- SPDX-License-Identifier: Apache-2.0 -->
# Next.js + Vercel Rust crates — Cartographie pour intégration aphrody

> Document de recherche pour OBJECTIF #1 + #4 — intégrer NATIVEMENT Turbopack
> et tout l'écosystème Rust Vercel dans `aphrody/Cargo.toml workspace.dependencies`.
>
> Source : agent Explore sur C:\worktree\next.js (branche aphrody, 2026-05-17).

---

## 1. Inventaire (60+ crates internes)

### Next.js crates (6)

| Crate | Version | Chemin |
|---|---|---|
| `next-api` | 0.1.0 | `/crates/next-api` |
| `next-build` | 0.1.0 | `/crates/next-build` |
| `next-core` | 0.1.0 | `/crates/next-core` — Moteur principal Turbopack |
| `next-code-frame` | 0.0.1 | `/crates/next-code-frame` — Code frame errors |
| `next-custom-transforms` | 0.0.0 | `/crates/next-custom-transforms` — SWC transforms |
| `next-taskless` | 0.0.1 | `/crates/next-taskless` |

### Turbopack runtime — turbo-tasks-* (17)

| Crate | Rôle |
|---|---|
| `turbo-tasks` | **Task execution + memoization framework** |
| `turbo-tasks-backend` | Backend runtime |
| `turbo-tasks-fs` | Filesystem abstraction |
| `turbo-tasks-fetch` | Fetch/HTTP primitives |
| `turbo-tasks-env` | Environment variables |
| `turbo-tasks-hash` | Hashing utilities |
| `turbo-tasks-bytes` | Bytes utilities |
| `turbo-tasks-malloc` | Allocator wrapper (mimalloc/system) |
| `turbo-tasks-macros` | Procedural macros |
| `turbo-tasks-testing` | Testing utilities |
| `turbo-bincode` | Bincode serialization |
| `turbo-rcstr` | Ref-counted strings |
| `turbo-esregex` | ES regex support |
| `turbo-frozenmap` | Immutable map |
| `turbo-persistence` | Persistence layer |
| `turbo-prehash` | Pre-hashing utilities |
| `turbo-unix-path` | Unix path utilities |

### Turbopack compiler — turbopack-* (13)

| Crate | Rôle |
|---|---|
| `turbopack` | Core bundler |
| `turbopack-core` | Build graph + modules |
| `turbopack-ecmascript` | JS/TS handling |
| `turbopack-ecmascript-plugins` | JS plugins (emotion, relay) |
| `turbopack-ecmascript-runtime` | Runtime helpers |
| `turbopack-ecmascript-hmr-protocol` | HMR protocol |
| `turbopack-css` | CSS processing (lightning-css) |
| `turbopack-image` | Image optimization |
| `turbopack-mdx` | MDX support |
| `turbopack-wasm` | WebAssembly |
| `turbopack-static` | Static file handling |
| `turbopack-resolve` | Module resolution |
| `turbopack-nft` | Dependency tracing |

### Turbopack CLI/runtime (6)

| Crate | Rôle |
|---|---|
| `turbopack-cli` | CLI interface |
| `turbopack-cli-utils` | CLI utilities |
| `turbopack-dev-server` | Dev server |
| `turbopack-nodejs` | Node.js runtime |
| `turbopack-node` | Node.js module loading |
| `turbopack-browser` | Browser runtime |

### Utilities/tools (8)

`turbopack-swc-utils`, `turbopack-analyze`, `turbopack-trace-server`,
`turbopack-trace-utils`, `turbopack-tracing`, `turbopack-env`,
`turbopack-test-utils`, `turbopack-tests`

### Additional

`next-napi-bindings`, `wasm`, `send-trace-to-jaeger`, `swc-plugin-env-check`, `xtask`

---

## 2. Top crates publics crates.io (utilisables direct)

| # | Crate | Version | Utilisation |
|---|---|---|---|
| 1 | `swc_core` | 63.1.1 | **Parser/compiler JS (SWC)** |
| 2 | `lightningcss` | 1.0.0-alpha.70 | **CSS processing Vercel** |
| 3 | `lightningcss-napi` | 0.4.6 | NAPI bindings lightningcss |
| 4 | `swc_sourcemap` | 10.0.2 | Source maps |
| 5 | `swc_plugin_backend_wasmtime` | 9.0.0 | WASM runtime SWC |
| 6 | `mdxjs` | 1.0.3 (git patch) | MDX |
| 7 | `modularize_imports` | 3.0.0 | Module transform |
| 8 | `react_remove_properties` | 3.0.0 | React optim |
| 9 | `remove_console` | 3.0.0 | Console removal |
| 10 | `swc_emotion` | 3.0.0 | Emotion CSS-in-JS |
| 11 | `swc_relay` | 3.0.0 | Relay optim |
| 12 | `styled_components` | 3.0.0 | styled-components |
| 13 | `styled_jsx` | 3.0.0 | styled-jsx |
| 14 | `preset_env_base` | 7.0.0 | ES preset base |
| 15 | `browserslist-rs` | 0.19.0 | Browser target resolution |

**Absent** : oxc, biome, parcel-css. Next.js utilise seulement lightningcss.

---

## 3. Stratégie d'intégration aphrody

### Option retenue : git+branch="aphrody" pour internes + workspace.dependencies pour publics

#### A. Crates publics → `[workspace.dependencies]` direct

```toml
# --- SWC ecosystem (Vercel Rust) ---
swc_core                       = { version = "63", features = ["common", "ecma_parser", "ecma_codegen", "ecma_transforms"] }
swc_sourcemap                  = { version = "10" }
swc_plugin_backend_wasmtime    = { version = "9" }
swc_emotion                    = { version = "3" }
swc_relay                      = { version = "3" }
styled_components              = { version = "3" }
styled_jsx                     = { version = "3" }
modularize_imports             = { version = "3" }
react_remove_properties        = { version = "3" }
remove_console                 = { version = "3" }
preset_env_base                = { version = "7" }
browserslist-rs                = { version = "0.19" }

# --- Lightning CSS (Vercel/Parcel) ---
lightningcss                   = { version = "1.0.0-alpha.70" }
lightningcss-napi              = { version = "0.4" }

# --- MDX ---
mdxjs                          = { version = "1" }
```

#### B. Crates internes Next.js/Turbopack → git+branch="aphrody"

```toml
# --- Turbopack runtime (turbo-tasks-*) ---
turbo-tasks            = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbo-tasks" }
turbo-tasks-fs         = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbo-tasks-fs" }
turbo-tasks-fetch      = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbo-tasks-fetch" }
turbo-tasks-env        = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbo-tasks-env" }
turbo-tasks-malloc     = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbo-tasks-malloc" }
# ... 12 autres turbo-* / turbo-tasks-*

# --- Turbopack compiler ---
turbopack              = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbopack" }
turbopack-core         = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbopack-core" }
turbopack-ecmascript   = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbopack-ecmascript" }
turbopack-css          = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbopack-css" }
turbopack-image        = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbopack-image" }
turbopack-wasm         = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbopack-wasm" }
turbopack-mdx          = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbopack-mdx" }
turbopack-dev-server   = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "turbopack-dev-server" }
# ... 25 autres turbopack-*

# --- Next.js core ---
next-core              = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "next-core" }
next-api               = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "next-api" }
next-build             = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "next-build" }
next-custom-transforms = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "next-custom-transforms" }
next-code-frame        = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "next-code-frame" }
next-napi-bindings     = { git = "https://github.com/aphrody-code/next.js.git", branch = "aphrody", package = "next-napi-bindings" }
```

### Priorisation d'intégration

#### Priorité 1 (immédiat) — Crates publics
- `swc_core`, `lightningcss` : foundations
- `modularize_imports`, `styled_components`, `styled_jsx`, `swc_emotion` : transforms communs
- `preset_env_base`, `browserslist-rs` : target detection

#### Priorité 2 (haute) — Turbopack core
- `turbo-tasks` + `turbo-tasks-fs` + `turbo-tasks-env`
- `turbopack-core` + `turbopack-ecmascript` + `turbopack-css`

#### Priorité 3 (moyenne) — Dev server + runtime
- `turbopack-dev-server` + `turbopack-nodejs` + `turbopack-browser`
- `turbopack-image` + `turbopack-wasm` (pour notre cible WASM/WebGPU)

#### Priorité 4 (basse) — Next.js spécifiques
- `next-core`, `next-api`, `next-build` (utiles si on intègre la pipeline Next dans aphrody CLI)

---

## 4. Patches connus à propager dans aphrody/Cargo.toml `[patch.crates-io]`

Next.js déclare ces patches dans son root Cargo.toml :
- `triomphe` : patch sokra
- `mdxjs` : patch mdxjs-rs-turbopack
- `bincode` : patch éventuel

À reproduire dans aphrody/Cargo.toml si on intègre Turbopack natif.

---

## 5. Métriques

- **70 fichiers Cargo.toml** dans next.js workspace
- **60+ crates internes** version 0.1.0/0.0.0 (path deps)
- **15+ crates Vercel/Rust publics** sur crates.io
- **Patches git** : 3 (triomphe, mdxjs, bincode)

---

## 6. Docs à étudier

- [SWC documentation](https://swc.rs/docs/usage/core)
- [Turbopack documentation](https://turbo.build/pack/docs)
- [turbo-tasks framework](https://github.com/vercel/next.js/tree/canary/turbopack/crates/turbo-tasks)
- [Lightning CSS](https://lightningcss.dev/)
- [next-core architecture](https://github.com/vercel/next.js/tree/canary/crates/next-core)
