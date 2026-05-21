<!-- SPDX-License-Identifier: Apache-2.0 -->

# Documentation Index

Master map of every Markdown / text document under `docs/` in the aphrody
repository. Updated 2026-05-21 to match the files actually present.

## 1. How to read this index

This page lists `docs/**/*.{md,txt}` grouped by purpose, with a one-line
description per entry. Working / auto-generated docs are flagged in section 9.
For the canonical workspace state, start with `SOURCE_OF_TRUTH.md` and
`ARCHITECTURE.md`.

## 2. Overview

- [`SOURCE_OF_TRUTH.md`](SOURCE_OF_TRUTH.md) — consolidated executive summary (read first)
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — 57-member workspace map with ASCII diagrams
- [`AWESOME.md`](AWESOME.md) — curated catalogue of aphrody ecosystem resources
- [`COMPARISON.md`](COMPARISON.md) — aphrody vs just, taskfile, gh, devcontainer, asdf
- [`EXAMPLES.md`](EXAMPLES.md) — copy-paste recipes for common workflows
- [`FAQ.md`](FAQ.md) — anticipated questions and crisp answers
- [`GOOGLE.md`](GOOGLE.md) — Google design / m3 reference notes
- [`MCP_SETUP.md`](MCP_SETUP.md) — native `aphrody-mcp` server setup
- [`MIGRATION.md`](MIGRATION.md) — moving from competing tools to aphrody
- [`PROTOCOL.md`](PROTOCOL.md) — normative A2A protocol plus aphrody extensions
- [`ROADMAP.md`](ROADMAP.md) — milestones
- [`UI-ARCHITECTURE.md`](UI-ARCHITECTURE.md) — Material Design 3 / UI crate map
- [`libc.md`](libc.md) — libc / FFI reference notes

## 3. Performance & planning

- [`PERFORMANCE.md`](PERFORMANCE.md) — bench claims with reproducible recipes
- [`PERFORMANCE-HISTORY.md`](PERFORMANCE-HISTORY.md) — historical perf trend log
- [`PLAN.md`](PLAN.md) — current work queue
- [`PLAN-MOONSHOT.md`](PLAN-MOONSHOT.md) — 30-day moonshot star-maximisation plan

## 4. Operational runbooks

- [`INSTALL.md`](INSTALL.md) — distribution channels and install paths
- [`CI-CD.md`](CI-CD.md) — CI workflows overview
- [`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md) — per-release maintainer checklist
- [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md) — common pitfalls and fixes
- [`POST-LAUNCH.md`](POST-LAUNCH.md) — Show HN engagement protocol
- [`COMMUNITY.md`](COMMUNITY.md) — engagement channels and norms

## 5. Security and privacy

- [`PRIVACY.md`](PRIVACY.md) — zero-telemetry policy
- [`SECURITY-MODEL.md`](SECURITY-MODEL.md) — STRIDE threat model

## 6. Cargo / build (`docs/cargo/`)

- [`cargo/README.md`](cargo/README.md) — cargo docs entrypoint
- [`cargo/WORKSPACE.md`](cargo/WORKSPACE.md) — workspace description (57 members)
- [`cargo/CRATES.md`](cargo/CRATES.md) — per-crate inventory
- [`cargo/CROSS_PLATFORM.md`](cargo/CROSS_PLATFORM.md) — multi-target strategy
- [`cargo/ANDROID_TARGET.md`](cargo/ANDROID_TARGET.md) — Android target notes
- [`cargo/CHROMIUM_ANDROID_PATTERNS.md`](cargo/CHROMIUM_ANDROID_PATTERNS.md) — Google-grade patterns
- [`cargo/FFI_POLICY.md`](cargo/FFI_POLICY.md) — FFI rules (`cxx` / `bindgen`)
- [`cargo/DEPENDENCIES.md`](cargo/DEPENDENCIES.md) — dependency policy
- [`cargo/DEV-ENV.md`](cargo/DEV-ENV.md) — dev environment setup
- [`cargo/BUILD-SPEED.md`](cargo/BUILD-SPEED.md) — build-speed tactics
- [`cargo/PIPELINE-OPTIMIZATION.md`](cargo/PIPELINE-OPTIMIZATION.md) — CI pipeline tuning
- [`cargo/PROFILES.md`](cargo/PROFILES.md) — cargo profiles
- [`cargo/LINTS.md`](cargo/LINTS.md) — workspace lint policy
- [`cargo/CHEATSHEET.md`](cargo/CHEATSHEET.md) — cargo one-liners
- [`cargo/MIGRATION.md`](cargo/MIGRATION.md) — migration notes
- [`cargo/GOOGLE_MODE.md`](cargo/GOOGLE_MODE.md) — Google-mode conventions
- [`cargo/SKILLS.md`](cargo/SKILLS.md) — skill ecosystem spec and runtime
- [`cargo/SUPPLY_CHAIN.md`](cargo/SUPPLY_CHAIN.md) — cargo-vet / cargo-deny
- [`cargo/SECURITY-DEEP.md`](cargo/SECURITY-DEEP.md) — supply-chain deep dive
- [`cargo/PUBLISH-LADDER.md`](cargo/PUBLISH-LADDER.md) — crates.io publish ladder

## 7. WASM (`docs/WASM/`)

- [`WASM/README.md`](WASM/README.md) — WASM target overview
- [`WASM/build-targets.md`](WASM/build-targets.md) — wasm32-unknown-unknown vs wasm32-wasi
- [`WASM/rust-wasm-fundamentals.md`](WASM/rust-wasm-fundamentals.md) — wasm-bindgen primer
- [`WASM/tooling.md`](WASM/tooling.md) — wasm-pack, wasm-opt, twiggy
- [`WASM/wgpu-webgpu.md`](WASM/wgpu-webgpu.md) — wgpu in browser

## 8. Research (`docs/research/`)

- [`research/NEXTJS_VERCEL_RUST_CRATES.md`](research/NEXTJS_VERCEL_RUST_CRATES.md) — Vercel Rust crate survey
- [`research/SHADCN_M3_MAPPING.md`](research/SHADCN_M3_MAPPING.md) — shadcn → MD3 mapping

## 9. Internal audits (`docs/audits/`)

Dated, point-in-time records (2026-05-17 → 2026-05-19). They describe work as
it stood on their date and may reference now-removed tooling (`n2b`, `bxc`,
`xtask`, `mrx-*`); treat them as **historical**. The full list is in the
auto-generated [`SUMMARY.md`](SUMMARY.md) under the "Audits" section.

## 10. Working / auto-generated (may drift)

- [`SUMMARY.md`](SUMMARY.md) — mdBook ToC, **auto-generated** by
  `cargo run -p aphrody-summary`; do not hand-edit.
- [`llms.txt`](llms.txt) — flattened corpus, **auto-generated** alongside
  `SUMMARY.md`.
- [`PLAN.md`](PLAN.md) — live work queue.

## 11. Sub-project READMEs

For per-crate READMEs, see `crates/<name>/README.md`. Cross-linked from the
root [`README.md`](../README.md) doc tree.
