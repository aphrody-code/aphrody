<!-- SPDX-License-Identifier: Apache-2.0 -->
<p align="center"><img src="assets/aphrody.webp" alt="aphrody" width="200"></p>

# aphrody

> **Self-hosted Rust agent platform inspired by OpenAI Codex:** a local-model
> RAG engine with OCR, web server, CLI/TUI, GUI surfaces, and a persistent
> agent home (`SOUL.md`, persona, memory, and tools). It ships the same
> command surface to Linux, Windows, and the browser (`wasm32-unknown-unknown`).
> The default policy is proactive full-auto execution with concise output and
> no API key requirement; `.aphrody/settings.json` is the project baseline.
> Built on a hermetic,
> lockfile-only supply chain — `cargo-deny` + `cargo-vet` audits from Google,
> Mozilla, Fuchsia, ChromeOS, Bytecode Alliance, Embark, Zcash.
>
> Cibles prioritaires : **Linux Ubuntu 26.04** > **Windows 11 Insider Canary** > **WebAssembly**.
> Rust nightly, Edition 2024.

[![Build](https://github.com/aphrody-code/aphrody/actions/workflows/cross-platform.yml/badge.svg?branch=main)](https://github.com/aphrody-code/aphrody/actions/workflows/cross-platform.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-1.0.0--canary-orange.svg)](docs/PLAN.md)
[![Rust](https://img.shields.io/badge/rust-nightly%20(edition%202024)-orange.svg)](rust-toolchain.toml)
[![X](https://img.shields.io/badge/X-%40aphrody__code-black.svg?logo=x)](https://x.com/aphrody_code)
[![Supply-chain](https://img.shields.io/badge/supply--chain-cargo--deny%20%2B%20cargo--vet-green.svg)](docs/cargo/SUPPLY_CHAIN.md)
[![Cross-platform](https://img.shields.io/badge/cross--platform-Linux%20%7C%20Win%20%7C%20wasm-blueviolet.svg)](docs/cargo/CROSS_PLATFORM.md)

---

## Ce que c'est

### App-server Codex local

Les surfaces GUI, TUI et CLI peuvent partager le même serveur agentique :

```bash
# JSON-RPC NDJSON sur stdin/stdout
aphrody session serve

# API HTTP locale pour une GUI ou un client distant de la même machine
aphrody session serve --http --addr 127.0.0.1:8789
```

Le protocole expose `initialize`, `thread/start`, `thread/list`, `turn/start`,
`tools/list` et `shutdown`. Comme Codex, un client doit appeler `initialize`,
puis envoyer la notification `initialized`, avant toute opération de thread.
Les modèles, le RAG, l'OCR et le home agent restent
locaux ; aucune API key n'est nécessaire.

Pour consommer un turn progressivement, passez `waitForCompletion:false` à
`turn/start`, puis interrogez `turn/events` avec le `threadId`. Sans cette
option, `turn/start` conserve le mode synchrone historique et renvoie le
résultat final avec les événements accumulés.

Les modèles locaux sont guidés par défaut vers une boucle d'analyse structurée :
exploration du dépôt, extraction de données, récupération RAG/mémoire, OCR et
computer-use/browser lorsqu'ils sont disponibles, puis vérification et synthèse.

Un **monorepo polyglotte avec Rust comme langage primaire**. Le cœur — le
binaire `aphrody`, le serveur MCP `aphrody-mcp`, le scanner `mrx`, le protocole
agent-to-agent `a2a-*` — est **100 % Rust** et se distribue sur
Linux/Windows/wasm. Autour gravitent deux surfaces de première classe :

| Surface | Emplacement | Toolchain | Rôle |
|---|---|---|---|
| **Rust** (primaire) | `crates/*` (~70 crates) | `rust-toolchain.toml` | CLI, systems, FFI, MCP, A2A |
| **Bun / TypeScript** | `packages/*`, `examples/*` | `mise.toml` | Bibliothèques UI Material Design 3 (`@aphrody/*`) |
| **Python** | `py/` | `.python-version` (uv) | ML / data / bridges |

> Le **cœur Rust ne dépend d'aucune autre toolchain au runtime.** Bun et
> Python sont confinés aux surfaces où ils dominent (UI web, ML). Voir
> [`CLAUDE.md`](CLAUDE.md) §2 pour la politique de langages complète.

---

## Demo — `mrx scan` on this repo (real run, no edit)

```console
$ aphrody --version
aphrody 1.0.0-canary

$ mrx scan --root .
# writes <root>/path.json + <root>/monorepo-map.json, then exits

$ jq '.stats' monorepo-map.json     # real run on this repo, 2026-06-04
{
  "total_files":     1513,
  "total_workspaces":  12,
  "scan_duration_ms":  47,          // walk parallèle (rayon + ignore)
  "bytes_scanned": 8435119,
  "languages": {
    "CSS":        { "files": 695, "bytes": 2326717 },
    "TypeScript": { "files": 673, "bytes": 2446884 },
    "Markdown":   { "files":  88, "bytes": 1185790 },
    "JSON":       { "files":  21, "bytes":  108686 }
  }
}

$ jq '.root_kind' monorepo-map.json
{
  "task_runners":     ["turbo"],
  "package_managers": ["bun"],
  "lockfiles":        ["bun.lock", "Cargo.lock"],
  "has_cargo_workspace": true,
  "has_bun_workspaces":  true
}
```

C'est le binaire `mrx` sur son propre dépôt. Recettes de benchmark
reproductibles dans [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) et ledger de
régression dans [`docs/PERFORMANCE-HISTORY.md`](docs/PERFORMANCE-HISTORY.md).

> Latest changes: [`CHANGELOG.md`](CHANGELOG.md).

### Agent stack on this VPS (Claude · Grok · Gemini · bxc · aphrody)

**Deploy:** [`DEPLOY.md`](DEPLOY.md) (Rust CLI, MCP, A2A, Python `:8082`) · bxc [`../bxc/DEPLOY.md`](../bxc/DEPLOY.md) · quick memory [`docs/agent-stack/DEPLOY.md`](docs/agent-stack/DEPLOY.md).

Shared MCP: `~/.config/aphrody/mcp.json` (`aphrody-mcp`, `bxc-mcp`). Setup: [`docs/MCP_SETUP.md`](docs/MCP_SETUP.md).

---

## Install

### Gestionnaire des CLI officiels

Une installation d'Aphrody peut gérer tout l'écosystème sans connaître les
commandes Cargo ou Bun propres à chaque dépôt :

```bash
aphrody package catalog
aphrody package install n2b
aphrody package update all
aphrody package status
```

Voir [`docs/PACKAGES.md`](docs/PACKAGES.md) pour le cycle complet, les chemins
multiplateformes et la désinstallation sécurisée.

> **État (2026-06-04).** Les canaux packagés (`.deb`, Snap, AUR, Flatpak, Nix,
> Scoop, Winget, Homebrew, npm wasm) sont **prévus mais pas encore publiés** :
> le répertoire `packaging/` et les one-liners `install.sh` / `install.ps1`
> n'existent pas encore. Le **seul chemin garanti aujourd'hui est « depuis les
> sources »** (ci-dessous). Suivre [`docs/INSTALL.md`](docs/INSTALL.md) et
> [`docs/PLAN.md`](docs/PLAN.md) pour l'avancement des canaux packagés.

### Depuis les sources (chemin garanti)

```bash
git clone https://github.com/aphrody-code/aphrody.git && cd aphrody
cargo build --release -p aphrody
./target/release/aphrody --help

# Ou : build + install des binaires (aphrody, mrx, aphrody-mcp) dans ~/.local/bin
./scripts/deploy.sh                 # Linux / macOS  (--dry-run pour prévisualiser)
.\scripts\deploy.ps1                # Windows PowerShell 7+
```

---

## Quick start (build from source)

### Linux Ubuntu 26.04 (cible #1)

```bash
# Pré-requis
sudo apt install -y build-essential pkg-config libssl-dev curl

# Toolchain (le pin exact vit dans rust-toolchain.toml)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly -y
rustup component add clippy rustfmt rust-src

# Clone + build
git clone https://github.com/aphrody-code/aphrody.git && cd aphrody
cargo build --release -p aphrody
./target/release/aphrody --help
```

### Windows 11 Insider Canary Build (cible #2)

```powershell
# Pré-requis : Visual Studio 2026 Insiders + Windows SDK 26100 + Ninja + NASM
git clone https://github.com/aphrody-code/aphrody.git
cd aphrody
cargo build --release -p aphrody
.\target\release\aphrody.exe --help
```

### WebAssembly (cible #3)

État réel (matrice 2026-05-17) :

| Crate           | `wasm32-unknown-unknown` (browser) | `wasm32-wasip1` (WASI) |
|-----------------|:----------------------------------:|:----------------------:|
| `base`          | ✅                                 | ✅                     |
| `mrx`           | n/a                                | ✅                     |
| `aphrody` (bin) | ✅ (stub : `--version` / `--help`) | ✅ (stub)              |
| `backend`/`a2a*`| ❌ (tokio "full" + mio + reqwest)  | ❌                     |

```bash
rustup target add wasm32-unknown-unknown wasm32-wasip1

cargo check -p base    --target wasm32-unknown-unknown      # ✅
cargo check -p aphrody --target wasm32-unknown-unknown      # ✅ (stub binaire)
cargo check -p base    --target wasm32-wasip1               # ✅
cargo check -p mrx     --target wasm32-wasip1               # ✅
cargo check -p aphrody --target wasm32-wasip1               # ✅ (stub binaire)
```

Le binaire `aphrody` se compile sur `wasm32-*` en *stub minimal* : il n'expose
que `--version` et `--help` ; les commandes OS-bound renvoient un message « pas
disponible sur wasm » et redirigent vers le binaire natif. Cf.
`crates/cli/src/main.rs` pour les `cfg(not(target_arch = "wasm32"))` qui isolent
`mimalloc`, `tokio` *full*, `reqwest`, `rustls`, `backend` et `a2a-client`.

## Highlight — `mrx` Monorepo Real-time X-platform Mapper

Un binaire Rust autonome (`mrx`) qui scanne **n'importe quel monorepo** (Bun,
pnpm, Turborepo, Cargo, Lerna, Nx, Deno, Yarn, npm) et émet :

- `path.json` — audit de path hardening (chemins absolus, fragiles, système).
- `monorepo-map.json` — carte content-addressed (blake3) avec runtimes,
  lockfiles, workspaces, langages détectés, stats par workspace.

Walker parallèle basé sur `ignore` (le moteur de ripgrep) + agrégation rayon.
Serverless-friendly (Lambda / Cloud Run). Trois sous-commandes :

```bash
mrx scan  --root .   # one-shot audit + map, exits
mrx watch --root .   # daemon notify, debounced (1500 ms par défaut)
mrx check --root .   # comme scan, exit non-zéro si findings → gate CI
```

Le hash `content_hash` (blake3) couvre les fichiers de configuration racine
(`turbo.json`, `package.json`, lockfiles, `Cargo.toml`, `.gitmodules`, …) —
même invariant que Turborepo : si ces fichiers changent, le cache aval est bust.

## Stack 2026

| Couche | Tech |
|---|---|
| **Systems** | Rust nightly + **Edition 2024**, `windows-rs` (Win) / `libc` + `nix` (Linux) / `wasm-bindgen` (wasm) |
| **Runtime** | **mimalloc** global allocator, `tokio` portable, `io_uring` (Linux) / IOCP (Win) |
| **Surfaces** | **Rust** primaire ; **Bun/TypeScript** (UI Material Design 3) ; **Python** (ML/data) — cf. [`CLAUDE.md`](CLAUDE.md) §2 |
| **Build** | sparse registry, sccache, `cargo zigbuild` cross-compile, `cargo-auditable` SBOM |
| **Supply-chain** | `cargo deny` + `cargo vet` (feeds Google / Mozilla / Fuchsia / ChromeOS) |

---

## Frontend — Material Design 3 (monorepo TypeScript)

Bun + Turborepo workspace (la surface UI du dépôt, séparée du cœur Rust). Les
bibliothèques sont publiées sur **npm** (`registry.npmjs.org`) sous le scope
`@aphrody/*` (tag `m3-v*` → `.github/workflows/release-m3-packages.yml`).

```
packages/
  material-web/     @aphrody/material-web    # lib Lit (web components <md-*>), self-contained sur --md-sys-*
  react/            @aphrody/m3-react         # wrappers React (@lit/react), 1 par <md-*>
  m3-tokens/        @aphrody/m3-tokens        # tokens M3 + Material You runtime (dynamic-color)
  m3-motion/        @aphrody/m3-motion        # transitions / motion M3 (React)
  m3-theme/         @aphrody/m3-theme         # tokens « fusion » M3 + shadcn/ui + Tailwind v4
  m3-design/        @aphrody/m3-design        # design compiler : brief NL → scaffold React M3
  eslint-plugin-m3/ @aphrody/eslint-plugin-m3 # règles lint M3 (oxlint + ESLint)
  doc-ai/           @aphrody/doc-ai           # CLI doc/traduction
  bun-rs/           @aphrody/bun-rs           # FFI native Rust (Sass, HCT) via bun:ffi (hors workspace Cargo)
crates/
  aphrody-site/     # origine HTTP axum/tokio de la page blanche aphrody.com
examples/
  showcase/         # galerie m3-react + Material You, 100 % Bun
```

```bash
bun install          # racine — lie le workspace, applique les patches (MCU 0.4.0, @webgpu/types)
bun run build        # turbo : build des @aphrody/*
bun run typecheck    # turbo tsc
cargo run -p aphrody-site     # origine Rust aphrody.com sur 127.0.0.1:8083
```

## Roadmap 2026

- **Deployment distribution** : GitHub Releases (Linux x64/ARM64, Windows x64/ARM64, macOS x64/ARM64), Homebrew, Scoop, apt/deb — *en cours, voir [`docs/PLAN.md`](docs/PLAN.md)*.
- **crates.io publication** : `aphrody` + SDK public une fois `base`/`backend` stabilisés.
- **CI Linux-first** : validation primaire sur Ubuntu 26.04 (cible #1).
- Roadmap publique complète : [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Build & Deploy

Canonical VPS guide: **[`DEPLOY.md`](DEPLOY.md)** (`vps-deploy-bxc-aphrody.sh`, Linux `config.linux-vps.toml`, A2A, systemd). Unified sync: `bash scripts/vps-sync-agent-stack.sh`.

### Development & CI validation

```bash
# --- Dev (rapide, debug) -------------------------------------------------
cargo check  --workspace --locked
cargo build  --workspace --locked

# --- Validation CI (hermétique) ------------------------------------------
cargo ci-offline     # = clippy --workspace --all-targets --locked --offline -- -D warnings
cargo xt-offline     # = nextest run --workspace --locked --offline

# --- Cross-platform (les 3 cibles prioritaires) --------------------------
cargo check -p aphrody --target x86_64-unknown-linux-gnu     # #1 Linux
cargo check -p aphrody --target x86_64-pc-windows-msvc       # #2 Windows
cargo check -p aphrody --target wasm32-unknown-unknown       # #3 wasm

# --- Supply-chain audits -------------------------------------------------
cargo deny check     # CVE + licences + bans + sources
cargo vet            # audits signés
cargo audit-machete  # unused dependencies
```

### Building specific binaries

```bash
cargo build --release --locked -p aphrody           # CLI main binary
cargo build --release --locked --bin aphrody-mcp    # MCP server (from google_mcp crate)
cargo build --release --locked -p mrx               # Monorepo scanner
```

## Structure du dépôt

| Chemin | Rôle |
|---|---|
| `crates/cli` | **Binaire `aphrody`** — cross-platform pur |
| `crates/base` | Primitives no_std partagées |
| `crates/backend` | Forensics + network (cross-platform) |
| `crates/a2a*` | Protocole agent-to-agent |
| `crates/google_mcp` | Serveur MCP (binaire `aphrody-mcp`) |
| `crates/mrx` | Monorepo Real-time X-platform mapper (binaire `mrx`) |
| `crates/aphrody-re` | Reverse engineering (triage / disasm / strings / sections / yara) |
| `crates/aphrody-translate` | CLI traduction commentaires + scrub AI/émoji |
| `packages/`, `apps/`, `examples/` | Surface TypeScript / Material Design 3 (Bun + Turborepo) |
| `py/` | Surface Python (uv / ruff / pytest) |
| `docs/` | Documentation centralisée |
| `scripts/` | Deploy & build automation |
| `deny.toml` | `cargo-deny` policy |
| `Cargo.toml` (root) | Workspace manifest |
| `rust-toolchain.toml` | Nightly + components + targets |
| `docs/SOURCE_OF_TRUTH.md` | **Vue d'ensemble consolidée** |

## Documentation

### Overview
- [SOURCE_OF_TRUTH.md](./docs/SOURCE_OF_TRUTH.md) — executive overview
- [ARCHITECTURE.md](./docs/ARCHITECTURE.md) — workspace layers + dep graph
- [DESIGN.md](./DESIGN.md) — décisions d'architecture / produit
- [COMPARISON.md](./docs/COMPARISON.md) — aphrody vs just/taskfile/gh/devcontainer/asdf
- [FAQ.md](./docs/FAQ.md) — anticipated questions
- [ROADMAP.md](./docs/ROADMAP.md) — public roadmap
- [PLAN.md](./docs/PLAN.md) — plan stratégique
- [PLAN-MOONSHOT.md](./docs/PLAN-MOONSHOT.md) — 30-day star-maximisation plan
- [PERFORMANCE.md](./docs/PERFORMANCE.md) — bench recipes + numbers
- [SUMMARY.md](./docs/SUMMARY.md) — sommaire mdBook
- [INDEX.md](./docs/INDEX.md) — master index of all docs

### Guides
- [INSTALL.md](./docs/INSTALL.md) — installation paths (source + planned channels)
- [MIGRATION.md](./docs/MIGRATION.md) — adoption path from just/taskfile/gh
- [EXAMPLES.md](./docs/EXAMPLES.md) — recipe collection
- [MCP_SETUP.md](./docs/MCP_SETUP.md) — MCP server wiring
- [UI-ARCHITECTURE.md](./docs/UI-ARCHITECTURE.md) — terminal & UI architecture
- [STACK.md](./docs/STACK.md) — full toolchain stack

### Cargo / workspace
- [docs/cargo/](./docs/cargo/) — workspace, FFI policy, cross-platform
- [docs/cargo/CRATES.md](./docs/cargo/CRATES.md) — per-crate inventory
- [docs/cargo/CROSS_PLATFORM.md](./docs/cargo/CROSS_PLATFORM.md) — multi-target strategy
- [docs/cargo/SUPPLY_CHAIN.md](./docs/cargo/SUPPLY_CHAIN.md) — cargo-vet / cargo-deny details
- [docs/cargo/BUILD-SPEED.md](./docs/cargo/BUILD-SPEED.md) — build-speed tactics
- [docs/cargo/PIPELINE-OPTIMIZATION.md](./docs/cargo/PIPELINE-OPTIMIZATION.md) — pipeline optimization

### Protocol & security
- [PROTOCOL.md](./docs/PROTOCOL.md) — a2a/v0.4 protocol reference
- [SECURITY-MODEL.md](./docs/SECURITY-MODEL.md) — threat model + trust boundaries
- [PRIVACY.md](./docs/PRIVACY.md) — telemetry-zero policy
- [CI-CD.md](./docs/CI-CD.md) — GitHub Actions workflows overview

### OSS hygiene
- [CONTRIBUTING.md](./CONTRIBUTING.md)
- [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- [SECURITY.md](./SECURITY.md) — vulnerability reporting
- [CHANGELOG.md](./CHANGELOG.md)
- [COMMUNITY.md](./docs/COMMUNITY.md)
- [CLAUDE.md](./CLAUDE.md) — conventions pour agents CLI

## Security

Vulnerabilities should be reported privately. See [`SECURITY.md`](./SECURITY.md)
for supported versions, the GitHub Security Advisory channel, the response
window, scope, and safe harbor.

## Supply-chain (Google-grade 2026)

Build hermétique sans vendoring source (`Cargo.lock` SHA-256 + `cargo-vet`) :

1. **`Cargo.lock`** commit → pin SHA-256 de chaque crate. Reproductibilité.
2. **Sparse registry** (`.cargo/config.toml`) → 10-100× plus rapide que git.
3. **cargo-vet** → audits signés importés depuis Google, Mozilla, Fuchsia, ChromeOS, Bytecode Alliance, Embark, Zcash.
4. **cargo-deny** ([`deny.toml`](deny.toml)) → CVE RustSec DB + licences + bans + sources.
5. **CI** : `cargo ci-offline` → `--locked --offline -D warnings` (zéro réseau).

Détails : [`docs/cargo/SUPPLY_CHAIN.md`](docs/cargo/SUPPLY_CHAIN.md).

## Contribuer

Lire dans l'ordre :

1. [`docs/SOURCE_OF_TRUTH.md`](./docs/SOURCE_OF_TRUTH.md) — vue d'ensemble.
2. [`CLAUDE.md`](./CLAUDE.md) — directives langages + conventions.
3. [`docs/PLAN.md`](./docs/PLAN.md) — chantiers ouverts.
4. **Avant push** : `cargo ci-offline && cargo deny check` doit être vert **sur Linux d'abord**.

---

*Licence Apache 2.0 — voir [LICENSE](./LICENSE).*
