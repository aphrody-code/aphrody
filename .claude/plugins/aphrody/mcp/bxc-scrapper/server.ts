#!/usr/bin/env bun
/**
 * bxc-scrapper — MCP stdio server for the aphrody Claude Code plugin.
 *
 * Exposes 7 tools backed by the bxc (Bun + Lightpanda) scraping engine.
 *
 * Backend resolution (per call):
 *   1. POST to ${BXC_DAEMON_URL}/v1/<tool> (default http://127.0.0.1:8765)
 *   2. On failure, spawn `${BXC_ENGINE_BIN}` (default `bxc-engine`) with
 *      `<tool> --json '<args>'` and parse stdout as JSON.
 *   3. On both failures, return { error: "BXC_UNAVAILABLE", reason }.
 *
 * NO stubs, NO synthetic data — every tool is a real wrapper.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

// ---- Configuration -------------------------------------------------------

const BXC_DAEMON_URL = (process.env.BXC_DAEMON_URL ?? "http://127.0.0.1:8765").replace(/\/$/, "");
const BXC_ENGINE_BIN = process.env.BXC_ENGINE_BIN ?? "bxc-engine";
const BXC_TIMEOUT_MS = Number.parseInt(process.env.BXC_TIMEOUT_MS ?? "30000", 10);
const BXC_VISION_MIN_BYTES = Number.parseInt(process.env.BXC_VISION_MIN_BYTES ?? "1024", 10);

const SERVER_NAME = "bxc-scrapper";
const SERVER_VERSION = "0.1.0";

// ---- Backend invocation --------------------------------------------------

interface BxcError {
  error: "BXC_UNAVAILABLE" | "BXC_INVALID_RESPONSE" | "BXC_TIMEOUT" | "BXC_BAD_REQUEST";
  reason: string;
  daemon_attempt?: string;
  engine_attempt?: string;
}

type BxcResult<T> = { ok: true; value: T } | { ok: false; error: BxcError };

function isBxcError(v: unknown): v is BxcError {
  return (
    typeof v === "object" &&
    v !== null &&
    "error" in v &&
    typeof (v as { error: unknown }).error === "string"
  );
}

async function callDaemon<T>(tool: string, args: Record<string, unknown>): Promise<BxcResult<T>> {
  const url = `${BXC_DAEMON_URL}/v1/${tool}`;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), BXC_TIMEOUT_MS);
  try {
    const res = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "User-Agent": `${SERVER_NAME}/${SERVER_VERSION}`,
      },
      body: JSON.stringify(args),
      signal: controller.signal,
    });
    clearTimeout(timer);
    if (!res.ok) {
      const body = await res.text().catch(() => "");
      return {
        ok: false,
        error: {
          error: "BXC_BAD_REQUEST",
          reason: `daemon HTTP ${res.status}: ${body.slice(0, 512)}`,
          daemon_attempt: url,
        },
      };
    }
    const json = (await res.json()) as unknown;
    if (isBxcError(json)) {
      return { ok: false, error: { ...json, daemon_attempt: url } };
    }
    return { ok: true, value: json as T };
  } catch (err) {
    clearTimeout(timer);
    return {
      ok: false,
      error: {
        error: "BXC_UNAVAILABLE",
        reason: `daemon ${url}: ${(err as Error).message}`,
        daemon_attempt: url,
      },
    };
  }
}

async function callEngine<T>(tool: string, args: Record<string, unknown>): Promise<BxcResult<T>> {
  const argv = [BXC_ENGINE_BIN, tool, "--json", JSON.stringify(args)];
  try {
    const proc = Bun.spawn(argv, {
      stdout: "pipe",
      stderr: "pipe",
      stdin: "ignore",
      env: { ...process.env },
    });
    const timer = setTimeout(() => {
      try {
        proc.kill();
      } catch {
        /* ignore */
      }
    }, BXC_TIMEOUT_MS);

    const [stdout, stderr, code] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
      proc.exited,
    ]);
    clearTimeout(timer);

    if (code !== 0) {
      return {
        ok: false,
        error: {
          error: "BXC_BAD_REQUEST",
          reason: `${BXC_ENGINE_BIN} exit ${code}: ${stderr.slice(0, 512) || stdout.slice(0, 512)}`,
          engine_attempt: argv.join(" "),
        },
      };
    }
    const trimmed = stdout.trim();
    if (trimmed.length === 0) {
      return {
        ok: false,
        error: {
          error: "BXC_INVALID_RESPONSE",
          reason: `${BXC_ENGINE_BIN} returned empty stdout`,
          engine_attempt: argv.join(" "),
        },
      };
    }
    try {
      const json = JSON.parse(trimmed) as unknown;
      if (isBxcError(json)) {
        return { ok: false, error: { ...json, engine_attempt: argv.join(" ") } };
      }
      return { ok: true, value: json as T };
    } catch (err) {
      return {
        ok: false,
        error: {
          error: "BXC_INVALID_RESPONSE",
          reason: `${BXC_ENGINE_BIN} stdout is not JSON: ${(err as Error).message}`,
          engine_attempt: argv.join(" "),
        },
      };
    }
  } catch (err) {
    // Spawn itself failed (binary not found, EACCES, ...)
    return {
      ok: false,
      error: {
        error: "BXC_UNAVAILABLE",
        reason: `${BXC_ENGINE_BIN} spawn failed: ${(err as Error).message}`,
        engine_attempt: argv.join(" "),
      },
    };
  }
}

