<!-- SPDX-License-Identifier: Apache-2.0 -->
# Dependencies — 80 centralisées dans `[workspace.dependencies]`

> Réf. : `[workspace.dependencies]` dans `Cargo.toml` racine.

## Stratégie

**100% des deps externes** sont déclarées au workspace level. Chaque crate hérite via `{ workspace = true }`. Versions en **caret MAJOR.MINOR** (pas patch) — le `Cargo.lock` pin SHA-256 garantit la reproductibilité.

```toml
# Cargo.toml root
[workspace.dependencies]
tokio = { version = "1.43", features = ["full"] }

# crates/cli/Cargo.toml
[dependencies]
tokio = { workspace = true }                              # hérite tout
reqwest = { workspace = true, features = ["blocking"] }   # ajoute la feature
```

**Règle :** features additionnelles dans la crate consommatrice sont **additives** au workspace.

## Catalogue (groupé par domaine)

### Async runtime & primitives
- `tokio` (1.43) — runtime async principal
- `tokio-stream`, `tokio-rustls`
- `futures`, `futures-util`
- `async-trait` (0.1) — traits async (max version dispo 0.1.89)
- `auto_impl`, `pin-project-lite`
- `crossbeam`, `crossbeam-channel`, `rayon`

### Allocator & FFI
- `mimalloc` (0.1) — **allocator global obligatoire** (cf. `bun_ffi`)
- `libc`, `bytes`, `memchr`, `memmap2`
- `hashbrown` (0.15) — `default-features = false, features = ["inline-more", "allocator-api2"]`
- `allocator-api2` (polyfill `core::alloc::Allocator`)
- `smallvec`, `bumpalo`, `typed-arena`, `scopeguard`, `self_cell`
- `once_cell`, `foldhash`, `rustc-hash`, `itoa`

### Serde / data formats
- `serde` (1) — features `["derive"]`
- `serde_json`, `serde_yaml`, `toml`, `bstr`
- `hex`, `base64`, `uuid` (v4 + v7), `semver`
- `chrono` (features `["serde"]`), `jiff`, `regex`

### Logging / errors
- `tracing` (0.1)
- `tracing-subscriber` (**pinned `0.3.22`** — 0.3.23+ bug `mod env`)
- `anyhow`, `thiserror` (v2)
- `miette` (7) — features `["fancy"]`

### CLI / TUI
- `clap` (4) — features `["derive", "color", "env"]`
- `clap_complete`, `clap_mangen`
- `indicatif`, `colored`, `crossterm`

### Bitflags / enums
- `bitflags` (2)
- `enum-map`, `enumset`, `strum`, `const_format`
- `phf` — features `["macros"]`

### Filesystem / system
- `walkdir`, `fs_extra`, `globset`, `ignore`
- `tempfile`, `filetime`, `notify`, `which`
- `rustix` — `default-features = false, features = ["std", "fs", "event", "process", "net"]`

### Crypto / security
- `aes-gcm`, `sha2`, `sha3`, `md-5`, `blake3`, `digest`
- `zeroize` (features `["derive"]`)
- `rcgen` — `default-features = false, features = ["crypto", "pem", "ring"]`
- `rustls` (0.23) — `default-features = false, features = ["ring", "std", "tls12"]`
- `rustls-pemfile`
- `getrandom` (0.4)
- **`rand` (0.8)** — forcé à 0.8 par `denokv_proto` (rand_core 0.6)
- `rand_chacha` (0.3)

### HTTP / networking
- `reqwest` (0.12) — `default-features = false, features = ["json", "stream", "charset", "http2", "system-proxy", "rustls-tls"]`
- `http` (1), `hyper` (1)
- `axum` (0.8) — `default-features = false, features = ["json", "query", "tokio", "http1", "http2"]`
- `tower`, `tower-http`
- `dns-lookup`, `hostname`, `url`

### A2A protocol family (internal path deps)
- `a2a` (path = "crates/a2a", package = `a2a-lf`)
- `a2a-client` (path = "crates/a2a-client", package = `a2a-client-lf`, `default-features = false`)
- `a2a-server` (path = "crates/a2a-server", package = `a2a-server-lf`, `default-features = false`)
- `a2a-pb`, `a2a-grpc`, `a2a-slimrpc`
- `slim_bindings` (package = `agntcy-slim-bindings`) — utilisé par a2a-slimrpc (excluded)

### gRPC / protobuf
- `tonic` (0.14) — gRPC framework
- `tonic-build`, `tonic-prost`, `tonic-prost-build`
- `tonic-tls` — `default-features = false`
- `prost`, `prost-types`
- `pbjson`, `pbjson-types`, `pbjson-build`
- `protoc-bin-vendored` (3)

### Database
- `rusqlite` (0.37) — features `["bundled"]`
- `denokv_sqlite`, `denokv_proto`, `deno_error`

### UI / desktop
- `wry` (0.47), `tao` (0.31)

### Python FFI
- `pyo3` (0.21) — features `["auto-initialize", "abi3-py311"]`

### HTML / JS tooling
- `scraper`
- `oxc_allocator`, `oxc_ast`, `oxc_ast_visit`, `oxc_parser`, `oxc_span` (0.126)
- `schemars`, `ts-rs`, `similar`, `octocrab`

### MCP (Model Context Protocol)
- `rmcp` — git dep `https://github.com/modelcontextprotocol/rust-sdk.git`
  - Features : `["server", "transport-io", "macros", "schemars"]`

### Terminal
- `vte` (0.14) — VT escape sequence parser

### Windows bindings
- `windows` (0.61) — features de base, étendues per-crate (ex. `google_os`)
- `windows-sys` (0.59)
- `winapi-util`

### Test / bench / fuzz
- `rstest`, `proptest`, `criterion`, `pretty_assertions`

### Misc
- `itertools` (0.10), `nix` (0.30)

## Pourquoi caret MAJOR.MINOR ?

| Trade-off | Choix |
|---|---|
| **Strict** (`= "1.0.219"`) | Pin exact mais bloque tout patch upstream |
| **Caret minor** (`"1.0"`) | Permet patches automatiques (recommandé) |
| **Caret major** (`"1"`) | Trop large, peut break sur minor breaking |
| **Wildcard** (`"*"`) | **INTERDIT** par `cargo deny` (`bans.wildcards = "deny"`) |

Le **`Cargo.lock`** est la source de vérité reproductible (SHA-256 par crate).

## Ajouter une nouvelle dep


Workflow synthétique :
1. Ajouter dans `[workspace.dependencies]` du root.
2. Dans la crate consommatrice : `dep = { workspace = true }`.
3. `cargo update -p <new-dep>` puis `cargo deny check` (CVE + licence).
4. Si licence ou CVE pose problème : audit + justification dans `deny.toml` ou `supply-chain/`.
