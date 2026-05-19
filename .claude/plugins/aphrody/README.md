<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — Claude Code plugin

**Version : 0.6.0** · License: Apache-2.0 · Owner: [aphrody-code](https://github.com/aphrody-code)

Unified **bxc / n2b / aphrody CLI** automation surface for Claude Code.
Slash commands drive the native `aphrody` binary, which auto-spawns the
bxc Rust daemon (`bxc-engine-daemon`) on demand and exposes a stable
`/api/*` JSON surface end-to-end. **100 % Rust runtime** — no Bun, no
Node, no JS subprocess.

Cross-platform: Linux Ubuntu 26.04 (priority **#1**), Windows 11 Insider
Canary (#2), WebAssembly (#3).

## Quick start

```bash
# 1. Install the aphrody binary (CLI driver — required)
cargo build --release -p aphrody --locked
cp target/x86_64-pc-windows-msvc/release/aphrody.exe ~/.local/bin/aphrody.exe
# (Linux: target/x86_64-unknown-linux-gnu/release/aphrody → ~/.local/bin/aphrody)

# 2. Install the unified MCP server (powers /scrape, /tokens, /status)
cargo build --release -p google_mcp --bin aphrody-mcp --locked
cp target/x86_64-pc-windows-msvc/release/aphrody-mcp.exe ~/.local/bin/aphrody-mcp.exe
# (Linux: target/x86_64-unknown-linux-gnu/release/aphrody-mcp → ~/.local/bin/aphrody-mcp)

# 3. Install the bxc HTTP daemon (pure-Rust, required by 7 of the 17 MCP tools)
cargo build --release -p bxc-engine --bin bxc-engine-daemon --locked
cp target/x86_64-pc-windows-msvc/release/bxc-engine-daemon.exe ~/.local/bin/bxc-engine-daemon.exe

# 4. Restart Claude Code — the plugin is auto-discovered under
#    `.claude/plugins/aphrody/`
```

## Plugin layout

```
.claude/plugins/aphrody/
├── .claude-plugin/
│   └── plugin.json             # manifest (v0.6.0)
├── README.md                   # this file
├── CHANGELOG.md
├── commands/                   # 3 slash commands
│   ├── scrape.md               # /scrape <url> [selector]
│   ├── tokens.md               # /tokens [url]
│   └── status.md               # /status (read-only diagnostics)
├── agents/                     # 27 agents (aphrody-cli + 26 specialised)
│   └── …
├── skills/                     # 37 skills (pixel-perfect, m3-component, …)
│   └── …
└── hooks/
    └── hooks.json              # PostToolUse hooks (currently neutralized)
```

The single MCP server (`aphrody-mcp`) is declared inline in
`.claude-plugin/plugin.json#mcpServers.aphrody` — no `mcp/` sub-tree
is needed.

## Slash commands

| Command | Purpose | Backed by |
|---|---|---|
| `/scrape <url> [selector]` | Recon + (optional) CSS scrape via `aphrody bxc recon`, `aphrody bxc detect`, `aphrody scrape --selector`. Daemon auto-starts. | `aphrody` CLI → bxc Bun `/api/*` |
| `/tokens [url]` | M3 design token extraction via `aphrody tokens --url … --output … --force`. | `aphrody` CLI → bxc Bun `/api/scrape` (`:root` + `--md-*` regex) |
| `/status` | Read-only project diagnostic: binary, plugin, branch, PLAN ⏳, A2A peer, bxc daemon health. | `aphrody --version`, `aphrody doctor --json`, `git`, `curl /healthz` |

## Agents (27)

### Unified entrypoint
- **`aphrody-cli`** — wraps the 27 aphrody sub-commands. Default agent
  for any "scrape this", "extract M3 tokens", "run bxc daemon", "send
  Slack message via aphrody", "doctor", "scan", "term" requests.

### Specialised (delegation targets)
| Domain | Agents |
|---|---|
| Rust | `rust-architect`, `rust-engineer`, `cargo-auditor` |
| C++/FFI | `cpp-engineer`, `ffi-architect` |
| Zig | `zig-engineer` |
| Cross-platform | `cross-platform-validator` |
| Node→Bun | `n2b`, `n2b-ultra`, `n2b-contract-guard` |
| Material Design 3 | `m3-spec-auditor`, `material`, `design-google-curator` |
| Infra | `deployment-engineer`, `devops-engineer`, `sre-engineer`, `incident-responder`, `postgres-pro` |
| Quality | `code-review`, `security-engineer`, `performance-engineer` |
| Workflow | `build`, `explore`, `move`, `yolo-prod-ready` |
| Comms | `discord-bot` |

## MCP server (1 — pure Rust, fully unified)

The previous `bxc-scrapper`, `google_mcp`, `context7`, and
`microsoft-learn` separate servers are now **all fused into a single
Rust binary** `aphrody-mcp` (~7 MB, sub-millisecond cold-start, zero JS
runtime, zero secondary MCP server, zero external subprocess).

| Server | Type | Binary | Tools (24) | Env |
|---|---|---|---|---|
| **`aphrody`** | stdio | `aphrody-mcp` (Rust, rmcp 1.7.0, ex-`google_mcp` + ex-`bxc-mcp` + Context7 + Microsoft Learn all fused) | 8 ex-google_mcp (`coding_style_guide, universal_web_fetch, dns_recon, auth_extract, chrome_autopsy, advanced_recon, native_hooks, start_dashboard`) + 7 ex-bxc-mcp (`bxc_scrape, bxc_recon, bxc_detect, google_search, google_atlas_route, extract_structured, vision_analyze`) + 2 voice (`voice_synthesize, voice_transcribe`) + 2 Context7 (`context7_resolve_library_id, context7_query_docs`) + 3 Microsoft Learn (`microsoft_docs_search, microsoft_docs_fetch, microsoft_code_sample_search` — native Rust HTTP MCP proxy onto `learn.microsoft.com/api/mcp`) + 1 fanout aggregator (`docs_auto_search` — parallel Context7 + Microsoft Learn + Microsoft code samples + Google in one call) + 1 reverse-engine (`re_triage`) | `BXC_DAEMON_URL`, `BXC_TIMEOUT_MS`, `BXC_VISION_MIN_BYTES`, `ELEVENLABS_API_KEY`, `CONTEXT7_API_KEY` (optional Bearer), `CONTEXT7_API_BASE` (optional override) |

Third-party SaaS MCPs (`github`, `context7`) are intentionally **not
bundled** — install them in your own `.claude/settings.json` if needed.

Install : `cargo build --release -p google_mcp --bin aphrody-mcp` then
copy `target/<triple>/release/aphrody-mcp[.exe]` to `~/.local/bin/`.

End-to-end smoke test (drives the binary over real stdio, exercises every
tool with the smallest valid argument set, asserts expected-error envelope
on tools that need an external dep) :

```bash
cargo run --release -p aphrody-mcp-smoke -- --report var/smoke/mcp-smoke.ndjson
# stdout: NDJSON, one line per tool + final summary
# exit 0 on PASS/SKIP, exit 1 on unexpected FAIL
```

## Hooks (PostToolUse on Edit | Write | MultiEdit)

| Hook | Command | Timeout | Behaviour |
|---|---|---|---|
| `cargo-check` | `cargo check --workspace --message-format json --locked --offline` | 60 s | Advisory — surfaces clippy/check errors as stderr to Claude |
| `cargo-toml-validate` | `cargo xtask toml-validate` | 30 s | Blocking on broken `Cargo.toml` |

Hot-swap: not possible (Claude Code reads `hooks.json` once at session
start). Restart Claude Code after editing.

## Skills (35+)

Auto-discovered from `skills/<name>/SKILL.md`. Highlights:

- **`pixel-perfect`** — M3 component fidelity audit (token + DOM + visual).
- **`m3-component`** — Material Web 3 component scaffolder.
- **`agent-browser`** — preferred browser automation entrypoint
  (delegates to bxc / agent-browser CLI).
- **`aphrody-yolo-grind`** + **`aphrody-perfect-grind`** — parallel
  multi-agent grind modes.
- **`n2b`** — Node → Bun migration workflow.
- **`a2a-duel-loop`** — sustained A2A coordination duel.
- **`rust-target-check`** — parallel 3-target `cargo check`.
- **`apple-hig`, `web-design-guidelines`, `creative-director`, …** — design
  reference packs.

Run `ls .claude/plugins/aphrody/skills/` for the full inventory.

## Configuration (`.local.md` pattern)

Per-project settings live in `.claude/aphrody.local.md` (gitignored). See
`examples/aphrody.local.md.example` for the template. Frontmatter
recognised :

```yaml
---
enabled: true                                       # disable the plugin per project
bxc_driver: bun                                     # bun | rust (cf. APHRODY_BXC_DRIVER)
bxc_port: 8765                                      # daemon port
bxc_auto_start: true                                # auto-spawn on scrape/tokens calls
hooks_blocking: false                               # cargo-check hook: advisory vs blocking
---
```

After editing, **restart Claude Code** — hooks load once per session.

## Environment variables

| Variable | Default | Used by |
|---|---|---|
| `BXC_DAEMON_URL` | `http://127.0.0.1:8765` | `aphrody scrape`, `aphrody bxc *`, `aphrody-mcp` (bxc tools) |
| `BXC_TIMEOUT_MS` | `30000` | `aphrody-mcp` (bxc tools HTTP timeout) |
| `BXC_VISION_MIN_BYTES` | `1024` | `aphrody-mcp` (`vision_analyze` min screenshot size) |
| `APHRODY_BXC_DAEMON_BIN` | `which bxc-engine-daemon` | `aphrody bxc daemon` Rust driver |
| `APHRODY_BXC_ENGINE_BIN` | `which bxc-engine-daemon` | alias legacy (deprecated) |
| `ELEVENLABS_API_KEY` | (none) | `aphrody-mcp` `voice_synthesize` / `voice_transcribe` |

## Validation

```bash
# Plugin manifest parses (Rust-only, no Bun)
cargo xtask plugin-manifest-check          # or: python -c "import json; json.load(open('.claude/plugins/aphrody/.claude-plugin/plugin.json'))"

# Unified MCP server handshake + tools/list + tools/call sweep
cargo run --release -p aphrody-mcp-smoke   # exit 0 = all 17 tools healthy or expected-error

# bxc daemon (Rust) — required by 7 of the 17 MCP tools
bxc-engine-daemon --port 8765 &
sleep 2
curl -fsS http://localhost:8765/healthz    # 200 OK

# aphrody CLI smoke
aphrody version
aphrody doctor --json | head -20
aphrody scrape --selector "h1" https://example.com  # auto-starts bxc daemon
```

## Troubleshooting

**`aphrody-mcp` not found** : `cargo install --locked --path crates/google_mcp --bin aphrody-mcp` (or build + copy as in Quick start §2).

**`bxc daemon failed to start within 10 s`** : verify `bxc-engine-daemon` is on PATH (`cargo install --locked --path crates/bxc-engine --bin bxc-engine-daemon`).

**`aphrody scrape` returns parse error** : version mismatch between
`crates/cli` and `packages/bxc`. Rebuild aphrody : `cargo build --release -p aphrody --locked`.

## Compliance with plugin-dev standards

- ✅ Manifest in `.claude-plugin/plugin.json`
- ✅ Components at plugin root (commands/, agents/, skills/, hooks/)
- ✅ `${CLAUDE_PLUGIN_ROOT}` used for relative paths
- ✅ Environment variables documented
- ✅ Kebab-case file naming
- ✅ All MCP servers use stable URLs / token env vars (no hardcoded credentials)
- ✅ All commands have YAML frontmatter (description, allowed-tools, argument-hint, model)
- ✅ All listed agents exist on disk (27/27)
- ✅ README + CHANGELOG present

## Links

- Repo: https://github.com/aphrody-code/aphrody
- Issues: https://github.com/aphrody-code/aphrody/issues
- License: [Apache-2.0](../../LICENSE)
- Author: aphrody-code
