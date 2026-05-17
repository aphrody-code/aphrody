<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody-code contributors
-->

# Changelog

All notable changes follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `aphrody-wasm` bridge crate — npm-publishable `@aphrody-code/aphrody-wasm`
  (89 KB wasm + 12 KB JS, SIMD + bulk-memory release profile) (a6132a789, 8ff3dc592).
- `aphrody-translate` crate — comment translator EN→FR with AI-isms scrub and
  Aphrody style enforcement, dual wasm32 target green (e3c4bf92c, 4e8d3f7c7).
- `mrx` workspace migration — Monorepo Real-time X-platform mapper
  (`mrx-core`, `mrx-detect`, `mrx-audit`, `mrx-watch`, `mrx-cli`) (11a718bad).
- `ai.json` universal A2A coordination manifest v1 + reconciliation with
  `a2a-client`/`a2a-pb` wasm32 port (9a44204d2, 650f4ae0b).
- `ievr-serve` and `ievr-verify` wrappers for the 2/5 light gate (c69472e46).
- Rich `aphrody --version` output via `build.rs` — commit SHA, target triple,
  profile, A2A protocol version (5e6e89bb6).
- Packaging templates: Debian/Ubuntu `.deb` (81b5a7179), winget manifests
  schema 1.6.0 (e9c592908), scoop + homebrew templates (8ff3dc592),
  Windows Terminal profile fragment (f00a1c815), one-line install scripts
  for Linux/macOS/Windows (32b04690b).
- `BENCHMARKS.md` — mrx scan real numbers (19k files / 1.4s) (a24b887bf).
- `CONTRIBUTING.md` + `CODE_OF_CONDUCT.md` for OSS launch readiness (cf9bcadd8).
- `.mcp.json` team-wide MCP server config + `docs/MCP_SETUP.md` (15be649d1).
- `/a2a-duel-loop` skill — sustained 2-Claude coordination tick (8bcbbbd97).
- `/aphrody-yolo-grind` skill + `yolo-prod-ready` agent for 4-agent parallel
  grind (50bbba056).
- `docs/SOURCE_OF_TRUTH.md` consolidated reference (efa4477e7).
- Workspace deps pinned for `ievr-engine` handoff — `glam=0.30`,
  `bytemuck=1.25` (e55e37d93).
- D+14 technical post draft on `ai.json` A2A coordination (7343772de).
- 6 institutional-memory `CLAUDE.md` entries from `/start` session (09c141508,
  340387913).
- Public endpoints catalog — azalee GraphQL, Steam APIs, inagle (f369320ea).

### Changed

- Pivot from `google-cli` to `aphrody` — ultimate cross-platform CLI
  (efa4477e7, 3f3e64734).
- Workspace package renamed `cli` → `aphrody` (78ec50380).
- Produced binary renamed `cli` → `aphrody` (95bd8535f).
- README headlines mrx as the demo angle + live demo block above the fold
  (ea0796125, d89bcb8f3).
- README tagline sharpened — wasm32 surface + hermetic supply chain
  (a9b319f94, b77453755).
- `mrx-audit` output is now generic monorepo (dropped `vps/` hardcoding)
  (facdba039).
- `sccache` is opt-in via env var instead of hard-coded in cargo config
  (fb42cc29a).
- Cross-platform wasm target alias switched `wasm32-wasi` → `wasm32-wasip1`
  (a1d5d97ca).
- Workspace-wide repo metadata + SPDX headers + a2a-pb OUT_DIR layout +
  a2a-client wasm SSE transport (696072f71).
- Adopted `aphrody-code/.github` org standards + anonymized legacy identity
  refs (d045abc1d).
- Permissions widened, blocking hooks removed, generous timeouts (aa9d588b7).
- Root files refreshed to align with session standards (bc6a5b1cb).

### Fixed

- `aphrody --version` panicked under rustls 0.23 — `CryptoProvider` is now
  installed at boot (8859ca785).
- Chromium forensics cfg-gated to Windows — unblocks Linux build (d222d0061).
- Workspace clippy + deny + wasm matrix restored to green baseline post-pivot
  (3cd489eff, 13514d98d).
- `mrx-audit` formatting sweep after refactor (53f59d054).

### Build / CI

- 3 new fail-fast gates: `cargo machete`, binary smoke test, `cargo vet`
  (0372fe57c).
- 11 dead deps removed via `cargo machete` cleanup (c58bc25d6).
- `--icf=all` dropped from linux-gnu rustflags (98cd26e43).
- Lint job pinned to native Linux — fix cross-compile MSVC fallout
  (5034858c9).
- `sccache` disabled temporarily — GHA cache backend 503ing (959e97297).
- Supply-chain job unblocked — orphan policy dropped, machete non-blocking
  (3100fcc5a).
- MSVC `-W0` CFLAGS gated to per-target — fix Linux/WSL builds (6c21523da).
- `docs.yml` trigger fixed to `main` + cross-link post + PLAN sync
  (39385737b).
- `set-github-token` script + hardened `.gitignore` for secrets (347e31780).
- nextest 387/387 green on Windows native in 2.914s (e594d2fad).
- WSL Ubuntu nightly 1.97 cli local-green for P-Linux (6df75032b).
- 4-agent grind tick — google_mcp Linux + hygiene + binary smoke + wasm npm
  (bf283025f).

### Security

- SPDX-License-Identifier: Apache-2.0 headers + workspace metadata
  inheritance across all crates (696072f71).
- Steam screenshot leak removed from publication batch + `steam*.png`
  gitignored at repo root (4298f78ea, 4b76f2b56).
- Steam IEVR monitor script moved off-scope to `C:\winclean\`
  (a48cd92b9).
- Stale `.clinerules` removed — contradicted post-pivot `CLAUDE.md`
  (51d67cee5).

### Removed

- `crates/google_os/` archived to `C:\google-os-archive\` (pivot scope cut)
  (efa4477e7).

[Unreleased]: https://github.com/aphrody-code/aphrody/compare/HEAD
