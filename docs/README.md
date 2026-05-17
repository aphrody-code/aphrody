<!-- SPDX-License-Identifier: Apache-2.0 -->
# Google OS & CLI Documentation

Welcome to the unified documentation for Google OS and Aphrody.

## Key subsystems

- **Aphrody** : High-performance Rust command-line interface with autonomous natural-language execution mode powered by the native `a2a` (Agent-to-Agent) engine. The CLI intercepts NL prompts and streams AI responses with zero-buffering latency.
- **Google OS** : Hybrid POSIX subsystem for Windows bridging Uutils/GNU userland to native Windows APIs (IOCP, NTDLL, IoRing API on Win11).
- **A2A ecosystem** : Native `a2a`, `a2a-client`, `a2a-server`, `a2a-pb`, `a2a-grpc` crates fully integrated into the core workspace.

## Core directives

1. **Production-ready only** â€” no stubs, no placeholders. Every implementation is robust and securely integrated.
2. **Zero-copy memory** â€” strict memory safety, zero-copy FFI via `mimalloc` and `bun_ffi`.
3. **Supply-chain hardened** â€” `cargo-vet` audits (Google/Mozilla/Fuchsia/ChromeOS feeds) + `cargo-deny` (CVE/licences/sources). No `cargo vendor` â€” lockfile + SHA-256 pins ensure reproducibility.
4. **Build hermÃ©tique** â€” `--locked --offline` in CI (`cargo ci-offline` alias).

## Navigation

| Section | Contenu |
|---|---|
| [`PLAN.md`](./PLAN.md) | Plan stratÃ©gique, phases livrÃ©es, roadmap |
| [`SUMMARY.md`](./SUMMARY.md) | mdBook table of contents |
| [`DESIGN.md`](./DESIGN.md) | Architecture decisions |
| [`GOOGLE.md`](./GOOGLE.md) | Google ecosystem alignment (Canary track) |
| [`libc.md`](./libc.md) | glibc spec alignment for `google_os` |
| [`bun-rs.md`](./bun-rs.md) | Bun runtime Rust port notes |
| [`cargo/`](./cargo/) | Workspace, FFI policy, supply-chain |
| [`google-os-plan/`](./google-os-plan/) | Kernel roadmap |
| [`winget/`](./winget/) | WinGet catalog, DSC config |
| [`pwsh/`](./pwsh/) | PowerShell 7 profiles |
| [`md3/`](./md3/) | Material Design 3 references |
| [`terminal/`](./terminal/) | microsoft/terminal integration |
