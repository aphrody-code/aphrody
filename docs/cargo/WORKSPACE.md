<!-- SPDX-License-Identifier: Apache-2.0 -->
# Workspace architecture

> Réf. : `Cargo.toml` racine, `.cargo/config.toml`, `rust-toolchain.toml`.

## Identité

```toml
[workspace]
resolver = "3"        # MSRV-aware resolver (Cargo 1.93+)
members  = [57 crates]
exclude  = [gui, agui-bridge, mui-rs*, tuono*, aphrody-x-client,
            a2a-slimrpc, coreutils, util-linux]

[workspace.package]
version      = "1.0.0-canary"
edition      = "2024"
rust-version = "1.97"
license      = "Apache-2.0"
```

## Resolver 3 (MSRV-aware)

Cargo 1.93+ supporte `resolver = "3"` qui rend la résolution **MSRV-aware** :
- Préfère les versions de deps compatibles avec notre `rust-version = "1.97"`.
- Change le défaut de `resolver.incompatible-rust-versions` de `allow` → `fallback`.
- Stable depuis Rust 1.84.

## Membres (57 crates)

Le workspace compte **57 membres actifs**. Au total **71 crates existent sur
disque** sous `crates/` : 57 dans `members`, 14 dans `exclude` (clusters UI/web
lourds, voir plus bas). La liste exhaustive et les rôles sont décrits dans
[`CRATES.md`](./CRATES.md). Vue d'ensemble par famille :

```text
crates/
├── cli                     ← binaire principal (package « aphrody »)
├── base                    ← primitives no_std partagées
├── backend                 ← forensics, réseau, introspection cross-platform
├── mrx                     ← monorepo mapper unifié (ex mrx-{core,detect,audit,watch,cli})
│
├── a2a (a2a-lf)            ← protocole agent-to-agent (core)
├── a2a-client (a2a-client-lf)
├── a2a-server (a2a-server-lf)
├── a2a-pb                  ← protobuf gen
├── a2a-grpc                ← binding gRPC
├── a2a-ui                  ← viewer WASM des canaux
├── google_mcp              ← serveur MCP natif
│
├── aphrody-llm-infra       ← cost + rateguard + retry + cache (unifié)
├── aphrody-skills          ← runtime + hooks + permissions (unifié)
├── aphrody-skills-forge    ← scaffolding/registry/lint des SKILL.md
├── aphrody-messaging       ← connecteurs sortants + canaux bidirectionnels (unifié)
├── aphrody-design          ← sidecar + daemon design (unifié)
├── aphrody-design-agents   ← spawner CLI agents (Claude/Gemini/Antigravity)
├── aphrody-voice           ← TTS + STT (unifié)
├── aphrody-memory          ← MemoryBackend async (JSONL/HNSW/SQLite/LanceDB)
├── aphrody-gateway         ← AI gateway OpenAI-compatible
├── aphrody-mcp             ← client OAuth 2.1 MCP HTTP/SSE
├── aphrody-mcp-smoke       ← harness smoke E2E MCP
├── aphrody-router          ← routeur LLM (whitelist 3-only)
├── aphrody-providers       ← enum Provider canonique
├── aphrody-prompts         ← templates minijinja
├── aphrody-context         ← gestion de la fenêtre de contexte
├── aphrody-session         ← suivi de session conversationnelle
├── aphrody-tools           ← registre de tool descriptors
├── aphrody-events          ← bus pub-sub in-process
├── aphrody-secrets         ← secret-store chiffré
├── aphrody-settings        ← loader JSON hiérarchique
├── aphrody-telemetry       ← spans/compteurs/histogrammes
├── aphrody-task-runner     ← exécuteur DAG parallèle
├── aphrody-search          ← full-text in-memory (BM25-lite)
├── aphrody-cron            ← scheduler interval/daily/cron
├── aphrody-marketplace     ← index skills/MCP/hooks/themes
├── aphrody-re              ← primitives reverse engineering (PE/ELF)
├── aphrody-chat            ← REPL turn-loop
├── aphrody-sdk             ← SDK public d'embarquement
├── aphrody-translate       ← traduction FR + scrub AI-isms
├── aphrody-summary         ← génère docs/SUMMARY.md + docs/llms.txt
├── gemini-runtime          ← adaptateur runtime Gemini CLI
├── notebooklm              ← client NotebookLM Boq RPC
│
├── aphrody-terminal-vt         ┐
├── aphrody-terminal-wasm       │
├── aphrody-terminal-backend    │
├── aphrody-terminal-llm        ├ stack terminal LLM-first (8 crates)
├── aphrody-terminal-browser    │
├── aphrody-terminal-json-out   │
├── aphrody-terminal-markdown   │
├── aphrody-terminal-config     ┘
├── aphrody-tui                 ← DSL TUI pur Rust
│
├── aphrody-wasm            ← wrapper wasm-bindgen de base
├── aphrody-react-reconciler← reconciler React host-side
├── m3-tokens               ← tokens M3 (color/typo/elevation/motion)
├── aphrody-icons           ← font/CSS icônes Material Symbols
└── ievr-tools              ← analyse d'inventaire binaire IEVR
```

