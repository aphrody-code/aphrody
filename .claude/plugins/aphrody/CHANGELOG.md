<!-- SPDX-License-Identifier: Apache-2.0 -->
# Changelog

## 0.8.1 - 2026-06-03

### Added

- VPS agent-stack docs sync; `bxc_xpro_deck` MCP (bxc 0.6.1); X Pro + Radar via `@aphrody-code/x` 1.0.6.

# Changelog — aphrody plugin

All notable changes to the `.claude/plugins/aphrody/` plugin. Format:
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning:
[SemVer](https://semver.org/spec/v2.0.0.html).

## [0.7.0] — 2026-05-19 — **single-MCP-server architecture**

### Changed (BREAKING)
- **`plugin.json#mcpServers` is now a single entry (`aphrody`).** The
  previous `microsoft-learn` HTTP MCP entry was removed. Microsoft
  Learn coverage is preserved via three native Rust tools fused into
  the unified `aphrody-mcp` binary.
- **All references `mcp__microsoft-learn__*` were rewritten to
  `mcp__aphrody__*`** across the imported skills
  (`microsoft-docs`, `microsoft-code-reference`, `skill-creator`,
  `context7-mcp`), the imported agent (`docs-researcher`), and the
  skill-templates reference.

### Added
- **Microsoft Learn ported to native Rust inside `aphrody-mcp`.** Three
  new tools wrap the upstream HTTP MCP at
  `https://learn.microsoft.com/api/mcp` via stateless JSON-RPC over
  HTTP with SSE response framing :
  - `microsoft_docs_search(query)`
  - `microsoft_docs_fetch(url)`
  - `microsoft_code_sample_search(query, language?)`

  The proxy unwraps `event: message\ndata: {json}` SSE packets,
  concatenates every text-typed `result.content[]` entry, and emits a
  structured-error envelope shape (`MSLEARN_TIMEOUT` /
  `MSLEARN_UNAVAILABLE` / `MSLEARN_BAD_REQUEST` /
  `MSLEARN_RPC_ERROR` / `MSLEARN_INVALID_RESPONSE`).
- **`docs_auto_search` — fanout aggregator.** Single tool call that
  fires Context7 (resolve + optional deep fetch), Microsoft Learn
  search, Microsoft code-sample search, and Google web search **in
  parallel** via `tokio::join!`. Returns a fused markdown report with
  four sections. Replaces 3-4 sequential round-trips with one
  max-latency round-trip — saves wall-clock time and saves context-
  window tool descriptors.
- **`skills/docs-auto/SKILL.md`** — top-trigger skill that routes any
  library / framework / SDK / API / cloud-service question to a single
  `docs_auto_search` call. Documents drill-down patterns for when the
  fused report identifies a single canonical URL.
- **`commands/docs.md` extended** : the slash command now uses the
  fanout aggregator by default ; explicit Context7 IDs (starting with
  `/`) still take the direct `context7_query_docs` fast path.
- **`aphrody-mcp-smoke` extended to 24 tools** with fixtures for the
  three Microsoft Learn tools and `docs_auto_search`. All flagged
  `network_dependent` (returns structured-error envelope when the
  upstream is unreachable).

### Notes
- The previous `0.6.x` series advertised 2 MCP servers (1 stdio + 1
  HTTP). The new `0.7.x` series advertises **1 MCP server** with 24
  native tools spanning 7 capability groups : forensics + network
  recon + bxc scraping + vision + voice + Context7 + Microsoft Learn +
  reverse-engine.
- The HTTP MCP transport detection on `learn.microsoft.com/api/mcp` was
  verified live with `curl -X POST -H "Accept: application/json,
  text/event-stream"` — stateless `tools/call` works without prior
  `initialize` or `Mcp-Session-Id` header. The Rust proxy is therefore
  fire-and-forget per call.

## [0.6.3] — 2026-05-19

### Added
- **Imported the upstream Context7 Claude plugin** (MIT,
  `upstash/context7` `plugins/claude/context7/`), rewired onto the
  native Rust `mcp__aphrody__context7_*` tools :
  - `agents/docs-researcher.md` — delegation agent that researches a
    library doc question and returns a focused answer with code
    examples, avoiding bloat in the parent conversation. Restricted to
    the 4 documentation MCP tools (Context7 + Microsoft Learn).
  - `commands/docs.md` — `/docs <library> [query]` slash command. Two
    invocation modes : plain name (`react hooks`) → auto-resolve, or
    explicit ID (`/vercel/next.js/v15.1.8 middleware`) → direct fetch.

### Notes
- The upstream plugin's `.mcp.json` declares
  `context7 → https://mcp.context7.com/mcp` (HTTP MCP server). We did
  **not** wire that — the same surface is already covered by the 2
  native Rust tools (`context7_resolve_library_id`,
  `context7_query_docs`) shipped in `aphrody-mcp` since 0.6.2. Wiring
  the HTTP MCP would duplicate the surface.
- The upstream plugin's `skills/context7-mcp/` is identical to the one
  imported in 0.6.2 — not re-imported.

## [0.6.2] — 2026-05-19

### Added
- **Context7 ported to native Rust inside `aphrody-mcp`.** Two new
  tools — `context7_resolve_library_id` and `context7_query_docs` —
  fuse the upstream `mcp-bridge.ts` (Bun, MIT, in
  `packages/mcp/cli/src/mcp-bridge.ts:114-206`) into the unified Rust
  binary. They hit `https://mcp.context7.com/api/v2/{libs/search,context}`
  via the shared `reqwest` client, honour `CONTEXT7_API_KEY` (Bearer,
  optional — public tier works unauthenticated), and emit the same
  structured-error envelope shape (`CONTEXT7_TIMEOUT` /
  `CONTEXT7_UNAVAILABLE` / `CONTEXT7_BAD_REQUEST` /
  `CONTEXT7_INVALID_RESPONSE`) as the bxc tools. Cf.
  `crates/google_mcp/src/main.rs`.
- **Smoke runner extended to 20 tools.** `crates/aphrody-mcp-smoke/`
  fixtures now cover `context7_*` (network-dependent) and `re_triage`
  (triages the `aphrody-mcp.exe` binary itself). Live verify : 19 pass /
  0 fail / 1 skip / p95 = 449 ms.
- **`skills/context7-mcp/SKILL.md`** — imported from upstream
  `upstash/context7` (MIT, `skills/context7-mcp/SKILL.md`) and rewired
  onto the native `mcp__aphrody__context7_*` tool names. Documents the
  2-step resolve → query flow, version-pinning, structured-error
  handling, and the comparison matrix with the other documentation
  tools in this plugin.

### Changed
- `plugin.json#description` updated : aphrody server is now **20 tools**
  (17 + 2 context7 + 1 re_triage), total surface across both MCPs is
  **23 tools** (20 + 3 microsoft-learn).
- `re_triage` is now formally advertised (was previously cfg-gated
  hidden) — listed in the manifest description and exercised by the
  smoke runner.

### Notes
- Upstream `context7-cli/SKILL.md` (npm-based) and `find-docs/SKILL.md`
  (npm-based) **were not imported** — they recommend
  `npm install -g ctx7@latest`, incompatible with the aphrody Rust-only
  policy. The Rust-port + `context7-mcp` skill cover the same use case.
- Upstream `packages/{cli,mcp,sdk,tools-ai-sdk}/` were **not imported**
  — they are TypeScript / pnpm packages. The wire spec we needed
  (`/api/v2/libs/search`, `/api/v2/context`, Bearer auth) was already
  captured by the in-tree `packages/mcp/cli/src/mcp-bridge.ts` mirror
  and is now re-implemented in pure Rust.

## [0.6.1] — 2026-05-19

### Added
- **Second MCP server: `microsoft-learn` (HTTP, MIT)** — wired in
  `plugin.json#mcpServers.microsoft-learn` pointing at
  `https://learn.microsoft.com/api/mcp`. Exposes 3 official Microsoft
  Learn tools: `microsoft_docs_search`, `microsoft_docs_fetch`,
  `microsoft_code_sample_search`. No auth, no subprocess, no JS
  runtime (pure HTTP) — consistent with the aphrody Rust-only policy.
- **3 skills imported from upstream `microsoftdocs/mcp` (MIT)** :
  `skills/microsoft-docs/`, `skills/microsoft-code-reference/`,
  `skills/skill-creator/` (generalised from `microsoft-skill-creator`).
  The `skill-creator` skill gains a 5th template (Rust Crate) and
  references both MCP servers in its discovery flow. CLI fallback
  sections that called `bunx @microsoft/learn-cli` were stripped to
  honor the Rust-only runtime policy.
- **`plugin.json#interface` block** (inspired by upstream Microsoft
  manifest) with `displayName`, `brandColor` (#CE422B = Rust orange),
  `category`, `capabilities`, and a 3-line `defaultPrompt` exposing the
  plugin's most useful flows (M3 token scrape, DNS recon chain, Azure
  doc lookup).
- **`crates/aphrody-mcp-smoke/`** : end-to-end Rust smoke test runner
  that spawns `aphrody-mcp`, performs the MCP handshake, lists all
  advertised tools, and exercises each of the 17 with minimal valid
  arguments. Emits an NDJSON report (1 line per tool + summary line)
  with p50 / p95 latency. Exit 0 = clean ; exit 1 = unexpected fail.

### Removed
- `.claude/plugins/aphrody/mcp/` sub-tree (both `mcp/aphrody/` empty
  dir and `mcp/bxc-scrapper/` orphan README+src). The single MCP server
  is declared inline in `plugin.json` — no per-server doc tree needed.
- Legacy `mcp__bxc-scrapper__*` tool references in
  `skills/pixel-perfect/SKILL.md`, `skills/pixel-perfect/references/validation-checklist.md`,
  `commands/tokens.md`, `agents/explore.md`, `README.md`,
  `CHANGELOG.md` — renamed to the actual exposed names
  (`mcp__aphrody__*`).
- Stale Bun / `bxc-mcp` install steps in `README.md` quick-start —
  replaced with the canonical `cargo build --release -p google_mcp` +
  `bxc-engine-daemon` install sequence.

### Fixed
- `plugin.json#description` claimed `15 + 2 voice + 1 re_triage = 18`
  tools, but `tools/list` on the running server returns 17 (no
  `re_triage` ever shipped). Description now correctly says
  `17 + 3 (microsoft-learn) = 20` tools across the two MCP servers.

## [0.6.0] — 2026-05-19

### Changed
- **Switch the unified MCP server from Bun to pure Rust.**
  `mcpServers.aphrody` in plugin.json now points to the `aphrody-mcp`
  Rust binary (compiled from `crates/google_mcp` with
  `[[bin]] name = "aphrody-mcp"`) instead of `bun run mcp/aphrody/server/index.ts`.
  - Binary size : **6.5 MB** (single-file, statically linked rustls + ring).
  - Cold-start : sub-millisecond (no Node/Bun runtime to bootstrap).
  - Build : `cargo build --release -p google_mcp --bin aphrody-mcp` (2 min).

### Added
- `crates/google_mcp/src/main.rs` gains **7 new tools fused from the
  ex-`bxc-mcp` binary** : `bxc_scrape`, `bxc_recon`, `bxc_detect`,
  `google_search`, `google_atlas_route`, `extract_structured`,
  `vision_analyze`. All proxied via `reqwest::Client` (shared,
  `OnceLock`-cached) to the bxc daemon at `BXC_DAEMON_URL`.
- `crates/google_mcp/Cargo.toml` : `[[bin]] name = "aphrody-mcp"`
  override so the package keeps its historical name but the binary
  ships under the user-facing name.
- 5 new DTOs : `BxcScrapeRequest`, `BxcUrlOnlyRequest`,
  `GoogleSearchRequest`, `ExtractStructuredRequest`,
  `VisionAnalyzeRequest`. All `schemars::JsonSchema`-derived → MCP
  inputSchema auto-generated by `rmcp`.

### Removed
- `mcpServers.aphrody.command: "bun"` → replaced by `"aphrody-mcp"`.
- Env vars `BXC_MEMORY_DB`, `APHRODY_BIN`, `BXC_MCP_BIN` (no longer
  needed — the Rust binary is the MCP server, not a wrapper).
- The 3 memory tools (`memory_set/get/list`) — were Bun-SQLite specific,
  not portable to the pure-Rust binary. (Re-add as `rusqlite` tools in
  a follow-up if needed ; persistent memory was experimental.)
- The 8 aphrody CLI exec wrappers (`doctor, version, dns, notify,
  scan_tree, scan_manifests, chromium_sync, a2a_prompt`) — they were
  Bun subprocess wrappers ; the Rust binary now exposes the equivalent
  Rust-native versions (e.g. `dns_recon` instead of `aphrody_dns`).
- The `mcp/aphrody/` Bun directory is deprecated (Claude Code may still
  hold a file handle on it during the current session ; clean up
  manually after restart).

### Validation
- `bun smoke handshake` returns `{serverInfo: {name: "rmcp", version: "1.7.0"}}`.
- `tools/list` returns 15 tools (8 ex-google_mcp + 7 ex-bxc-mcp).
- Binary on disk : 6.5 MB at `~/.local/bin/aphrody-mcp.exe`.
- Compile time : 2 min 01 s release (cargo + ring + rmcp + rustls).

### Migration notes
- External consumers of the old MCP names :
  - `bxc-scrapper` (Rust) → still ships as `bxc-mcp` binary
    (back-compat alias kept).
  - `bxc` (Bun, memory/vision) → REMOVED (memory tools dropped in
    this version ; the ex-bxc-mcp scraping tools are now under
    `aphrody-mcp`).
  - The unified Bun server `aphrody` (v0.4-0.5) → REPLACED by the
    Rust `aphrody-mcp` binary.

## [0.5.1] — 2026-05-19

### Removed
- **Reverted GitHub + Context7 proxy tools** from the unified `aphrody`
  MCP server. These are third-party SaaS endpoints with their own
  canonical hosts (api.github.com / mcp.context7.com) — bundling them
  into a first-party plugin proxy was overreach. Users who need them
  install them in their own `.claude/settings.json`.
  - Dropped 7 tools : `aphrody_github_{list_issues, create_issue,
    list_prs, search_repos, get_repo}` + `aphrody_context7_{resolve_library,
    get_docs}`.
  - Dropped 2 env vars from plugin.json + manifest.json user_config
    (`GITHUB_PERSONAL_ACCESS_TOKEN`, `CONTEXT7_API_KEY`).
  - Tool count : 25 → 18.

### Validation
- 18/18 tools registered, all first-party (bxc + memory + aphrody CLI).
- `plugin.json` + `manifest.json` parse OK.
- `bun run server/index.ts --list-tools` lists 18 names.

## [0.5.0] — 2026-05-19

### Added
- **11 new tools in the unified `aphrody` MCP server** (14 → 25 total) :
  - 4 aphrody CLI wrappers : `aphrody_scan_tree`, `aphrody_scan_manifests`,
    `aphrody_chromium_sync`, `aphrody_a2a_prompt`.
  - 5 GitHub REST proxy tools : `aphrody_github_list_issues`,
    `aphrody_github_create_issue`, `aphrody_github_list_prs`,
    `aphrody_github_search_repos`, `aphrody_github_get_repo`. Direct
    `fetch()` against api.github.com with `GITHUB_PERSONAL_ACCESS_TOKEN`.
  - 2 Context7 docs proxy tools : `aphrody_context7_resolve_library`,
    `aphrody_context7_get_docs`. Direct `fetch()` against mcp.context7.com.
- `manifest.json` user_config : 2 new sensitive fields (`github_token`,
  `context7_api_key`) for MCPB bundle installs.

### Changed
- **`plugin.json` mcpServers** : `github` + `context7` cloud entries
  removed. The unified `aphrody` server now proxies them as native tools,
  reusing the same env vars (`GITHUB_PERSONAL_ACCESS_TOKEN`,
  `CONTEXT7_API_KEY`). Single server entry, zero loss of functionality.
- Plugin version 0.4.0 → 0.5.0.
- `mcp/aphrody/` server version 0.1.0 → 0.2.0.
- README rewrites : tool catalogue table now covers 5 categories
  (scraping / memory / CLI / GitHub / Context7) instead of 3.

### Removed
- `github` cloud MCP server (streamable-http) — proxied by 5 unified
  tools instead.
- `context7` cloud MCP server (http) — proxied by 2 unified tools
  instead.
- `aphrody_search` CLI wrapper (was redundant with the bxc-proxied
  `aphrody_search` Google SERP tool ; the CLI flavour was flaky).

### Validation
- 25/25 tools registered (was 14/14).
- `bun run server/index.ts --list-tools` lists all 25 names.
- `bun -e JSON.parse(...)` valid on plugin.json + manifest.json.
- No duplicate-tool error after the rename pass.

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