async function callBxc<T>(tool: string, args: Record<string, unknown>): Promise<BxcResult<T>> {
  const daemon = await callDaemon<T>(tool, args);
  if (daemon.ok) return daemon;

  const engine = await callEngine<T>(tool, args);
  if (engine.ok) return engine;

  // Both failed — merge the diagnostics so the caller knows exactly what
  // we tried.
  return {
    ok: false,
    error: {
      error: "BXC_UNAVAILABLE",
      reason: `daemon and engine both unavailable. daemon: ${daemon.error.reason} | engine: ${engine.error.reason}`,
      daemon_attempt: daemon.error.daemon_attempt,
      engine_attempt: engine.error.engine_attempt,
    },
  };
}

// ---- MCP response helpers ------------------------------------------------

type McpResult = {
  content: Array<{ type: "text"; text: string }>;
  isError?: boolean;
  structuredContent?: Record<string, unknown>;
};

function asMcpResult<T>(res: BxcResult<T>): McpResult {
  if (res.ok) {
    const payload = res.value as unknown as Record<string, unknown>;
    return {
      content: [{ type: "text", text: JSON.stringify(payload, null, 2) }],
      structuredContent: payload,
    };
  }
  return {
    content: [{ type: "text", text: JSON.stringify(res.error, null, 2) }],
    isError: true,
    structuredContent: res.error as unknown as Record<string, unknown>,
  };
}

// ---- Server registration -------------------------------------------------

const server = new McpServer(
  { name: SERVER_NAME, version: SERVER_VERSION },
  {
    instructions:
      "bxc-scrapper exposes the aphrody bxc engine. Prefer `bxc_recon` for first-touch reconnaissance, `bxc_scrape` for targeted text extraction, `bxc_detect` for framework fingerprinting, the `google_*` tools for SERP work, and `vision_analyze` / `extract_structured` for structured data extraction. All tools return JSON; errors are surfaced with isError=true and a stable `error` code (BXC_UNAVAILABLE / BXC_INVALID_RESPONSE / BXC_BAD_REQUEST / BXC_TIMEOUT). Never invent values when a tool returns an error — surface the error verbatim.",
  },
);

