<!-- SPDX-License-Identifier: Apache-2.0 -->

# google_mcp

## What is `google_mcp`?

`google_mcp` is the Aphrody MCP (Model Context Protocol) server. It exposes
forensics, reconnaissance, and host-introspection tools to any MCP-aware agent
host (Claude Desktop, Cursor, Continue, Zed) through stdio or HTTP. The server
is built on `rmcp` (the official MCP Rust SDK), `axum` for HTTP, and the local
`backend` crate for real DNS / Chromium / process work. Every tool issues real
OS calls — no stubs.

## Install

Add to `Cargo.toml`:

```toml
[dependencies]
google_mcp = { path = "../google_mcp", version = "1.0.0-canary" }
```

The crate also ships a binary; install from the workspace with:

```bash
cargo install --path crates/google_mcp --locked
```

## Quick start

Run the server over the default stdio transport (used by Claude Desktop and
most MCP clients):

```bash
cargo run -p google_mcp
```

Then point your MCP client at the binary. Example `mcp.json` snippet:

```json
{
  "mcpServers": {
    "aphrody": { "command": "google_mcp" }
  }
}
```

## Exposed tools

Source of truth: `crates/google_mcp/src/main.rs` (`#[tool_router]` impl on
`GoogleMcpServer`). The current registry exposes eight tools:

| Tool | Description |
|---|---|
| `coding_style_guide` | Fetches the Google style guide for `cpp`, `python`, `typescript`, `java`, `shell`, `html`, `android_build`, or `chromium_build_win`. |
| `universal_web_fetch` | Generic HTTP-to-Markdown fetcher via the Jina reader proxy for external docs. |
| `dns_recon` | Runs the OSINT DNS pipeline from `backend::dns::DnsRecon`, returning deduplicated subdomains. |
| `advanced_recon` | Native OS DNS resolution plus 200 ms TCP probes across user-supplied ports (defaults: 80/443/8080). |
| `native_hooks` | OS state direct: `GetSystemInfo` + `GlobalMemoryStatusEx` on Windows, `/proc` via `sysinfo` on Linux/macOS. |
| `start_dashboard` | Spawns an `axum` server in a background tokio task exposing `GET /health` and `GET /info`. |
| `auth_extract` | Windows-only: DPAPI Chrome Canary cookie inspection via `backend::chromium::ChromiumParser`. |
| `chrome_autopsy` | Windows-only: `OpenProcess` + `ReadProcessMemory` against a target PID, 4 KiB cap, hex+ASCII dump. |

## Transports

`stdio` is the default (line-delimited JSON-RPC over `stdin`/`stdout`), matched
to the canonical MCP host workflow. HTTP is wired via `axum`, exposed by the
`start_dashboard` tool for liveness checks and PID/uptime telemetry.

## Cross-platform

Linux (cible #1) and Windows 11 Canary are fully supported. Windows-only tools
(`auth_extract`, `chrome_autopsy`) return an explicit `unsupported` message on
Linux instead of failing the request, so an agent can probe capability cleanly.

## License

Apache-2.0. Copyright aphrody-code contributors.

## Related

- `aphrody` — the CLI binary that consumes the same `backend` primitives.
- `a2a` / `a2a-client` / `a2a-server` — peer-to-peer agent protocol used in
  parallel with MCP for cross-agent coordination.
- `rmcp` — upstream Rust SDK powering the tool router and stdio transport.
