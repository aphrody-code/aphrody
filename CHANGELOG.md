<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody-code contributors
-->

# Changelog

All notable changes follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `aphrody doctor` subcommand — environment health check with rustls
  `CryptoProvider` probe, `ai.json` schema parse, `.well-known` pointer
  check, HTTP listener ping on `:8788`, cargo-vet audit count, peer
  heartbeat freshness, DEGRADED verdict surface (commit 2a721d2).
- `aphrody doctor --json` machine-readable output mode via `serde_json`
  (commit 96ae82e).
- `aphrody completions <shell>` subcommand for bash/zsh/fish/powershell
  shell completion script generation (commit 588cd25).
- `docs/ARCHITECTURE.md` — workspace topology, dependency flow, target
  matrix, A2A coordination boundary (staged, no commit yet).
- `docs/ADR/0000-template.md` Michael Nygard ADR template plus 3 ratified
  ADRs: `0001-cross-platform-rust.md`, `0002-a2a-file-based.md`,
  `0003-yolo-parallel-grind.md` (commit b228509).
- `docs/ROADMAP.md` (477 words, Q2 2026 to Q1 2027 horizon) and
  `docs/FAQ.md` (672 words, 12 questions) for OSS-launch readiness
  (commit b228509).
- `docs/COMPARISON.md` — 11-row capability table vs `just` / `taskfile`
  / `gh` / `devcontainer` / `asdf` (commit b228509).
- `docs/launch/SHOW-HN.md` — 5 title candidates, pre-launch checklist
  (commit b228509).
- `docs/posts/2026-05-yolo-grind-loop.md` — D+14 milestone post #2,
  2001 words on the parallel-grind loop (commit b228509).
- `docs/extensions/honest-delivery-v1.md` ai.json channel extension —
  FAIT / NON_FAIT / INCOMPLET classification protocol (staged, no commit
  yet).
- `docs/extensions/file-transport-v1.md` ai.json channel extension —
  JSONL mailbox + HTTP listener wire format (staged, no commit yet).
- `docs/extensions/context7-version-pinning-v1.md` ai.json channel
  extension — library version pin handshake (staged, no commit yet).
- `docs/extensions/index.md` — registry of all published ai.json
  channel extensions (staged, no commit yet).
- `crates/aphrody-wasm/examples/browser-playground.html` — 584-line
  interactive wasm demo with import-map, runs offline (commit b228509).
- `crates/aphrody-wasm/examples/README.md` + `crates/aphrody-wasm/README.md`
  npm-publishable usage docs (commit b228509).
- Per-crate `README.md` files for `base`, `backend`, `google_mcp`,
  `mrx-core`, `mrx-detect`, `mrx-audit`, `mrx-watch`, `mrx-cli`, plus
  `a2a-lf` (staged, no commit yet); updates to `a2a`, `a2a-client`,
  `a2a-grpc`, `a2a-pb`, `a2a-server`, `aphrody-translate` READMEs
  (staged, no commit yet).
- `crates/cli/tests/doctor.rs` — 6 integration tests using `assert_cmd`
  + `predicates`, nextest exit 0, DEGRADED exit 0, JSON deserialise
  verified (commit b228509).
- `crates/backend/benches/backend_bench.rs` — 180-line criterion bench
  suite for the backend crate (commit b228509).
- `.github/workflows/codeql.yml` — 70-line CodeQL security scan, on
  push/pr to main, weekly cron (commit b228509).
- `.github/workflows/release-please.yml` + `release-please-config.json`
  + `.release-please-manifest.json` — automated semver release + tag +
  changelog ladder (staged, no commit yet).
- `.github/workflows/dependabot-auto-merge.yml` — `pull_request_target`
  trigger gated on `github.actor == 'dependabot[bot]'`, auto-merge
  semver-patch and semver-minor dev deps, 9-entry deny-list (commit
  ca73ee2).
- `.devcontainer/devcontainer.json` — 60 lines, bun + zig + gh features,
  port 8788 forwarded, rust-analyzer + bun-vscode extensions (commit
  b228509).
- `packaging/snap/snapcraft.yaml` Ubuntu Snap core24 + `packaging/arch/PKGBUILD`
  Arch AUR template — combined ~85% Linux desktop reach with the existing
  `.deb` (commit b228509).
- `packaging/nix/flake.nix` + `packaging/nix/README.md` Nix flake template
  (staged, no commit yet).
- `packaging/flatpak/com.aphrody.aphrody.json` + `packaging/flatpak/README.md`
  Flatpak manifest (staged, no commit yet).
- `assets/aphrody-doctor-demo.cast` — 111-line asciinema v2 cast,
  3-second runtime, 100x30 terminal (commit b228509).
- `assets/aphrody-demo.cast` + `assets/aphrody-demo.cast.README` +
  `assets/README.md` play/embed instructions (commit 4b3bdce).
- `supply-chain/imports.lock` regenerated (2115 lines) for cargo-vet
  imports synchronisation (commit 96ae82e).
- `SECURITY.md` strengthened — supported versions, GHSA + email reporting
  channel, safe-harbor clause, 437 words; new `SECURITY-HALL-OF-FAME.md`
  + README cross-link (commit b228509).
- `CHANGELOG.md` initial population in Keep-a-Changelog 1.1.0 format
  (commit ca73ee2).
- `aphrody-perfect-grind` skill — forced-loop wrapper around
  `aphrody-yolo-grind` with 15-gate perfection oracle (commit 4fb5793).
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

