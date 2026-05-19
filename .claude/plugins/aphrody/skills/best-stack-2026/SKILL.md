---
name: best-stack-2026
description: "Canonical Rust 2026 stack chooser — recommends the right crate (+ exact version + alternative rejected + reason) for any domain : HTTP servers, async runtimes, databases, crypto, parsing, CLI, terminal UIs, WASM, GUI, observability, ML. Use this skill *aggressively* — whenever the user asks 'which crate', 'best library for X', 'recommend a stack', 'should I use Y or Z', 'how do I do some-thing in Rust', is about to write or edit any Cargo.toml, mentions adding a dependency, or hesitates between several libraries. Also use proactively before suggesting any crate when discussing architecture, even if the user did not ask explicitly. The recommendations are fact-checked against awesome-rust top-ranked sources, RUSTSEC advisories, OSI license safety (no GPL contamination of Apache-2.0 aphrody), and last-commit recency (active maintenance). Refuses deprecated stacks (openssl 0.9, actix-web 4.x for new code, tokio 0.2, hyper 0.14 raw, native-tls on Linux servers, GPL crypto wrappers)."
license: Apache-2.0
metadata:
  version: 1.0.0
  source: aphrody-marketplace::awesome (curator) + awesome-rust 50k★ + RUSTSEC + crates.io 2026-05-19
---

# Best Rust stack — May 2026 canonical chooser

When a user asks "what should I use for X in Rust?", do not improvise. Do not
default to the first crate that comes to mind. Consult this table, pick the
canonical choice, and **explain *why* the alternative was rejected**. Naming
the rejected alternative is half the value — it tells the user you considered
the obvious option and ruled it out for a reason.

## When to engage this skill

