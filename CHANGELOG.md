<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody-code contributors
-->

# Changelog

All notable changes follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Merged the standalone `material-web` Material Design 3 monorepo into
  this repo (2026-06-01).** The 9 `@aphrody-code/*` packages
  (`material-web`, `react`/m3-react, `m3-tokens`, `m3-motion`, `m3-theme`,
  `m3-design`, `eslint-plugin-m3`, `doc-ai`, `bun-rs`) + `examples/showcase`
  are now Bun + Turborepo workspace members under `packages/*`; root
  `package.json` gained the shared catalog + `patchedDependencies`; `bun-rs`
  is excluded from the Cargo workspace; GitHub Packages publishing kept via
  `.github/workflows/release-m3-packages.yml` (tag `m3-v*`).
- **`apps/web`** — public consumer client (React + `@aphrody-code/m3-react`
  + TanStack Router/Query, Bun-native `Bun.serve` + `bun build`), powered by
  custom RAG/LLMs (shenron, rpbey). Includes a React port of the Angular
  admin (`apps/desktop`) under `/a` (assistant, dashboard, skills, mcp,
  commands, reverse, forensics, network, diagnostic, settings, about).
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

## [Unreleased] - 2026-05-18

Mega-day batch — 5 commits, 11+ delivery lanes, terminal LLM-first foundation,
WASM/M3 stack, integration matrix, kernel subcommands, voice WASM, gemini-app
port, pipeline optimization.

### Added

- `aphrody n2b [args]` kernel subcommand — façade over `packages/n2b/src/cli.ts`
  via bun spawn, `aphrody n2b watch --interval N` tokio infinite loop
  (commit 2c1602cb).
- `aphrody bxc {daemon,recon,scrape,detect,tokens}` kernel subcommand —
  passthrough to bxc-engine via `ScrapeClient`, daemon mode uses
  `DETACHED_PROCESS` on Windows (commit 2c1602cb).
- `aphrody term [--addr --shell --cwd]` subcommand — binds
  `aphrody_terminal_backend::serve()` (default `127.0.0.1:8788`)
  (commit 77ddbff8).
- `crates/aphrody-terminal-vt` NEW (695+ lines) — vte parser, ScreenBuffer,
  Cursor, Color (RGB + presets), Attr (bold/italic/underline/inverse/blink),
  SGR 16-color, CSI CUU/CUD/CUF/CUB/CUP/ED/EL, LF/CR/BS/HT/BEL/scroll.
  11/11 tests pass (commit 9355ddc5).
- VT Ink/React essentials in `crates/aphrody-terminal-vt` — alt-screen DECSET
  1049, mouse SGR 1006, true color 24-bit, OSC 52 clipboard, bracketed paste
  2004, focus events 1004, DECSTBM, OSC 0 title; modules `alt_screen.rs`,
  `mouse.rs`, `osc.rs` (uncommitted, staged in working tree).
- `crates/aphrody-terminal-wasm` NEW (339+ lines) — wasm-bindgen DOM mount,
  M3 colors via m3-tokens, keyboard ANSI mapping (arrows, Enter, BS, Tab),
  one span per cell, dirty-row rerender (commit 9355ddc5).
- `crates/aphrody-terminal-backend` NEW (287 lines) — portable-pty
  (ConPTY/openpty) + tokio-tungstenite WS server + JSON resize protocol +
  cross-platform shell autodetect (commit 9355ddc5).
- `crates/aphrody-terminal-llm` NEW (1060 lines, 13 tests) — EventBus tokio
  broadcast + SubAgent/McpStatus/Skill/TaskTree registries + HookEventLog
  ring buffer 1000-cap + OSC parser for 7 `aphrody-*` LLM event sequences
  (commit 77ddbff8); MCP probe loop rewrite (708 lines), `McpServerSpec`,
  `McpTransport` (Stdio/Http), `OAuthConfig`, `probe_server()`,
  `probe_loop()`, `load_mcp_json()` (compat `.mcp.json` schema),
  `default_server_specs()` for bxc + google_mcp (commit 04a5f676).
- `crates/aphrody-terminal-browser` NEW (1744 lines) — LLM↔DOM bridge with
  3 pluggable backends (bxc spawn / agent-browser RPC stdio / edge headless
  fallback) + OSC parser for 7 `aphrody-browser-*` sequences (commit 77ddbff8).
- `crates/aphrody-terminal-wasm/src/coord_pane.rs` NEW (369 lines) — embeds
  `a2a-ui::Envelope::parse_jsonl` + `render_envelope_list`, 2s polling
  delta-append (commit 04a5f676).
- `crates/aphrody-terminal-markdown` NEW (uncommitted) — comrak CommonMark
  + syntect highlight + OSC `aphrody-md` detector.
- `crates/aphrody-terminal-json-out` NEW (uncommitted) — JSONL framing
  stdout/stderr + passthrough app-JSON + base64 binary.
