// SPDX-License-Identifier: MIT
// Upstream: @mui/mcp 0.1.0 + @mui-chat/tools (MUI private workspace package)
// Source: https://registry.npmjs.org/@mui/mcp/-/mcp-0.1.0.tgz
// gitHead: d481aac9962399271e3312bd30fbf17206add95b
// License: MIT (preserved from upstream)
// Bunized by aphrody-code/material-web: replaced @mastra/* + @mui-chat/tools with
// @modelcontextprotocol/sdk + p-queue + zod, runs directly under `bun run`.

import PQueue from "p-queue";
import { z } from "zod";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface PackageEntry {
  name: string;
  version: string;
  llmsUrl: string;
}

export interface ToolDefinition<TInput, TOutput> {
  name: string;
  description: string;
  inputSchema: z.ZodType<TInput>;
  outputSchema: z.ZodType<TOutput>;
  execute: (input: TInput) => Promise<TOutput>;
}

// Queue options: p-queue 9.x dropped throwOnTimeout; only concurrency + timeout remain.
interface QueueOpts {
  concurrency?: number;
  timeout?: number;
}

// ---------------------------------------------------------------------------
// Cache (in-memory, per process lifetime — mirrors upstream behavior)
// ---------------------------------------------------------------------------

const urlCache = new Map<string, string>();

// ---------------------------------------------------------------------------
// Core fetch helper
// Mirrors the xa() function from the upstream compiled bundle:
//   xa(queue, fetcher, urls, cacheEnabled?) => Promise<string>
// ---------------------------------------------------------------------------

async function fetchUrls(
  queue: PQueue,
  urls: string[],
  cacheEnabled: boolean = false,
): Promise<string> {
  const results = await Promise.all(
    urls.map((url) =>
      queue.add(async () => {
        if (cacheEnabled) {
          const cached = urlCache.get(url);
          if (cached !== undefined) return cached;
        }
        try {
          const response = await fetch(url);
          if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
          }
          const text = await response.text();
          if (cacheEnabled) {
            queueMicrotask(() => {
              try {
                urlCache.set(url, text);
              } catch (err) {
                console.error("Failed to update cache:", err);
              }
            });
          }
          return text;
        } catch (err) {
          console.error(`Failed to fetch ${url}:`, err);
          return "Could not fetch documentation";
        }
      }),
    ),
  );

  const valid = results.filter(Boolean) as string[];
  return valid.length > 0 ? valid.join("\n") : "No documentation could be retrieved";
}

// ---------------------------------------------------------------------------
// Tool: use_mui_docs
// Mirrors createUseMuiDocsTool from @mui-chat/tools
// ---------------------------------------------------------------------------

export interface UseMuiDocsOptions {
  getPackagesList: () => Promise<PackageEntry[]>;
  queue?: QueueOpts;
  cache?: boolean;
}

export async function createUseMuiDocsTool(
  opts: UseMuiDocsOptions,
): Promise<ToolDefinition<{ urlList: string[] }, string>> {
  const queue = new PQueue({
    concurrency: opts.queue?.concurrency ?? 10,
    timeout: opts.queue?.timeout,
  });
  const cacheEnabled = opts.cache ?? false;

  // Fetch package list at construction time so the description is populated
  // before the first tools/list response is sent (mirrors upstream behavior
  // where @mui-chat/tools awaits the list during createUseMuiDocsTool).
  const packages = await opts.getPackagesList();
  const packagesDescription = packages
    .map((p) => `[${p.name}@${p.version}](${p.llmsUrl})`)
    .join("\n");

  return {
    name: "use_mui_docs",
    description: `
      You must use this tool to answer any questions related to MUI components or documentation.

      The description of the tool contains links to llms.txt files or local file paths that the user has made available.

      ${packagesDescription}

      1. Pick the most suitable URL from the above list, and use that as the "urlList" argument for this tool's execution, to get the docs content. If it's just one, let it be an array with one URL.
      2. Analyze the URLs listed in the llms.txt file
      3. Then fetch specific documentation pages relevant to the user's question with the subsequent tool call.
    `,
    inputSchema: z.object({
      urlList: z.array(z.string()).describe("The list of urls to fetch the documentation from"),
    }),
    outputSchema: z.string().describe("A string containing the fetched documentation content"),
    execute: async ({ urlList }) => fetchUrls(queue, urlList, cacheEnabled),
  };
}

