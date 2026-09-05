<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cargo workspace profile reference

Reference: `[profile.*]` in the workspace root `Cargo.toml`.

## 1. Available profiles

The root `Cargo.toml` defines nine profiles:

- `dev` — default `cargo build` (debug, fast iteration).
- `release` — default `cargo build --release` (fat LTO, stripped).
- `dist` — distribution-grade release (inherits `release`, room for PGO/BOLT).
- `release-fast` — CI-friendly release (thin LTO, parallel codegen).
- `release-debug` — release opts with line-table symbols for profiling.
- `bench` — Criterion harness (thin LTO + symbols for flamegraphs).
- `asan` — AddressSanitizer (nightly Linux only).
- `careful` — `cargo-careful` checked test runs (extra UB checks).
- `reproducible` — deterministic dist (used by `release.yml`).

There is no `release-perf`, `release-min`, or `release-android` profile. WASM and Android rely on overrides plus per-target `RUSTFLAGS` from `.cargo/config.toml`.

## 2. Profile selection matrix

| Use case | Profile | Compile time | Binary size | Runtime perf |
|---|---|---|---|---|
| Local dev | `dev` | seconds | large | slow |
| CI fast smoke | `release-fast` | ~1 min | medium | ~80% of release |
| Default ship build | `release` | ~2-4 min | small (stripped) | fast |
| Tagged release artefact | `reproducible` | ~3-5 min | smallest, deterministic | fast |
| Criterion benchmarks | `bench` | ~1-2 min | medium | fastest realistic |
| Profiling | `release-debug` | ~3 min | large | fast |
| Memory bug hunt | `asan` | ~3 min | large | slow |
| UB test stress | `careful` | seconds | large | slow |

## 3. Profile internals

Settings from the root manifest:

- `dev`: opt-level 0, lto off, codegen-units 256, panic unwind, debug "limited", incremental, split-debuginfo unpacked. Build scripts and deps overridden to opt-level 3 / 1.
- `release`: opt-level 3, lto "fat", codegen-units 1, panic abort, strip symbols, debug false, incremental false, rpath false.
- `dist`: inherits `release`, identical today, reserved for PGO/BOLT.
- `release-fast`: inherits `release` with `lto = "thin"`, `codegen-units = 16`. ~20% runtime cost for 3-5x faster builds.
- `release-debug`: inherits `release` with `debug = "line-tables-only"`, `strip = "none"`, `split-debuginfo = "packed"`. For `perf`, ETW, samply.
- `bench`: inherits `release` with thin LTO, codegen-units 16, line-table debug.
- `asan`: inherits `release-debug`, opt-level 1, debug full, lto off. Pair with `RUSTFLAGS="-Z sanitizer=address"`.
- `careful`: inherits `dev`, debug-assertions + overflow-checks on.
- `reproducible`: inherits `dist`. Requires `SOURCE_DATE_EPOCH` and `--remap-path-prefix` (set in `.cargo/config.toml`).

Crypto crates (`aes-gcm`, `sha2`, `rustls`) get opt-level 3 under `dev` via `[profile.dev.package.<crate>]` overrides.

## 4. How to invoke

```bash
cargo build --release                            # release profile
cargo build --profile dist --workspace --locked  # ship-grade
cargo build --profile release-fast -p aphrody    # CI smoke per-crate
cargo bench --workspace --locked                 # bench auto-applied
cargo build --profile reproducible --locked      # deterministic dist
```

## 5. Per-crate overrides

Workspace profiles can be tightened per-crate in `crates/<name>/Cargo.toml`:

```toml
[profile.release.package.heavy-crate]
opt-level     = 3
codegen-units = 1
```

Discouraged outside hot loops — prefer central workspace control. Legitimate cases: forcing opt-level 3 on crypto in `dev` (already done at workspace level), or shrinking a WASM-only crate.

## 6. WASM profile considerations

`crates/aphrody-wasm` ships via `wasm-pack` and post-processes with `wasm-opt`:

```toml
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-O", "--enable-simd", "--enable-bulk-memory"]
```

- Base `release` (fat LTO + abort + strip) is already WASM-friendly.
- `--enable-simd` required because `aes-gcm` auto-vectorizes via LLVM.
- `wasm-opt -O` adds 30-50% size reduction after `rustc`.
- For size-critical builds, override per-crate with `opt-level = "s"` or `"z"`; never change workspace `release` to `s` (regresses CLI perf).

## 7. Profile and CI matrix

- `.github/workflows/cross-platform.yml` — default `release` profile for the matrix (Linux, Windows, WASM); caps `CARGO_PROFILE_DEV_DEBUG=limited`.
- `.github/workflows/bench.yml` — Criterion harness auto-selects `bench`.
- `.github/workflows/release.yml` — shipped binary built with `--profile reproducible` for byte-for-byte verifiable artefacts.
- `.github/workflows/build.yml` — workspace defaults, no override.

## 8. Choosing a profile

- Iterating locally: `dev`.
- Committing a PR: `release-fast` for the quickest meaningful signal.
- Tagging a release: `reproducible` (the shipped artefact).
- Benchmarking: `bench`.
- Profiling a release-only bug: `release-debug` plus `perf` / samply.
- Hunting UB: `careful` for tests, `asan` for runtime memory bugs.
- WASM bundle: `dist` plus a per-crate `opt-level = "s"` override.

## 9. References

- Cargo profiles: https://doc.rust-lang.org/cargo/reference/profiles.html
- `docs/PERFORMANCE.md` — bench recipes per profile.
- `docs/cargo/SECURITY-DEEP.md` — `cargo-auditable` wraps `release`.
- `docs/cargo/CROSS_PLATFORM.md` — per-target `RUSTFLAGS` in `.cargo/config.toml`.
