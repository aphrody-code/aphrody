<!-- SPDX-License-Identifier: Apache-2.0 -->

# Documentation Index

Master map of every Markdown / text document under `docs/` in the aphrody repository. Updated 2026-05-24 to match the files actually present.

## 1. How to read this index

This page lists `docs/**/*.{md,txt}` grouped by purpose, with a one-line description per entry. For the canonical workspace state, start with `SOURCE_OF_TRUTH.md` and `ARCHITECTURE.md`.

---

## 2. Overview

- [`SOURCE_OF_TRUTH.md`](SOURCE_OF_TRUTH.md) — Consolidated executive summary (read first)
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — 58-member workspace map with ASCII diagrams
- [`AWESOME.md`](AWESOME.md) — Curated catalogue of aphrody ecosystem resources
- [`COMPARISON.md`](COMPARISON.md) — aphrody vs just, taskfile, gh, devcontainer, asdf
- [`EXAMPLES.md`](EXAMPLES.md) — Copy-paste recipes for common workflows
- [`FAQ.md`](FAQ.md) — Anticipated questions and crisp answers
- [`GOOGLE.md`](GOOGLE.md) — Google design / m3 reference notes
- [`MCP_SETUP.md`](MCP_SETUP.md) — Native `aphrody-mcp` server setup
- [`MIGRATION.md`](MIGRATION.md) — Moving from competing tools to aphrody
- [`PROTOCOL.md`](PROTOCOL.md) — Normative A2A protocol plus aphrody extensions
- [`api-unified-pattern.md`](api-unified-pattern.md) — Canonical cross-repo HTTP contract (REST/GraphQL/`Bun.serve`/cron) the downstream bots conform to
- [`rag-unified-pattern.md`](rag-unified-pattern.md) — Canonical cross-repo RAG/retrieval contract
- [`ROADMAP.md`](ROADMAP.md) — Project milestones and roadmap
- [`libc.md`](libc.md) — libc / FFI reference notes
- [`notebooklm-d68c5204-report.md`](notebooklm-d68c5204-report.md) — Research report on NotebookLM Boq RPC surface
- [`peer-a2a-mcp-csharp.md`](peer-a2a-mcp-csharp.md) — Peer A2A coordination via C# MCP bridge
- [`nextjs-canary-reference.md`](nextjs-canary-reference.md) — Next.js `16.3.0-canary.39` reference: agent/AI surface, Rust/SWC/Turbopack stack vs aphrody, `@next/playwright` API + bxc port plan

---

## 3. Performance & planning

- [`PERFORMANCE.md`](PERFORMANCE.md) — Bench claims with reproducible recipes
- [`PERFORMANCE-HISTORY.md`](PERFORMANCE-HISTORY.md) — Historical perf trend log
- [`PLAN.md`](PLAN.md) — Current work queue
- [`PLAN-MOONSHOT.md`](PLAN-MOONSHOT.md) — 30-day moonshot star-maximisation plan
- [`plans/agent-home.md`](plans/agent-home.md) — Agent Home persistence migration plan
- [`plans/antigravity-exploitation.md`](plans/antigravity-exploitation.md) — Antigravity cloud API exploitation plan
- [`plans/tauri-app.md`](plans/tauri-app.md) — Tauri App desktop shell implementation roadmap

---

## 4. Operational runbooks

