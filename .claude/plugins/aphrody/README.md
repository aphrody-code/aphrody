<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — Claude Code plugin

**Version : 0.7.0** · License: Apache-2.0 · Owner: [aphrody-code](https://github.com/aphrody-code)

Unified **aphrody CLI** automation surface for Claude Code.
Slash commands drive the native `aphrody` binary and expose a stable
JSON surface end-to-end. **100 % Rust runtime** — no Bun, no
Node, no JS subprocess.

Cross-platform: Linux Ubuntu 26.04 (priority **#1**), Windows 11 Insider
Canary (#2), WebAssembly (#3).

## Quick start

```bash
# 1. Install the aphrody binary (CLI driver — required)
cargo build --release -p aphrody --locked
cp target/x86_64-pc-windows-msvc/release/aphrody.exe ~/.local/bin/aphrody.exe
# (Linux: target/x86_64-unknown-linux-gnu/release/aphrody → ~/.local/bin/aphrody)

# 2. Install the unified MCP server (powers /docs, /status)
cargo build --release -p google_mcp --bin aphrody-mcp --locked
cp target/x86_64-pc-windows-msvc/release/aphrody-mcp.exe ~/.local/bin/aphrody-mcp.exe
# (Linux: target/x86_64-unknown-linux-gnu/release/aphrody-mcp → ~/.local/bin/aphrody-mcp)

# 3. Restart Claude Code — the plugin is auto-discovered under
#    `.claude/plugins/aphrody/`
```

## Plugin layout

```
.claude/plugins/aphrody/
├── .claude-plugin/
│   └── plugin.json             # manifest (v0.7.0)
├── README.md                   # this file
├── CHANGELOG.md
├── commands/                   # slash commands
│   ├── docs.md                 # /docs <library> [query]
│   └── status.md               # /status (read-only diagnostics)
├── agents/                     # 21 agents
│   └── …
├── skills/                     # 34 skills
│   └── …
└── hooks/
    └── hooks.json              # SessionStart advisory hook (docs-tool reminder)
```

The single MCP server (`aphrody-mcp`) is declared inline in
`.claude-plugin/plugin.json#mcpServers.aphrody` — no `mcp/` sub-tree
is needed.

## Slash commands

| Command | Purpose | Backed by |
|---|---|---|
| `/docs <library> [query]` | Look up library documentation via Context7 & Microsoft Learn | `aphrody-mcp` |
| `/status` | Read-only project diagnostic: binary, plugin, branch, PLAN ⏳, A2A peer health. | `aphrody --version`, `aphrody doctor --json`, `git` |

## Agents (21)

### Unified entrypoint
- **`aphrody-cli`** — wraps the aphrody sub-commands. Default agent
  for any "doctor", "scan", "term" requests.

### Specialised (delegation targets)
| Domain | Agents |
|---|---|
| Rust | `rust-architect`, `rust-engineer`, `cargo-auditor` |
| C++/FFI | `cpp-engineer`, `ffi-architect` |
| Zig | `zig-engineer` |
| Cross-platform | `cross-platform-validator` |
| Material Design 3 | `design-google-curator` |
| Infra | `deployment-engineer`, `devops-engineer`, `sre-engineer`, `incident-responder`, `postgres-pro` |
| Quality | `code-review`, `security-engineer`, `performance-engineer` |
| Workflow | `build`, `explore`, `yolo-prod-ready` |

## MCP server (1 — pure Rust, fully unified)

The previous separate servers are now **all fused into a single
Rust binary` `aphrody-mcp` (~7 MB, sub-millisecond cold-start, zero JS
runtime, zero secondary MCP server, zero external subprocess).

| Server | Type | Binary | Tools (18) | Env |
|---|---|---|---|---|
| **`aphrody`** | stdio | `aphrody-mcp` (Rust, rmcp 1.7.0) | 8 ex-google_mcp (`coding_style_guide, universal_web_fetch, dns_recon, auth_extract, chrome_autopsy, advanced_recon, native_hooks, start_dashboard`) + 2 voice (`voice_synthesize, voice_transcribe`) + 1 re (`re_triage`) + 2 Context7 (`context7_resolve_library_id, context7_query_docs`) + 3 Microsoft Learn (`microsoft_docs_search, microsoft_docs_fetch, microsoft_code_sample_search`) + 2 aggregators (`docs_auto_search, aphrody_mcp_call`) | `ELEVENLABS_API_KEY`, `CONTEXT7_API_KEY` (optional) |

Third-party SaaS MCPs (`github`, `context7`) are intentionally **not
bundled** — install them in your own `.claude/settings.json` if needed.

## Hooks (SessionStart)

| Hook | Command | Timeout | Behaviour |
|---|---|---|---|
| `mcp-reminder` | `echo '…docs_auto_search FIRST…'` | 5 s | Advisory — steers Claude towards the native MCP docs tools at session start |

> The previous `cargo xtask install-mcp` auto-build/redeploy hooks were
> removed with the `aphrody-xtask` crate (2026-05-21). Rebuild manually
> after touching `crates/google_mcp/src/*.rs`:
> `cargo build --release --bin aphrody-mcp` (produced by the `google_mcp`
> crate) then copy `target/release/aphrody-mcp` to `~/.local/bin/`.

## Skills (34)

Auto-discovered from `skills/<name>/SKILL.md`. Highlights:

- **`agent-browser`** — preferred browser automation entrypoint.
- **`aphrody-yolo-grind`** + **`aphrody-perfect-grind`** — parallel
  multi-agent grind modes.
- **`a2a-duel-loop`** — sustained A2A coordination duel.
- **`rust-target-check`** — parallel 3-target `cargo check`.
- **`apple-hig`, `brand-guidelines`, `creative-director`, …** — design
  reference packs.

## Compliance with plugin-dev standards

- ✅ Manifest in `.claude-plugin/plugin.json`
- ✅ Components at plugin root (commands/, agents/, skills/, hooks/)
- ✅ `${CLAUDE_PLUGIN_ROOT}` used for relative paths
- ✅ Environment variables documented
- ✅ Kebab-case file naming
- ✅ All MCP servers use stable URLs / token env vars (no hardcoded credentials)
- ✅ All commands have YAML frontmatter (description, allowed-tools, argument-hint, model)
- ✅ All listed agents exist on disk (21/21)
- ✅ README + CHANGELOG present

## Links

- Repo: https://github.com/aphrody-code/aphrody
- Issues: https://github.com/aphrody-code/aphrody/issues
- License: [Apache-2.0](../../LICENSE)
- Author: aphrody-code