// bxc_scrape — text extraction
server.registerTool(
  "bxc_scrape",
  {
    title: "bxc scrape",
    description:
      "Extracts text content from <url> matching <selector> (default `body`). Uses the bxc static profile when JS is not needed. Returns { extractions: string[] }.",
    inputSchema: {
      url: z.string().url().describe("Fully-qualified http(s) URL to scrape."),
      selector: z
        .string()
        .min(1)
        .default("body")
        .describe("CSS selector. Defaults to `body` (whole page text)."),
    },
  },
  async ({ url, selector }) => {
    const res = await callBxc<{ extractions: string[] }>("scrape", { url, selector });
    return asMcpResult(res);
  },
);

// bxc_recon — first-touch reconnaissance
server.registerTool(
  "bxc_recon",
  {
    title: "bxc recon",
    description:
      "First-touch reconnaissance: HTTP headers, CDN, framework fingerprints, asset inventory (js/css/img/font), embedded CSS, and a screenshot saved by the daemon. Returns { headers, cdn, frameworks, assets, css, screenshot_path }.",
    inputSchema: {
      url: z.string().url().describe("Fully-qualified http(s) URL to recon."),
    },
  },
  async ({ url }) => {
    const res = await callBxc<{
      headers: Record<string, string>;
      cdn: string[];
      frameworks: string[];
      assets: { js: number; css: number; img: number; font: number };
      css: string[];
      screenshot_path: string;
    }>("recon", { url });
    return asMcpResult(res);
  },
);

// bxc_detect — framework/CMS detection (wappalyzergo-backed)
server.registerTool(
  "bxc_detect",
  {
    title: "bxc detect",
    description:
      "Framework / CMS / library fingerprinting via wappalyzergo, IP-range matching, header heuristics, and CSP analysis. Returns { tech: DetectedTech[] }.",
    inputSchema: {
      url: z.string().url().describe("Fully-qualified http(s) URL to fingerprint."),
    },
  },
  async ({ url }) => {
    const res = await callBxc<{
      tech: Array<{
        name: string;
        category: string;
        confidence: number;
        version?: string;
        evidence?: string[];
      }>;
    }>("detect", { url });
    return asMcpResult(res);
  },
);

// google_search — SERP organic results
server.registerTool(
  "google_search",
  {
    title: "Google search",
    description:
      "Google web search via bxc's ghost profile (Lightpanda + stealth). Returns organic SERP results. Honors mandate-guard (Google-only routing). Returns { results: OrganicResult[] }.",
    inputSchema: {
      query: z.string().min(1).describe("Search query string."),
      hl: z
        .string()
        .min(2)
        .max(5)
        .default("en")
        .describe("Interface language (ISO 639-1 code). Default `en`."),
    },
  },
  async ({ query, hl }) => {
    const res = await callBxc<{
      results: Array<{
        position: number;
        title: string;
        url: string;
        snippet?: string;
        displayed_url?: string;
      }>;
    }>("google_search", { query, hl });
    return asMcpResult(res);
  },
);

// google_atlas_route — recommended profile for a URL
server.registerTool(
  "google_atlas_route",
  {
    title: "Google Atlas route",
    description:
      "For a Google-domain URL, returns the bxc Atlas recommendation: which profile to use (static / fast / stealth-wiz / stealth-spa / stealth-lit / max), stealth hints, and the detected framework. Returns { profile, stealth_hints, framework }.",
    inputSchema: {
      url: z.string().url().describe("Fully-qualified Google-domain URL."),
    },
  },
  async ({ url }) => {
    const res = await callBxc<{
      profile: "static" | "fast" | "stealth" | "stealth-wiz" | "stealth-spa" | "stealth-lit" | "max" | "http";
      stealth_hints: string[];
      framework: string;
    }>("google_atlas_route", { url });
    return asMcpResult(res);
  },
);