- [`INSTALL.md`](INSTALL.md) — Distribution channels and install paths
- [`CI-CD.md`](CI-CD.md) — CI workflows overview
- [`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md) — Per-release maintainer checklist
- [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md) — Common pitfalls and fixes
- [`POST-LAUNCH.md`](POST-LAUNCH.md) — Show HN engagement protocol
- [`COMMUNITY.md`](COMMUNITY.md) — Engagement channels and norms
- [`agy-cli/README.md`](agy-cli/README.md) — antigravity CLI wrapper manual

---

## 5. Security and privacy

- [`PRIVACY.md`](PRIVACY.md) — Zero-telemetry policy
- [`SECURITY-MODEL.md`](SECURITY-MODEL.md) — STRIDE threat model

---

## 6. Cargo / build (`docs/cargo/`)

- [`cargo/README.md`](cargo/README.md) — Cargo docs entrypoint
- [`cargo/WORKSPACE.md`](cargo/WORKSPACE.md) — Workspace description (58 members)
- [`cargo/CRATES.md`](cargo/CRATES.md) — Per-crate inventory
- [`cargo/CROSS_PLATFORM.md`](cargo/CROSS_PLATFORM.md) — Multi-target strategy
- [`cargo/ANDROID_TARGET.md`](cargo/ANDROID_TARGET.md) — Android target notes
- [`cargo/CHROMIUM_ANDROID_PATTERNS.md`](cargo/CHROMIUM_ANDROID_PATTERNS.md) — Google-grade patterns
- [`cargo/FFI_POLICY.md`](cargo/FFI_POLICY.md) — FFI rules (`cxx` / `bindgen`)
- [`cargo/DEPENDENCIES.md`](cargo/DEPENDENCIES.md) — Dependency policy
- [`cargo/DEV-ENV.md`](cargo/DEV-ENV.md) — Dev environment setup
- [`cargo/BUILD-SPEED.md`](cargo/BUILD-SPEED.md) — Build-speed tactics
- [`cargo/PIPELINE-OPTIMIZATION.md`](cargo/PIPELINE-OPTIMIZATION.md) — CI pipeline tuning
- [`cargo/PROFILES.md`](cargo/PROFILES.md) — Cargo profiles
- [`cargo/LINTS.md`](cargo/LINTS.md) — Workspace lint policy
- [`cargo/CHEATSHEET.md`](cargo/CHEATSHEET.md) — Cargo one-liners
- [`cargo/MIGRATION.md`](cargo/MIGRATION.md) — Migration notes
- [`cargo/GOOGLE_MODE.md`](cargo/GOOGLE_MODE.md) — Google-mode conventions
- [`cargo/SKILLS.md`](cargo/SKILLS.md) — Skill ecosystem spec and runtime
- [`cargo/SUPPLY_CHAIN.md`](cargo/SUPPLY_CHAIN.md) — cargo-vet / cargo-deny
- [`cargo/SECURITY-DEEP.md`](cargo/SECURITY-DEEP.md) — Supply-chain deep dive
- [`cargo/PUBLISH-LADDER.md`](cargo/PUBLISH-LADDER.md) — crates.io publish ladder

---

## 7. WASM (`docs/WASM/`)

- [`WASM/README.md`](WASM/README.md) — WASM target overview
- [`WASM/build-targets.md`](WASM/build-targets.md) — wasm32-unknown-unknown vs wasm32-wasi
- [`WASM/rust-wasm-fundamentals.md`](WASM/rust-wasm-fundamentals.md) — wasm-bindgen primer
- [`WASM/tooling.md`](WASM/tooling.md) — wasm-pack, wasm-opt, twiggy
- [`WASM/wgpu-webgpu.md`](WASM/wgpu-webgpu.md) — wgpu in browser

---

## 8. Research (`docs/research/`)

- [`research/NEXTJS_VERCEL_RUST_CRATES.md`](research/NEXTJS_VERCEL_RUST_CRATES.md) — Vercel Rust crate survey
- [`research/adobe-creative-integration.md`](research/adobe-creative-integration.md) — Adobe Photoshop and Creative Cloud API survey
- [`research/animate-tui-motion.md`](research/animate-tui-motion.md) — Frame-buffer animations in TUI environments
- [`research/antigravity-ide-re.md`](research/antigravity-ide-re.md) — Reverse-engineering of Antigravity internal plugins
- [`research/antigravity-sdk-analysis.md`](research/antigravity-sdk-analysis.md) — Technical analysis of the Antigravity auth/gRPC endpoints
- [`research/antigravity-site-recon.md`](research/antigravity-site-recon.md) — Cloud endpoint recon
- [`research/aphrody-search-upgrades.md`](research/aphrody-search-upgrades.md) — Upgrades for local FTS matching
- [`research/awesome-rust-ml-for-aphrody.md`](research/awesome-rust-ml-for-aphrody.md) — Rust-based ML models and libraries evaluation
- [`research/bun-rust-ffi-best-practices.md`](research/bun-rust-ffi-best-practices.md) — Low-latency Bun-to-Rust C-ABI bridges
- [`research/bun-vs-vite-2026.md`](research/bun-vs-vite-2026.md) — Comparison of Bun bundler vs Vite for MD3 frontend tooling
- [`research/bxc-google-module-chrome-mcp.md`](research/bxc-google-module-chrome-mcp.md) — Chrome extension MCP protocol reverse engineering
- [`research/electron-re-intel.md`](research/electron-re-intel.md) — Electron main-process runtime hooking
- [`research/gemini-web-cdp-exploitation.md`](research/gemini-web-cdp-exploitation.md) — Headless Chrome CDP script patterns for consumer Gemini App
- [`research/gemini-web-feature-matrix.md`](research/gemini-web-feature-matrix.md) — Scraped feature matrix of the web chat client
- [`research/gemini-web-protocol.md`](research/gemini-web-protocol.md) — Detailed mapping of Gemini batchexecute Boq endpoints
- [`research/ghidra-aphrody-integration.md`](research/ghidra-aphrody-integration.md) — Programmatic Ghidra API mapping for reverse-engineering tools
- [`research/google-local-install-map.md`](research/google-local-install-map.md) — On-disk path mapping of Google Cloud SDK and Chrome installs
- [`research/gui-options-2026.md`](research/gui-options-2026.md) — Wgpu/Vello/Tauri/Angular comparison for the desktop client shell
- [`research/obscura-headless-browser.md`](research/obscura-headless-browser.md) — Technical documentation on the headless scraping engine
- [`research/obscura-integration-spec.md`](research/obscura-integration-spec.md) — Bridge spec mapping the CLI to the Obscura scraper
- [`research/openclaw-vs-aphrody.md`](research/openclaw-vs-aphrody.md) — Structural differences between openclaw and the aphrody CLI
- [`research/re-tooling-landscape.md`](research/re-tooling-landscape.md) — Survey of binary reverse-engineering frameworks
- [`research/vscode-fork-re-intel.md`](research/vscode-fork-re-intel.md) — Introspection of VS Code workspace extensions
- [`research/webgpu-performance.md`](research/webgpu-performance.md) — WebGPU canvas rendering benchmarking in headless CI runs
- [`research/adobe-connector/README.md`](research/adobe-connector/README.md) — Adobe Photoshop and Creative Cloud batch integration survey

---

## 9. Internal audits (`docs/audits/`)

Dated, point-in-time records (2026-05-17 → 2026-05-19). They describe work as it stood on their date and may reference now-removed tooling (`n2b`, `bxc`, `xtask`, `mrx-*`); treat them as **historical**. The full list of audit reports is indexed in the auto-generated [`SUMMARY.md`](SUMMARY.md).

---

## 10. Other directories

### Gcloud (`docs/gcloud/`)
- [`gcloud/README.md`](gcloud/README.md) — Google Cloud CLI auth and setup

### Awesome (`docs/awesome/`)
- [`awesome/awesome-rust-ml.md`](awesome/awesome-rust-ml.md) Curated list of machine learning libraries in Rust

### Integrations (`docs/integrations/`)
- [`integrations/photoshop-uxp-panel.md`](integrations/photoshop-uxp-panel.md) Photoshop UXP panel design specifications

### Rust (`docs/rust/`)
- [`rust/android-rust-practices.md`](rust/android-rust-practices.md) — Best practices for Rust on Android
- [`rust/chromium-rust-practices.md`](rust/chromium-rust-practices.md) — Rust programming practices inside Chromium codebase
- [`rust/google-rust-crates.md`](rust/google-rust-crates.md) — Audit of Google-authored Rust libraries

### Tauri (`docs/tauri/`)
- [`tauri/README.md`](tauri/README.md) — Tauri integration overview
- [`tauri/aphrody-integration.md`](tauri/aphrody-integration.md) — CLI integration with Tauri shell
- [`tauri/architecture.md`](tauri/architecture.md) — Desktop client IPC design
- [`tauri/bun-in-tauri.md`](tauri/bun-in-tauri.md) — Bun integration as a sidecar process in Tauri
- [`tauri/plugins.md`](tauri/plugins.md) — Tauri custom plugin layout
- [`tauri/risks.md`](tauri/risks.md) — Security risk analysis for webview contexts
- [`tauri/ui-framework.md`](tauri/ui-framework.md) — UI styling frameworks for Tauri shell

### X / Twitter (`docs/x/`)
- [`x/README.md`](x/README.md) — X integration API overview
- [`x/architecture.md`](x/architecture.md) — GraphQL client layout
- [`x/commands.md`](x/commands.md) — CLI commands for X interactions
- [`x/store.md`](x/store.md) — Cache and session storage for X client

---

## 11. Working / auto-generated (may drift)

- [`SUMMARY.md`](SUMMARY.md) — mdBook ToC, **auto-generated** by `cargo run -p aphrody-summary`; do not hand-edit.
- [`llms.txt`](llms.txt) — Flattened corpus, **auto-generated** alongside `SUMMARY.md`.
- [`PLAN.md`](PLAN.md) — Live work queue.

---

## 12. Sub-project READMEs

For per-crate READMEs, see `crates/<name>/README.md`. Cross-linked from the root [`README.md`](../README.md) doc tree.
