<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — unified MCP server

**Single stdio server** consolidating the previous in-tree
`bxc-scrapper` + `bxc` dual-server config. Exposes **18 first-party
tools** spanning scraping, in-process memory, and aphrody CLI wrappers.

Third-party SaaS MCP servers (GitHub, Context7) are **intentionally
NOT bundled** — they have their own canonical endpoints. Install them
separately in your `.claude/settings.json` (or `~/.claude/settings.json`)
if you need them.

## Tool catalogue (18)

### Scraping (7) — proxied to `bxc-mcp` Rust subprocess
| Tool | Args | Returns |
|---|---|---|
| `aphrody_scrape` | `url`, `selector?` (default `body`) | `[{index, text}]` |
| `aphrody_recon` | `url` | `bxc-recon-v1` envelope (status, bytes, headers, cdn, frameworks, assets, gotoMs) |
| `aphrody_detect` | `url` | deep tech : frontend / backend / cdn / dns / hosting / cms |
| `aphrody_search` | `query`, `hl?` (default `en`) | Google SERP organic results |
| `aphrody_atlas_route` | `url` | `{profile, stealth_hints, framework}` |
| `aphrody_extract_structured` | `html`, `zod_schema_json` | typed JSON via local Gemma 4 |
| `aphrody_vision_analyze` | `screenshot_path` | elements / text / colors / fonts / hierarchy |

### In-process memory (3) — Bun SQLite, no daemon
| Tool | Args | Returns |
|---|---|---|
| `aphrody_memory_set` | `key`, `value` | `{ok, key}` (upsert) |
| `aphrody_memory_get` | `key` | `{value, created_at, updated_at}` or `{value:null}` |
| `aphrody_memory_list` | (none) | `[{key, created_at, updated_at}, ...]` |

### aphrody CLI wrappers (8) — exec native binary
| Tool | Args | Returns |
|---|---|---|
| `aphrody_doctor` | (none) | `aphrody doctor --json` parsed envelope |
| `aphrody_version` | (none) | binary version + commit + target |
| `aphrody_dns` | `domain` | OSINT passive DNS recon |
| `aphrody_notify` | `channel`, `message`, `room?` | confirmation or structured error |
| `aphrody_scan_tree` | `root?`, `groups?`, `top_ext?` | size + file-count breakdown JSON |
| `aphrody_scan_manifests` | `root?` | Cargo / package / pyproject sweep JSON |
| `aphrody_chromium_sync` | (none) | Chromium profiles + master key (Windows-only) |
| `aphrody_a2a_prompt` | `prompt` | A2A reply (falls back to Gemini CLI) |


## Architecture

```
┌─────────────────────────────────────────────────────────┐
│             aphrody-mcp (Bun stdio server)              │
│                                                         │
│  ┌───────────┐  ┌───────────┐  ┌───────────────────┐   │
│  │ scraping  │  │  memory   │  │  CLI wrappers     │   │
│  │  (7 tools)│  │  (3 tools)│  │  (4 tools)        │   │
│  └─────┬─────┘  └─────┬─────┘  └─────────┬─────────┘   │
│        │              │                  │             │
│        ▼              ▼                  ▼             │
│   ┌─────────┐    ┌────────┐         ┌─────────┐        │
│   │bxc-mcp  │    │ SQLite │         │ aphrody │        │
│   │subproc  │    │ Bun db │         │   CLI   │        │
│   │(spawned │    │        │         │  exec   │        │
│   │on first │    │        │         │         │        │
│   │tool use)│    │        │         │         │        │
│   └────┬────┘    └────────┘         └────┬────┘        │
└────────┼─────────────────────────────────┼─────────────┘
         │                                 │
         ▼                                 ▼
   bxc Bun /api/*                    aphrody n binaries
   (port 8765)                       (scrape, doctor, …)
```

The bxc-mcp Rust subprocess is **lazy-spawned** on first scraping call,
so callers that only use memory or CLI wrappers don't pay its boot cost.

## Install

### As a plugin MCP server (default)

Already wired in `.claude/plugins/aphrody/.claude-plugin/plugin.json`
under the `aphrody` server entry — Claude Code picks it up on next
session start. Run `bun install` once :

```bash
cd .claude/plugins/aphrody/mcp/aphrody
bun install
```

### As a standalone MCPB bundle

```bash
cd .claude/plugins/aphrody/mcp/aphrody
bun install
npx @anthropic-ai/mcpb pack    # produces aphrody-0.1.0.mcpb
# Drag the .mcpb file onto Claude Desktop to install.
```

The bundled `manifest.json` declares all four `user_config` settings
(bxc daemon URL, memory DB path, aphrody binary, bxc-mcp binary) so the
installer sees a native config UI.

## Environment variables

| Var | Default | Purpose |
|---|---|---|
| `BXC_DAEMON_URL` | `http://127.0.0.1:8765` | bxc Bun daemon URL — used by the bxc-mcp subprocess for scraping |
| `BXC_MEMORY_DB` | `$HOME/.aphrody/aphrody-memory.sqlite` | SQLite file for memory tools |
| `APHRODY_BIN` | `aphrody` (PATH) | aphrody CLI binary path or name |
| `BXC_MCP_BIN` | `bxc-mcp` (PATH) | bxc-mcp Rust binary path or name |

## Smoke test

```bash
bun install
bun run server/index.ts --list-tools | head -3
# [
#   {"name":"aphrody_scrape","description":"Extract textContent of …"},
#   …

# Drive it interactively with the MCP inspector :
npx @modelcontextprotocol/inspector bun run server/index.ts
# Open the URL it prints, click "List Tools" → 14 tools visible.
```

## Why one server, not three

Before (plugin v0.3.x) :
- `bxc-scrapper` (stdio, Rust binary) — 7 scraping tools
- `bxc` (stdio, Bun server.ts) — memory / vision / CDP

After (plugin v0.4.0+) :
- `aphrody` (stdio, Bun) — 14 tools fused (scraping subprocessed +
  memory in-process + CLI wrappers)

Gains :
- 1 server lifecycle to manage (one PID, one SIGINT path).
- Shared config surface (one `user_config` block in MCPB).
- CLI wrappers add 4 net-new tools the dual-server config didn't have.
- Bun subprocess can spawn `bxc-mcp` lazily — same RAM cost only when
  scraping tools are called.

The cloud MCP servers (`github`, `context7`) stay separate because they
have nothing to share with a local server.

## Security

- Memory DB lives under `$HOME/.aphrody/` (700 on Unix). Set
  `BXC_MEMORY_DB` to override.
- `aphrody_notify` reads credentials from env — never from MCP args.
- bxc-mcp subprocess inherits the parent env; do not export secrets
  globally before launching Claude Code.
- All scraping tools have `readOnlyHint: true`; only `aphrody_notify`
  and `aphrody_memory_set` write state (notify = network, memory_set =
  local SQLite).

## Roadmap

- v0.2 : add `aphrody_scan_tree`, `aphrody_scan_manifests`,
  `aphrody_term_status` CLI wrappers.
- v0.3 : pure-Rust MCPB binary build (no Bun runtime, single
  static-linked exe) once `crates/aphrody-mcp` lands.
