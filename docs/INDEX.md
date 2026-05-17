<!-- SPDX-License-Identifier: Apache-2.0 -->

# Documentation Index

Master map of every Markdown document under `docs/` in the aphrody repository.

## 1. How to read this index

This page lists all `docs/**/*.md` files grouped by purpose. Each entry is a one-line description so engineers landing in `docs/` directly can orient themselves. For deep dives, follow the link. For quick orientation, scan the descriptions. Entries are grouped alphabetically within each section. Working/auto-generated docs are flagged in section 11.

## 2. Overview

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — 17-crate workspace map with ASCII diagrams
- [`COMPARISON.md`](COMPARISON.md) — aphrody vs just, taskfile, gh, devcontainer, asdf
- [`EXAMPLES.md`](EXAMPLES.md) — 12 copy-paste recipes for common workflows
- [`FAQ.md`](FAQ.md) — 12 anticipated questions and crisp answers
- [`MIGRATION.md`](MIGRATION.md) — moving from competing tools to aphrody
- [`PERFORMANCE.md`](PERFORMANCE.md) — bench claims with reproducible recipes
- [`PROTOCOL.md`](PROTOCOL.md) — normative A2A/v0.4 plus aphrody extensions
- [`ROADMAP.md`](ROADMAP.md) — quarterly milestones Q2 2026 through Q1 2027
- [`SOURCE_OF_TRUTH.md`](SOURCE_OF_TRUTH.md) — consolidated executive summary

## 3. Architecture decisions (ADRs)

- [`adr/0000-template.md`](adr/0000-template.md) — ADR boilerplate
- [`adr/0001-cross-platform-rust.md`](adr/0001-cross-platform-rust.md) — Rust-only, gated cfg per OS
- [`adr/0002-a2a-file-based.md`](adr/0002-a2a-file-based.md) — file-based A2A mailbox rationale
- [`adr/0003-yolo-parallel-grind.md`](adr/0003-yolo-parallel-grind.md) — 4-agent parallel loop policy

## 4. A2A extensions

- [`extensions/context7-version-pinning-v1.md`](extensions/context7-version-pinning-v1.md) — pin dep versions via context7 MCP
- [`extensions/file-transport-v1.md`](extensions/file-transport-v1.md) — JSONL mailbox transport semantics
- [`extensions/honest-delivery-v1.md`](extensions/honest-delivery-v1.md) — non-repudiation and ack rules
- [`extensions/index.md`](extensions/index.md) — extension registry

## 5. Operational runbooks

- [`cargo/PUBLISH-LADDER.md`](cargo/PUBLISH-LADDER.md) — 9-rung crates.io publish ladder
- [`cargo/SECURITY-DEEP.md`](cargo/SECURITY-DEEP.md) — supply-chain deep dive
- [`cargo/SKILLS.md`](cargo/SKILLS.md) — skill ecosystem spec and runtime
- [`INSTALL.md`](INSTALL.md) — 8 distribution channels
- [`POST-LAUNCH.md`](POST-LAUNCH.md) — Show HN +24h/+72h/+7d engagement protocol
- [`pwsh/CHEATSHEET.md`](pwsh/CHEATSHEET.md) — PowerShell one-liners for aphrody ops
- [`pwsh/MODULES.md`](pwsh/MODULES.md) — required pwsh modules and install order
- [`pwsh/README.md`](pwsh/README.md) — PowerShell ops overview
- [`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md) — per-release maintainer checklist
- [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md) — 14 common pitfalls and fixes
- [`WASM/README.md`](WASM/README.md) — WASM target overview
- [`WASM/build-targets.md`](WASM/build-targets.md) — wasm32-unknown-unknown vs wasm32-wasi
- [`WASM/bun-native-wasm.md`](WASM/bun-native-wasm.md) — running wasm via Bun
- [`WASM/nextjs-integration.md`](WASM/nextjs-integration.md) — Next.js + wasm-bindgen
- [`WASM/rust-wasm-fundamentals.md`](WASM/rust-wasm-fundamentals.md) — wasm-bindgen primer
- [`WASM/tooling.md`](WASM/tooling.md) — wasm-pack, wasm-opt, twiggy
- [`WASM/wgpu-webgpu.md`](WASM/wgpu-webgpu.md) — wgpu in browser
- [`winget/CATALOG.md`](winget/CATALOG.md) — curated winget manifests
- [`winget/CHEATSHEET.md`](winget/CHEATSHEET.md) — winget one-liners
- [`winget/README.md`](winget/README.md) — Windows packaging notes

## 6. Security and privacy

- [`CI-CD.md`](CI-CD.md) — CI workflows overview
- [`PRIVACY.md`](PRIVACY.md) — zero-telemetry policy
- [`SECURITY-MODEL.md`](SECURITY-MODEL.md) — STRIDE threat model

## 7. Community

- [`COMMUNITY.md`](COMMUNITY.md) — engagement channels and norms

## 8. Launch material

- [`launch/SHOW-HN.md`](launch/SHOW-HN.md) — Show HN title candidates and comment templates

## 9. Technical posts (chronological)

- [`posts/2026-05-ai-json.md`](posts/2026-05-ai-json.md) — cross-Claude A2A file-based protocol
- [`posts/2026-05-cross-platform-rust.md`](posts/2026-05-cross-platform-rust.md) — Linux + Win + WASM patterns
- [`posts/2026-05-yolo-grind-loop.md`](posts/2026-05-yolo-grind-loop.md) — 4-agent parallel grind loop

## 10. Internal audits

- [`audits/2026-05-17-bxc-scrape-request.md`](audits/2026-05-17-bxc-scrape-request.md) — bxc scrape audit
- [`audits/2026-05-17-mrx-aggressive.md`](audits/2026-05-17-mrx-aggressive.md) — mrx aggressive-scan audit
- [`audits/2026-05-17-n2b-scan.md`](audits/2026-05-17-n2b-scan.md) — n2b Next.js scan audit

## 11. Planning (working docs, may drift)

- [`DESIGN.md`](DESIGN.md) — original design notes
- [`PLAN.md`](PLAN.md) — current work queue
- [`SUMMARY.md`](SUMMARY.md) — auto-generated mdBook table of contents, do not hand-edit

## 12. Sub-project READMEs

For per-crate READMEs, see `crates/<name>/README.md`. Cross-linked from the root [`README.md`](../README.md) doc tree.
