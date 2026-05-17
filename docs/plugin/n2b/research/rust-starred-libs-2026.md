# Rust — Librairies starred ⭐ pour stack 2026

Date de compilation : 2026-04-18
Source : awesome-rust + crates.io API live

---

## Table

| # | Crate | Version | Téléchargements | Rôle |
|---|---|---|---|---|
| 1 | [axum](#1-axum) | 0.8.9 | 290 243 146 | HTTP server framework |
| 2 | [leptos](#2-leptos) | 0.8.19 | 2 740 046 | Frontend full-stack (signals + SSR) |
| 3 | [reqwest](#3-reqwest) | 0.13.2 | 440 770 766 | HTTP client |
| 4 | [tungstenite](#4-tungstenite) | 0.29.0 | 185 911 425 | WebSocket |
| 5 | [utoipa](#5-utoipa) | 5.4.0 | 24 580 673 | OpenAPI code-first |
| 6 | [zola](#6-zola) | — | — | Static site generator |
| 7 | [tauri](#7-tauri) | 2.10.3 | 14 075 064 | Shell desktop/mobile natif |
| 8 | [dioxus](#8-dioxus) | 0.7.5 | 1 346 228 | UI cross-platform (web/desktop/mobile) |
| 9 | [gpui-component](#9-gpui-component) | 0.5.1 | 42 053 | Composants GPUI (Zed-like) |
| 10 | [blinc_app](#10-blinc) | 0.5.1 | 337 | UI GPU-accélérée multi-plateforme |

---

## 1. axum

Framework HTTP ergonomique sur Tokio/Tower/Hyper — **backend par défaut 2026**.

- **GitHub** : https://github.com/tokio-rs/axum
- **crates.io** : https://crates.io/crates/axum
- **docs.rs** : https://docs.rs/axum
- **Documentation officielle** : https://docs.rs/axum/latest/axum/
- **Examples** : https://github.com/tokio-rs/axum/tree/main/examples
- **Changelog** : https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md

Points clés :
- Routing déclaratif + extractors type-safe
- Middleware via `tower::Service`
- Intégration native avec Leptos (server functions)
- WebSocket upgrade natif

---

## 2. leptos

Framework full-stack Rust basé signals fine-grained — **frontend favori 2026**.

- **Site officiel** : https://leptos.dev
- **GitHub** : https://github.com/leptos-rs/leptos
- **crates.io** : https://crates.io/crates/leptos
- **Documentation API (docs.rs)** : https://docs.rs/leptos
- **Leptos Book (guide complet)** : https://book.leptos.dev
- **Getting started** : https://book.leptos.dev/getting_started/index.html
- **Deployment guide** : https://book.leptos.dev/deployment/index.html
- **Awesome Leptos** : https://github.com/leptos-rs/awesome-leptos

Écosystème à coupler :
- **leptos-use** — utilitaires (type VueUse) : https://leptos-use.rs
- **thaw** — composants Fluent Design : https://thawui.vercel.app
- **cargo-leptos** (tooling officiel) : https://github.com/leptos-rs/cargo-leptos

---

## 3. reqwest

Client HTTP ergonomique — **le standard** de l'écosystème.

- **GitHub** : https://github.com/seanmonstar/reqwest
- **crates.io** : https://crates.io/crates/reqwest
- **Documentation** : https://docs.rs/reqwest
- **Examples** : https://github.com/seanmonstar/reqwest/tree/master/examples

Points clés :
- Sync + async (Tokio)
- TLS natif ou rustls
- Multipart, cookies, redirections, proxy
- Compile vers WASM (sous-ensemble) → utilisable côté frontend

---

## 4. tungstenite

Implémentation WebSocket légère, stream-based.

- **GitHub** : https://github.com/snapview/tungstenite-rs
- **crates.io** : https://crates.io/crates/tungstenite
- **Documentation** : https://docs.rs/tungstenite

Variante async : **tokio-tungstenite**
- **GitHub** : https://github.com/snapview/tokio-tungstenite
- **crates.io** : https://crates.io/crates/tokio-tungstenite
- **Documentation** : https://docs.rs/tokio-tungstenite

---

## 5. utoipa

Génération OpenAPI code-first via macros — dérive `#[derive(ToSchema, OpenApi)]`.

- **GitHub** : https://github.com/juhaku/utoipa
- **crates.io** : https://crates.io/crates/utoipa
- **Documentation** : https://docs.rs/utoipa
- **Examples** : https://github.com/juhaku/utoipa/tree/master/examples

Crates complémentaires :
- **utoipa-swagger-ui** — intègre Swagger UI : https://docs.rs/utoipa-swagger-ui
- **utoipa-redoc** — intègre Redoc : https://docs.rs/utoipa-redoc
- **utoipa-rapidoc** — intègre RapiDoc : https://docs.rs/utoipa-rapidoc
- **utoipa-axum** — bindings Axum : https://docs.rs/utoipa-axum
- **utoipauto** — auto-collecte des schémas : https://github.com/ProbablyClem/utoipauto

---

## 6. zola

Static site generator opiniaté, un seul binaire, tout intégré (templates Tera, Sass, syntax highlighting…).

- **Site officiel** : https://www.getzola.org
- **GitHub** : https://github.com/getzola/zola
- **Documentation** : https://www.getzola.org/documentation/getting-started/overview/
- **Thèmes** : https://www.getzola.org/themes/
- **CLI reference** : https://www.getzola.org/documentation/getting-started/cli-usage/

---

## 7. tauri

Shell desktop + mobile natif, WebView OS, backend Rust — **2–10 Mo binaires, vs 60+ Mo Electron**.

- **Site officiel** : https://tauri.app
- **Documentation v2** : https://v2.tauri.app
- **GitHub** : https://github.com/tauri-apps/tauri
- **crates.io** : https://crates.io/crates/tauri
- **Guide mise en route** : https://v2.tauri.app/start/
- **API JS** : https://v2.tauri.app/reference/javascript/api/
- **API Rust** : https://docs.rs/tauri
- **Plugins officiels** : https://v2.tauri.app/plugin/
- **awesome-tauri** : https://github.com/tauri-apps/awesome-tauri

Plugins clés pour look natif :
- **window-vibrancy** : https://github.com/tauri-apps/window-vibrancy
- **tauri-plugin-decorum** : https://github.com/clearlysid/tauri-plugin-decorum
- **wry** (WebView) : https://github.com/tauri-apps/wry

Frontend intégrations :
- Leptos : https://v2.tauri.app/start/frontend/leptos/
- Dioxus : manuel (config projet)
- React/Vue/Svelte/Solid : templates `create-tauri-app`

---

## 8. dioxus

Framework fullstack cross-platform (web, desktop, mobile, TUI) — VDOM React-like + hot-patching Rust.

- **Site officiel** : https://dioxuslabs.com
- **GitHub** : https://github.com/DioxusLabs/dioxus
- **crates.io** : https://crates.io/crates/dioxus
- **Documentation API** : https://docs.rs/dioxus
- **Guide 0.7** : https://dioxuslabs.com/learn/0.7/
- **Examples** : https://github.com/DioxusLabs/dioxus/tree/main/examples
- **Awesome Dioxus** : https://github.com/DioxusLabs/awesome-dioxus

Sous-projets 0.7 :
- **Dioxus Native / Blitz** (HTML/CSS pur Rust sans WebView) : https://github.com/DioxusLabs/blitz
- **dioxus-desktop** : https://docs.rs/dioxus-desktop
- **dioxus-mobile** : https://docs.rs/dioxus-mobile
- **dioxus-fullstack** : https://docs.rs/dioxus-fullstack
- **dioxus-cli (`dx`)** : https://github.com/DioxusLabs/dioxus/tree/main/packages/cli

---

## 9. gpui-component

Bibliothèque de composants UI construits sur **GPUI** (le framework UI de Zed, 120 FPS, GPU-accéléré).

- **GitHub** : https://github.com/longbridge/gpui-component
- **crates.io** : https://crates.io/crates/gpui-component
- **Documentation API** : https://docs.rs/gpui-component
- **Site/démos** : https://longbridge.github.io/gpui-component
- **Framework sous-jacent (GPUI)** : https://www.gpui.rs
- **GPUI dans Zed** : https://github.com/zed-industries/zed/tree/main/crates/gpui
- **Awesome GPUI** : https://github.com/zed-industries/awesome-gpui

---

## 10. blinc_app (Blinc)

Framework UI Rust déclaratif, réactif, GPU-accéléré (nouveau, 2025/2026).

- **Site de documentation** : https://project-blinc.github.io/Blinc/
- **GitHub** : https://github.com/project-blinc/Blinc
- **crates.io** : https://crates.io/crates/blinc_app
- **Documentation API** : https://docs.rs/blinc_app
- **Organisation** : https://github.com/project-blinc

Points distinctifs :
- 40+ composants style shadcn/ui
- State machines first-class
- Spring physics animations
- Glassmorphism natif
- Cible **Desktop (macOS/Windows/Linux) + Android + iOS + Web (WebGPU)**
- Builder API chainable

---

## Récap stack complète recommandée 2026

```text
┌─────────────────────────────────────────────────────┐
│  Backend                                            │
│  ├─ axum 0.8 (HTTP)                                 │
│  ├─ utoipa 5.4 (OpenAPI)                            │
│  └─ tokio-tungstenite (WebSocket server)            │
├─────────────────────────────────────────────────────┤
│  Frontend web (Rust → WASM)                         │
│  ├─ leptos 0.8                                      │
│  ├─ leptos-use + thaw (composants Fluent)           │
│  └─ cargo-leptos                                    │
├─────────────────────────────────────────────────────┤
│  Desktop + Mobile                                   │
│  ├─ tauri 2.10                                      │
│  ├─ window-vibrancy + tauri-plugin-decorum          │
│  └─ (alt) dioxus 0.7 pour cross-platform pur Rust   │
├─────────────────────────────────────────────────────┤
│  Client HTTP (SSR + native)                         │
│  └─ reqwest 0.13                                    │
├─────────────────────────────────────────────────────┤
│  Marketing / Docs site                              │
│  └─ zola                                            │
├─────────────────────────────────────────────────────┤
│  GUI 100 % native (option premium)                  │
│  ├─ gpui-component 0.5 (style Zed)                  │
│  └─ blinc_app 0.5 (GPU + animations spring)         │
└─────────────────────────────────────────────────────┘
```

---

## Baromètres officiels à suivre

- **Are we web yet ?** — https://www.arewewebyet.org
- **Are we GUI yet ?** — https://areweguiyet.com
- **blessed.rs** (recommandations curatées) — https://blessed.rs/crates
- **lib.rs** (stats + classements) — https://lib.rs/stats
- **flosse/rust-web-framework-comparison** — https://github.com/flosse/rust-web-framework-comparison

## Listes awesome de référence

- **rust-unofficial/awesome-rust** — https://github.com/rust-unofficial/awesome-rust (56 811 ⭐)
- **aalemayhu/WebRustList** — https://github.com/aalemayhu/WebRustList
- **tauri-apps/awesome-tauri** — https://github.com/tauri-apps/awesome-tauri
- **leptos-rs/awesome-leptos** — https://github.com/leptos-rs/awesome-leptos
- **DioxusLabs/awesome-dioxus** — https://github.com/DioxusLabs/awesome-dioxus
- **zed-industries/awesome-gpui** — https://github.com/zed-industries/awesome-gpui

---

# Annexe — Librairies par catégorie étendue

Versions et téléchargements relevés live sur crates.io le 2026-04-18.

---

## A. PostgreSQL / SQL / ORM

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **sqlx** | 0.8.6 | 88 M | https://docs.rs/sqlx — https://github.com/launchbadge/sqlx |
| **sea-orm** | 1.1.20 | 18 M | https://www.sea-ql.org/SeaORM — https://docs.rs/sea-orm |
| **sea-query** | 0.32.7 | 28 M | https://docs.rs/sea-query (query builder pur) |
| **diesel** | 2.3.7 | 25 M | https://diesel.rs — https://docs.rs/diesel |
| **tokio-postgres** | 0.7.17 | 42 M | https://docs.rs/tokio-postgres — https://github.com/rust-postgres/rust-postgres |
| **refinery** | 0.9.1 | 7 M | https://docs.rs/refinery (migrations SQL versionnées) |

**Reco 2026** : **sqlx** (SQL brut + compile-time checks) ou **sea-orm** (ORM moderne async).

---

## B. Discord bot

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **serenity** | 0.12.5 | 4,9 M | https://docs.rs/serenity — https://github.com/serenity-rs/serenity |
| **poise** | 0.6.2 | 385 K | https://docs.rs/poise (framework commands sur serenity) |
| **twilight** | 0.17.1 | 38 K | https://twilight.rs — https://github.com/twilight-rs/twilight |

**Reco 2026** : **serenity + poise** pour commandes slash, **twilight** pour bots à haute échelle (modulaire, cluster/shard).

---

## C. Graphics GPU / wgpu

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **wgpu** | 29.0.1 | 20 M | https://wgpu.rs — https://docs.rs/wgpu |
| **winit** | 0.30.13 | 37 M | https://docs.rs/winit (fenêtre cross-platform) |

- **wgpu book** : https://sotrh.github.io/learn-wgpu/
- **Examples officiels** : https://github.com/gfx-rs/wgpu/tree/trunk/examples
- **Cibles** : Vulkan, Metal, DX12, WebGPU, WebGL2 → **un seul code**

---

## D. 2D Canvas / rendu vectoriel

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **vello** | 0.8.0 | 250 K | https://docs.rs/vello — https://github.com/linebender/vello (GPU compute-based) |
| **tiny-skia** | 0.12.0 | 25 M | https://docs.rs/tiny-skia (sous-ensemble Skia en pur Rust) |
| **skia-safe** | 0.93.1 | 2,5 M | https://docs.rs/skia-safe (bindings Skia officiels) |

**Vello** = future référence (utilisé par Xilem, Blitz, Dioxus Native).

---

## E. Game engines

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **bevy** | 0.18.1 | 5 M | https://bevy.org — https://bevy.org/learn/ |
| **macroquad** | 0.4.14 | 1,4 M | https://macroquad.rs — https://docs.rs/macroquad |
| **fyrox** | 1.0.1 | 60 K | https://fyrox.rs — https://docs.rs/fyrox |

- **Bevy book** : https://bevy.org/learn/book/
- **Bevy cheatbook** : https://bevy-cheatbook.github.io
- **Are we game yet?** : https://arewegameyet.rs

---

## F. Parsers / Compilateurs

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **winnow** | 1.0.1 | 508 M | https://docs.rs/winnow (fork moderne de nom) |
| **nom** | 8.0.0 | 484 M | https://docs.rs/nom — https://github.com/rust-bakery/nom |
| **pest** | 2.8.6 | 224 M | https://pest.rs — https://docs.rs/pest |
| **logos** | 0.16.1 | 45 M | https://logos.maciej.codes — https://docs.rs/logos (lexer ultra rapide) |
| **chumsky** | 0.12.0 | 16 M | https://docs.rs/chumsky (parser combinators + error recovery) |
| **tree-sitter** | 0.26.8 | 17 M | https://tree-sitter.github.io — https://docs.rs/tree-sitter |

**Reco 2026** : **winnow** (parser combinators, zero-copy) + **logos** (lexer), ou **tree-sitter** pour coloration syntaxique et IDE-like.

Compagnons diagnostics : **miette** (erreurs jolies), **ariadne** (renderer de diagnostics).

---

## G. Linters / Formatters / Static analysis

| Outil | Version | DL | Documentation |
|---|---|---|---|
| **clippy** | 0.0.302 | 1,9 M | https://doc.rust-lang.org/clippy/ (linter officiel) |
| **rustfmt** | 0.10.0 | 546 K | https://rust-lang.github.io/rustfmt/ (formatter officiel) |

Écosystème au-delà (non-Rust mais écrits en Rust) :
- **biome** (JS/TS/JSON/CSS lint+format) : https://biomejs.dev — https://github.com/biomejs/biome
- **ruff** (Python lint + format) : https://docs.astral.sh/ruff/
- **dprint** (multi-langages) : https://dprint.dev

---

## H. Compression / Archives

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **flate2** | 1.1.9 | 446 M | https://docs.rs/flate2 (gzip/deflate/zlib) |
| **zstd** | 0.13.3 | 264 M | https://docs.rs/zstd |
| **brotli** | 8.0.2 | 171 M | https://docs.rs/brotli |
| **zip** | 8.5.1 | 164 M | https://docs.rs/zip — https://github.com/zip-rs/zip2 |
| **tar** | 0.4.45 | 151 M | https://docs.rs/tar |

Combo courant : **flate2 + tar** (.tar.gz), **zstd** si perf critique.
CLI : **ouch** (compression universelle, déjà installé sur ce VPS).

---

## I. Serialization

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **serde** | 1.0.228 | 935 M | https://serde.rs — https://docs.rs/serde |
| **serde_json** | — | 839 M | https://docs.rs/serde_json |
| **bincode** | 3.0.0 | 224 M | https://docs.rs/bincode (binaire compact) |
| **rkyv** | 0.8.15 | 99 M | https://rkyv.org — https://docs.rs/rkyv (zero-copy deserialization) |
| **prost** | 0.14.3 | 375 M | https://docs.rs/prost (Protocol Buffers) |

**Reco 2026** : **serde + serde_json** toujours par défaut, **rkyv** pour perf zero-copy, **prost** pour gRPC.

---

## J. Async runtime

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **tokio** | 1.52.1 | 618 M | https://tokio.rs — https://docs.rs/tokio |

Livre officiel : **Tokio Mini-Redis / Tokio Tutorial** : https://tokio.rs/tokio/tutorial

---

## K. Observability / Tracing

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **tracing** | 0.1.44 | 550 M | https://docs.rs/tracing — https://tokio.rs |
| **tracing-subscriber** | — | — | https://docs.rs/tracing-subscriber |
| **opentelemetry** | — | — | https://docs.rs/opentelemetry |

Compagnons : **metrics**, **prometheus**, **tracing-opentelemetry**.

---

## L. Error handling / Diagnostics

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **anyhow** | 1.0.102 | 632 M | https://docs.rs/anyhow (erreurs dynamiques, apps) |
| **thiserror** | 2.0.18 | 907 M | https://docs.rs/thiserror (erreurs typées, libs) |
| **miette** | 7.6.0 | 48 M | https://docs.rs/miette (diagnostics fancy à la Rust compiler) |

**Convention** : `thiserror` dans les libs, `anyhow` dans les binaires, `miette` si tu veux afficher des erreurs compilateur-like.

---

## M. CLI / TUI

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **clap** | 4.6.1 | 772 M | https://docs.rs/clap — https://docs.rs/clap/latest/clap/_derive/_tutorial/ |
| **ratatui** | 0.30.0 | 24 M | https://ratatui.rs — https://docs.rs/ratatui (TUI moderne, fork tui-rs) |

Compagnons : **indicatif** (progress bars), **dialoguer** (prompts), **crossterm** (events terminal).

---

## N. Testing / Benchmarks

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **criterion** | 0.8.2 | 191 M | https://bheisler.github.io/criterion.rs/book/ |
| **proptest** | — | — | https://docs.rs/proptest (property testing) |
| **insta** | — | — | https://insta.rs (snapshot testing) |
| **mockall** | — | — | https://docs.rs/mockall |

---

## O. Crypto / TLS

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **ring** | 0.17.14 | 482 M | https://docs.rs/ring |
| **rustls** | 0.23.38 | 596 M | https://docs.rs/rustls — https://github.com/rustls/rustls |

Compagnons : **webpki**, **rustls-pemfile**, **age** (file encryption), **ed25519-dalek**.

---

## P. Image / Regex / Utilitaires ultra-utilisés

| Crate | Version | DL | Documentation |
|---|---|---|---|
| **image** | 0.25.10 | 117 M | https://docs.rs/image |
| **regex** | 1.12.3 | 779 M | https://docs.rs/regex |
| **axum-extra** | 0.12.6 | 31 M | https://docs.rs/axum-extra (extensions axum : cookies, typed headers…) |

---

## Q. WebAssembly — LA lib manquante : `wasm-bindgen` ⭐⭐⭐

**La pièce centrale qu'il manquait** : sans `wasm-bindgen`, aucun framework web Rust ne peut tourner dans un navigateur. Leptos, Dioxus web, Yew, trunk, wasm-pack, wgpu-web, gloo — **tout** dépend de wasm-bindgen. 326 M de téléchargements, la 5e crate la plus utilisée.

### Q.1 Le cœur — bindings Rust ↔ JS

| Crate | Version | DL | Rôle |
|---|---|---|---|
| **wasm-bindgen** ⭐ | 0.2.118 | **326 M** | Pont Rust↔JS : macros `#[wasm_bindgen]`, types, import/export, GC. LE fondement |
| **js-sys** | 0.3.95 | **303 M** | Bindings de tous les globals JS standards (`Array`, `Promise`, `Object`…) |
| **web-sys** | 0.3.95 | **240 M** | Bindings de toutes les Web APIs, **auto-généré depuis WebIDL** |
| **wasm-bindgen-futures** | 0.4.68 | 201 M | Bridge Rust `Future` ↔ JS `Promise` (async/await côté web) |
| **serde-wasm-bindgen** | 0.6.5 | 57 M | Adapter serde natif pour passer des structs Rust ↔ `JsValue` |
| **gloo** | 0.12.0 | 6 M | Toolkit utilitaires wasm-bindgen (timers, storage, events, history, network…) |

**Documentation officielle** :
- **Site** : https://rustwasm.github.io/wasm-bindgen/
- **GitHub** : https://github.com/rustwasm/wasm-bindgen
- **The wasm-bindgen Guide** : https://rustwasm.github.io/wasm-bindgen/introduction.html
- **API** : https://docs.rs/wasm-bindgen
- **web-sys API browser** : https://rustwasm.github.io/wasm-bindgen/api/web_sys/
- **js-sys API browser** : https://rustwasm.github.io/wasm-bindgen/api/js_sys/
- **Rust + WASM book** : https://rustwasm.github.io/book/
- **MDN Rust to Wasm guide** : https://developer.mozilla.org/docs/WebAssembly/Guides/Rust_to_Wasm

### Q.2 Build tools / pipeline

| Outil | Version | DL | Rôle |
|---|---|---|---|
| **wasm-pack** | — | — | Build + package pour bundlers/npm. https://rustwasm.github.io/wasm-pack/ |
| **trunk** | 0.21.14 | 988 K | Dev server + build SPA Rust WASM (Yew/Leptos). https://trunkrs.dev |
| **wasm-opt** (binaryen) | 0.116.1 | 7,6 M | Optimisation binaire `.wasm`. https://github.com/WebAssembly/binaryen |
| **twiggy** | 0.8.0 | 80 K | Profiler de taille `.wasm`. https://rustwasm.github.io/twiggy/ |
| **walrus** | 0.26.1 | 11 M | Transformation `.wasm` programmatique. https://docs.rs/walrus |
| **cargo-leptos** | — | — | Tooling Leptos (bundler + Tailwind intégré) |

Tout ce pipeline est **wrappé** par la commande `n2b wasm` locale (init, doctor, build, opt, size, spec).

### Q.3 Runtimes WebAssembly standalone (server-side)

| Crate | Version | DL | Rôle |
|---|---|---|---|
| **wasmtime** ⭐ | 43.0.1 | 21 M | Runtime WASM standalone **Bytecode Alliance**. Support WASI + Component Model |
| **wasmer** | 7.1.0 | 7 M | Runtime concurrent, plugin architecture, bindings multi-langages |
| **wasmi** | 1.0.9 | 17 M | Interpréteur pur Rust, `no_std`, parfait pour embarqué et blockchain |

**Documentation** :
- **Wasmtime** : https://wasmtime.dev — https://docs.wasmtime.dev/
- **Wasmer** : https://wasmer.io — https://docs.rs/wasmer
- **Wasmi** : https://github.com/wasmi-labs/wasmi

**Reco 2026** : **wasmtime** par défaut pour exécution WASM côté serveur (plugins, sandbox, WASI). C'est aussi le runtime de référence pour la spec Component Model.

### Q.4 Analyse / parsing de fichiers `.wasm`

| Crate | Version | DL | Rôle |
|---|---|---|---|
| **wasmparser** ⭐ | 0.247.0 | 102 M | Parser event-driven `.wasm` (utilisé par wasmtime, cargo, etc.) |
| **wat** | 1.247.0 | 25 M | Parser du format texte WAT (.wat/.wast) |
| **walrus** | 0.26.1 | 11 M | Transformation de modules `.wasm` (round-trip parse + modify + emit) |

### Q.5 Pourquoi wasm-bindgen est **incontournable**

C'est **la seule abstraction** qui permet :
- D'exporter des **structs/fonctions Rust** vers JS avec typage sûr
- D'importer **n'importe quelle API JS/Web** (via `#[wasm_bindgen(js_namespace = …)]`)
- De traverser la frontière Rust↔JS avec **copies zéro** pour certains types (`&[u8]`, `Vec<u8>`)
- De bénéficier des bindings Web auto-générés depuis **WebIDL** (via `web-sys`)
- Un **async interop natif** (`wasm-bindgen-futures`) : un `async fn` Rust devient une `Promise` JS et vice-versa

**Dépendances transitives** : tout Leptos, Dioxus web, Yew, gloo, wgpu (sur WebGPU), bevy (web target), three-d, egui (web target), etc. dépendent de wasm-bindgen.

### Q.6 Stack WASM complète

```
┌───────────────────────────────────────────────────────┐
│  Web (Rust → WASM → navigateur)                       │
│  ├─ wasm-bindgen 0.2         (cœur)                   │
│  ├─ js-sys                   (JS globals)             │
│  ├─ web-sys                  (Web APIs)               │
│  ├─ wasm-bindgen-futures     (async)                  │
│  ├─ serde-wasm-bindgen       (structs ↔ JsValue)      │
│  ├─ gloo                     (utilitaires)            │
│  ├─ Build : wasm-pack / trunk / cargo-leptos          │
│  └─ Optim : wasm-opt (binaryen) + twiggy              │
├───────────────────────────────────────────────────────┤
│  Server-side (runtime standalone)                     │
│  ├─ wasmtime 43              (WASI + Component)       │
│  ├─ wasmer 7                 (alternative)            │
│  └─ wasmi 1                  (interpréteur no_std)    │
├───────────────────────────────────────────────────────┤
│  Analyse / transformation                             │
│  ├─ wasmparser               (parse)                  │
│  ├─ wat                      (text format)            │
│  └─ walrus                   (transform)              │
└───────────────────────────────────────────────────────┘
```

### Q.7 Tooling local (ce VPS)

- **Repos** : `/home/ubuntu/rsbun/wasm/{binaryen,wabt,wasm-bindgen,wasm-pack,spec}` + forks Bun-native (`binaryen-bun`, `wabt-bun`)
- **Patches Bun-native** : `WASM_BINDGEN_BUN_PATCH.md`, `WASM_PACK_BUN_PATCH.md` — remplace `Command::new("node")` par détection runtime (bun|node)
- **CLI maison** : `n2b wasm` (init/doctor/build/opt/size/spec) wrappe toute la chaîne

---


---

---

# Annexe 2 — Arbitrages : quel choisir dans chaque doublon ?

Basé sur benchmarks, maintenance et état de l'écosystème en **2026-04-18**.

---

## 🏆 SQL / ORM — **sqlx**

| Choix | Verdict |
|---|---|
| **sqlx** ✅ | **Défaut 2026**. Async natif, SQL brut avec **compile-time checks** via `query!`. Parfait si tu penses en SQL. |
| sea-orm | Choisir si tu **veux un vrai ORM** avec relations, viens de Django/Rails, ou besoin de queries dynamiques complexes. Construit au-dessus de sqlx. |
| diesel | **À éviter** pour nouveau projet async. Sync nativement, async via `diesel-async` (dépendance en plus). Meilleur si tu veux le DSL compile-time Rust. |

**Combo recommandé** : `sqlx` + `sea-query` (query builder typé) + `refinery` (migrations) pour 90 % des cas.

---

## 🏆 Discord bot — **serenity + poise**

| Choix | Verdict |
|---|---|
| **serenity + poise** ✅ | **Défaut 2026**. Le framework standard de serenity est **déprécié depuis 0.12.1** → utiliser **poise** pour les commandes. Slash commands, edit tracking, argument parsing — tout en une signature de fonction. |
| twilight | Choisir si tu veux **haute échelle** (cluster/shard explicite), maximum de flexibilité, ou si tu es très à l'aise avec Rust et connais l'API Discord. Modulaire (chaque sous-crate séparé). |

**Reco** : **serenity + poise** pour 95 % des bots, **twilight** si tu bâtis l'équivalent d'un bot modération/analytics multi-serveurs à 10 K+ guildes.

---

## 🏆 Canvas 2D — **vello** (GPU) / **tiny-skia** (CPU)

| Choix | Verdict |
|---|---|
| **vello** ✅ | **Défaut 2026 GPU**. Compute-centric sur wgpu. Utilisé par Xilem, Blitz, Dioxus Native. Aussi **vello_cpu** (alpha) : bat déjà Skia/Cairo en pur CPU. |
| **tiny-skia** ✅ | **Défaut 2026 CPU**. 14 KLOC, compile en 5 s, +200 Ko binaire. 20–100 % plus lent que Skia x86, 100–300 % plus lent ARM. |
| skia-safe | Bindings Skia **officiels** via FFI. À choisir uniquement si tu as **déjà du Skia natif** côté C++ ou besoin de 100 % de compatibilité. Gros binaire. |

**Reco** : **vello** si tu cibles WebGPU / desktop GPU, **tiny-skia** pour un outil léger / embarqué / WASM léger.

---

## 🏆 Game engine — **bevy** (défaut) / **fyrox** (éditeur) / **macroquad** (jam)

| Choix | Verdict |
|---|---|
| **bevy 0.18** ✅ | **Défaut 2026**. 44 K+ ⭐. ECS data-driven, code-only, 2D+3D, threading auto, plugin ecosystem énorme. Courbe d'apprentissage moyenne. |
| **fyrox 1.0** | Choisir si **éditeur visuel requis** avec hot reload, comme Godot/Unity/Unreal. Seul de cette catégorie en Rust. 3D complet. |
| **macroquad 0.4** | Choisir pour **game jam / prototype / 2D simple** — inspiré Raylib, zéro friction, compile vite. Aussi compile en WASM trivialement. |

**Reco** : commence par **bevy** par défaut, passe à **fyrox** si tu as besoin d'un vrai éditeur scene-graph.

---

## 🏆 Parsers / Compilateurs — **winnow** (défaut) / **chumsky** (langages) / **logos** (lexer)

Benchmark (plus bas = mieux) :

| Lib | Temps | Style |
|---|---|---|
| **winnow** | 179 µs | Combinators (fork moderne de nom) |
| **chumsky** | 210 µs | Framework, excellente error recovery |
| **nom** | 527 µs | Combinators, référence historique |
| **pest** | 1 970 µs | Grammaire PEG externe |

| Choix | Verdict |
|---|---|
| **winnow** ✅ | **Défaut 2026**. Le plus rapide. Fork maintenu activement de nom par epage. Zero-copy. API "toolbox". |
| **chumsky** ✅ | Choisir pour **langages de programmation** — l'**error recovery** et les messages d'erreur sont supérieurs. |
| **logos** ✅ | **Lexer** dédié, ultra rapide. À coupler avec winnow/chumsky. |
| **pest** | Choisir si tu préfères **grammaire externe** type PEG (fichier `.pest`). Lent mais très lisible. |
| **nom** | À **remplacer par winnow** dans tout nouveau projet (même API approchante). |
| **tree-sitter** | Pour **coloration syntaxique / IDE / outils** — pas un parser généraliste mais un parser incrémental. |

**Compagnons diagnostics** : **miette** (affichage erreurs) + **ariadne** (renderer).

---

## 🏆 Serialization binaire — **postcard** (défaut) / **rkyv** (perf)

⚠️ **Alerte 2025/2026** : **bincode est marqué unmaintained**. Migrer.

| Choix | Verdict |
|---|---|
| **postcard** ✅ | **Défaut 2026**. Drop-in replacement de bincode, dans l'écosystème serde, maintenu, utilisé par 7000+ repos. Consistent **plus petit** que bincode sur zlib. Pensé embedded-first. |
| **rkyv** ✅ | **Le plus rapide**. Zero-copy deserialization vraie. Remporte quasiment tous les benchs perf et taille. Ses traits sont **séparées de serde**. Choisir pour : base de données embarquée, caches disque, IPC haute perf. |
| bincode | **À migrer**. Marqué unmaintained. |
| prost | **Protobuf** (gRPC) — cas spécifique. |

**Reco** : **postcard** pour remplacer bincode par défaut, **rkyv** si tu as besoin de zero-copy pour perf extrême.

---

## 🏆 Compression — **zstd** (défaut) / **flate2** (compat) / **brotli** (web)

| Choix | Verdict |
|---|---|
| **zstd** ✅ | **Défaut 2026**. Compresse aussi vite ou plus vite que brotli, décompresse plus vite que brotli, ratio 50–70 %. Le meilleur compromis. |
| **flate2** ✅ | **Requis** si tu dois produire du **gzip/deflate/zlib** (compat universelle). Backend **zlib-rs** (pur Rust) = le plus rapide en 2026. |
| **brotli** | Spécifique **web** (Content-Encoding: br supporté par tous les navigateurs). Décompression lente par rapport à zstd/flate2. |
| lzma/xz | Meilleur ratio mais très lent à compresser. Archivage froid. |
| **zip / tar** | Formats **conteneurs**, utiliser avec un des ci-dessus. |

**Reco** : **zstd** pour perf interne, **brotli** pour HTTP public, **flate2** pour compat gzip.

---

## 🏆 Async runtime — **tokio** (toujours)

⚠️ **Alerte 2025** : **async-std est officiellement discontinué**. Équipe recommande migration vers **smol** ou **tokio**.

| Choix | Verdict |
|---|---|
| **tokio** ✅ | **Défaut 2026 absolu**. Ecosystem dominance : axum, reqwest, tonic, hyper, sqlx, warp — tout tourne sur Tokio. Mio reactor. Traits AsyncRead/Write propres. |
| smol | Pour **libs** très légères, embarqué, workloads atypiques. API plus simple. Bridges possible via `async-compat`. |
| async-std | **MORT**. Ne plus utiliser. |

**Reco** : **tokio** sans hésiter pour 99 % des projets.

---

## 🏆 Error handling — **thiserror** (libs) + **anyhow** (bins) + **miette** (UX)

Pas un doublon — **utiliser les trois ensemble** selon le contexte :

| Crate | Quand |
|---|---|
| **thiserror** ✅ | Dans une **lib** : type d'erreur explicite pour ton API publique (`#[derive(Error)]`) |
| **anyhow** ✅ | Dans un **binaire/app** : `Result<T>` ergonomique avec contexte (`.context()`) |
| **miette** ✅ | Dans un outil CLI/compilateur : affichage **fancy** avec highlights et snippets |

---

## Matrice récapitulative des décisions

| Catégorie | 🥇 Choix | 🥈 Si besoin spécifique |
|---|---|---|
| SQL / ORM | **sqlx** | sea-orm (feel ORM) |
| Discord bot | **serenity + poise** | twilight (haute échelle) |
| Canvas 2D | **vello** (GPU) / **tiny-skia** (CPU) | skia-safe (compat native) |
| Game engine | **bevy** | fyrox (éditeur), macroquad (jam) |
| Parser | **winnow + logos** | chumsky (langages), pest (grammaires externes) |
| Serialization binaire | **postcard** | rkyv (zero-copy perf) |
| Compression | **zstd** | flate2 (gzip compat), brotli (HTTP) |
| Async runtime | **tokio** | — |
| Error handling | **thiserror + anyhow + miette** (combo) | — |
| HTTP server | **axum** | actix-web (perf brute) |
| HTTP client | **reqwest** | hyper (bas niveau) |
| Frontend WASM | **leptos** | dioxus (cross-platform) |
| Desktop shell | **tauri 2** | dioxus desktop (pur Rust) |
| Native GUI | **gpui-component** | blinc_app (early, GPU premium) |
| TUI | **ratatui** | — |
| CLI parsing | **clap** | — |
| TLS | **rustls** | — |
| Crypto | **ring** | — |
| **WebAssembly (Rust↔JS)** | **wasm-bindgen + web-sys + js-sys** | serde-wasm-bindgen + gloo |
| WASM runtime serveur | **wasmtime** | wasmer, wasmi (no_std) |
| WASM parsing | **wasmparser** | wat, walrus |

---

## Stack complète finale — toutes catégories

```text
Runtime async   : tokio
HTTP server     : axum + axum-extra + utoipa + tower + tower-http
HTTP client     : reqwest
WebSocket       : tokio-tungstenite
DB              : sqlx (PostgreSQL) + sea-query (builder) + refinery (migrations)
Frontend WASM   : leptos + leptos-use + thaw
WASM bindings   : wasm-bindgen + js-sys + web-sys + wasm-bindgen-futures + serde-wasm-bindgen + gloo
WASM runtime    : wasmtime (serveur/plugins) + wasmparser (analyse) + walrus (transformation)
WASM tooling    : wasm-pack + trunk + wasm-opt + twiggy
Desktop/Mobile  : tauri 2 + window-vibrancy + tauri-plugin-decorum
Cross-platform  : dioxus 0.7 (+ Blitz pour rendu natif)
Native UI       : gpui-component / blinc_app
Discord bot     : serenity + poise (ou twilight si scaling)
Graphics GPU    : wgpu + winit
Canvas 2D       : vello (GPU) ou tiny-skia (CPU)
Game engine     : bevy
Parsers         : winnow + logos + miette
Serialization   : serde + serde_json + postcard + rkyv (zero-copy perf) + prost (gRPC)
Compression     : zstd (+ flate2 gzip legacy, brotli HTTP)
Error handling  : anyhow (bin) + thiserror (lib) + miette (UX)
Observability   : tracing + tracing-subscriber + opentelemetry
CLI/TUI         : clap + ratatui + indicatif
Testing         : criterion + proptest + insta
Crypto/TLS      : rustls + ring
Static site     : zola
Image           : image
Regex           : regex
```
