# aphrody

> **Cross-platform Rust CLI that ships the same command surface to Linux,
> Windows, and the browser (wasm32-unknown-unknown).** Built on a hermetic
> Google-grade supply chain — `cargo-vet` feeds from Google, Mozilla, Fuchsia,
> ChromeOS, Bytecode Alliance, Embark, Zcash — with byte-reproducible builds.
>
> Cibles prioritaires : **Linux Ubuntu 26.04** > **Windows 11 Insider Canary** > **WebAssembly**.
> Rust nightly, Edition 2024.

[![Build](https://github.com/aphrody-code/aphrody/actions/workflows/cross-platform.yml/badge.svg?branch=main)](https://github.com/aphrody-code/aphrody/actions/workflows/cross-platform.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-1.0.0--canary-orange.svg)](docs/PLAN.md)
[![Rust](https://img.shields.io/badge/rust-nightly%201.97%20(edition%202024)-orange.svg)](rust-toolchain.toml)
[![Bun](https://img.shields.io/badge/scripting-Bun%20(no%20node)-black.svg)](https://bun.sh)
[![Supply-chain](https://img.shields.io/badge/supply--chain-cargo--vet%20%2B%20cargo--deny-green.svg)](supply-chain/config.toml)
[![Cross-platform](https://img.shields.io/badge/cross--platform-Linux%20%7C%20Win%20%7C%20wasm-blueviolet.svg)](docs/cargo/CROSS_PLATFORM.md)

> 📝 Latest deep-dive: [**ai.json — a file-based A2A manifest two Claude Codes used to coordinate across repos**](docs/posts/2026-05-ai-json.md) (2 000 mots, vraies traces, schéma + listener Bun, AGNTCY a2a/v0.4 compatible).

---

## Demo — `mrx scan` on this repo (real run, no edit)

```console
$ aphrody --version
aphrody 1.0.0-canary

$ mrx scan --root .
real    0m0.055s     # 55 ms wall-clock (Windows 11, cold disk cache)

$ jq '.stats' monorepo-map.json
{
  "total_files":      119,
  "total_workspaces":   6,
  "bytes_scanned":  16437373,
  "scan_duration_ms":  14,        // 14 ms internal walk (rayon + ignore)
  "languages": {
    "TypeScript": { "files": 30, "bytes":  81554 },
    "JSON":       { "files": 16, "bytes":  11297 },
    "Markdown":   { "files":  5, "bytes":   7460 },
    "CSS":        { "files":  1, "bytes":   3519 }
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

That's the binary on its own repo. On a real monorepo (19,213 files / 482 MB)
the same binary runs in **1.4 s warm** — full numbers + comparisons in
[`BENCHMARKS.md`](BENCHMARKS.md).

> Latest changes: [`CHANGELOG.md`](CHANGELOG.md) — Unreleased section tracks all of this session's shipped work.

---

## Install (60 secondes)

```bash
# Linux + macOS
curl -sSf https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.sh | sh

# Windows (PowerShell 7+)
irm https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.ps1 | iex

# Scoop (Windows)         scoop bucket add aphrody https://github.com/aphrody-code/scoop-bucket && scoop install aphrody
# Homebrew (mac + Linux)  brew install aphrody-code/tap/aphrody
```

Les binaires GitHub Releases sont vérifiés SHA-256, statiquement liés
(musl sur Linux, MSVC CRT-static sur Windows), embarquent un SBOM
`cargo-auditable`, et couvrent Linux x64/ARM64, Windows x64/ARM64, macOS x64/ARM64.

---

## Quick start (build from source)

### Linux Ubuntu 26.04 (cible #1)

```bash
# Pré-requis
sudo apt install -y build-essential pkg-config libssl-dev curl

# Toolchain
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
winget configure .config/configuration.winget

# Clone + build
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
| `mrx-core`      | n/a                                | ✅                     |
| `cli` (binary)  | ✅ (stub : `--version` / `--help`) | ✅ (stub)              |
| `backend`/`a2a*`| ❌ (tokio "full" + mio + reqwest)  | ❌                     |

```bash
rustup target add wasm32-unknown-unknown wasm32-wasip1

# Browser-ready :
cargo check -p base --target wasm32-unknown-unknown      # ✅
cargo check -p aphrody  --target wasm32-unknown-unknown      # ✅ (stub binaire)

# WASI :
cargo check -p base     --target wasm32-wasip1           # ✅
cargo check -p mrx-core --target wasm32-wasip1           # ✅
cargo check -p aphrody      --target wasm32-wasip1           # ✅ (stub binaire)
```

Le binaire `cli` se compile sur `wasm32-*` mais en *stub minimal* : il
n'expose que `--version` et `--help` ; les commandes OS-bound (`auth`,
`chromium`, `dns`, …) renvoient un message « pas disponible sur wasm »
et redirigent vers le binaire natif. Cf. `crates/cli/src/main.rs`
pour les `cfg(not(target_arch = "wasm32"))` qui isolent `mimalloc`,
`tokio` *full*, `reqwest`, `rustls`, `backend` et `a2a-client`.

Démo wasmtime (binaire 334 KiB, release LTO) :

```bash
$ wasmtime target/wasm32-wasip1/release/aphrody.wasm --version
aphrody 1.0.0-canary

$ wasmtime target/wasm32-wasip1/release/aphrody.wasm --help
Aphrody — cross-platform Rust binary (Linux / Windows / macOS / wasm).

Usage: aphrody.wasm [COMMAND]

Commands:
  auth        Authentification Google (God Mode / OAuth2)
  mirror      Gère le mirroring des assets MD3
  dns         Résolution DNS OSINT (reconnaissance agressive)
  version     Affiche la version et l'état du système
  chromium    Forensics Chromium
  a2a         Client natif A2A
  ...
```

## Highlight — `mrx` Monorepo Real-time X-platform Mapper

Un binaire Rust autonome (`mrx`) qui scanne **n'importe quel monorepo**
(Bun, pnpm, Turborepo, Cargo, Lerna, Nx, Deno, Yarn, npm) et émet :

- `path.json` — audit de path hardening (chemins absolus, fragiles, system).
- `monorepo-map.json` — carte content-addressed (blake3) avec runtimes,
  lockfiles, workspaces, langages détectés, stats par workspace.

Walker parallèle basé sur `ignore` (le moteur de ripgrep) + agrégation
rayon. **19 213 fichiers / 482 MB scannés en 1,4 s warm** sur un vrai
monorepo polyglotte (mesures reproductibles dans
[`BENCHMARKS.md`](BENCHMARKS.md)). Serverless-friendly (Lambda / Cloud
Run). Trois sous-commandes :

```bash
mrx scan  --root .   # one-shot audit + map, exits
mrx watch --root .   # daemon notify, debounced (1500 ms par défaut)
mrx check --root .   # comme scan, exit non-zéro si findings → gate CI
```

Output canonique (extrait, host scan du repo aphrody) :

```json
{
  "stats": {
    "total_files": 119,
    "total_workspaces": 6,
    "scan_duration_ms": 47,
    "languages": { "TypeScript": { "files": 30, "bytes": 81554 }, ... }
  },
  "root_kind": {
    "task_runners": ["turbo"],
    "package_managers": ["bun"],
    "lockfiles": ["bun.lock", "Cargo.lock"],
    "has_cargo_workspace": true,
    "has_bun_workspaces": true
  },
  "content_hash": "5d0c9ea6ef74263b924280eaffcdf8a7e00048ee8439a60279200e3056941839"
}
```

Le hash `content_hash` (blake3) couvre les fichiers de configuration
racine (`turbo.json`, `package.json`, lockfiles, `Cargo.toml`,
`.gitmodules`, …) — même invariant que Turborepo : si ces fichiers
changent, le cache aval est bust.

## Stack 2026

| Couche | Tech |
|---|---|
| **Systems** | Rust nightly 1.97 + **Edition 2024**, `windows-rs` (Win) / `libc` + `nix` (Linux) / `wasm-bindgen` (wasm) |
| **Runtime** | **mimalloc** global allocator, `tokio` portable, `io_uring` (Linux) / IOCP (Win) |
| **Scripting** | **Bun** (TypeScript / MCP / tooling — **node interdit**) |
| **Build** | sparse registry, sccache, `cargo zigbuild` cross-compile, `cargo-auditable` SBOM |
| **Supply-chain** | `cargo deny` + `cargo vet` (feeds Google / Mozilla / Fuchsia / ChromeOS) |

---

## Ce que ce projet est aujourd'hui

Un workspace Rust nightly cross-platform centré sur **un binaire `aphrody`
distribuable sur Linux/Windows/wasm** :

1. **Workspace Rust nightly** (`crates/`) — 17 membres : `cli`, `gui`, `backend`,
   `base`, `google_mcp`, `a2a` × 5 (protocole agent-to-agent : `a2a`, `a2a-client`,
   `a2a-server`, `a2a-grpc`, `a2a-pb`), `mrx-*` × 5 (Monorepo Real-time X-platform
   mapper : `mrx-core`, `mrx-detect`, `mrx-audit`, `mrx-watch`, `mrx-cli`),
   `aphrody-translate`, `aphrody-wasm`. Build hermétique avec lockfile pin SHA-256.
2. **Bun + TypeScript** (`packages/`, `bun.lock`) — scripting, FFI bridge,
   serveurs MCP. **node interdit** : tout passe par bun.
3. **Bridges natifs vendored** (`vendor/bun/`, `vendor/electron-prebuilt/`,
   `vendor/coreutils/`, `vendor/util-linux/`) — sub-projets externes
   conservés en read-only, hors workspace members.

## Ce qu'il deviendra

- **Le binaire `aphrody` distribué nativement sur Linux/Windows/wasm**, via les
  aliases `cargo build-linux-x64` / `cargo build-win-x64` / `cargo build-wasm`.
- **a2a / google_mcp** : adaptés cross-platform pur, retrait des dépendances
  Windows-only.
- **CI Linux first** : `ubuntu-26.04` runner dès disponibilité, sinon
  `ubuntu-latest`.
- **Distribution** : crates.io (`aphrody`), Homebrew (`brew install aphrody`),
  scoop (Windows), `apt`/`deb` PPA, packages wasm sur npm.
- **path-bases (RFC 3529)** activé workspace-wide quand stable Cargo 1.98+.

## Pré-requis détaillés

### Linux Ubuntu 26.04

| Outil | Version | Rôle |
|---|---|---|
| **Rust** nightly | 1.97 | Workspace (`rust-toolchain.toml`) |
| **gcc / clang** | latest | Compilateur C pour deps natives |
| **pkg-config** | latest | Discovery de deps système |
| **libssl-dev** | latest | OpenSSL (alternative à aws-lc-sys) |
| **Bun** | latest | CLI TypeScript, FFI bridge |
| **sccache** | 0.15+ | Cache compilation partagé |

### Windows 11 Insider Canary

| Outil | Version | Rôle |
|---|---|---|
| **Rust** nightly | 1.97 | Workspace |
| **MSVC** (VS 2026 Insiders) | 14.51.36231 | Compilateur C/C++ |
| **Windows SDK** | 10.0.26100.0+ | Headers / libs Win32 |
| **LLVM/Clang** | 22.1+ | Linker `lld-link` |
| **CMake** + **Ninja** | latest | Build natif (aws-lc-sys) |
| **NASM** | latest | aws-lc-sys prebuilt asm |
| **Bun** | latest | CLI TypeScript |

Installation automatique : `winget configure .config/configuration.winget`.

### WebAssembly

```bash
rustup target add wasm32-unknown-unknown wasm32-wasip1
cargo install wasm-bindgen-cli wasm-pack
```

## Build

```bash
# --- Dev (rapide, debug) -------------------------------------------------
cargo check  --workspace --locked
cargo build  --workspace --locked

# --- Release distribuable ------------------------------------------------
cargo build  --workspace --locked --profile dist          # LTO fat, strip, panic=abort
cargo build  --workspace --locked --profile release-fast  # LTO thin, codegen-units=16

# --- Validation CI (hermétique) ------------------------------------------
cargo ci-offline     # = clippy --workspace --all-targets --locked --offline -- -D warnings
cargo xt-offline     # = nextest run --workspace --locked --offline

# --- Cross-platform (les 3 cibles prioritaires) --------------------------
cargo check -p aphrody --target x86_64-unknown-linux-gnu     # #1 Linux
cargo check -p aphrody --target x86_64-pc-windows-msvc       # #2 Windows
cargo check -p aphrody --target wasm32-unknown-unknown       # #3 wasm — sur ce repo, scope limité à `cli` ;
                                                          #         périmètre étendu réservé aux 3 forks
                                                          #         `aphrody-code/{next.js, ui, A2UI}` (cf. docs/WASM/).

# --- Supply-chain audits -------------------------------------------------
cargo deny check     # CVE + licences + bans + sources
cargo vet            # audits signés
cargo audit-machete  # unused dependencies
```

## Structure du dépôt

| Chemin | Rôle |
|---|---|
| `crates/cli` | **Binaire `aphrody`** — cross-platform pur |
| `crates/base` | Primitives no_std partagées |
| `crates/backend` | Forensics + network (cross-platform) |
| `crates/gui` | wry + tao desktop GUI (exclu de `cli`) |
| `crates/a2a*` | Protocole agent-to-agent |
| `crates/google_mcp` | Serveur MCP (en cours d'adaptation cross-platform) |
| `crates/mrx-*` | Monorepo Real-time X-platform mapper (5 crates : core / detect / audit / watch / cli) |
| `crates/aphrody-translate` | CLI traduction commentaires EN→FR + scrub AI/émoji + style Aphrody |
| `packages/` | Bun TypeScript packages |
| `docs/` | Documentation mdBook centralisée |
| `scripts/` | Tooling Bun + PowerShell |
| `supply-chain/` | `cargo-vet` audits |
| `deny.toml` | `cargo-deny` policy |
| `Cargo.toml` (root) | Workspace manifest |
| `.cargo/config.toml` | Aliases, MSVC linker, NASM/Ninja env |
| `rust-toolchain.toml` | Nightly + components + targets |
| `docs/SOURCE_OF_TRUTH.md` | **Vue d'ensemble consolidée** |

## Documentation

### Overview
- [SOURCE_OF_TRUTH.md](./docs/SOURCE_OF_TRUTH.md) — executive overview
- [ARCHITECTURE.md](./docs/ARCHITECTURE.md) — workspace layers + dep graph
- [COMPARISON.md](./docs/COMPARISON.md) — aphrody vs just/taskfile/gh/devcontainer/asdf
- [FAQ.md](./docs/FAQ.md) — anticipated questions
- [ROADMAP.md](./docs/ROADMAP.md) — Q2 2026 → Q1 2027 targets
- [BENCHMARKS.md](./BENCHMARKS.md) — mrx scan + criterion micro-benches
- [PLAN.md](./docs/PLAN.md) — plan stratégique post-pivot
- [DESIGN.md](./docs/DESIGN.md) — décisions d'architecture
- [SUMMARY.md](./docs/SUMMARY.md) — sommaire mdBook
- [docs/cargo/](./docs/cargo/) — workspace, FFI policy, cross-platform
- [docs/WASM/](./docs/WASM/) — référence WASM (Rust + wgpu + Next.js + Bun)
- [docs/winget/](./docs/winget/) — WinGet : catalogue 40+ packages, DSC
- [docs/pwsh/](./docs/pwsh/) — PowerShell 7 : profils, modules

### Technical posts
- [A2A cross-Claude coordination](./docs/posts/2026-05-ai-json.md)
- [Parallel YOLO grind loop](./docs/posts/2026-05-yolo-grind-loop.md)

### Architecture decisions
- [ADR-0000 template](./docs/adr/0000-template.md)
- [ADR-0001 cross-platform Rust](./docs/adr/0001-cross-platform-rust.md)
- [ADR-0002 A2A file-based](./docs/adr/0002-a2a-file-based.md)
- [ADR-0003 YOLO parallel grind](./docs/adr/0003-yolo-parallel-grind.md)

### A2A protocol extensions
- [Extensions index](./docs/extensions/index.md)
- [file-transport/v1](./docs/extensions/file-transport-v1.md)
- [honest-delivery/v1](./docs/extensions/honest-delivery-v1.md)
- [context7-version-pinning/v1](./docs/extensions/context7-version-pinning-v1.md)

### IEVR reverse-engineering
- [IEVR docs index](./docs/ievr/INDEX.md) — full doc map
- [CHANGELOG](./docs/ievr/CHANGELOG.md) — reverse-engineering progress log
- [Asset formats](./docs/ievr/asset-formats.md) — CPK / USM / ADX / HCA entry-point
- [Asset classification pipeline](./docs/ievr/asset-classification-pipeline.md)
- [Audio pipeline](./docs/ievr/audio-pipeline.md)
- [Network protocol notes](./docs/ievr/network-protocol-notes.md)
- [Scripting VM notes](./docs/ievr/scripting-vm-notes.md)
- [Text extraction strategy](./docs/ievr/text-extraction-strategy.md)
- [Binaries inventory](./docs/ievr/binaries-inventory.md)

### Live demos
- [WASM browser playground](./crates/aphrody-wasm/examples/browser-playground.html) — open in-browser, no install

### OSS hygiene
- [CONTRIBUTING.md](./CONTRIBUTING.md)
- [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- [SECURITY.md](./SECURITY.md) — vulnerability reporting
- [CHANGELOG.md](./CHANGELOG.md)
- [CLAUDE.md](./CLAUDE.md), [GEMINI.md](./GEMINI.md) — conventions pour agents CLI tiers

### Launch material (pre-publication)
- [SHOW-HN.md](./docs/launch/SHOW-HN.md) — title candidates + comment templates

### Internal audits (transparency)
- [2026-05-17 mrx aggressive](./docs/audits/2026-05-17-mrx-aggressive.md)
- [2026-05-17 n2b scan](./docs/audits/2026-05-17-n2b-scan.md)
- [2026-05-17 bxc scrape request](./docs/audits/2026-05-17-bxc-scrape-request.md)

## Security

Vulnerabilities should be reported privately. See [`SECURITY.md`](./SECURITY.md)
for supported versions, the GitHub Security Advisory channel, the
`security@aphrody.dev` mailbox, the 48 h / 30 d response window, scope, safe
harbor, and acknowledgement policy. Credited reporters are listed in
[`SECURITY-HALL-OF-FAME.md`](./SECURITY-HALL-OF-FAME.md).

## Standards organisationnels

Les défauts community-health (issue/PR templates, dependabot, workflows, CODEOWNERS) sont fournis org-wide par [`aphrody-code/.github`](https://github.com/aphrody-code/.github) et appliqués à tous les repos `aphrody-code/*` via le mécanisme de fallback GitHub.

Bootstrap d'un nouveau repo :

```bash
cd <new-repo>
curl -sSL https://raw.githubusercontent.com/aphrody-code/.github/main/scripts/bootstrap.sh | bash
```

## Supply-chain (Google-grade 2026)

Build hermétique sans vendoring source (`Cargo.lock` SHA-256 + `cargo-vet`) :

1. **`Cargo.lock`** commit → pin SHA-256 de chaque crate. Reproductibilité.
2. **Sparse registry** (`.cargo/config.toml`) → 10-100× plus rapide que git.
3. **cargo-vet** (`supply-chain/`) → audits signés importés depuis Google,
   Mozilla, Fuchsia, ChromeOS, Bytecode Alliance, Embark, Zcash.
4. **cargo-deny** (`deny.toml`) → CVE RustSec DB + licences + bans + sources.
5. **CI** : `cargo ci-offline` → `--locked --offline -D warnings` (zéro réseau).

## Contribuer

Lire dans l'ordre :

1. [`docs/SOURCE_OF_TRUTH.md`](./docs/SOURCE_OF_TRUTH.md) — vue d'ensemble.
2. [`CLAUDE.md`](./CLAUDE.md) + [`GEMINI.md`](./GEMINI.md) — directives langages.
3. [`docs/PLAN.md`](./docs/PLAN.md) — chantiers ouverts.
4. **Avant push** : `cargo ci-offline && cargo deny check` doit être vert
   **sur Linux d'abord**.

---

*Licence Apache 2.0 — voir [LICENSE](./LICENSE).*