// extract_structured — typed extraction via local LLM + selector cache
server.registerTool(
  "extract_structured",
  {
    title: "Structured extraction",
    description:
      "Typed structured extraction: takes raw HTML and a JSON-Schema (or stringified Zod schema). First call uses the local Gemma 4 (llama.cpp) to learn CSS selectors; subsequent calls reuse the cache for <1ms/page. Returns the validated typed JSON.",
    inputSchema: {
      html: z.string().min(1).describe("Raw HTML to extract from."),
      zod_schema_json: z
        .string()
        .min(2)
        .describe(
          "JSON-Schema-shaped extraction target (Zod-compatible). Server validates the LLM output against this schema.",
        ),
    },
  },
  async ({ html, zod_schema_json }) => {
    // Validate the schema string is parseable JSON before we hand it to
    // the backend — this catches client-side mistakes early.
    try {
      JSON.parse(zod_schema_json);
    } catch (err) {
      return asMcpResult<unknown>({
        ok: false,
        error: {
          error: "BXC_BAD_REQUEST",
          reason: `zod_schema_json is not valid JSON: ${(err as Error).message}`,
        },
      });
    }
    const res = await callBxc<Record<string, unknown>>("extract_structured", {
      html,
      schema: zod_schema_json,
    });
    return asMcpResult(res);
  },
);

// vision_analyze — screenshot vision pipeline
server.registerTool(
  "vision_analyze",
  {
    title: "Vision analyze",
    description:
      "Vision pipeline over a screenshot saved by bxc_recon (or any local PNG/WebP). Extracts dominant colors, font candidates, visible text, element layout, and a coarse hierarchy. Returns { elements, text, colors, fonts, hierarchy }.",
    inputSchema: {
      screenshot_path: z
        .string()
        .min(1)
        .describe("Absolute or workspace-relative path to a PNG / WebP screenshot."),
    },
  },
  async ({ screenshot_path }) => {
    // Sanity: file must exist and be > BXC_VISION_MIN_BYTES, otherwise
    // the backend will waste time on a corrupt image.
    try {
      const file = Bun.file(screenshot_path);
      const size = file.size;
      if (size === 0) {
        return asMcpResult<unknown>({
          ok: false,
          error: {
            error: "BXC_BAD_REQUEST",
            reason: `screenshot_path '${screenshot_path}' does not exist or is empty`,
          },
        });
      }
      if (size < BXC_VISION_MIN_BYTES) {
        return asMcpResult<unknown>({
          ok: false,
          error: {
            error: "BXC_BAD_REQUEST",
            reason: `screenshot_path '${screenshot_path}' is ${size}B (< BXC_VISION_MIN_BYTES=${BXC_VISION_MIN_BYTES}); refusing to vision-analyze a likely corrupt image`,
          },
        });
      }
    } catch (err) {
      return asMcpResult<unknown>({
        ok: false,
        error: {
          error: "BXC_BAD_REQUEST",
          reason: `unable to stat screenshot_path '${screenshot_path}': ${(err as Error).message}`,
        },
      });
    }

    const res = await callBxc<{
      elements: Array<{
        type: string;
        bbox: [number, number, number, number];
        confidence: number;
      }>;
      text: Array<{ text: string; bbox: [number, number, number, number] }>;
      colors: Array<{ hex: string; ratio: number; role?: string }>;
      fonts: Array<{ family: string; weight?: number; size_px?: number }>;
      hierarchy: Array<{ id: string; parent?: string; tag?: string; role?: string }>;
    }>("vision_analyze", { screenshot_path });
    return asMcpResult(res);
  },
);

// ---- Boot ----------------------------------------------------------------

async function main(): Promise<void> {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  // stderr is fine for logs — stdout is the JSON-RPC channel.
  console.error(
    `[${SERVER_NAME}] v${SERVER_VERSION} on stdio | daemon=${BXC_DAEMON_URL} engine=${BXC_ENGINE_BIN} timeout=${BXC_TIMEOUT_MS}ms`,
  );
}

process.on("SIGINT", async () => {
  try {
    await server.close();
  } finally {
    process.exit(0);
  }
});
process.on("SIGTERM", async () => {
  try {
    await server.close();
  } finally {
    process.exit(0);
  }
});

main().catch((err: unknown) => {
  console.error(`[${SERVER_NAME}] fatal:`, err);
  process.exit(1);
});