## Exclusions (présentes sur disque, hors build par défaut)

Ces crates **existent toujours** sous `crates/` mais sont listées dans le bloc
`exclude` du `Cargo.toml`. Raison : clusters UI/web lourds (wgpu/vello/winit/
wasmtime + Next.js/SWC/lightningcss/napi) qui dominaient le temps de
`cargo nextest run --workspace` sur la machine de référence (4c/8t, 16 Go) ;
aucun n'est dans la chaîne de dépendance du binaire `aphrody`. Pour les
rebâtir : les re-lister temporairement dans `members` ou utiliser un workspace
ad hoc.

| Path | Raison |
|---|---|
| `crates/gui/` | Agrège `mui-rs*` + `tuono*` (wry + tao desktop) |
| `crates/agui-bridge/` | Consomme `mui-rs-components` |
| `crates/mui-rs{,-core,-components,-macros,-motion,-renderer}/` | Renderer MD3 natif (wgpu, vello, winit, wasmtime, parley, fontique) |
| `crates/tuono{,_internal,_lib,_lib_macros}/` | Intégration Next.js SSR (swc_core, lightningcss, mdxjs, napi) |
| `crates/aphrody-x-client/` | Workspace auto-rooté, en attente de `agent-twitter-client` stable |
| `crates/a2a-slimrpc/` | Bloqué upstream `agntcy-slim-mls` (lifetime/async-trait nightly) |

> `crates/coreutils/` et `crates/util-linux/` figurent encore dans le bloc
> `exclude` pour raisons historiques mais n'existent plus sur disque.

## Crates supprimées (historique)

Supprimées du dépôt le 2026-05-21 (cf. la section « historique » de
[`ARCHITECTURE.md`](../ARCHITECTURE.md)) : les 11 `n2b-*`, `bxc-engine`,
`aphrody-xtask`, plus 18 doublons fusionnés dans leurs crates canoniques
(`aphrody-llm-infra`, `aphrody-messaging`, `aphrody-skills`, `aphrody-design`,
`aphrody-voice`, `mrx`) et les orphelins `aphrody-shell` / `aphrody-sandbox`.
Plus tôt (pivot 2026-05-17) : `google_os`, `bun_ffi`, `google_kv`,
`python_ffi`.

## Path dependencies

Tous nos crates membres se réfèrent les uns aux autres via `path = "../crate"` :

```toml
# crates/cli/Cargo.toml
backend = { path = "../backend" }
base    = { path = "../base" }
```

**Roadmap** : passer à `path-bases` (RFC 3529) quand stable Cargo 1.98+
(instable sur nightly 1.97, désactivé pour l'instant).

## Toolchain (`rust-toolchain.toml`)

```toml
[toolchain]
channel    = "nightly"
profile    = "minimal"
components = ["rust-src", "rustfmt", "clippy", "miri",
              "llvm-tools-preview", "rust-analyzer",
              "rustc-codegen-cranelift-preview"]
targets    = [8 targets: Win x64/arm64, Linux x64/arm64,
              macOS x64/arm64, wasm32-unknown, wasm32-wasi]
```

## `.cargo/config.toml`

| Section | Rôle |
|---|---|
| `[build]` | `rustc-wrapper = "sccache"`, `jobs = 8`, default `target = x86_64-pc-windows-msvc`, rustflags nightly (`-Z threads=8`, `-Z share-generics=y`) |
| `[env]` | `SCCACHE_CACHE_SIZE = "30G"`, VS_INSTALL_DIR, WINDOWS_SDK, NASM prebuilt, Ninja generator (tous avec `force = true`) |
| `[target.x86_64-pc-windows-msvc]` | linker MSVC absolu, `target-cpu=x86-64-v3`, hardening (CETCOMPAT, GUARD:CF, NXCOMPAT) |
| `[target.x86_64-unknown-linux-gnu]` | `-fuse-ld=lld`, `--gc-sections`, `stack-protector=strong` |
| `[alias]` | `ci`/`ci-offline`/`ci-frozen`, `xt`/`xt-offline`, `build-dist`, `release-fast`, `build-wasm`, `audit-vet`/`audit-deny`/`audit-machete`/`audit-udeps`, `dev-fast`/`build-fast`/`test-fast` |

> Le déploiement du binaire n'est plus un alias cargo : utiliser
> `scripts/deploy.ps1` (Windows) ou `scripts/deploy.sh` (Linux/macOS).

## Workspace metadata

```toml
[workspace.metadata.aws-lc-sys]
notice = "Forced via cargo update -p aws-lc-sys --precise 0.41.0 if needed."
```
