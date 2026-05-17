# aphrody — Claude Code plugin

Production-grade plugin shipping the four assets that drive the aphrody
2026-05-17 roadmap: a Material Design 3 validation skill, a Node→Bun
migration agent, two `bxc`-powered slash commands, an oxlint PostToolUse
hook, and the local MCP server that backs the slash commands.

Cross-platform support: Linux Ubuntu 26.04 (priority #1), Windows 11
Insider Canary (#2), WebAssembly (#3). Bun is required everywhere;
`node` is forbidden by repo policy (see `feedback_bun_only`).

## Layout

```
.claude/plugins/aphrody/
├── .claude-plugin/plugin.json         # manifest (loaded by Claude Code)
├── README.md                          # this file
├── skills/pixel-perfect/              # auto-discovered skill
│   ├── SKILL.md
│   └── references/{m3-spec,validation-checklist}.md
├── agents/n2b-ultra.md                # sub-agent definition
├── commands/{scrape,tokens}.md        # slash commands
├── hooks/oxclint.json                 # PostToolUse Edit|Write -> oxlint
└── mcp/bxc-scrapper/                  # MCP stdio server (Bun + TS)
    ├── package.json
    ├── tsconfig.json
    ├── README.md
    └── server.ts
```

## Install

The plugin is project-scoped under `.claude/plugins/aphrody/`. Claude Code
discovers it automatically on session start; if not, add it explicitly to
the repo's `.claude/settings.json`:

```json
{
  "plugins": {
    "aphrody": {
      "path": ".claude/plugins/aphrody"
    }
  }
}
```

Then install the MCP server's runtime deps once (Bun, never node):

```bash
cd .claude/plugins/aphrody/mcp/bxc-scrapper
bun install
```

Restart your Claude Code session so the manifest is re-read.

## Components

### Skill — `pixel-perfect`

Triggers automatically when you ask Claude to validate an M3 component
implementation against the spec, audit design tokens, or compare a
rendered component to its Material Design 3 reference. Follows a strict
four-step workflow (scrape → compare → diff → patch) and writes findings
into `packages/ui/components/<name>/spec.report.md`. See
`skills/pixel-perfect/SKILL.md`.

### Agent — `n2b-ultra`

Invoke with the Agent tool (`subagent_type: "n2b-ultra"`) for any
Node.js → Bun migration. Drives the n2b crate, applies AST-driven
rewrites, runs `bun install`, validates with `bun test` and `bun run
lint`. Refuses to introduce node-only deps. See `agents/n2b-ultra.md`.

### Slash commands

- `/scrape <url> [selector]` — runs the `bxc-scrapper` MCP `bxc_recon`
  tool against `<url>` and (optionally) extracts `<selector>` via
  `bxc_scrape`. Returns headers, CDN, framework detection, CSS asset
  inventory, and a screenshot path.
- `/tokens [url]` — scrapes Material Design 3 design tokens from
  `m3.material.io` (default `https://m3.material.io/foundations/design-tokens`),
  normalizes them, and writes `packages/ui/tokens/m3.json`.

### Hook — `oxclint`

PostToolUse hook on `Edit|Write` for `.ts/.tsx/.js/.jsx`. Runs
`bunx oxlint --quiet --max-warnings 0` on the touched file. Non-blocking
for warnings, blocking for errors (exit code 2 makes Claude Code surface
the diagnostic). Cross-platform: uses portable Bun invocation.

### MCP server — `bxc-scrapper`

Local stdio MCP server exposing seven tools:

| Tool | Description |
|---|---|
| `bxc_scrape(url, selector?)` | Text extraction (selector defaults to `body`) |
| `bxc_recon(url)` | Headers, CDN, frameworks, assets, screenshot |
| `bxc_detect(url)` | DetectedTech[] via wappalyzergo |
| `google_search(query, hl?)` | SERP organic results |
| `google_atlas_route(url)` | {profile, stealth_hints, framework} |
| `extract_structured(html, zod_schema_json)` | Typed JSON via local LLM |
| `vision_analyze(screenshot_path)` | Elements, text, colors, fonts, hierarchy |

Backed by the bxc daemon at `$BXC_DAEMON_URL` (default
`http://127.0.0.1:8765`). If the daemon is unreachable, the server
falls back to spawning `bxc-engine` directly with the matching
subcommand and the same JSON arguments. If neither is available, it
returns a precise error (`BXC_UNAVAILABLE`) — no silent stubs.

## Testing each component

```bash
# Skill loaded?
agent-skills list .claude/plugins/aphrody/skills

# MCP server boots?
bun run .claude/plugins/aphrody/mcp/bxc-scrapper/server.ts < /dev/null
# (expect a JSON-RPC initialize handshake on stdin; abort with Ctrl-C)

# Hook fires? Edit any .ts file in a Claude Code session and watch
# the PostToolUse line in the transcript.

# Slash commands? In a Claude session: /scrape https://m3.material.io
# or /tokens
```

## Policy reminders (do not violate)

- Bun only. No `npm`/`node`/`yarn`/`pnpm` calls anywhere in this plugin.
- Zero stubs, TODO, FIXME, "implement later". Every tool does the work.
- Linux is the bloquant cible. Hooks and the MCP server must run on
  bare Ubuntu 26.04 with no Windows-only assumptions.
