# bxc-scrapper — MCP server for aphrody

Pure-Rust MCP stdio server wrapping the
[bxc](https://github.com/aphrody-code/bxc) Lightpanda scraping engine.
Built as the `bxc-mcp` binary in the [`bxc-engine`](../../../../crates/bxc-engine)
crate, it exposes seven tools consumed by the aphrody Claude Code plugin.

Replaces the previous Bun/TypeScript shim (`server.ts`, `package.json`,
`tsconfig.json` — removed). Cross-platform, zero JS runtime, AOT-ready.

## Run

Claude Code launches the server automatically per the manifest
(`.claude-plugin/plugin.json#mcpServers.bxc-scrapper`); you do not
normally start it by hand. For manual smoke tests:

```bash
# JSON tool catalog
cargo run --quiet --release -p bxc-engine --bin bxc-mcp -- --list-tools

# Drive an MCP session by hand
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' \
  | cargo run --quiet --release -p bxc-engine --bin bxc-mcp
```

## Backend resolution

The server reads `$BXC_DAEMON_URL` (default `http://127.0.0.1:8765`) and
treats it as a long-running bxc daemon. For every tool call:

1. POST `${BXC_DAEMON_URL}/v1/<tool>` with a JSON body.
2. On any network error or non-2xx, the tool returns a structured error
   `{ "error": "BXC_UNAVAILABLE" | "BXC_TIMEOUT" | "BXC_BAD_REQUEST" | "BXC_INVALID_RESPONSE", "reason": "<details>", "daemon_attempt": "<url>" }`.

No subprocess fallback: this binary is itself part of `bxc-engine`, so
spawning a sibling would be a tautology. Start the daemon with
`bxc-engine launch` (or the appropriate aphrody subcommand) when the
tools need to do real work.

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
| `BXC_TIMEOUT_MS` | `30000` | per-tool HTTP timeout |
| `BXC_VISION_MIN_BYTES` | `1024` | minimum screenshot size before vision analysis |

## Cross-platform notes

- Pure Rust, single binary — works on Linux #1, Windows #2, and any
  target where the workspace builds.
- TLS via rustls + ring; ring provider installed at startup before any
  request goes out (cf. CLAUDE.md §7).
- stdin must be a pipe; the binary refuses to dangle on a TTY and exits
  with a helpful hint.