- Direct question: "best crate for X", "which library", "should I use Y or Z"
- Architecture discussion that will result in a dependency choice
- About to edit a `Cargo.toml` (add, bump, replace a dep)
- User wrote "let's use openssl" / "let's use actix" / any other phrase
  matching the [§ Anti-patterns](#anti-patterns) table — flag it, recommend
  the modern equivalent, but do not block (advisory per project policy).

## Why a curated stack matters

aphrody is **Apache-2.0** and ships across Linux (priority #1), Windows, and
wasm32. Three failure modes the canonical stack protects against:

1. **License contamination** — GPL/AGPL crates virally infect Apache-2.0
   binaries at link time. Apex offenders: `unicorn-engine 2.x` (CPU emu),
   `radare2-rs`, certain reverse-engineering wrappers.
2. **Cross-platform breakage** — `native-tls` on Linux pulls OpenSSL system
   libs that drift across distros; `tao/wry` pull GTK3 (CVE-tagged). Anything
   that breaks on Linux #1 is non-mergeable (cf. CLAUDE.md §6).
3. **Bit-rot** — popular ≠ maintained. `actix-web 4.x` keeps working but
   maintenance velocity has dropped; `tokio 0.2`/`0.3` is fossil. Recency
   matters more than star count past ~10k★.

For toolchain / language features (Rust 1.95, Edition 2024 idioms, async
closures, CVE-2026-33056, WASM 1.96 breakage), consult the sibling skill
`rust-best-practices-2026` — that skill covers *how* to write the code; this
one covers *what to depend on*.

---

## Canonical stack table

The **Pick** column is the recommendation. The **Reject** column is what NOT
to use (or what the user might reasonably suggest first). The **Why** is the
half-sentence that lets the user accept the trade-off.

### Web / HTTP / API

| Domain | Pick (version) | Reject | Why |
|---|---|---|---|
| HTTP server | **`axum` 0.8** | `actix-web` 4.x ; `warp` ; raw `hyper` | tokio/tower native, lowest friction, biggest 2026 mindshare |
| HTTP client | **`reqwest` 0.12** with `rustls-tls` feature | `surf` ; `isahc` ; `curl` crate | de facto standard, rustls-tls works on Linux+WASM |
| HTTP/3, QUIC | **`quinn` 0.11** | `quiche` ; raw `s2n-quic` | pure-Rust, Apache-2.0, integrates with tokio + rustls |
| WebSocket | **`tokio-tungstenite` 0.24** | `ws-rs` (unmaintained) | tokio native, autobahn-clean |
| gRPC | **`tonic` 0.13** | `grpc-rs` (C++ FFI) | pure-Rust, hyper-based, prost codegen |
| GraphQL | **`async-graphql` 7.x** | `juniper` | schema-first, subscriptions, federation v2 |
| OpenAPI codegen | **`utoipa` 5.x** (server) ; **`progenitor`** (client) | `openapi-generator-cli` (Java) | native Rust, derive macros |
| Routing helpers | (use axum directly) | `actix-router` standalone | axum's router is sufficient and faster |

### Async runtime

| Domain | Pick | Reject | Why |
|---|---|---|---|
| Runtime | **`tokio` 1.x** with `rt-multi-thread` + `macros` features | `async-std` (unmaintained) ; `smol` (niche) ; `embassy` (embedded only) | 99% of the ecosystem standardised |
| Spawning sets | **`tokio::task::JoinSet`** | manual `Vec<JoinHandle>` | cancellation-safe, easier abort |
| Utilities | **`futures` 0.3** + **`tokio-stream` 0.1** | `futures-lite` for prod | full Stream/Sink, well-known |
| Channels | **`tokio::sync::{mpsc,oneshot,broadcast,watch}`** | `flume` for ergonomics ; `crossbeam-channel` for sync-only | first-party, integrates with select! |
| Sync primitives | **`parking_lot`** for `Mutex`/`RwLock` non-async ; `tokio::sync::*` for async | `std::sync::Mutex` for new code | parking_lot is faster + smaller, std works but legacy reflex |

### Database / persistence

| Domain | Pick | Reject | Why |
|---|---|---|---|
| Postgres + compile-time SQL check | **`sqlx` 0.8** with `postgres,runtime-tokio,tls-rustls,macros` | `diesel` for greenfield ; `tokio-postgres` raw | `query!` macro verifies SQL at compile time |
| Postgres + ORM | **`sea-orm` 1.x** (only if you really want an ORM) | `diesel` (sync-blocking model) | async, dynamic queries, migrations |
| Migrations | **`sqlx-cli`** | `refinery` ; `barrel` | matches sqlx schema, ergonomic |
| Embedded KV | **`redb` 2.x** for pure-Rust ; **`sled` 0.34** for write-heavy (note: maintained-mode) | `lmdb-rkv` (unmaintained) ; `rocksdb` (heavy C++ dep) | pure Rust, mmap, ACID |
| SQLite | **`rusqlite` 0.32** or **`sqlx` 0.8 sqlite feature** | `diesel` sqlite | bundled feature avoids system libsqlite version drift |
| Redis | **`fred` 10.x** (production async) ; **`redis-rs` 0.27** (simpler) | `darkredis` (stale) | fred handles cluster + sentinel + reconnect natively |
| Time-series / OLAP | **`duckdb-rs`** for in-process analytics | spawning Python | embedded, columnar, vectorised |

### Crypto / TLS

| Domain | Pick | Reject | Why |
|---|---|---|---|
| TLS stack | **`rustls` 0.23** with `aws-lc-rs` provider | `native-tls` (OpenSSL) ; `openssl` 0.10 | pure-Rust on Linux+WASM, no system dep drift |
| Hash | **`sha2` 0.10** (general) ; **`blake3` 1.5** (perf-critical) | OpenSSL hashes | RustCrypto, audited, AVX-512 in blake3 |
| AEAD | **`aes-gcm` 0.10** ; **`chacha20poly1305` 0.10** | OpenSSL EVP | RustCrypto traits, constant-time |
| Asymmetric | **`ed25519-dalek` 2.x** ; **`x25519-dalek` 2.x** | `ring` directly for new code | dalek is audited and ergonomic |
| Password hash | **`argon2` 0.5** | `bcrypt` for new schemes | OWASP 2026 recommendation |
| Random | **`rand` 0.8** (note: pinned by `denokv_proto` per CLAUDE.md §7) | `oorandom` for crypto | rand 0.8 is workspace-pinned |
| JWT | **`jsonwebtoken` 9.x** | hand-rolled | maintained, supports all common algs |

### Serialisation / parsing

| Domain | Pick | Reject | Why |
|---|---|---|---|
| Schema-typed | **`serde` 1.x** + **`serde_json` 1.x** | hand-rolled JSON | universal |
| YAML | **`serde_yaml_ng` 0.10** (fork of unmaintained serde_yaml) | `serde_yaml` 0.9 | original archived 2024, fork is the maintained branch |
| TOML | **`toml` 0.8** with `serde` feature | `toml_edit` unless you need format preservation | `toml_edit` for round-tripping Cargo.toml, `toml` for plain parse |
| Binary (compact) | **`bincode` 2.x** | `rmp-serde` for cross-lang | binary serde with checked encoding |
| CBOR | **`ciborium` 0.2** | `serde_cbor` (archived) | RustCrypto-stewarded |
| Protocol Buffers | **`prost` 0.13** | `rust-protobuf` (slow) | tonic ecosystem aligned |
| MessagePack | **`rmp-serde` 1.x** | `msgpack` (stale) | maintained, serde compat |
| ELF/PE/Mach-O | **`goblin` 0.9** | `object` raw (lower level) ; `elf` crate | reads all formats, idiomatic |
| Decompilation/disasm | **`iced-x86`** (x86) ; **`capstone` 0.13** (multi-arch) | `unicorn-engine` (GPL!) | capstone is Apache-2.0; iced-x86 fast x86-only |
| Compression | **`zstd` 0.13** ; **`flate2` 1.x** (gzip) ; **`lz4_flex`** (LZ4) | C bindings only when no pure-Rust available | pure-Rust where possible |
| Archive | **`tar` 0.4.45+** (CVE-2026-33056) ; **`zip` 2.x** | older tar | post-CVE versions |

### CLI / config / observability

| Domain | Pick | Reject | Why |
|---|---|---|---|
| Args | **`clap` 4.x** with `derive` feature | `structopt` (merged into clap) ; `argh` for tiny | derive macros, completions, manpage gen |
| Config files | **`figment` 0.10** | hand-rolled merge | layered (file + env + CLI) |
| Env vars | **`envy`** for simple ; **`figment::providers::Env`** for complex | `dotenv` (loading only, not parsing) | typed deserialization |
| Errors (apps) | **`anyhow` 1.x** | `eyre` for richer reports | thread-safe, no_std-friendly variant exists |
| Errors (libs) | **`thiserror` 2.x** | hand-rolled `impl Error` | derive macros, fewer footguns |
| Logging | **`tracing` 0.1** + **`tracing-subscriber` 0.3.22** (pinned per CLAUDE.md §7) | `log` for new apps | spans, structured, OTel integration |
| OpenTelemetry | **`opentelemetry` 0.27** + **`tracing-opentelemetry`** | Jaeger client raw | vendor-neutral, OTel collector compat |
| Metrics | **`metrics` 0.23** + **`metrics-exporter-prometheus`** | `prometheus` crate directly | facade pattern, swappable backend |

### Terminal / TUI / interactive

| Domain | Pick | Reject | Why |
|---|---|---|---|
| TUI framework | **`ratatui` 0.29** | `cursive` ; `tui-rs` (archived predecessor) | active, theme-friendly, OSC-aware |
| Terminal I/O | **`crossterm` 0.28** | `termion` (Unix-only) ; `console` | cross-platform Linux+Windows+WASM stub |
| ANSI parsing | **`vte` 0.13** | hand-rolled state machine | alacritty-grade parser |
| Progress bars | **`indicatif` 0.17** | `tqdm` ports | de facto, multi-bar |
| REPL / prompt | **`reedline` 0.36** | `rustyline` (older) | nu-shell stewarded, modern |
| Shell colors | **`owo-colors` 4.x** | `colored` (alloc-heavier) | const styles, no alloc per call |

### GUI / desktop / web rendering

| Domain | Pick | Reject | Why |
|---|---|---|---|
| Native window | **`winit` 0.30** | `glutin` directly ; `tao` (uses GTK3 on Linux) | active, raw_window_handle support |
| GPU rendering | **`wgpu` 25** | OpenGL/Vulkan direct (verbose) ; `tao+wry` | Linux+Windows+WASM webgpu single API |
| Egui (immediate-mode GUI) | **`egui` 0.30** + `eframe` | `iced` for retained-mode | iterate fast, ships everywhere |
| Iced (Elm-like GUI) | **`iced` 0.13** | egui if you need retained-mode | architecture clean, harder ramp |
| Web (WASM) | **`leptos` 0.7** OR **`dioxus` 0.6** | `yew` (slower SSR story) | fine-grained reactivity, no virtual-DOM tax |
| Embedded webview (last resort) | (avoid — use WASM instead) | `wry` (GTK3 CVE on Linux) | the project policy is WASM-first (CLAUDE.md §2) |

### ML / AI / vector

| Domain | Pick | Reject | Why |
|---|---|---|---|
| Inference (CPU) | **`candle` 0.8** | `tch-rs` (libtorch dep) ; ONNX runtime FFI | pure Rust, HF-stewarded |
| Embeddings | **`fastembed` 4.x** | `sentence-transformers` Python | ONNX backed, fast cold start |
| Vector DB | **`qdrant-client` 1.x** ; **`lancedb` 0.13** embedded | `pinecone` (proprietary SaaS) | open-source, self-host |
| Tokenizer | **`tiktoken-rs`** for OpenAI ; **`tokenizers` 0.20** for HF | hand-rolled BPE | upstream-parity |
| Tensor primitive | (use `candle` tensors) | `ndarray` for non-ML | candle has GPU ops; ndarray is CPU-only |

### Networking / IPC / RPC

| Domain | Pick | Reject | Why |
|---|---|---|---|
| Async DNS | **`hickory-resolver` 0.24** | `trust-dns` (renamed) ; system resolver only | hickory = rename of trust-dns, current upstream |
| Raw sockets | **`socket2` 0.5** | `nix::sys::socket` directly | cross-platform shim |
| MQTT | **`rumqttc` 0.24** | `paho-mqtt` (C++ FFI) | pure-Rust, async |
| gRPC reflection | **`tonic-reflection` 0.13** | hand-rolled | matches tonic |
| Postcard (no_std RPC) | **`postcard` 1.x** | bincode in no_std contexts | zero-copy, no_std |

### Reverse engineering / binary analysis (aphrody-specific)

| Domain | Pick | Reject | Why |
|---|---|---|---|
| PE/ELF parser | **`goblin` 0.9** | `object` (lower-level) | covers PE+ELF+Mach-O+TE |
| PDB symbols | **`pdb` 0.8** | manual MSF parse | mature, used by rust-analyzer |
| Disassembly | **`iced-x86` 1.21** (x86 only) ; **`capstone` 0.13** (multi) | `unicorn-engine` (GPL!) | both Apache-2.0; emu separately |
| Memory scan | **`memchr` 2.x** + **`aho-corasick` 1.x** | hand-rolled SIMD | SIMD-optimised, well-tested |
| Hashing files | **`blake3` 1.5** (preferred) or `sha2` 0.10 | OpenSSL | blake3 SIMD + parallel |
| Entropy / strings | (custom — see `crates/aphrody-re`) | — | already in-tree |

### Testing / quality

| Domain | Pick | Reject | Why |
|---|---|---|---|
| Runner | **`cargo-nextest`** | `cargo test` for big suites | parallel, isolation, retries |
| Snapshot | **`insta` 1.40** | hand-rolled diff | inline review, cargo-insta tool |
| Property | **`proptest` 1.5** | `quickcheck` (less ergonomic) | shrinking, derive support |
| Mock | **`mockall` 0.13** | `mockito` for HTTP only | trait-aware codegen |
| Bench | **`criterion` 0.5** | `cargo bench` raw | statistical analysis, regressions |
| Coverage | **`cargo-llvm-cov`** | `tarpaulin` (slower) | LLVM source-based |
| Fuzzing | **`cargo-fuzz`** (libFuzzer) ; **`afl.rs`** (AFL++) | hand-rolled | tie into Rust UB sanitisers |

---

## Anti-patterns (refuse politely, propose alternative)

If the user names one of these, do not silently accept. Surface the issue,
state the modern equivalent, let them override consciously.

| User says | Issue | Recommend instead |
|---|---|---|
| `openssl = "0.9"` or `0.10` | OpenSSL system-lib drift across distros + history of CVEs ; native-tls pulls it | **`rustls` 0.23** + `aws-lc-rs` or `ring` provider |
| `actix-web` for greenfield | Maintenance velocity dropped vs axum ; less aligned with tokio ecosystem | **`axum` 0.8** |
| `tokio` 0.2 / 0.3 | Pre-1.0 fossil, no `JoinSet`, no current API | **`tokio` 1.x** with rt-multi-thread |
| `hyper` 0.14 raw | Low level for app code ; 1.x has different API | **`axum` 0.8** (servers) or **`reqwest` 0.12** (clients) |
| `async-std` | Project effectively unmaintained since 2023 | **`tokio` 1.x** |
| `tao` + `wry` on Linux | Pulls GTK3 → RUSTSEC GTK CVEs (cf. CLAUDE.md §7) ; webview heavy | **WASM via wasm-bindgen** or **`egui`/`iced`** native |
| `unicorn-engine` | **GPL-2.0** virally infects Apache-2.0 binary | **`iced-x86`** (analysis) + separate emulation strategy |
| `radare2-rs` | GPL transitive | **`goblin` + capstone + custom analysis** |
| `serde_yaml` 0.9 | Upstream archived September 2024 | **`serde_yaml_ng` 0.10** (drop-in fork) |
| `chrono` 0.4 with `default-features = true` | Old default pulls oldtime CVE + bloat | **`chrono` 0.4** with `default-features = false`, `["clock", "serde"]` — or **`jiff` 0.1** for new code |
| `time` 0.1 / 0.2 | Pre-1.0, no tz | **`time` 0.3** or **`jiff` 0.1** |
| `failure` / `error-chain` | Both archived | **`anyhow` + `thiserror`** |
| `rand` 0.7 / 0.9 | Workspace pins 0.8 (CLAUDE.md §7) | **`rand` 0.8** |
| `tracing-subscriber` 0.3.23+ | Known packaging bug (CLAUDE.md §7) | **`tracing-subscriber` 0.3.22** |
| `native-tls` on Linux | OpenSSL system dep, cross-platform pain | **`rustls` + `rustls-tls` reqwest feature** |
| `dotenv` for parsing | Loading only ; not typed | **`figment`** or **`envy`** |
| `lazy_static` for new code | std has it now | **`std::sync::LazyLock`** (1.80+) |
| `once_cell::sync::Lazy` for new code | std has it now | **`std::sync::LazyLock`** |
| `error-chain` style chained errors | Pre-`?` operator era | **`anyhow::Context::with_context`** |

---

## How to answer a "best crate for X" question

Apply this template — short, decisive, citation-light. Avoid hedging.

```text
For X in Rust 2026: use **{crate} {version}** ({short rationale}).

Reject: {alternative} — {one-line reason}.

Snippet:
```rust
{minimal `Cargo.toml` snippet}
{optional minimal usage example, ~5 lines}
```

Cross-platform: {if applicable, note Linux/Windows/WASM caveats}.
License: {SPDX} (compatible Apache-2.0 aphrody).
```

If the user's question covers **multiple domains** (e.g. "build a REST API
with auth and Postgres"), produce one block per domain, ordered by build-up
dependency (runtime → server → db → auth → observability).

If the user explicitly **insists** on a rejected stack ("I want actix because
my team knows it"), accept the choice but record the trade-off briefly:
*"Fine, actix-web 4 is still production-grade; the maintenance gap with
axum's velocity is the watchpoint. Pin to 4.9.x and re-evaluate in 6mo."*

---

## Integration with aphrody-marketplace::awesome

When the workspace has run `aphrody marketplace awesome manifest`, a curated
JSON whitelist sits at `var/data/awesome/stack-whitelist.json` (cf.
`aphrody_marketplace::awesome::to_stack_policy`). Prefer that file as the
**dynamic source of truth** — it reflects the most recent fact-checked
top-of-awesome-rust snapshot. The table above is the **embedded fallback**
when the manifest is not yet generated (first-run, hermetic CI).

Both sources are consistent by design — the table is hand-curated from the
same top-200 awesome-rust sources the curator scrapes.

For deeper notes on why each Pick won (RUSTSEC history, license audit,
benchmark deltas), see `references/anti-patterns.md` and the live data in
`var/data/awesome/ranked.json`.

---

## Output discipline

- **Always** name the version range (`"0.8"`, not just `axum`).
- **Always** explain the rejected alternative in one line.
- **Never** recommend a crate from the [§ Anti-patterns](#anti-patterns)
  table without flagging it.
- **Refuse** GPL / AGPL crates outright when the user's project is
  Apache-2.0 / MIT — state the contamination risk in one sentence.
- **Defer** to `rust-best-practices-2026` skill for language/toolchain
  guidance (Edition 2024, async closures, WASM 1.96 break, CVE-2026-33056).