- `crates/aphrody-terminal-config` NEW (uncommitted) — schemars
  `~/.aphrody/terminal.json` + claude.json/mcp.json/settings.json import
  shims + merge precedence.
- `crates/aphrody-tui` NEW (uncommitted) — pure Rust ratatui-style DSL,
  60fps target.
- `crates/a2a-ui/src/native/` + `examples/a2a-tui.rs` — ratatui TUI feature
  `native` (default off, preserves WASM cdylib), header peer status +
  envelope list + detail JSON + footer keymap; ratatui added to
  `workspace.dependencies` (commit 04a5f676).
- `packages/aphrody-jsx/` NEW (27 files, 2503 LOC) — Bun-native React
  reconciler emitting `aphrody-jsx-*` OSC sequences, 12/12 tests, hello
  example emits mount opcode (727 bytes) (commit d8863d5b).
- `packages/gemini-app-aphrody/` NEW (27 files, 2591 LOC, 13/13 tests) —
  port of Next.js gemini.google.com/app, link:next resolved to fork
  next@16.3.0-canary.2, M3 tokens + WebGPU gradient + voice hooks
  (commit 2c1602cb).
- `crates/cli/src/auto_command.rs` NEW (uncommitted) — autonomous CLI agent
  NL → A2A streaming via JsonRpcTransport.
- `crates/base/benches/base_bench.rs` NEW (uncommitted) — criterion suite,
  6 benches across vfs and aes-gcm.
- `crates/a2a-client/src/transport.rs` — `TransportKind` enum cross-platform
  observability (NativeHyper / BrowserFetch / Unsupported) (uncommitted).
- `crates/aphrody-wasm/examples/aphrody-terminal-demo.html` NEW (1361 lines,
  uncommitted) — pixel-perfect M3 showcase 8 panes, HTTP 200 verified.
- `scripts/Install-AphrodyToPath.ps1` + `scripts/install-aphrody-path.sh` —
  auto-install binary into PATH (Windows HKCU / Linux `$HOME/.local/bin`),
  `-BuildIfMissing` / `--build` flags (commit 2c1602cb).
- `scripts/n2b-batch.{ps1,sh}` (168 + 157 LOC) — parallel migration
  (`ForEach-Object -Parallel` / `xargs -P`), NDJSON streamable p50/p95
  metrics (commit 2c1602cb).
- `scripts/bxc-crawl.{ps1,sh}` (262 + 267 LOC) — parallel crawl URLs ×
  actions + `--loop --interval` + body-hash cache (commit 2c1602cb).
- `scripts/bxc-supervise.{ps1,sh}` (124 + 128 LOC) — watchdog daemon NDJSON
  heartbeats + auto-restart, SIGINT trap, exit codes 0/1/2/130
  (commit 2c1602cb).
- `scripts/check-worktrees.ts` + `scripts/setup-worktrees.ts` — worktrees
  catalogue 15-entry bootstrap, exit 0 / 15 (commit 9355ddc5).
- `docs/PLAN-MOONSHOT.md` NEW (680 lines) — mining 13 worktrees + 30-day
  star arc + top-50 punch list + risk register (commit 9355ddc5).
- `docs/design/aphrody-terminal-spec.md` NEW — normative LLM-first spec:
  5 pillars (JSON out / markdown inline / JSON config / sub-agent+MCP+
  hooks+skills first-class / Ink-TUI compat), 9-crate stack, 22 Ink-
  essential sequences, 14 `aphrody-*` OSC extensions (commit 9355ddc5);
  extended with Ink/React-TUI 3-layer fusion strategy + 6 new
  `aphrody-jsx-*` OSC sequences (commit 77ddbff8).
- `docs/design/aphrody-terminal-integration-matrix.md` NEW (108 lines) —
  28-crate workspace ↔ aphrody-terminal contract, 5 wired / 22 ⏳
  T-INT-* tickets / 1 N/A (commit 77ddbff8).
- `docs/audits/2026-05-18-gemini-app-port-audit.md` NEW — surface map 14
  entries + asset reuse audit (commit 2c1602cb).
- `docs/audits/2026-05-18-wterm-vs-microsoft-terminal-vs-aphrody-terminal.md`
  NEW (uncommitted) — 3-way cross-reference audit.
- `docs/audits/2026-05-18-plan-status-audit.md` NEW (101 lines) — PLAN.md
  19 → 11 ⏳ flip evidence with file:line proofs (commit 04a5f676).
- `docs/audits/2026-05-18-dedup-cohesion-sweep.md` NEW (uncommitted).
- `docs/cargo/PIPELINE-OPTIMIZATION.md` NEW (236 lines) — pattern extraction
  + baseline + applied + future opt-in (commit 2c1602cb).
- `docs/cargo/BUILD-SPEED.md` NEW (126 lines) — build-speed audit
  (commit 2c1602cb).
