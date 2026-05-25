<!-- SPDX-License-Identifier: Apache-2.0 -->
# Crates — 58 membres du workspace

> Réf. : `Cargo.toml` racine, `crates/*/Cargo.toml`.
> Dernière mise à jour : 2026-05-24.

Le workspace compte **58 membres actifs**. Au total **71 crates existent sur
disque** : 58 dans `members`, 14 dans `exclude` (clusters UI/web lourds). Tous
nos crates internes ont `publish = false`.

## Cœur

| Crate (dir) | Package | Rôle |
|---|---|---|
| `cli` | `aphrody` | Binaire principal. `clap` derive, mimalloc, miette. |
| `base` | `base` | Primitives no_std partagées (IDs, erreurs, time). |
| `backend` | `backend` | Forensics, DNS recon, réseau, parser Chromium. |
| `mrx` | `mrx` | Monorepo mapper unifié (ex `mrx-{core,detect,audit,watch,cli}`). |
| `aphrody-ffi` | `aphrody-ffi` | Native C-ABI surface. Load the full CLI in-process from Bun (`bun:ffi`) or any C host. |

## Agent-to-Agent (A2A)

| Crate (dir) | Package | Rôle |
|---|---|---|
| `a2a` | `a2a-lf` | Protocol core (types, A2AError, agent_card). |
| `a2a-client` | `a2a-client-lf` | Client async multi-transport. |
| `a2a-server` | `a2a-server-lf` | Serveur axum + tokio. |
| `a2a-pb` | `a2a-pb` | Types protobuf générés (prost/pbjson). |
| `a2a-grpc` | `a2a-grpc` | Binding gRPC (tonic + tokio-rustls). |
| `a2a-ui` | `a2a-ui` | Viewer WASM des canaux. |
| `google_mcp` | `google_mcp` | Serveur MCP natif ; produit le binaire **`aphrody-mcp`**. |

