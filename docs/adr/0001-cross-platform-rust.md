<!-- SPDX-License-Identifier: Apache-2.0 -->

# ADR-0001: Rust nightly Edition 2024 for the cross-platform CLI core

## Status

Accepted

Date: 2026-05-17

Author: aphrody-code

## Context

The `aphrody` CLI must run, from a single codebase, on three first-class
targets in strict priority order:

1. Linux Ubuntu 26.04 (`x86_64-unknown-linux-gnu`) — canonical build and
   merge gate. If it does not compile on Linux, it does not merge.
2. Windows 11 Insider Canary (`x86_64-pc-windows-msvc`) — feature parity
   for forensics and process inspection.
3. WebAssembly browser (`wasm32-unknown-unknown`) — library surface so the
   same domain logic ships into the IEVR web UI without a rewrite.

macOS is best-effort, never blocking. The CLI also needs deep OS hooks
(`/proc/<pid>` on Linux, `NtQuerySystemInformation` on Windows, real DNS
and network IO), a static single-binary distribution story, and must
validate AGNTCY a2a/v0.4 manifests at the type level — see ADR-0002.

## Decision

We use **Rust nightly with Edition 2024** as the sole implementation
language for the CLI core and all distributed crates. Platform-specific
code lives behind `#[cfg(target_os = "windows")]` /
`#[cfg(target_arch = "wasm32")]` gates so the Linux build never pulls
Windows-only crates (`windows-rs`, NTDLL, IOCP) into its dependency graph.
The workspace pins a nightly toolchain via `rust-toolchain.toml` and
CI validates all three target triples on every PR (`cargo check --target ...`).

## Alternatives Considered

- **Go**: excellent single-binary on Linux and Windows, but browser-WASM
  goes through TinyGo with significant runtime overhead and no equivalent
  to `wasm-bindgen` for ergonomic JS interop. Eliminates a primary target.
- **C++ (Clang 19, modules)**: gives low-level control and WASM via
  Emscripten, but cross-platform build complexity (CMake matrices, vcpkg,
  MSVC vs Clang divergence) costs more engineer-weeks per platform than
  `cargo`'s built-in cross-compilation, and no single ecosystem matches
  `wasm-bindgen` + `serde` + `clap`.
- **Zig 0.14**: best-in-class native cross-compilation, but pre-1.0, the
  package manager still hardening, and the WASM ecosystem is bare.
  Tooling risk unacceptable for a serious CLI.
- **TypeScript + Bun `--compile`**: ergonomic, but Linux output is
  ~50 MB minimum, FFI is weaker than Rust's, and browser-WASM needs a
  separate pipeline.

## Consequences

Positive:

- Single source tree compiles to all three targets via `cargo check`
  against three triples — already wired in CI.
- Memory safety eliminates an entire class of CVEs; `cargo deny` +
  `cargo vet` + `cargo audit` give Google-grade supply-chain hygiene.
- `serde` gives compile-time validation of the `ai.json` manifest schema —
  ADR-0002 inherits this for free.
- Single static binary per target, no runtime beyond `glibc` / UCRT /
  host JS engine.

Negative:

- Compile times noticeably longer than Go; mitigated by `sccache` and
  incremental builds.
- Nightly pin demands periodic refresh (tracked in `PLAN.md`).
- Some crates (`tokio` full, `gtk-rs`) cannot cross to WASM — we gate
  `tokio` behind selected features and use `wasm-bindgen-futures` on the
  web build (CLAUDE.md §7).

## References

- Related: ADR-0002.
- Workspace `Cargo.toml` `[workspace.lints]` block.
- Commit `696072f71` — SPDX headers + workspace metadata inheritance.
- Commit `8859ca785` — rustls `CryptoProvider` boot fix, evidence that
  the cfg-gated discipline pays off.
- CLAUDE.md §0 (Pivot 2026-05-17) and §2 (Politique de langages).
