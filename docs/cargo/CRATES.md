<!-- SPDX-License-Identifier: Apache-2.0 -->
# Crates — 16 members du workspace

> Réf. : `Cargo.toml` racine, `crates/*/Cargo.toml`.
> Dernière mise à jour : 2026-05-16.

## Crates de base (`crates/base`, `crates/bun_ffi`)

### `base` — no_std primitives
- Crypto DPAPI (Windows), AES-GCM, zeroize.
- Tracing infrastructure.
- Réutilisable across tous les autres crates.
- **Statut : Stable.**

### `bun_ffi` — Zero-copy FFI V8↔Rust
- `wc_alloc(size)` / `wc_free(ptr, size)` — allocation partagée via mimalloc.
- `[lib] crate-type = ["cdylib", "rlib"]` → produit `bun_ffi.dll` consommable par Bun.
- Garantie zero-copy : ownership transféré entre Rust et V8 via raw pointers encapsulés.
- **Statut : Stable.**

## Noyau (`crates/google_os`)

### `google_os` — Noyau hybride POSIX↔NT
- **Modules** : `kernel/` (process, ipc, ebpf, vfs, io_uring, mman), `libc/` (50+ glibc-spec funcs), `ntdll/`, `firewall`.
- **Bindings** : `windows-rs` v0.61 avec ~30 features Win32 activées (Console, Direct2D, DirectWrite, Dxgi, Direct3D, WinSock, etc.).
- **Build** : `[lib] crate-type = ["cdylib", "rlib"]` → produit `google_os.dll` + librairie Rust.
- **Tests** : rstest + proptest + criterion benchmarks.
- **Action requise** : finir DxEngine, brancher io_uring sur Windows IoRing API.

## CLI / UI

### `cli` — Binary entrypoint
- `clap` derive, mimalloc global, miette pour les erreurs.
- A2A natif : intercepte les prompts NL via `AutoCommand` → streaming zero-buffering.
- `indicatif` + `colored` pour l'UX.
- Path deps : `backend`, `base`, `n2b`, `a2a`, `a2a-client`.
- **Statut : Stable.**

### `gui` — Desktop wry + tao
- `wry = 0.47`, `tao = 0.31` (webview wrapping WebKit).
- `mimalloc` global, lien vers `backend` + `base`.
- **Action requise** : migration GTK4 quand wry 1.0 ships (CVE GTK3 actifs).

### `backend` — Forensics & network
- HTTP via reqwest (rustls-tls + ring).
- Chromium parser (DPAPI decryption via `base`).
- DNS recon, SQLite via rusqlite bundled.
- **Action requise** : suite de tests intégration.

## Agent-to-Agent (A2A)

### `a2a` — Protocol core (`package = "a2a-lf"`)
- A2AError, types métier, agent_card normalization (gRPC URL fix).
- **Statut : Stable.**

### `a2a-client` — Async client (`package = "a2a-client-lf"`)
- Factory pattern pour binding multi-protocols (HTTP, gRPC, SlimRPC).
- Features : `rustls-tls` (ring), `native-tls`.
- Optional dep : `rustls = features = ["ring"]`.
- **Statut : Stable.**

### `a2a-server` — Async server (`package = "a2a-server-lf"`)
- axum + tokio.
- Crypto provider : `rustls::crypto::ring`.
- **Statut : Stable.**

### `a2a-pb` — Protobuf generated types (`package = "a2a-pb"`)
- Code généré par prost-build / pbjson-build.
- `#[allow(clippy::all, warnings)]` sur les modules `proto` / `protojson`.
- **Statut : Stable.**

### `a2a-grpc` — gRPC binding
- Pont tonic + tokio-rustls.
- Crypto provider : `tokio_rustls::rustls::crypto::ring`.
- **Statut : Stable.**

### `a2a-slimrpc` — SlimRPC binding **(exclu actuellement)**
- Bloqué upstream `agntcy-slim-mls` nightly issue.
- Ré-intégration prévue (cf. `docs/PLAN.md` P7).

## Bridges spécialisés

### `google_mcp` — Serveur MCP (Model Context Protocol)
- `rmcp` git dep (modelcontextprotocol/rust-sdk).
- Features : `server`, `transport-io`, `macros`, `schemars`.
- Path deps : `backend`, `base`, `google_os`.
- **Statut : Stable.**

### `google_kv` — Deno KV-compatible, SQLite-backed
- `denokv_sqlite` + `denokv_proto` + `deno_error`.
- `rusqlite` bundled.
- `rand 0.8` (imposé par denokv_proto).
- Path deps vendor : `bun_jsc`, `bun_jsc_macros`.
- **Statut : Stable.**

### `n2b` — Natural language → bash
- `oxc_*` (parser TS), `clap`, `octocrab` (GitHub API), `regex`, `walkdir`.
- `fastembed` optionnel (feature `ai`) — pull native-tls/openssl transitivement (auditté & toléré).
- `reqwest = { ..., features = ["blocking"] }` pour requêtes synchrones.
- **Statut : Stable.**

### `python_ffi` — PyO3 bridge
- `pyo3 = 0.21` avec features `auto-initialize`, `abi3-py311`.
- `[lib] crate-type = ["cdylib", "rlib"]`.
- Path deps vendor : `bun_jsc`, `bun_jsc_macros`.
- **Action requise** : upgrade pyo3 0.22 (CVE PyString 0.21).

## Hors workspace (`exclude`)

| Crate | Localisation | Raison de l'exclusion |
|---|---|---|
| `coreutils` | `crates/coreutils/` | Userland GNU en référence, build via Makefile externe |
| `util-linux` | `crates/util-linux/` | Idem |
| `a2a-slimrpc` | `crates/a2a-slimrpc/` | Bloqué upstream (mls-rs nightly issue) |
| `bun_*` (107 sub-crates) | `vendor/bun/src/*/` | Sub-projet Bun runtime fork, hors workspace mais accessible via path deps |
| `electron-prebuilt` | `vendor/electron-prebuilt/` | Binaires Electron |

## Visibilité publish

Tous nos crates internes ont `publish = false` — empêche un `cargo publish` accidentel et signale à `cargo-deny` qu'ils sont privés (skip license check).