// ---------------------------------------------------------------------------
// Tool: fetch_docs
// Mirrors createFetchDocTool from @mui-chat/tools
// ---------------------------------------------------------------------------

export interface FetchDocOptions {
  overrides?: { description?: string };
  queue?: QueueOpts;
  cache?: boolean;
}

export function createFetchDocTool(
  opts: FetchDocOptions = {},
): ToolDefinition<{ url: string }, string> {
  const queue = new PQueue({
    concurrency: opts.queue?.concurrency ?? 10,
    timeout: opts.queue?.timeout,
  });
  const cacheEnabled = opts.cache ?? false;

  return {
    name: "fetch_docs",
    description:
      opts.overrides?.description ??
      "Fetch and parse documentation from a given URL file to markdown format.",
    inputSchema: z.object({
      url: z.string().describe("The URL to fetch documentation from"),
    }),
    outputSchema: z.string().describe("The markdown content of the documentation"),
    execute: async ({ url }) => fetchUrls(queue, [url], cacheEnabled),
  };
}

// ---------------------------------------------------------------------------
// Tool: list_doc_sources
// Mirrors createListDocSourcesTool from @mui-chat/tools
// ---------------------------------------------------------------------------

export interface ListDocSourcesOptions {
  getPackagesList: () => Promise<PackageEntry[]>;
  overrides?: { description?: string };
  queue?: QueueOpts;
}

export async function createListDocSourcesTool(
  opts: ListDocSourcesOptions,
): Promise<ToolDefinition<{ packages: string[] }, string>> {
  const packageMap = await opts.getPackagesList().then((list) =>
    list.reduce<Record<string, PackageEntry>>((acc, p) => {
      acc[`${p.name}@${p.version}`] = p;
      return acc;
    }, {}),
  );

  const availableList = Object.keys(packageMap)
    .map((k) => `- ${k}`)
    .join("\n");

  const queue = new PQueue({
    concurrency: opts.queue?.concurrency ?? 10,
    timeout: opts.queue?.timeout,
  });

  return {
    name: "list_doc_sources",
    description:
      opts.overrides?.description ??
      `
List all available documentation sources.

This is the first tool you should call in the documentation workflow.
It provides URLs to llms.txt files or local file paths that the user has made available for all packages.

## Available Packages
${availableList}
`,
    inputSchema: z.object({
      packages: z.array(z.string()).describe("The list of packages you need to get docs list for"),
    }),
    outputSchema: z.string().describe("A string containing the fetched documentation content"),
    execute: async ({ packages }) => {
      const urls = packages
        .map((pkg) => packageMap[pkg]?.llmsUrl)
        .filter((u): u is string => Boolean(u));
      return fetchUrls(queue, urls, false);
    },
  };
}

// ---------------------------------------------------------------------------
// Tool: fetch_mui_docs (per-package list + fetch combined)
// Mirrors createMUIDocsTool from @mui-chat/tools
// ---------------------------------------------------------------------------

export interface FetchMuiDocsOptions {
  getPackagesList: () => Promise<PackageEntry[]>;
  queue?: QueueOpts;
  cache?: boolean;
}

export function createFetchMuiDocsTool(
  opts: FetchMuiDocsOptions,
): ToolDefinition<{ url: string }, string> {
  const queue = new PQueue({
    concurrency: opts.queue?.concurrency ?? 10,
    timeout: opts.queue?.timeout,
  });
  const cacheEnabled = opts.cache ?? false;

  return {
    name: "fetch_mui_docs",
    description: "Fetch and parse documentation from a given URL file to markdown format.",
    inputSchema: z.object({
      url: z.string().describe("The URL to fetch documentation from"),
    }),
    outputSchema: z.string().describe("The markdown content of the documentation"),
    execute: async ({ url }) => fetchUrls(queue, [url], cacheEnabled),
  };
}
