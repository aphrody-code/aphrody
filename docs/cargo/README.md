<!-- SPDX-License-Identifier: Apache-2.0 -->
# Cargo Workspace — Documentation

Documentation complète du workspace Rust de Aphrody.
Dernière mise à jour : 2026-05-16.

## Quick links

| Page | Contenu |
|---|---|
| [`WORKSPACE.md`](./WORKSPACE.md) | Architecture du workspace (resolver, members, exclude, metadata) |
| [`CRATES.md`](./CRATES.md) | Liste des 58 crates membres, rôle et statut |
| [`PROFILES.md`](./PROFILES.md) | 6 profils : `dev`, `release`, `dist`, `release-fast`, `release-debug`, `bench` |
| [`LINTS.md`](./LINTS.md) | Policy `[workspace.lints]` rust/rustdoc/clippy |
| [`DEPENDENCIES.md`](./DEPENDENCIES.md) | 80 deps centralisées dans `[workspace.dependencies]` |
| [`SUPPLY_CHAIN.md`](./SUPPLY_CHAIN.md) | `cargo-vet` + `cargo-deny` workflow Google-grade |
| [`FFI_POLICY.md`](./FFI_POLICY.md) | Règles FFI Rust ↔ C/C++ ↔ Bun, zero-copy, mimalloc |
| [`MIGRATION.md`](./MIGRATION.md) | Migration C++ → Rust (incrémentale, par sous-système) |
| [`CROSS_PLATFORM.md`](./CROSS_PLATFORM.md) | Binaire `cli` cross-platform Win/Linux/macOS/wasm, `platform.rs`, aliases multi-target, zigbuild |
| [`CHROMIUM_ANDROID_PATTERNS.md`](./CHROMIUM_ANDROID_PATTERNS.md) | Adoption des patterns AOSP `rust_*` + Chromium `cxx`/`rust_static_library` |
| [`ANDROID_TARGET.md`](./ANDROID_TARGET.md) | Build Android via cargo-ndk : 4 targets (aarch64/armv7/x86_64/i686), JNI, CI |
| [`GOOGLE_MODE.md`](./GOOGLE_MODE.md) | **Matrice complète 31/34 patterns Google Production-grade adoptés** |
| [`SKILLS.md`](./SKILLS.md) | Inventaire centralisé des skills (`.claude/skills/`), format SKILL.md, intégration `skill` Rust + `vercel-labs/agent-skills` |
| [`CHEATSHEET.md`](./CHEATSHEET.md) | Commandes cargo essentielles + alias CI custom |

## Vue d'ensemble

Le workspace Aphrody suit les **best practices Cargo 2026** :

```
┌──────────────────────────────────────────────────────────────────┐
│  Cargo workspace (root Cargo.toml)                               │
│  ────────────────────────────────                                │
│   resolver = "3"   edition = "2024"   rust-version = "1.97"      │
│   [workspace.package]    — métadonnées héritées                  │
│   [workspace.dependencies] — 80 deps centralisées (caret minor)  │
│   [workspace.lints]      — policy stricte avec assouplissements  │
│                            ciblés pour FFI/kernel/bridge code    │
│   [profile.dist]         — LTO fat + strip + panic=abort         │
│   [profile.release-fast] — LTO thin + codegen-units=16 (CI)      │
│                                                                  │
│   members  : 58 crates (cli, backend, mrx, ...)                  │
│   exclude  : coreutils, util-linux, a2a-slimrpc, vendor          │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ├── crates/                  (58 active members)
                              ├── supply-chain/            (cargo-vet)
                              ├── deny.toml                (cargo-deny)
                              ├── rust-toolchain.toml      (nightly 2026-05-17)
                              └── .cargo/config.toml       (MSVC + alias)
```

## Validation rapide

```bash
cargo check  --workspace --locked          # type-check
cargo ci-offline                           # clippy + -D warnings, hermetic
cargo deny check                           # CVE + licences + bans + sources
cargo vet                                  # audits signés Google/Mozilla/...
cargo xt-offline                           # nextest tous targets
```

## Build artefacts distribuables

```bash
cargo build --profile dist --workspace --locked       # release LTO fat
cargo build --profile release-fast --workspace        # CI rapide LTO thin
cargo build --profile release-debug --workspace       # release + line-tables-only (profiling)
```
