#!/usr/bin/env bun
// SPDX-License-Identifier: MIT
// Upstream: @mui/mcp 0.1.0 (npm, gitHead d481aac9962399271e3312bd30fbf17206add95b)
// License: MIT (preserved from upstream MUI source)
// Bunized by aphrody-code/material-web: replaced @mastra/mcp + @mastra/core/tools +
// @mui-chat/tools (private workspace) with @modelcontextprotocol/sdk + p-queue + zod.
// Run: bun run src/index.ts

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  type Tool,
} from "@modelcontextprotocol/sdk/types.js";
import { zodToJsonSchema } from "./zod-to-json-schema.js";
import {
  createUseMuiDocsTool,
  createFetchDocTool,
  createListDocSourcesTool,
  createFetchMuiDocsTool,
  type PackageEntry,
  type ToolDefinition,
} from "./tools.js";

// ---------------------------------------------------------------------------
// Backend API
// ---------------------------------------------------------------------------

const BACKEND_BASE = "https://chat-backend.mui.com";
const PACKAGES_LIST_URL = `${BACKEND_BASE}/v1/public/packages/list`;

async function fetchRemotePackages(): Promise<PackageEntry[]> {
  const response = await fetch(PACKAGES_LIST_URL);
  if (!response.ok) {
    throw new Error(`Failed to fetch packages list: HTTP ${response.status}`);
  }
  const data = (await response.json()) as unknown;
  if (!Array.isArray(data) || data.length === 0) {
    throw new Error("Packages list not found or empty");
  }
  return data as PackageEntry[];
}

// ---------------------------------------------------------------------------
// Build MCP tools
// ---------------------------------------------------------------------------

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyToolDef = ToolDefinition<any, any>;

async function buildTools(): Promise<Map<string, AnyToolDef>> {
  const toolMap = new Map<string, AnyToolDef>();

  const useMuiDocs = await createUseMuiDocsTool({
    getPackagesList: fetchRemotePackages,
    queue: { concurrency: 10 },
    cache: true,
  });
  toolMap.set(useMuiDocs.name, useMuiDocs);

  const fetchDocs = createFetchDocTool({
    overrides: {
      description:
        'Fetch documentation for one or more URLs extracted from previous tool calls responses. The URLs should be passed as an array in the "urls" argument.',
    },
    cache: true,
  });
  toolMap.set(fetchDocs.name, fetchDocs);

  const listDocSources = await createListDocSourcesTool({
    getPackagesList: fetchRemotePackages,
    queue: { concurrency: 10 },
  });
  toolMap.set(listDocSources.name, listDocSources);

  const fetchMuiDocs = createFetchMuiDocsTool({
    getPackagesList: fetchRemotePackages,
    queue: { concurrency: 10 },
    cache: true,
  });
  toolMap.set(fetchMuiDocs.name, fetchMuiDocs);

  return toolMap;
}

// ---------------------------------------------------------------------------
// MCP Server bootstrap
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  const toolMap = await buildTools();

  const server = new Server({ name: "mui-mcp", version: "0.1.0" }, { capabilities: { tools: {} } });

  // List tools handler
  server.setRequestHandler(ListToolsRequestSchema, () => {
    const tools: Tool[] = Array.from(toolMap.values()).map((t) => ({
      name: t.name,
      description: t.description,
      inputSchema: zodToJsonSchema(t.inputSchema) as Tool["inputSchema"],
    }));
    return { tools };
  });

  // Call tool handler
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    const tool = toolMap.get(name);
    if (!tool) {
      return {
        content: [{ type: "text" as const, text: `Unknown tool: ${name}` }],
        isError: true,
      };
    }

    const startTime = Date.now();
    try {
      const parsed = tool.inputSchema.safeParse(args);
      if (!parsed.success) {
        return {
          content: [
            {
              type: "text" as const,
              text: `Invalid arguments: ${parsed.error.message}`,
            },
          ],
          isError: true,
        };
      }
      const result: unknown = await tool.execute(parsed.data);
      console.error(`Executed ${name} in ${Date.now() - startTime}ms`);
      return {
        content: [
          {
            type: "text" as const,
            text: typeof result === "string" ? result : JSON.stringify(result),
          },
        ],
      };
    } catch (err) {
      console.error(`Error executing ${name}:`, err);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error: ${err instanceof Error ? err.message : String(err)}`,
          },
        ],
        isError: true,
      };
    }
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("mui-mcp server started (stdio transport)");
}

main().catch((err: unknown) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