> Le transport file-based historique (`ai.json` + dossier `ai/`) a été supprimé
> en 2026 ; seul subsiste le miroir de compatibilité `C:\winclean\.coord\`.

## Infrastructure LLM / agent

| Crate | Rôle |
|---|---|
| `aphrody-llm-infra` | Runtime LLM unifié : cost + rateguard + retry + cache (ex `aphrody-{cost,rateguard,retry,cache}`). |
| `aphrody-router` | Routeur LLM, whitelist 3-only (anthropic/gemini/antigravity). |
| `aphrody-providers` | Enum `Provider` canonique + `ProviderError`. |
| `aphrody-gateway` | AI gateway OpenAI-compatible, provider-agnostique. |
| `aphrody-mcp` | Client OAuth 2.1 MCP HTTP/SSE (RFC 9728/8414/7591/7636). |
| `aphrody-mcp-smoke` | Harness smoke E2E pour `aphrody-mcp` (handshake + tools sweep). |
| `aphrody-prompts` | Registre de templates minijinja + scrubber PII. |
| `aphrody-context` | Gestion de la fenêtre de contexte (trim, summarization). |
| `aphrody-session` | Suivi de session (turns, tool calls, tokens, coût). |
| `aphrody-tools` | Registre de tool descriptors (Anthropic/Gemini/MCP). |
| `aphrody-memory` | MemoryBackend async (JSONL/HNSW/SQLite/LanceDB). |
| `aphrody-chat` | REPL turn-loop composant les autres briques. |
| `aphrody-sdk` | SDK public d'embarquement (Agent + Session + Tools). |
| `gemini-runtime` | Adaptateur runtime Gemini CLI (detect + version + stream). |
| `notebooklm` | Client Rust pur NotebookLM Boq RPC. |
| `antigravity-sdk` | Native Rust SDK for Antigravity (Google AI Ultra / Gemini): token extraction from Windows Credential Manager, OAuth refresh, typed HTTP client. |
| `aphrody-embed` | Local, offline text embeddings (fastembed/ONNX Runtime) text vectorisation with no external API. Feeds semantic memory/search. |
| `gemini-web` | Gemini web app (`gemini.google.com`) Boq RPC client: cookie auth, page-bootstrap token scrape, batchexecute envelope codec. Mirrors the `notebooklm` crate transport. |
| `aphrody-agent-home` | The agent's persistent home: Soul / Identity / User / Tools, mmap zero-copy file cache, content-addressed cache, bootstrap budget, system-prompt assembler, hot-reload, git backup. |
| `aphrody-images` | Nano Banana image generation, editing and composition. Async-native, concurrent batch generation, handles URLs and data-URI decoding, typed outputs, atomic writes. |
| `aphrody-firefly` | Pure-Rust Adobe Firefly Services client: IMS OAuth S2S token core, v3 async image generation, job polling and output download. Backs `aphrody firefly` CLI. |

## Skills / marketplace / orchestration

| Crate | Rôle |
|---|---|
| `aphrody-skills` | Infra skills unifiée : runtime + hooks + permissions (ex `aphrody-{skills-runtime,hooks,permissions}`). |
| `aphrody-skills-forge` | Scaffolding/registry/lint des `SKILL.md`. |
| `aphrody-marketplace` | Index skills/MCP/hooks/themes (embedded + file + http). |
| `aphrody-task-runner` | Exécuteur DAG parallèle (topo-sort + timeout/retry). |
| `aphrody-cron` | Scheduler interval/daily/cron avec job store JSON. |
| `aphrody-events` | Bus pub-sub in-process (topic filtering + NDJSON sink). |

## Plateforme / système

| Crate | Rôle |
|---|---|
| `aphrody-secrets` | Secret-store (env + fichier chiffré AES-256-GCM/Argon2id). |
| `aphrody-settings` | Loader JSON hiérarchique (user/project/local/env). |
| `aphrody-telemetry` | Spans/compteurs/histogrammes + exporteur NDJSON. |
| `aphrody-search` | Full-text in-memory (BM25-lite, EN+FR stop words). |
| `aphrody-re` | Reverse engineering (PE/ELF via goblin, entropy, strings). |
| `aphrody-messaging` | Connecteurs sortants + canaux bidirectionnels (ex `aphrody-channels`). |
| `aphrody-voice` | TTS + STT (ElevenLabs/Whisper, ex `aphrody-voice-stt`). |
| `ievr-tools` | Analyse d'inventaire binaire IEVR. |
| `aphrody-translate` | Traduction FR + scrub AI-isms (style Aphrody). |
| `aphrody-summary` | Génère `docs/SUMMARY.md` + `docs/llms.txt`. |
| `aphrody-capture` | Native Windows screen and window capture to PNG (GDI-based, zero unsafe outside FFI wrappers, RAII GDI handle guards). Non-Windows targets compile to stubs. |
| `aphrody-stdio-capture` | Cross-platform in-process stdout/stderr capture (`dup2` on Unix / `SetStdHandle` on Windows to a temp file). Shared by `aphrody-ffi` and Tauri app. |
| `obscura-runtime` | Headless browser façade for scraping (locates `obscura` or `obscura.exe` binary and executes `fetch` / `scrape`). |

## Design / UI / terminal

| Crate | Rôle |
|---|---|
| `aphrody-design` | Infra design unifiée : sidecar + daemon (ex `aphrody-design-{sidecar,daemon}`). |
| `aphrody-design-agents` | Spawner CLI agents (Claude/Gemini/Antigravity, ACP/Stdio). |
| `m3-tokens` | Tokens Material Design 3 (color/typo/elevation/motion). |
| `aphrody-icons` | Font + CSS icônes Material Symbols. |
| `aphrody-react-reconciler` | Reconciler React host-side (primitives). |
| `aphrody-tui` | DSL TUI pur Rust (style ratatui, cible 60 fps). |
| `aphrody-terminal-vt` | Parser VT (22 séquences essentielles). |
| `aphrody-terminal-wasm` | Renderer WASM. |
| `aphrody-terminal-backend` | Backend pty (portable-pty : ConPTY/openpty). |
| `aphrody-terminal-llm` | Bus d'événements LLM↔terminal. |
| `aphrody-terminal-browser` | Bridge browser/agent-browser. |
| `aphrody-terminal-json-out` | Sortie JSON. |
| `aphrody-terminal-markdown` | Rendu markdown inline (comrak). |
| `aphrody-terminal-config` | Config JSON full. |
| `aphrody-logo` | The canonical aphrody character icon (`assets/aphrody.webp`) embedded once, with derivations: multi-resolution `.ico`, scalable `.svg`, and pixel-perfect terminal rendering. |

## WASM

| Crate | Rôle |
|---|---|
| `aphrody-wasm` | Wrapper wasm-bindgen des primitives `base`. |
| (`aphrody-terminal-wasm`, `a2a-ui`) | également ciblés `wasm32-unknown-unknown`. |

## Exclus du workspace (`exclude`, présents sur disque)

Clusters UI/web lourds sortis du build par défaut (perf) ; le binaire `aphrody`
n'en dépend pas. Voir [`WORKSPACE.md`](./WORKSPACE.md) pour la justification.

| Path | Raison |
|---|---|
| `crates/gui/` | Desktop wry + tao ; agrège `mui-rs*` + `tuono*`. |
| `crates/agui-bridge/` | Consomme `mui-rs-components`. |
| `crates/mui-rs{,-core,-components,-macros,-motion,-renderer}/` | Renderer MD3 (wgpu, vello, winit, wasmtime). |
| `crates/tuono{,_internal,_lib,_lib_macros}/` | Next.js SSR (swc_core, lightningcss, mdxjs, napi). |
| `crates/aphrody-x-client/` | Workspace auto-rooté (agent-twitter-client). |
| `crates/a2a-slimrpc/` | Bloqué upstream `agntcy-slim-mls` (nightly). |

> `crates/coreutils/` et `crates/util-linux/` figurent encore dans `exclude`
> mais n'existent plus sur disque.

## Supprimés (historique)

`google_os`, `bun_ffi`, `google_kv`, `python_ffi` (pivot 2026-05-17), puis le
2026-05-21 : les 11 `n2b-*`, `bxc-engine`, `aphrody-xtask`, et 18 doublons
fusionnés dans `aphrody-llm-infra` / `aphrody-messaging` / `aphrody-skills` /
`aphrody-design` / `aphrody-voice` / `mrx` (+ orphelins `aphrody-shell`,
`aphrody-sandbox`).
