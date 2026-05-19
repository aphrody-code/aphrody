# Anti-pattern reference — why each rejected stack was rejected

This file expands the one-line rationales in `SKILL.md`'s anti-patterns table
into full justifications. Consult when the user pushes back on a rejection
("why not openssl? everyone uses it") and you need the longer story.

## openssl (0.9 / 0.10)

**Issue 1 — system dep drift.** `openssl-sys` links against the OpenSSL
package installed on the build host. Ubuntu 20.04 ships 1.1.1, 22.04 ships
3.0, 24.04 ships 3.0 too but with different cipher defaults. The resulting
binary works on the build box and fails on a slightly older or newer target.
`rustls` ships its own crypto provider and has zero system deps.

**Issue 2 — CVE history.** OpenSSL averages 6-10 high-severity CVEs per
year. Even if you patch promptly, the audit surface is enormous (≈500 KLOC
of C). `rustls` is ≈40 KLOC of Rust, externally audited (Cure53 in 2020 and
2023), and the cryptographic primitives are pluggable (`ring` or
`aws-lc-rs`).

**Issue 3 — cross-platform.** OpenSSL doesn't ship on Windows (vcpkg or
manual install). `rustls` works identically on Linux, Windows MSVC, and
wasm32. For aphrody, Linux #1 + Windows #2 + wasm #3 (CLAUDE.md §0), this
is decisive.

**When OpenSSL is unavoidable.** FIPS 140-3 boundary requirement that
mandates a FIPS-validated module — even then, prefer `aws-lc-rs` (which is
FIPS validated) over raw `openssl-sys`.

## actix-web 4.x for greenfield

actix-web is not unsafe or buggy — it is technically excellent and still in
production at major companies. The case against it for new code in 2026:

1. **Ecosystem alignment.** `axum` lives in the tokio/tower ecosystem; the
   middleware story (tower::Layer) composes directly with all tower crates
   (load balancing, retries, rate limits). actix has its own middleware
   model that doesn't reuse tower.
2. **Maintenance velocity.** actix-web releases averaged 2.x per quarter in
   2021–2022, ~1 per quarter in 2024, and roughly 1 release in the first
   half of 2026. axum and tower keep weekly velocity. For a 5-year project,
   pick the ecosystem with the longer expected lifetime.
3. **Onboarding tax.** Most current Rust web tutorials and example
   repositories target axum. New hires will land faster on axum.

**When actix is the right choice.** Existing actix codebase you don't want
to rewrite; a team with deep actix experience; need for the actix Actor
model in the same process. Otherwise, axum.

## tokio 0.2 / 0.3

Pre-1.0. Different `Runtime` API, no `JoinSet`, different `select!` macro,
`block_on` location moved, channels differ. Anything you write against 0.2
will need a substantial rewrite to compile against 1.x. There is no upgrade
path that is not a port. Start on 1.x.

## hyper 0.14 raw

hyper is a brilliant low-level library, but you very rarely want to write
the routing, body extraction, error mapping, and middleware glue yourself.
`axum` is the thin layer on top of hyper that handles those concerns; it
also follows hyper's 1.x migration so you're not stuck on 0.14 forever.

If you genuinely need the low level (custom protocol on top of HTTP/2, raw
server with non-HTTP framing), use `hyper` 1.x directly, not 0.14.

## async-std

The project has been effectively unmaintained since 2023. There is no
known successor — the ecosystem consolidated on tokio. Anything you build
on async-std today inherits frozen dependencies and unfixed bugs in the
runtime. There is no benefit over tokio that justifies the deprecation
risk.

## tao + wry (Linux)

Linux build pulls GTK3, which has multiple RUSTSEC advisories tracked but
not patched upstream (cf. CLAUDE.md §7). Project policy ignores them in
`deny.toml` for the `gui` crate, but you do not want to take that exception
on a new crate. WASM via `wasm-bindgen` is the project's preferred path; if
you genuinely need native, `egui`/`iced` ship via `wgpu` and avoid GTK
entirely.

When `wry` is unavoidable: existing Tauri app, need OS-native webview for
parity with a Mac/Windows-first product. Otherwise, WASM.

## unicorn-engine (GPL-2.0)

GPL-2.0 is **viral**: linking unicorn-engine into the aphrody binary
contaminates the whole binary. aphrody ships Apache-2.0, so this is an
outright license violation if released. The only safe usage is as an
**out-of-process** executable invoked via FFI/subprocess with no static
linking — which defeats most of the reason to embed an emulator.

For analysis-only workflows, `iced-x86` (Apache-2.0) handles x86 decoding
without emulation. For multi-arch, `capstone` (Apache-2.0). For emulation
with a permissive license: there is no perfect substitute today — most
high-quality CPU emulators are GPL. Plan a subprocess boundary if you
truly need emulation, or evaluate if static analysis suffices.

## radare2-rs

radare2's core is LGPL-3.0 with GPL-3.0 components. Similar contamination
story to unicorn. Use `goblin` + `capstone` + your own analysis layer.

## serde_yaml 0.9

Archived by upstream in September 2024 with a note recommending forks.
The community-maintained drop-in is `serde_yaml_ng`. API is compatible;
swap is a single `Cargo.toml` line.

## chrono 0.4 default features

The default features pull in `oldtime` (CVE history) and bloat the build.
For new code, prefer `jiff` 0.1 (single Tantek-style API, immutable,
no_std-friendly variants). For chrono-dependent code, set
`default-features = false` and enable only `clock` and `serde`.

## time 0.1 / 0.2

Pre-1.0. No timezone support. `time` 0.3 has full TZ support; `jiff` 0.1
is the modern alternative.

## failure / error-chain

Both were popular pre-`?` operator (Rust 1.39, late 2019). `anyhow`
(applications) + `thiserror` (libraries) is the consensus pattern post-2020
and remains so in 2026.

## rand 0.7 / rand 0.9

Workspace is pinned to 0.8 because `denokv_proto` (transitive) requires it
(cf. CLAUDE.md §7). Mixing 0.7/0.8/0.9 in a single workspace causes
duplicate crate compilation and incompatible `Rng` traits. Match the
workspace pin.

## tracing-subscriber 0.3.23+

Packaging bug in 0.3.23: the `env` module became private in a way that
breaks downstream code expecting `tracing_subscriber::EnvFilter`. Pin 0.3.22
until 0.3.24 lands with the fix (cf. CLAUDE.md §7).

## native-tls on Linux

`native-tls` selects the platform's TLS implementation: Schannel on
Windows, SecureTransport on macOS, OpenSSL on Linux. The Linux path
inherits all OpenSSL system-dep issues. For cross-platform aphrody where
Linux is priority #1, this is a non-starter. Use `rustls-tls` reqwest
feature.

## dotenv (for parsing)

`dotenv` loads `.env` files into process env, but doesn't help with typed
parsing or layering. `figment` (with `Env::prefixed("APP_")`) is the
typed-config story; `envy` is the lightweight derive-based parser.

## lazy_static, once_cell::sync::Lazy

`std::sync::LazyLock` stabilised in Rust 1.80 (July 2024). For any new
code under 1.80+, the std type is preferred. `once_cell` remains useful
for `OnceCell` (non-static), but the `Lazy` use case is std now.

## error-chain style chained errors

Pre-`?` operator pattern. The modern equivalent is:

```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text)
        .with_context(|| format!("parsing {}", path.display()))
}
```

This produces a chain on display (`anyhow::Error: Display` walks the
chain) without the boilerplate of error-chain's macro-driven types.
