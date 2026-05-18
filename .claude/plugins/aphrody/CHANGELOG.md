<!-- SPDX-License-Identifier: Apache-2.0 -->
# Changelog — aphrody plugin

All notable changes to the `.claude/plugins/aphrody/` plugin. Format:
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning:
[SemVer](https://semver.org/spec/v2.0.0.html).

## [0.4.0] — 2026-05-19

### Added
- **NEW unified MCP server `aphrody`** under
  `mcp/aphrody/` — Bun stdio server fusing **14 tools** :
  - 7 scraping tools proxied to the `bxc-mcp` Rust subprocess
    (lazy-spawned on first scraping call) : `aphrody_scrape`,
    `aphrody_recon`, `aphrody_detect`, `aphrody_search`,
    `aphrody_atlas_route`, `aphrody_extract_structured`,
    `aphrody_vision_analyze`.
  - 3 in-process SQLite memory tools (no daemon) :
    `aphrody_memory_set`, `aphrody_memory_get`, `aphrody_memory_list`.
  - 4 aphrody CLI exec wrappers : `aphrody_doctor`, `aphrody_version`,
    `aphrody_dns`, `aphrody_notify`.
- MCPB manifest (`mcp/aphrody/manifest.json`, v0.4) so the server can be
  packed standalone with `npx @anthropic-ai/mcpb pack` and dropped onto
  Claude Desktop. 4 `user_config` settings exposed (bxc daemon URL,
  memory DB path, aphrody binary, bxc-mcp binary).
- `mcp/aphrody/README.md` with full architecture diagram, tool catalogue,
  install instructions (plugin and MCPB), env var table, smoke tests,
  and security notes.

### Changed
- **`plugin.json` MCP block** : `bxc-scrapper` + `bxc` (dual stdio
  servers) replaced by a single `aphrody` stdio server pointing to
  `${CLAUDE_PLUGIN_ROOT}/mcp/aphrody/server/index.ts`. `github` +
  `context7` cloud servers kept as-is.
- Version 0.3.1 → 0.4.0 (one-server unification = breaking change for
  any external consumer that referenced the old MCP names).
- Plugin description updated to mention the unified surface and the
  14-tool fusion.

### Removed
- Standalone `bxc-scrapper` and `bxc` MCP entries from `plugin.json`.
  Both functionalities are now served by the unified `aphrody` server.
  (The `bxc-mcp` Rust binary and `packages/bxc/` Bun extension are still
  used — just composed inside the new server, not exposed as separate
  MCPs.)

### Validation
- `bun install` succeeds (93 deps, 7.5 s).
- `bun run server/index.ts --list-tools` returns 14 tools with valid
  JSON Schema draft-07 inputSchemas.
- Full MCP handshake roundtrip OK (`initialize` →
  `notifications/initialized` → `tools/list` → 14 tools listed →
  `tools/call name=aphrody_version` → real binary output).
- `aphrody_memory_set` + `aphrody_memory_get` roundtrip OK on
  `$HOME/.aphrody/aphrody-memory.sqlite`.

## [0.3.1] — 2026-05-19

### Added
- `CHANGELOG.md` (this file).
- `commands/status.md` : full frontmatter (description, allowed-tools,
  argument-hint, model) + structured Steps + anti-stub rules.
- `agents/aphrody-cli.md` : promoted to canonical entrypoint agent
  (frontmatter conform to plugin-dev `agent-development` standard with
  "When to invoke" prose section).
- `commands/scrape.md` + `commands/tokens.md` : `model: sonnet`
  frontmatter field.
- README.md : full rewrite — accurate component counts, install
  instructions, env var table, validation steps, troubleshooting.

### Changed
- `plugin.json` : MCP `bxc` (Bun) server `command.args[1]` now uses
  `${CLAUDE_PLUGIN_ROOT}/../../../packages/bxc/...` (was hardcoded
  `C:/src/aphrody/packages/bxc/...`). Same for `env.BXC_MEMORY_DB`.
  Portable across `git clone` locations.
- `plugin.json` : version bumped 0.3.0 → 0.3.1.
- `plugin.json` : description rewritten to spell out the bxc auto-start
  + `/api/*` routes + agent / MCP counts.

### Fixed
- `commands/status.md` : was a placeholder with no frontmatter and no
  steps — now renders a one-screen project status report (binary,
  plugin, branch, PLAN ⏳, A2A peer, bxc daemon, toolchain).

## [0.3.0] — 2026-05-19

### Added
- `agents/aphrody-cli.md` (NEW) : 130+ line catalogue of the 27 aphrody
  CLI sub-commands with workflow, anti-stub rules, delegation map.

### Changed
- `plugin.json` : version 0.2.0 → 0.3.0, description rewritten.
- `plugin.json` : MCP `bxc-scrapper` command changed from
  `cargo run --release -p bxc-engine --bin bxc-mcp` to plain `bxc-mcp`
  (binary expected on PATH). Cuts 3+ min cold-rebuild on first MCP load.
- `plugin.json` : MCP `bxc` (Bun) path fixed from dead
  `C:/worktree/bxc/packages/bxc-extension/server.ts` → in-tree
  `C:/src/aphrody/packages/bxc/packages/bxc-extension/server.ts`
  post-fusion 2026-05-19 (later promoted to `${CLAUDE_PLUGIN_ROOT}`
  in 0.3.1).
- `commands/scrape.md` : routed through `aphrody bxc recon` +
  `aphrody bxc detect` + `aphrody scrape --selector` instead of
  `mcp__bxc-scrapper__bxc_*`. Daemon auto-starts.
- `commands/tokens.md` : routed through `aphrody tokens --url … --output
  … --force` instead of `mcp__bxc-scrapper__extract_structured`.
- `agents/n2b.md` : new first-choice resolution `aphrody n2b` (was
  `command -v n2b` only). Standalone `n2b` binary demoted to fallback.
- `agents/n2b-ultra.md` : same — `aphrody n2b` PREFERRED, workspace
  crate (`cargo run -p n2b-cli`) second, `bunx @aphrody-code/n2b-cli`
  third.

### Removed
- 12 dead agent references from `plugin.json` agents list :
  `bun-api`, `bun-deployer`, `bun-dreamer`, `bun-explorer`, `bun-native`,
  `bun-reviewer`, `bun-runner`, `bun-wasm`, `bun-web-api`,
  `nextjs-developer`, `typescript-pro` (files never existed on disk).

### Validation
- `plugin.json` parses (bun JSON.parse OK).
- 27/27 agent refs exist on disk (was 22/35 with 13 dead/missing refs).
- `bxc-mcp --list-tools` returns 7-tool catalog.
- `aphrody scrape --selector "h1" https://example.com` works live
  end-to-end (auto-spawns bxc Bun daemon, returns
  `{matches:[{index:0,text:"Example Domain"}], selector:"h1", url:"https://example.com"}`).

## [0.2.0] — 2026-05-17

### Added
- Initial public release scaffolded under `.claude/plugins/aphrody/`.
- 3 skills : `pixel-perfect`, `rust-target-check`, `m3-component`.
- 3 agents : `n2b-ultra`, `cross-platform-validator`, `m3-spec-auditor`.
- 2 slash commands : `/scrape`, `/tokens`.
- 3 PostToolUse hooks : `oxclint`, `cargo-check`, `cargo-toml-validate`.
- MCP server `bxc-scrapper` (Bun + TS at the time).

### Known issues (carried into 0.3.x)
- MCP `bxc-scrapper` originally implemented as TS server (`server.ts`)
  — later replaced with Rust `bxc-mcp` binary (0.3.0).
- README listed only the original 3 skills + 3 agents — corrected in
  0.3.1.

[0.3.1]: https://github.com/aphrody-code/aphrody/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/aphrody-code/aphrody/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/aphrody-code/aphrody/releases/tag/v0.2.0