- `.mcp.json` — bxc MCP server entry (stdio via bun, 7 tools:
  `tune_memory_sqlite`, `vision_analyze`, `start_scraping_subagent`,
  `auto_detect_skills`, `bxc_cdp_{snapshot,evaluate,logs}`);
  `BXC_MEMORY_DB` → `var/data/` gitignored; smoke test green
  (commit 9355ddc5).
- Voice WASM real implementations: `crates/aphrody-voice/src/web.rs`
  (125 lines, WebSpeechSynth via web-sys SpeechSynthesis) +
  `crates/aphrody-voice-stt/src/web.rs` (162 lines, WebSpeechRecognition
  via `js_sys::Reflect` with webkit prefix), 11 methods total
  (commit 2c1602cb).
- 5 new cargo aliases in `.cargo/config.toml`: `dev-fast`, `lint-fast`,
  `bench-fast`, `build-fast`, `test-fast` (commit 2c1602cb).

### Changed

- `CLAUDE.md` mission-day rewrite (281 → 337 lines) — §0.5 PLAN ⏳ matrix
  (14 items × crate × fusion sources × verify command), §1 scaffold
  interdit (feature observable required), §4 kernel subcommands +
  install scripts, §4.1 high-perf scripts + bunnize template + pwsh
  gotchas, §7 verify=observable rule, §7.6 YOLO grind default workflow
  (commit 2c1602cb); §7.5 new aphrody-terminal LLM-first section + spec
  ref + worktrees ref + bxc MCP ref + 22 Ink essentials + 14 OSC
  extensions (commit 9355ddc5).
- `.github/workflows/cross-platform.yml` — sccache reactivated + v0.0.5
  unified + lint split fmt/clippy parallel + graceful `--show-stats || true`
  + bun cache lockb fallback (commit 2c1602cb).
- `.github/workflows/build.yml` — sccache env + action wiring +
  cargo-deny `RUSTC_WRAPPER` unset (commit 2c1602cb).
- `.github/workflows/bench.yml` — sccache + `benchmark-action/github-action-benchmark@v1`
  with 150% regression threshold (commit 2c1602cb).
- Workspace dependencies extended for new terminal stack: vte,
  portable-pty, tokio-tungstenite, ratatui, comrak, syntect, schemars,
  wasm-bindgen-futures, web-sys SpeechSynthesis/SpeechRecognition
  (commits 9355ddc5, 77ddbff8, 04a5f676).
- `aphrody-code/next.js@aphrody` fork bunisé: 54 → 0 pnpm in root
  scripts, `pnpm-lock.yaml` (38483 lines) → `bun.lock` (12762 lines),
  workspaces hoisted (related to commit 2c1602cb gemini-app).
- Worktrees catalogue 13 → 15 (`vercel-labs/wterm` 40MB API ref +
  `microsoft/terminal` 114MB algo ref), budget ~1095 MB (commit 9355ddc5).
- `docs/INDEX.md` (+12 entries, new §2.1 Terminal & design),
  `README.md` Documentation refs PLAN-MOONSHOT + spec + matrix,
  `docs/terminal/README.md` 3 forward-links design + matrix + moonshot
  (commit 04a5f676).
- `docs/SUMMARY.md` regenerated via `cargo run -p aphrody-summary`
  (commits 04a5f676, 2c1602cb).
- `docs/PLAN.md` audit: 19 → 11 ⏳ (8 flips ⏳→✅ with file:line proofs),
  PLAN-MOONSHOT 5 ⏳ legit (external registries not-yet-published)
  (commit 04a5f676).

### Fixed

- `crates/cli/Cargo.toml` — drop unused `tracing` dependency
  (cargo machete cleanup) (commit d8863d5b).
- `crates/a2a-slimrpc/Cargo.toml` — drop unused `prost-types` dependency
  (commit d8863d5b).
- `crates/aphrody-voice/Cargo.toml` + `crates/aphrody-voice-stt/Cargo.toml`
  — native items gated `not(target_arch = "wasm32")`; wasm32 deps gated
  in Cargo.toml; `cargo-machete ignored` for wasm-bindgen and friends
  (commit 2c1602cb).

### Performance

- CI pipeline sccache cold-fill ~3m42s (was 8-12 min), warm 3.2s (-92%)
  (commit 2c1602cb).
- `base` benches p50: `vfs_resolve_hit_short` 233 ns,
  `aes_gcm_decrypt_64b` 247 ns, `aes_gcm_decrypt_64kib` 43.4 µs
  (uncommitted criterion suite).

### Internal

- Mega-commit 2c1602cba — 62 files changed, +5715 / -70 lines
  (kernel n2b/bxc + scripts + pipeline + voice WASM + gemini-app +
  CLAUDE.md mission-day rewrite).
- 4-lane parallel YOLO grind delivery model formalised in
  `CLAUDE.md` §7.6 (default workflow going forward).
- Honest-delivery footers (FAIT / INCOMPLET / NON_FAIT) applied to
  every commit body per ai.json `honest-delivery-v1` extension.

[Unreleased]: https://github.com/aphrody-code/aphrody/compare/HEAD
