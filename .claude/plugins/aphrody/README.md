# aphrody — Claude Code plugin

Production-grade plugin shipping the assets that drive the aphrody
2026-05-17 roadmap: **3 skills**, **3 agents**, **2 slash commands**,
**3 PostToolUse hooks**, and the local **MCP server** backing the slash
commands.

Cross-platform support: Linux Ubuntu 26.04 (priority **#1**), Windows 11
Insider Canary (#2), WebAssembly (#3). Bun is required everywhere;
`node` is forbidden by repo policy (see `feedback_bun_only`).

## Layout

```
.claude/plugins/aphrody/
├── .claude-plugin/plugin.json
├── README.md
├── skills/
│   ├── pixel-perfect/          # M3 spec validator
│   │   ├── SKILL.md
│   │   └── references/{m3-spec,validation-checklist}.md
│   ├── rust-target-check/      # parallel xplatform cargo check
│   │   └── SKILL.md
│   └── m3-component/           # scaffold a new Material Web 3 wrapper
│       ├── SKILL.md
│       └── references/mapping.md
├── agents/
│   ├── n2b-ultra.md            # Node → Bun migrator
│   ├── cross-platform-validator.md  # 3-target verdict table
│   └── m3-spec-auditor.md      # composes pixel-perfect + bxc + playwright
├── commands/
│   ├── scrape.md
│   └── tokens.md
├── hooks/
│   ├── hooks.json              # aggregator (3 PostToolUse hooks)
│   ├── oxclint.ts              # ts/tsx/js/jsx → bunx oxlint
│   ├── cargo-check.ts          # .rs → cargo check -p <crate> --offline
│   └── cargo-toml-validate.ts  # Cargo.toml → cargo metadata --offline (blocking)
└── mcp/bxc-scrapper/           # MCP stdio server (Bun + TS, 7 tools)
    ├── package.json
    ├── tsconfig.json
    ├── README.md
    └── server.ts
```

## Install

Project-scoped under `.claude/plugins/aphrody/`. Claude Code discovers it
automatically on session start. If not, add it explicitly to the repo's
`.claude/settings.json`:

```json
{
  "plugins": {
    "aphrody": { "path": ".claude/plugins/aphrody" }
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

### Skills

#### `pixel-perfect`
Validates an M3 component implementation against the canonical
`m3.material.io` spec. Auto-triggers when the user asks to "audit
M3 tokens", "is this M3-compliant?", or "compare to `<md-…>`".
See `skills/pixel-perfect/SKILL.md`.

#### `rust-target-check` (user-only)
Runs `cargo check --offline` on the **3 priority targets** in parallel
(Linux x86_64, Windows MSVC, wasm32). Reports a consolidated verdict.
Invoke via `/rust-target-check [crate-name | --workspace]`.

#### `m3-component`
Scaffolds a new Material Web 3 wrapper under
`packages/ui/components/<name>/` from the Button POC template — `.tsx`
wrapper + `.css` token bridge + `.test.tsx` (3 cases). Mapping table
in `skills/m3-component/references/mapping.md`. Refuses to emit a
stub when the M3 spec has no equivalent.

### Agents

#### `n2b-ultra`
End-to-end Node → Bun migration. Drives the `n2b` crate (or the
published `@aphrody-code/n2b-cli` Bunx fallback). Strict anti-node
ruleset. Model: opus. Invoke via Agent tool with
`subagent_type: "n2b-ultra"`.

#### `cross-platform-validator`
Spawns `cargo check --workspace` on the 3 priority targets in parallel,
parses errors per crate, builds a verdict table, distinguishes regressions
from expected `compile_error!` gating. Read-only — surfaces data, doesn't
fix. Use before each push.

#### `m3-spec-auditor`
Composes the `pixel-perfect` skill, the **bxc-scrapper** MCP, and the
**playwright** MCP to audit a component against the live M3 spec page
(tokens, tags, motion, elevation, screenshot diff). Reports per-axis
pass/fail. Read-only.

### Slash commands

- **`/scrape <url> [selector]`** — runs the `bxc-scrapper` MCP
  `bxc_recon` tool against `<url>` and (optionally) extracts `<selector>`
  via `bxc_scrape`. Returns headers, CDN, framework detection, CSS
  asset inventory, screenshot path.
- **`/tokens [url]`** — scrapes Material Design 3 design tokens from
  `m3.material.io`, normalizes them, writes `packages/ui/tokens/m3.json`.

### Hooks (PostToolUse on Edit|Write|MultiEdit)

All three run in sequence, surfacing errors via stderr (Claude sees
them). Non-blocking by default; **`cargo-toml-validate` is blocking**
(exit 2) because a broken `Cargo.toml` poisons the workspace.

| Hook | Trigger | Action | Block? |
|---|---|---|---|
| `oxclint` | edited `.ts/.tsx/.js/.jsx` | `bunx oxlint --quiet --max-warnings 0` | yes on lint errors |
| `cargo-check` | edited `.rs` | detect parent crate → `cargo check -p <crate> --offline` | no (advisory; set `APHRODY_CARGO_HOOK_BLOCK=1` to enforce) |
| `cargo-toml-validate` | edited `Cargo.toml` | `cargo metadata --no-deps --offline` | **yes** (exit 2) |

### MCP server — `bxc-scrapper`

Local stdio MCP server (`bun run server.ts`) exposing 7 tools:

| Tool | Description |
|---|---|
| `bxc_scrape(url, selector?)` | Text extraction (selector defaults to `body`) |
| `bxc_recon(url)` | Headers, CDN, frameworks, assets, screenshot |
| `bxc_detect(url)` | DetectedTech[] via wappalyzergo |
| `google_search(query, hl?)` | SERP organic results |
| `google_atlas_route(url)` | {profile, stealth_hints, framework} |
| `extract_structured(html, zod_schema_json)` | Typed JSON via local Gemma 4 |
| `vision_analyze(screenshot_path)` | Elements, text, colors, fonts, hierarchy |

Backed by the bxc daemon at `$BXC_DAEMON_URL` (default
`http://127.0.0.1:8765`). If unreachable, falls back to spawning
`bxc-engine` directly. If both unavailable: precise `BXC_UNAVAILABLE`
error — **no silent stubs**.

## Testing each component

```bash
# Discover skills (3 expected)
agent-skills list .claude/plugins/aphrody/skills

# MCP server boots (Ctrl-C after the initialize handshake)
bun run .claude/plugins/aphrody/mcp/bxc-scrapper/server.ts

# Hooks fire — edit any .ts / .rs / Cargo.toml in a Claude Code session
# and watch the PostToolUse lines in the transcript

# Slash commands
#   /scrape https://m3.material.io/components/buttons/overview
#   /tokens
#   /rust-target-check
```

## Policy reminders

- **Bun only.** No `npm`/`node`/`yarn`/`pnpm` calls anywhere in this plugin.
- **Zero stubs / TODO / FIXME** — every tool does the work it claims.
- **Linux is the blocking target.** Hooks and the MCP server must run
  on bare Ubuntu 26.04 with no Windows-only assumptions.
- All hooks use `${CLAUDE_PLUGIN_ROOT}` (cross-platform) — never
  hard-coded paths.
