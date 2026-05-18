<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — Claude Code plugin

**Version : 0.3.1** · License: Apache-2.0 · Owner: [aphrody-code](https://github.com/aphrody-code)

Unified **bxc / n2b / aphrody CLI** automation surface for Claude Code.
Slash commands drive the native `aphrody` binary, which auto-spawns the
bxc Bun daemon on demand and exposes a stable `/api/*` JSON surface
end-to-end.

Cross-platform: Linux Ubuntu 26.04 (priority **#1**), Windows 11 Insider
Canary (#2), WebAssembly (#3).

## Quick start

```bash
# 1. Install the aphrody binary (CLI driver — required)
cargo build --release -p aphrody --locked
cp target/x86_64-pc-windows-msvc/release/aphrody.exe ~/.local/bin/aphrody.exe
# (Linux: target/x86_64-unknown-linux-gnu/release/aphrody → ~/.local/bin/aphrody)

# 2. Install the bxc-mcp Rust MCP server (powers /scrape, /tokens, …)
cargo build --release -p bxc-engine --bin bxc-mcp --locked
cp target/x86_64-pc-windows-msvc/release/bxc-mcp.exe ~/.local/bin/bxc-mcp.exe

# 3. Install Bun (>= 1.3.14) — required for the bxc HTTP API daemon
# https://bun.sh

# 4. Build the bxc rust-bridge cdylib (one-time, ~1m40s)
cd packages/bxc/rust-bridge
cargo build --release
cp target/x86_64-pc-windows-msvc/release/bxc_rust_bridge.dll \
   target/release/bxc_rust_bridge.dll        # bridge.ts expected path

# 5. Install bxc Bun deps (one-time)
cd ../ && bun install --silent

# 6. Restart Claude Code — the plugin is auto-discovered under
#    `.claude/plugins/aphrody/`
```

## Plugin layout

```
.claude/plugins/aphrody/
├── .claude-plugin/
│   └── plugin.json             # manifest (v0.3.1)
├── README.md                   # this file
├── CHANGELOG.md
├── commands/                   # 3 slash commands
│   ├── scrape.md               # /scrape <url> [selector]
│   ├── tokens.md               # /tokens [url]
│   └── status.md               # /status (read-only diagnostics)
├── agents/                     # 27 agents (aphrody-cli + 26 specialised)
│   └── …
├── skills/                     # 35+ skills (pixel-perfect, m3-component, …)
│   └── …
├── hooks/
│   └── hooks.json              # PostToolUse: cargo check + xtask toml-validate
└── mcp/
    └── bxc-scrapper/README.md  # bxc-scrapper MCP server docs (binary lives elsewhere)
```

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

## MCP servers (4)

| Server | Type | Powers | Auth |
|---|---|---|---|
| **`bxc-scrapper`** | stdio (Rust `bxc-mcp`) | 7 scraping tools : `bxc_scrape`, `bxc_recon`, `bxc_detect`, `google_search`, `google_atlas_route`, `extract_structured`, `vision_analyze` | `BXC_DAEMON_URL` env |
| **`bxc`** | stdio (Bun `bxc-extension/server.ts`) | Memory + vision + CDP tools : `tune_memory_sqlite`, `vision_analyze`, `start_scraping_subagent`, `auto_detect_skills`, `bxc_cdp_*` | `BXC_MEMORY_DB` env (defaults to `${CLAUDE_PLUGIN_ROOT}/../../../var/data/bxc-memory.sqlite`) |
| **`github`** | streamable-http | Official GitHub Copilot MCP. Issues, PRs, repos. | `GITHUB_PERSONAL_ACCESS_TOKEN` env |
| **`context7`** | http | Library docs + version checking (preferred over WebSearch for lib docs). | `CONTEXT7_API_KEY` env |

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
| `BXC_DAEMON_URL` | `http://127.0.0.1:8765` | `aphrody scrape`, `aphrody bxc *`, `bxc-mcp` |
| `APHRODY_BXC_DRIVER` | `bun` | `aphrody bxc daemon` |
| `APHRODY_BXC_ROOT` | `<repo>/packages/bxc` | `aphrody bxc daemon` Bun driver |
| `APHRODY_BXC_ENGINE_BIN` | `which bxc-engine` | `aphrody bxc daemon` Rust driver fallback |
| `BXC_MEMORY_DB` | `<repo>/var/data/bxc-memory.sqlite` | MCP `bxc` Bun (memory tools) |
| `GITHUB_PERSONAL_ACCESS_TOKEN` | (none) | MCP `github` |
| `CONTEXT7_API_KEY` | (none) | MCP `context7` |

## Validation

```bash
# Plugin manifest parses
bun -e "JSON.parse(require('node:fs').readFileSync('.claude/plugins/aphrody/.claude-plugin/plugin.json','utf8')); console.log('OK')"

# bxc-mcp on PATH (--list-tools should print 7 tools)
bxc-mcp --list-tools | head -1

# bxc Bun api server (one-time check)
cd packages/bxc && bun run src/cli/index.ts api --port 8765 &
sleep 5
curl -fsS http://localhost:8765/healthz   # {"ok":true,...}
kill %1

# aphrody CLI smoke
aphrody version
aphrody doctor --json | head -20
aphrody scrape --selector "h1" https://example.com  # auto-starts bxc daemon
```

## Troubleshooting

**`bxc-mcp` not found** : `cargo install --locked --path crates/bxc-engine --bin bxc-mcp` (or build + copy as in Quick start §2).

**`bxc daemon failed to start within 10 s`** : verify `bun` is on PATH and
`packages/bxc/rust-bridge/target/release/bxc_rust_bridge.dll` exists (see
Quick start §4).

**MCP `bxc` (Bun) load error** : the path is `${CLAUDE_PLUGIN_ROOT}/../../../packages/bxc/...`
so the plugin only works when installed inside a checkout of the
`aphrody-code/aphrody` repo. Marketplace install will fail this MCP — set
`APHRODY_BXC_BUN_SERVER` env var to override (planned).

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
