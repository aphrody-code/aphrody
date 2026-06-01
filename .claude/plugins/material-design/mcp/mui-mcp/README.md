<!-- SPDX-License-Identifier: MIT -->

# mui-mcp — vendored MUI MCP server (bunized)

Vendored and bunized copy of `@mui/mcp` 0.1.0 (MIT).

Upstream source: `https://registry.npmjs.org/@mui/mcp/-/mcp-0.1.0.tgz`
gitHead: `d481aac9962399271e3312bd30fbf17206add95b`
Upstream author: Bharat Kashyap, MUI

The original package ships a pre-compiled CJS bundle that depends on `@mastra/mcp`,
`@mastra/core/tools`, and `@mui-chat/tools` (a private MUI workspace package not
published to npm). This vendored copy re-implements all tool logic directly in TypeScript,
runs under `bun run` with no compile step, and replaces `@mastra/*` with the official
`@modelcontextprotocol/sdk`.

## What the server serves

The server exposes four MCP tools that give AI clients access to official MUI docs:

- `use_mui_docs` — primary entry point; description embeds live package list (55 packages,
  fetched from `chat-backend.mui.com` at startup) with links to llms.txt files for each
  version. Covers `@mui/material`, `@mui/icons-material`, `@mui/system`, `@mui/x-charts`,
  `@mui/x-data-grid`, `@mui/x-date-pickers`, `@mui/x-tree-view`, `@mui/x-scheduler`,
  `@mui/x-chat`, and more, across versions 5-9.
- `fetch_docs` — fetches the actual markdown content from a specific doc URL.
- `list_doc_sources` — lists all available packages with their llms.txt URLs.
- `fetch_mui_docs` — alias fetch tool for direct URL fetching.

All tools use `p-queue` (concurrency=10) and an in-memory URL cache.

## Changes from upstream

- Replaced `@mastra/mcp` + `@mastra/core/tools` with `@modelcontextprotocol/sdk` (stdio
  transport, direct JSON-RPC handler registration).
- Replaced `@mui-chat/tools` (private workspace) with native TypeScript implementations
  of all four tools, reverse-engineered from the compiled bundle (`dist/stdio.cjs.js`).
- Made `createUseMuiDocsTool` async so the package list is fetched at server startup and
  the tool description is fully populated before the first `tools/list` response.
- Dropped `throwOnTimeout` (removed from p-queue 9.x); kept `concurrency` + `timeout`.
- Added a minimal inline zod-to-JSON-Schema converter (only the subset used by these tools).
- Entry point: `src/index.ts` — runs directly with `bun run`, no build step.

## Running

```sh
cd plugins/material-design/mcp/mui-mcp
bun install
bun run src/index.ts
```

## Plugin wiring (mcpServers entry)

```json
{
  "command": "bun",
  "args": ["run", "${CLAUDE_PLUGIN_ROOT}/mcp/mui-mcp/src/index.ts"],
  "env": {}
}
```

No environment variables are required. The server fetches package metadata live from
`https://chat-backend.mui.com/v1/public/packages/list` and doc content from
`https://llms.mui.com/` on each tool invocation (cached in-process).

## License

MIT — same as upstream `@mui/mcp`. See the SPDX headers in each source file.