- `supply-chain/audits.toml` + `supply-chain/config.toml` cargo-vet
  formatting drift cleared, 5 attestations + 7 import URLs preserved
  (commit b228509).
- `crates/cli/src/commands.rs` `DoctorCommand` body shipped — 493 lines
  of helpers + variant + match-arm dispatch (commit 588cd25).
- `crates/cli/src/main.rs` Commands enum gains `Doctor { json }` arm
  wired into Pre + Post dispatch tables (commit 2a721d2).
- `.well-known/ai.json` `kind` aligned to `a2a.AgentCard`; root `ai.json`
  gains `schema_version: "1.0.0"` + `kind: "a2a.CollaborationManifest"`
  (commit 96ae82e).
- 7 CLEAN crates flipped `publish = true` for crates.io ladder: `base`,
  `a2a-lf`, `a2a-pb`, `a2a-client-lf`, `a2a-server-lf`, `a2a-grpc`,
  `aphrody-translate`; `backend` deferred pending base publish
  (commit 588cd25).
- `scripts/gen_summary.ts` mirrors root `CHANGELOG`, `CONTRIBUTING`,
  `SECURITY`, `CODE_OF_CONDUCT`, `BENCHMARKS` into `docs/_root/` so
  mdBook surface picks them up (commit 588cd25).
- `packages/ui/scripts/build-all.ts` + `scripts/scraper/Crawler-Pipeline.ts`
  imports migrated `node:` bare `→` `node:` prefix per bun-only policy
  (commit b228509).
- README adds CHANGELOG cross-link above the fold + CONTRIBUTING gains
  `[Unreleased] update per PR` hard-rule (commit 4b3bdce).
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

- `mrx workspace_key` Windows path-separator bug — scanner blind to
  `crates/` Rust side on `\` separators; honest-delivery audit at
  `docs/audits/2026-05-17-mrx-aggressive.md` (commit b228509).
- `aphrody doctor` heartbeat freshness label was inverted (`>TTL` said
  "fresh"); inbox path resolution corrected to
  `C:/winclean/.coord/inbox-from-winclean.jsonl` (was aphrody repo root)
  (commit 96ae82e).
- `aphrody --version` panicked under rustls 0.23 — `CryptoProvider` is now
  installed at boot (8859ca785).
- Chromium forensics cfg-gated to Windows — unblocks Linux build (d222d0061).
- Workspace clippy + deny + wasm matrix restored to green baseline post-pivot
  (3cd489eff, 13514d98d).
- `mrx-audit` formatting sweep after refactor (53f59d054).

### Security

- `.github/workflows/codeql.yml` CodeQL security analysis pipeline —
  push + pr + weekly cron (commit b228509).
- `docs/extensions/honest-delivery-v1.md` A2A extension formalising
  FAIT/NON_FAIT/INCOMPLET delivery classification so security claims
  cannot be silently degraded (staged, no commit yet).
- `SECURITY.md` strengthened with safe-harbor clause, GHSA channel,
  `security@aphrody.dev` mailbox, and supported-version policy
  (commit b228509).
- SPDX-License-Identifier: Apache-2.0 headers + workspace metadata
  inheritance across all crates (696072f71).
- Steam screenshot leak removed from publication batch + `steam*.png`
  gitignored at repo root (4298f78ea, 4b76f2b56).
- Steam IEVR monitor script moved off-scope to `C:\winclean\`
  (a48cd92b9).
- Stale `.clinerules` removed — contradicted post-pivot `CLAUDE.md`
  (51d67cee5).

### Build / CI

- `.github/workflows/cross-platform.yml` gains `linux-native` job —
  ubuntu-latest, build release `-p aphrody --locked`, nextest, smoke
  `aphrody --version` / `--help` / `doctor`; closes 3 unblocked PLAN
  items (commit 96ae82e).
- `release.yml` + `coverage.yml` + `docs.yml` rust-toolchain pinned to
  SHA `5b842231ba77f5c045dba54ac5560fed2db780e2` for reproducibility
  (commit 588cd25).
- `release.yml` cargo-auditable wraps zigbuild on cross targets — 4/8
  cross binaries now SBOM-bearing (commit 588cd25).
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

### Internal

- `docs/audits/2026-05-17-n2b-scan.md` — 117 lines, n2b scan baseline
  (commit b228509).
- `docs/audits/2026-05-17-mrx-aggressive.md` — 183 lines, exposes mrx
  scanner blind-spot on Windows `\` workspace key (commit b228509).
- `docs/audits/2026-05-17-bxc-scrape-request.md` — A2A ask envelope
  `apx-ask-bxc-scrape-1` POSTed to peer `:8788`, appended to
  `inbox-from-aphrody.jsonl` (commit b228509).
- A2A coordination protocol documented in `docs/posts/2026-05-ai-json.md`
  and `docs/posts/2026-05-yolo-grind-loop.md` (commits 7343772, b228509).
- `aphrody v0.2.0` plugin — 2 new skills (`rust-target-check`,
  `m3-component`), 2 new opus agents (`cross-platform-validator`,
  `m3-spec-auditor`), 3 PostToolUse hooks (`cargo-check`,
  `cargo-toml-validate`, oxlint merged) (commit 274eb66).

### Removed

- `crates/google_os/` archived to `C:\google-os-archive\` (pivot scope cut)
  (efa4477e7).

[Unreleased]: https://github.com/aphrody-code/aphrody/compare/HEAD
