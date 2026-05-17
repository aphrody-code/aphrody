<!-- SPDX-License-Identifier: Apache-2.0 -->
# Workspace architecture

> Réf. : `Cargo.toml` racine, `.cargo/config.toml`, `rust-toolchain.toml`.

## Identité

```toml
[workspace]
resolver = "3"        # MSRV-aware resolver (Cargo 1.93+)
members  = [16 crates]
exclude  = [coreutils, util-linux, a2a-slimrpc, vendor]

[workspace.package]
version      = "1.0.0-canary"
edition      = "2024"
rust-version = "1.93"
license      = "Apache-2.0"
```

## Resolver 3 (MSRV-aware)

Cargo 1.93+ supporte `resolver = "3"` qui rend la résolution **MSRV-aware** :
- Préfère les versions de deps compatibles avec notre `rust-version = "1.93"`.
- Change le défaut de `resolver.incompatible-rust-versions` de `allow` → `fallback`.
- Stable depuis Rust 1.84.

## Members (16 crates)

Voir [`CRATES.md`](./CRATES.md) pour la description détaillée.

```text
crates/
├── cli                  ← binary entrypoint
├── gui                  ← wry + tao desktop
├── backend              ← forensics, network, base de connaissances
├── base                 ← no_std primitives, crypto DPAPI
├── bun_ffi              ← zero-copy FFI V8↔Rust
├── google_os            ← noyau hybride NT (kernel, libc, ntdll)
├── google_mcp           ← serveur MCP natif
├── google_kv            ← Deno KV SQLite-backed
├── n2b                  ← natural language → bash
├── python_ffi           ← PyO3 bridge
├── a2a                  ← agent-to-agent core
├── a2a-client           ← async client
├── a2a-server           ← async server
├── a2a-pb               ← protobuf gen
└── a2a-grpc             ← gRPC binding
```

## Exclusions

| Path | Raison |
|---|---|
| `crates/coreutils/` | Userland GNU conservé en référence (pas dans members) |
| `crates/util-linux/` | Idem |
| `crates/a2a-slimrpc/` | Bloqué upstream `agntcy-slim-mls` lifetime/async-trait nightly |
| `vendor/` | Sub-projets externes (`bun`, `electron-prebuilt`, etc.) |

## Path dependencies

Tous nos crates membres se réfèrent les uns aux autres via `path = "../crate"` :

```toml
# crates/cli/Cargo.toml
backend = { path = "../backend" }
base    = { path = "../base" }
n2b     = { path = "../n2b" }
```

**Roadmap** : passer à `path-bases` (RFC 3529) quand stable Cargo 1.98+ :

```toml
# .cargo/config.toml
[path-bases]
monorepo = ""

# crates/cli/Cargo.toml
backend = { base = "monorepo", path = "crates/backend" }
```

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
| `[build]` | `rustc-wrapper = "sccache"`, default `target = x86_64-pc-windows-msvc`, `rustflags` nightly (`-Z threads=8`, `-Z share-generics=y`) |
| `[target.x86_64-pc-windows-msvc]` | MSVC 14.51 linker absolu, `target-cpu=x86-64-v3`, hardening (CETCOMPAT, GUARD:CF, NXCOMPAT) |
| `[target.x86_64-unknown-linux-gnu]` | `-fuse-ld=lld`, `--gc-sections`, `--icf=all`, `stack-protector=strong` |
| `[env]` | VS_INSTALL_DIR, WINDOWS_SDK, NASM prebuilt, Ninja generator (tous avec `force = true`) |
| `[alias]` | `ci-offline`, `xt-offline`, `dist`, `audit-vet`, `audit-deny`, `audit-machete`, `audit-udeps` |

## Workspace metadata

```toml
[workspace.metadata.aws-lc-sys]
notice = "Forced via cargo update -p aws-lc-sys --precise 0.41.0 if needed."
```
