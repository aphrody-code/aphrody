<!-- SPDX-License-Identifier: Apache-2.0 -->
# Aphrody Documentation

Welcome to the documentation for **aphrody**, the cross-platform Rust CLI
(Linux #1, Windows #2, WebAssembly #3, macOS best-effort).

For the master map of every document under `docs/`, start with
[`INDEX.md`](./INDEX.md). For the consolidated executive summary, read
[`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md).

## Key subsystems

- **aphrody CLI** — high-performance Rust command-line interface with an
  autonomous natural-language execution mode powered by the native A2A
  (Agent-to-Agent) engine.
- **A2A ecosystem** — native `a2a`, `a2a-client`, `a2a-server`, `a2a-pb`,
  `a2a-grpc` crates, fully integrated into the core workspace. Transport is
  **gRPC** (the legacy file-based `ai.json` mailbox was removed in 2026).
- **aphrody-mcp** — unified Rust MCP stdio server (crate `google_mcp`, binary
  `aphrody-mcp`).

## Core directives

1. **Production-ready only** — no stubs, no placeholders.
2. **Zero-copy memory** — strict memory safety, zero-copy FFI via `mimalloc`.
3. **Supply-chain hardened** — `cargo-vet` audits + `cargo-deny`
   (CVE/licences/sources). No `cargo vendor`: lockfile + SHA-256 pins.
4. **Build hermétique** — `--locked --offline` in CI (`cargo ci-offline`).

## Navigation

| Section | Contenu |
|---|---|
| [`INDEX.md`](./INDEX.md) | Master map of every doc under `docs/` |
| [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md) | Consolidated executive summary (read first) |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Workspace map with ASCII diagrams |
| [`PLAN.md`](./PLAN.md) | Plan stratégique, phases livrées, roadmap |
| [`INSTALL.md`](./INSTALL.md) | Installation per platform |
| [`MCP_SETUP.md`](./MCP_SETUP.md) | Native `aphrody-mcp` server setup |
| [`PROTOCOL.md`](./PROTOCOL.md) | A2A protocol notes |
| [`GOOGLE.md`](./GOOGLE.md) | Google design / M3 reference notes |
| [`libc.md`](./libc.md) | libc / FFI reference notes |
| [`cargo/`](./cargo/) | Workspace, FFI policy, supply-chain |
| [`WASM/`](./WASM/) | WebAssembly targets and tooling |
| [`SUMMARY.md`](./SUMMARY.md) | mdBook table of contents (auto-generated) |
