# bxc-scrapper — MCP server for aphrody

Local MCP stdio server (Bun + TypeScript) wrapping the
[bxc](https://github.com/aphrody-code/bxc) Bun+Lightpanda scraping
engine. Exposes seven tools consumed by the aphrody Claude Code plugin.

## Install & run

```bash
cd .claude/plugins/aphrody/mcp/bxc-scrapper
bun install
bun run server.ts < /dev/null  # ad-hoc smoke (kill with Ctrl-C)
```

Claude Code launches the server automatically per the manifest
(`.claude-plugin/plugin.json#mcpServers.bxc-scrapper`); you do not
normally start it by hand.

## Backend resolution

The server reads `$BXC_DAEMON_URL` (default
`http://127.0.0.1:8765`) and treats it as a long-running bxc daemon.
For every tool call:

1. POST `${BXC_DAEMON_URL}/v1/<tool>` with a JSON body.
2. On any network error or non-2xx, fall back to spawning
   `bxc-engine` on PATH with `bxc-engine <tool> --json '<args>'`. The
   spawn output is parsed as JSON.
3. If `bxc-engine` is also missing, the tool returns a structured
   error `{ "error": "BXC_UNAVAILABLE", "reason": "<details>" }` —
   never a stub.

## Tools

| Tool | Args | Returns |
|---|---|---|
| `bxc_scrape` | `url`, `selector?` (default `body`) | `{ extractions: string[] }` |
| `bxc_recon` | `url` | `{ headers, cdn, frameworks, assets, css, screenshot_path }` |
| `bxc_detect` | `url` | `{ tech: DetectedTech[] }` |
| `google_search` | `query`, `hl?` (default `en`) | `{ results: OrganicResult[] }` |
| `google_atlas_route` | `url` | `{ profile, stealth_hints, framework }` |
| `extract_structured` | `html`, `zod_schema_json` | structured JSON validated against the schema |
| `vision_analyze` | `screenshot_path` | `{ elements, text, colors, fonts, hierarchy }` |

## Environment variables

| Var | Default | Purpose |
|---|---|---|
| `BXC_DAEMON_URL` | `http://127.0.0.1:8765` | bxc daemon base URL |
| `BXC_ENGINE_BIN` | `bxc-engine` | fallback binary name / path |
| `BXC_TIMEOUT_MS` | `30000` | per-tool timeout |
| `BXC_VISION_MIN_BYTES` | `1024` | minimum screenshot size before vision analysis |

## Cross-platform notes

- The server uses only Bun APIs (`fetch`, `Bun.spawn`, `Bun.file`) — no
  Node-only modules.
- Spawn fallback works on Linux (PATH lookup) and Windows
  (PATHEXT-aware via Bun's spawn).
- `screenshot_path` is returned verbatim from bxc; if the daemon and
  Claude Code run on the same host (the normal case), the path
  resolves locally.
