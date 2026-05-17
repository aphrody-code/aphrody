# aphrody

> **the ultimate cross-platform CLI** — Rust nightly, Edition 2024.
> Cibles prioritaires : **Linux Ubuntu 26.04** > **Windows 11 Insider Canary** > **WebAssembly**.

[![Build](https://github.com/aphrody-code/aphrody/actions/workflows/cross-platform.yml/badge.svg?branch=main)](https://github.com/aphrody-code/aphrody/actions/workflows/cross-platform.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-1.0.0--canary-orange.svg)](docs/PLAN.md)
[![Rust](https://img.shields.io/badge/rust-nightly%201.97%20(edition%202024)-orange.svg)](rust-toolchain.toml)
[![Bun](https://img.shields.io/badge/scripting-Bun%20(no%20node)-black.svg)](https://bun.sh)
[![Supply-chain](https://img.shields.io/badge/supply--chain-cargo--vet%20%2B%20cargo--deny-green.svg)](supply-chain/config.toml)
[![Cross-platform](https://img.shields.io/badge/cross--platform-Linux%20%7C%20Win%20%7C%20wasm-blueviolet.svg)](docs/cargo/CROSS_PLATFORM.md)

---

## Quick start

### Linux Ubuntu 26.04 (cible #1)

```bash
# Pré-requis
sudo apt install -y build-essential pkg-config libssl-dev curl

# Toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly -y
rustup component add clippy rustfmt rust-src

# Clone + build
git clone https://github.com/aphrody-code/aphrody.git && cd aphrody
cargo build --release -p cli
./target/release/aphrody --help
```

### Windows 11 Insider Canary Build (cible #2)

```powershell
# Pré-requis : Visual Studio 2026 Insiders + Windows SDK 26100 + Ninja + NASM
winget configure .config/configuration.winget

# Clone + build
git clone https://github.com/aphrody-code/aphrody.git
cd aphrody
cargo build --release -p cli
.\target\release\aphrody.exe --help
```

### WebAssembly (cible #3)

État réel (matrice 2026-05-17) :

| Crate           | `wasm32-unknown-unknown` (browser) | `wasm32-wasip1` (WASI) |
|-----------------|:----------------------------------:|:----------------------:|
| `base`          | ✅                                 | ✅                     |
| `mrx-core`      | n/a                                | ✅                     |
| `cli` (binary)  | ❌ (tokio "full" + mio)            | ❌                     |
| `backend`/`a2a*`| ❌                                 | ❌                     |

```bash
rustup target add wasm32-unknown-unknown wasm32-wasip1

# Browser-ready (libraries only) :
cargo check -p base --target wasm32-unknown-unknown      # ✅
cargo check -p base --target wasm32-wasip1               # ✅
cargo check -p mrx-core --target wasm32-wasip1           # ✅

# CLI binary wasm port : work-in-progress (see docs/PLAN.md §P-Wasm-CLI).
```

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

1. **Workspace Rust nightly** (`crates/`) — 10 membres : `cli`, `gui`, `backend`,
   `base`, `google_mcp`, `a2a*` (protocole agent-to-agent). Build hermétique
   avec lockfile pin SHA-256.
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
rustup target add wasm32-unknown-unknown wasm32-wasi
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
cargo check -p cli --target x86_64-unknown-linux-gnu     # #1 Linux
cargo check -p cli --target x86_64-pc-windows-msvc       # #2 Windows
cargo check -p cli --target wasm32-unknown-unknown       # #3 wasm — sur ce repo, scope limité à `cli` ;
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

| Section | Contenu |
|---|---|
| [`docs/SOURCE_OF_TRUTH.md`](./docs/SOURCE_OF_TRUTH.md) | **Source de vérité unifiée** (pivot, architecture, plateformes) |
| [`docs/PLAN.md`](./docs/PLAN.md) | Plan stratégique post-pivot |
| [`docs/SUMMARY.md`](./docs/SUMMARY.md) | Sommaire mdBook (architecture, crates, FFI) |
| [`docs/DESIGN.md`](./docs/DESIGN.md) | Décisions d'architecture |
| [`docs/cargo/`](./docs/cargo/) | Workspace, FFI policy, cross-platform |
| [`docs/winget/`](./docs/winget/) | WinGet : catalogue 40+ packages, DSC |
| [`docs/pwsh/`](./docs/pwsh/) | PowerShell 7 : profils, modules |
| [`docs/WASM/`](./docs/WASM/) | Référence WASM (Rust + wgpu + Next.js + Bun) — versions pinned, pièges, migration |
| [`CLAUDE.md`](./CLAUDE.md), [`GEMINI.md`](./GEMINI.md) | Conventions pour agents CLI tiers (formats standards adoptés par les CLIs respectifs) |

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
