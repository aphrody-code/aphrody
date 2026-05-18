/**
 * @license Apache-2.0
 * @copyright 2026 aphrody-code
 *
 * aphrody-mcp — unified MCP stdio server for the aphrody Claude Code plugin.
 *
 * Fuses three previously-separate surfaces into one tool catalog :
 *
 *   1. **Scraping** (bxc-mcp Rust subprocess proxy)
 *      - aphrody_scrape, aphrody_recon, aphrody_detect, aphrody_search,
 *        aphrody_atlas_route, aphrody_extract_structured, aphrody_vision_analyze
 *      Implemented by spawning `bxc-mcp` and routing JSON-RPC calls.
 *
 *   2. **In-process memory** (Bun sqlite, no daemon)
 *      - aphrody_memory_get, aphrody_memory_set, aphrody_memory_list
 *      Persistent k/v store at $BXC_MEMORY_DB (defaults to
 *      $HOME/.aphrody/aphrody-memory.sqlite).
 *
 *   3. **aphrody CLI wrappers** (exec native binary, parse JSON)
 *      - aphrody_doctor, aphrody_version, aphrody_dns, aphrody_notify
 *      Forward to `aphrody {doctor --json, version, dns, notify}` and
 *      surface stdout. Honours $APHRODY_BIN (PATH lookup default).
 *
 * Replaces the previous `bxc-scrapper` + `bxc` dual MCP server config
 * with one unified server.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { Database } from "bun:sqlite";
import { spawn, type Subprocess } from "bun";
import { existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { homedir } from "node:os";

// ---------------------------------------------------------------------------
// Config (env-driven, sane defaults)
// ---------------------------------------------------------------------------

const BXC_DAEMON_URL = process.env.BXC_DAEMON_URL ?? "http://127.0.0.1:8765";
const APHRODY_BIN = process.env.APHRODY_BIN || "aphrody";
const BXC_MCP_BIN = process.env.BXC_MCP_BIN || "bxc-mcp";
const MEMORY_DB =
	process.env.BXC_MEMORY_DB ??
	join(homedir(), ".aphrody", "aphrody-memory.sqlite");
const GITHUB_TOKEN = process.env.GITHUB_PERSONAL_ACCESS_TOKEN ?? "";
const CONTEXT7_API_KEY = process.env.CONTEXT7_API_KEY ?? "";
const CONTEXT7_BASE = "https://mcp.context7.com";
const GITHUB_API = "https://api.github.com";

// Ensure memory dir exists before touching SQLite.
mkdirSync(dirname(MEMORY_DB), { recursive: true });
const db = new Database(MEMORY_DB);
db.run(`
  CREATE TABLE IF NOT EXISTS memories (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
  )
`);

// ---------------------------------------------------------------------------
// bxc-mcp subprocess proxy
//
// Lazy-spawned : the bxc-mcp child only starts on the first scraping tool
// call to avoid paying its boot cost when only memory/CLI tools are used.
// JSON-RPC 2.0 framed over newline-delimited stdio.
// ---------------------------------------------------------------------------

interface BxcMcp {
	proc: Subprocess<"pipe", "pipe", "pipe">;
	nextId: number;
	pending: Map<number, (response: unknown) => void>;
	buffer: string;
}

let bxcMcp: BxcMcp | null = null;

async function ensureBxcMcp(): Promise<BxcMcp> {
	if (bxcMcp && !bxcMcp.proc.killed) return bxcMcp;

	const proc = spawn({
		cmd: [BXC_MCP_BIN],
		stdin: "pipe",
		stdout: "pipe",
		stderr: "pipe",
		env: { ...process.env, BXC_DAEMON_URL },
	});

	bxcMcp = {
		proc,
		nextId: 1,
		pending: new Map(),
		buffer: "",
	};
	const ref = bxcMcp;

	// Read stdout, split on newlines, dispatch responses to pending callers.
	(async () => {
		const reader = proc.stdout.getReader();
		const decoder = new TextDecoder();
		while (true) {
			const { value, done } = await reader.read();
			if (done) break;
			ref.buffer += decoder.decode(value);
			let nl: number;
			while ((nl = ref.buffer.indexOf("\n")) >= 0) {
				const line = ref.buffer.slice(0, nl).trim();
				ref.buffer = ref.buffer.slice(nl + 1);
				if (!line) continue;
				try {
					const msg = JSON.parse(line) as { id?: number };
					if (typeof msg.id === "number") {
						const cb = ref.pending.get(msg.id);
						if (cb) {
							ref.pending.delete(msg.id);
							cb(msg);
						}
					}
				} catch {
					// Skip non-JSON noise (logs, banners).
				}
			}
		}
	})().catch(() => {
		// Stream closed; mark dead so next call respawns.
		bxcMcp = null;
	});

	// Initialise handshake.
	await rpcCall(ref, "initialize", {
		protocolVersion: "2025-06-18",
		capabilities: {},
		clientInfo: { name: "aphrody-mcp-proxy", version: "0.1.0" },
	});
	await rpcNotify(ref, "notifications/initialized", {});

	return ref;
}

async function rpcCall(
	ref: BxcMcp,
	method: string,
	params: unknown,
): Promise<unknown> {
	const id = ref.nextId++;
	const req = JSON.stringify({ jsonrpc: "2.0", id, method, params });
	const writer = ref.proc.stdin.getWriter();
	await writer.write(new TextEncoder().encode(`${req}\n`));
	writer.releaseLock();
	return new Promise((resolve, reject) => {
		ref.pending.set(id, (response) => {
			const r = response as { error?: { message: string }; result?: unknown };
			if (r.error) reject(new Error(r.error.message));
			else resolve(r.result);
		});
		// 60-second hard timeout matches `aphrody scrape` reqwest client.
		setTimeout(() => {
			if (ref.pending.delete(id)) {
				reject(new Error(`bxc-mcp ${method} timeout after 60s`));
			}
		}, 60_000);
	});
}

async function rpcNotify(
	ref: BxcMcp,
	method: string,
	params: unknown,
): Promise<void> {
	const req = JSON.stringify({ jsonrpc: "2.0", method, params });
	const writer = ref.proc.stdin.getWriter();
	await writer.write(new TextEncoder().encode(`${req}\n`));
	writer.releaseLock();
}

/**
 * Invoke a bxc-mcp tool by name and return the raw `content` array the
 * caller can pass through. Never invents data on error — surfaces the
 * upstream message verbatim.
 */
async function bxcCall(
	tool: string,
	args: Record<string, unknown>,
): Promise<{ content: Array<{ type: string; text: string }> }> {
	const ref = await ensureBxcMcp();
	const result = (await rpcCall(ref, "tools/call", {
		name: tool,
		arguments: args,
	})) as { content?: Array<{ type: string; text: string }> };
	return { content: result.content ?? [] };
}

// ---------------------------------------------------------------------------
// aphrody CLI exec wrapper
// ---------------------------------------------------------------------------

interface CliResult {
	stdout: string;
	stderr: string;
	exitCode: number;
}

async function runAphrody(args: string[]): Promise<CliResult> {
	const proc = spawn({
		cmd: [APHRODY_BIN, ...args],
		stdin: "ignore",
		stdout: "pipe",
		stderr: "pipe",
	});
	const [stdout, stderr] = await Promise.all([
		new Response(proc.stdout).text(),
		new Response(proc.stderr).text(),
	]);
	const exitCode = await proc.exited;
	return { stdout, stderr, exitCode };
}

function asTextContent(payload: unknown): {
	content: Array<{ type: "text"; text: string }>;
} {
	const text =
		typeof payload === "string"
			? payload
			: JSON.stringify(payload, null, 2);
	return { content: [{ type: "text", text }] };
}

function cliError(action: string, r: CliResult): never {
	throw new Error(
		`aphrody ${action} exited ${r.exitCode}: ${r.stderr.trim() || r.stdout.trim()}`,
	);
}

// ---------------------------------------------------------------------------
// MCP server registration
// ---------------------------------------------------------------------------

const server = new McpServer(
	{ name: "aphrody", version: "0.1.0" },
	{ capabilities: { tools: {} } },
);

// ── Scraping (forwarded to bxc-mcp Rust) ─────────────────────────────────────

server.registerTool(
	"aphrody_scrape",
	{
		description:
			"Extract textContent of elements matching a CSS selector via the bxc daemon. Auto-starts the daemon if absent (when wired through `aphrody scrape`). Returns array of {index, text}.",
		inputSchema: {
			url: z.string().url(),
			selector: z.string().default("body"),
		},
		annotations: { readOnlyHint: true },
	},
	async (args) => bxcCall("bxc_scrape", args),
);

server.registerTool(
	"aphrody_recon",
	{
		description:
			"Full-page reconnaissance: HTTP status, headers, CDN, framework fingerprints, asset inventory, screenshot path. Returns bxc-recon-v1 envelope.",
		inputSchema: { url: z.string().url() },
		annotations: { readOnlyHint: true },
	},
	async (args) => bxcCall("bxc_recon", args),
);

server.registerTool(
	"aphrody_detect",
	{
		description:
			"Deep technology detection: frontend, backend, CDN, DNS, hosting, CMS. Multi-bucket with confidence scores.",
		inputSchema: { url: z.string().url() },
		annotations: { readOnlyHint: true },
	},
	async (args) => bxcCall("bxc_detect", args),
);

server.registerTool(
	"aphrody_search",
	{
		description:
			"Google SERP organic results via bxc daemon. hl defaults to 'en'.",
		inputSchema: { query: z.string(), hl: z.string().default("en") },
		annotations: { readOnlyHint: true },
	},
	async (args) => bxcCall("google_search", args),
);

server.registerTool(
	"aphrody_atlas_route",
	{
		description:
			"Resolve the best bxc profile (static/fast/stealth/max) + stealth hints + detected framework for a URL.",
		inputSchema: { url: z.string().url() },
		annotations: { readOnlyHint: true },
	},
	async (args) => bxcCall("google_atlas_route", args),
);

server.registerTool(
	"aphrody_extract_structured",
	{
		description:
			"Extract typed JSON from raw HTML using a Zod-style JSON Schema. Backed by the local Gemma 4 in bxc.",
		inputSchema: {
			html: z.string(),
			zod_schema_json: z.string().describe("JSON-stringified Zod/JSON Schema"),
		},
		annotations: { readOnlyHint: true },
	},
	async (args) => bxcCall("extract_structured", args),
);

server.registerTool(
	"aphrody_vision_analyze",
	{
		description:
			"Analyse a saved screenshot (PNG/JPG) for elements, text, colors, fonts, and visual hierarchy.",
		inputSchema: { screenshot_path: z.string() },
		annotations: { readOnlyHint: true },
	},
	async (args) => bxcCall("vision_analyze", args),
);

// ── In-process memory (Bun sqlite) ──────────────────────────────────────────

server.registerTool(
	"aphrody_memory_set",
	{
		description:
			"Persist a key/value memory fact in the local SQLite store at $BXC_MEMORY_DB. Idempotent upsert.",
		inputSchema: { key: z.string(), value: z.string() },
	},
	async ({ key, value }) => {
		db.run(
			`INSERT INTO memories (key, value) VALUES (?, ?)
       ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                                      updated_at = CURRENT_TIMESTAMP`,
			[key, value],
		);
		return asTextContent({ ok: true, key });
	},
);

server.registerTool(
	"aphrody_memory_get",
	{
		description:
			"Retrieve a key's value from the local SQLite memory store. Returns null if not found.",
		inputSchema: { key: z.string() },
		annotations: { readOnlyHint: true },
	},
	async ({ key }) => {
		const row = db
			.query<{ value: string; created_at: string; updated_at: string }, string>(
				`SELECT value, created_at, updated_at FROM memories WHERE key = ?`,
			)
			.get(key);
		return asTextContent(row ?? { value: null });
	},
);

server.registerTool(
	"aphrody_memory_list",
	{
		description:
			"List all stored memory keys with timestamps. No values returned (use aphrody_memory_get).",
		inputSchema: {},
		annotations: { readOnlyHint: true },
	},
	async () => {
		const rows = db
			.query<
				{ key: string; created_at: string; updated_at: string },
				[]
			>(`SELECT key, created_at, updated_at FROM memories ORDER BY updated_at DESC`)
			.all();
		return asTextContent(rows);
	},
);

// ── aphrody CLI wrappers (exec) ──────────────────────────────────────────────

server.registerTool(
	"aphrody_doctor",
	{
		description:
			"Run `aphrody doctor --json` — environment + A2A peer + supply-chain diagnostic.",
		inputSchema: {},
		annotations: { readOnlyHint: true },
	},
	async () => {
		const r = await runAphrody(["doctor", "--json"]);
		if (r.exitCode !== 0) cliError("doctor", r);
		return asTextContent(JSON.parse(r.stdout));
	},
);

server.registerTool(
	"aphrody_version",
	{
		description:
			"Run `aphrody version` — binary version, commit, build target.",
		inputSchema: {},
		annotations: { readOnlyHint: true },
	},
	async () => {
		const r = await runAphrody(["version"]);
		if (r.exitCode !== 0) cliError("version", r);
		return asTextContent(r.stdout.trim());
	},
);

server.registerTool(
	"aphrody_dns",
	{
		description:
			"OSINT passive DNS recon — multi-source subdomain aggregation for a domain.",
		inputSchema: { domain: z.string() },
	},
	async ({ domain }) => {
		const r = await runAphrody(["dns", domain]);
		if (r.exitCode !== 0) cliError(`dns ${domain}`, r);
		return asTextContent(r.stdout.trim());
	},
);

server.registerTool(
	"aphrody_notify",
	{
		description:
			"Send a message via Slack / Telegram / Matrix. Reads creds from env (SLACK_BOT_TOKEN/SLACK_CHANNEL, TELEGRAM_BOT_TOKEN/TELEGRAM_CHAT_ID, MATRIX_*). Errors if env missing.",
		inputSchema: {
			channel: z.enum(["slack", "telegram", "matrix"]),
			message: z.string(),
			room: z.string().optional(),
		},
	},
	async ({ channel, message, room }) => {
		const args = ["notify", "--channel", channel, "--message", message];
		if (room) args.push("--room", room);
		const r = await runAphrody(args);
		if (r.exitCode !== 0) cliError(`notify ${channel}`, r);
		return asTextContent(r.stdout.trim() || "sent");
	},
);

server.registerTool(
	"aphrody_scan_tree",
	{
		description:
			"Repo analytics : size + file-count breakdown for top-level groups. Returns JSON (`{byGroup, directories}`).",
		inputSchema: {
			root: z.string().default("."),
			groups: z.string().default("crates"),
			top_ext: z.number().int().min(1).max(20).default(5),
		},
		annotations: { readOnlyHint: true },
	},
	async ({ root, groups, top_ext }) => {
		const r = await runAphrody([
			"scan",
			"tree",
			"--root",
			root,
			"--groups",
			groups,
			"--top-ext",
			String(top_ext),
			"-o",
			"-",
		]);
		if (r.exitCode !== 0) cliError(`scan tree ${root}`, r);
		// First lines are a human banner ; JSON starts at the first `{`.
		const idx = r.stdout.indexOf("{");
		return asTextContent(idx >= 0 ? JSON.parse(r.stdout.slice(idx)) : r.stdout);
	},
);

server.registerTool(
	"aphrody_scan_manifests",
	{
		description:
			"Walk the repo for Cargo.toml / package.json / pyproject.toml / etc. and emit metadata as JSON (`{byType, manifests}`).",
		inputSchema: { root: z.string().default(".") },
		annotations: { readOnlyHint: true },
	},
	async ({ root }) => {
		const r = await runAphrody(["scan", "manifests", "--root", root, "-o", "-"]);
		if (r.exitCode !== 0) cliError(`scan manifests ${root}`, r);
		const idx = r.stdout.indexOf("{");
		return asTextContent(idx >= 0 ? JSON.parse(r.stdout.slice(idx)) : r.stdout);
	},
);

server.registerTool(
	"aphrody_chromium_sync",
	{
		description:
			"Forensics : scan Chromium profiles and decrypt the master key (Windows-only path, no-op on other OSes).",
		inputSchema: {},
	},
	async () => {
		const r = await runAphrody(["chromium", "sync"]);
		if (r.exitCode !== 0) cliError("chromium sync", r);
		return asTextContent(r.stdout.trim());
	},
);

server.registerTool(
	"aphrody_a2a_prompt",
	{
		description:
			"Send a prompt to a running A2A agent. Falls back to the bundled Gemini CLI when no A2A server is reachable.",
		inputSchema: { prompt: z.string() },
	},
	async ({ prompt }) => {
		const r = await runAphrody(["a2a", prompt]);
		if (r.exitCode !== 0) cliError(`a2a ${prompt}`, r);
		return asTextContent(r.stdout.trim());
	},
);

// ── GitHub proxy (replaces cloud MCP github) ─────────────────────────────────

interface GhResponse {
	status: number;
	body: unknown;
	text: string;
}

async function gh(
	method: "GET" | "POST" | "PATCH",
	path: string,
	body?: unknown,
): Promise<GhResponse> {
	if (!GITHUB_TOKEN) {
		throw new Error(
			"GITHUB_PERSONAL_ACCESS_TOKEN env var not set — required for aphrody_github_* tools.",
		);
	}
	const init: RequestInit = {
		method,
		headers: {
			Authorization: `Bearer ${GITHUB_TOKEN}`,
			Accept: "application/vnd.github+json",
			"User-Agent": "aphrody-mcp/0.1.0",
			"X-GitHub-Api-Version": "2022-11-28",
			...(body ? { "Content-Type": "application/json" } : {}),
		},
		...(body ? { body: JSON.stringify(body) } : {}),
	};
	const resp = await fetch(`${GITHUB_API}${path}`, init);
	const text = await resp.text();
	let parsed: unknown;
	try {
		parsed = JSON.parse(text);
	} catch {
		parsed = text;
	}
	if (!resp.ok) {
		throw new Error(
			`GitHub API ${method} ${path} → HTTP ${resp.status}: ${typeof parsed === "string" ? parsed : JSON.stringify(parsed)}`,
		);
	}
	return { status: resp.status, body: parsed, text };
}

server.registerTool(
	"aphrody_github_list_issues",
	{
		description:
			"List issues on a GitHub repo. `state` defaults to 'open'. Returns the raw GitHub REST response (array of issues).",
		inputSchema: {
			owner: z.string(),
			repo: z.string(),
			state: z.enum(["open", "closed", "all"]).default("open"),
			per_page: z.number().int().min(1).max(100).default(30),
		},
		annotations: { readOnlyHint: true },
	},
	async ({ owner, repo, state, per_page }) => {
		const r = await gh(
			"GET",
			`/repos/${owner}/${repo}/issues?state=${state}&per_page=${per_page}`,
		);
		return asTextContent(r.body);
	},
);

server.registerTool(
	"aphrody_github_create_issue",
	{
		description:
			"Create a GitHub issue. Requires the token to have `repo` scope (or `public_repo` for public repos).",
		inputSchema: {
			owner: z.string(),
			repo: z.string(),
			title: z.string(),
			body: z.string().optional(),
			labels: z.array(z.string()).optional(),
			assignees: z.array(z.string()).optional(),
		},
	},
	async ({ owner, repo, ...rest }) => {
		const r = await gh("POST", `/repos/${owner}/${repo}/issues`, rest);
		return asTextContent(r.body);
	},
);

server.registerTool(
	"aphrody_github_list_prs",
	{
		description:
			"List pull requests on a GitHub repo. Returns the raw GitHub REST response.",
		inputSchema: {
			owner: z.string(),
			repo: z.string(),
			state: z.enum(["open", "closed", "all"]).default("open"),
			per_page: z.number().int().min(1).max(100).default(30),
		},
		annotations: { readOnlyHint: true },
	},
	async ({ owner, repo, state, per_page }) => {
		const r = await gh(
			"GET",
			`/repos/${owner}/${repo}/pulls?state=${state}&per_page=${per_page}`,
		);
		return asTextContent(r.body);
	},
);

server.registerTool(
	"aphrody_github_search_repos",
	{
		description:
			"Search GitHub repositories with a `q` query (e.g. 'language:rust topic:cli'). See GitHub REST docs for syntax.",
		inputSchema: {
			q: z.string(),
			sort: z.enum(["stars", "forks", "updated", "best-match"]).default("best-match"),
			per_page: z.number().int().min(1).max(100).default(20),
		},
		annotations: { readOnlyHint: true },
	},
	async ({ q, sort, per_page }) => {
		const sortParam = sort === "best-match" ? "" : `&sort=${sort}`;
		const r = await gh(
			"GET",
			`/search/repositories?q=${encodeURIComponent(q)}&per_page=${per_page}${sortParam}`,
		);
		return asTextContent(r.body);
	},
);

server.registerTool(
	"aphrody_github_get_repo",
	{
		description:
			"Fetch the metadata of a single GitHub repository (description, default_branch, stargazers_count, etc.).",
		inputSchema: { owner: z.string(), repo: z.string() },
		annotations: { readOnlyHint: true },
	},
	async ({ owner, repo }) => {
		const r = await gh("GET", `/repos/${owner}/${repo}`);
		return asTextContent(r.body);
	},
);

// ── Context7 proxy (replaces cloud MCP context7) ────────────────────────────

async function context7Fetch(
	method: "GET" | "POST",
	path: string,
	body?: unknown,
): Promise<unknown> {
	if (!CONTEXT7_API_KEY) {
		throw new Error(
			"CONTEXT7_API_KEY env var not set — required for aphrody_context7_* tools.",
		);
	}
	const init: RequestInit = {
		method,
		headers: {
			CONTEXT7_API_KEY,
			Accept: "application/json, text/event-stream",
			...(body ? { "Content-Type": "application/json" } : {}),
		},
		...(body ? { body: JSON.stringify(body) } : {}),
	};
	const resp = await fetch(`${CONTEXT7_BASE}${path}`, init);
	const text = await resp.text();
	if (!resp.ok) {
		throw new Error(`Context7 ${method} ${path} → HTTP ${resp.status}: ${text}`);
	}
	try {
		return JSON.parse(text);
	} catch {
		return text;
	}
}

server.registerTool(
	"aphrody_context7_resolve_library",
	{
		description:
			"Resolve a library name (e.g. 'next.js', 'wgpu', 'react') to a stable Context7 library ID usable in aphrody_context7_get_docs.",
		inputSchema: { library_name: z.string() },
		annotations: { readOnlyHint: true },
	},
	async ({ library_name }) => {
		const r = await context7Fetch("POST", "/mcp", {
			jsonrpc: "2.0",
			id: 1,
			method: "tools/call",
			params: {
				name: "resolve-library-id",
				arguments: { libraryName: library_name },
			},
		});
		return asTextContent(r);
	},
);

server.registerTool(
	"aphrody_context7_get_docs",
	{
		description:
			"Fetch up-to-date documentation for a Context7-resolved library. `topic` (optional) narrows the response (e.g. 'app-router', 'fetch-api').",
		inputSchema: {
			library_id: z
				.string()
				.describe("Context7 library ID (resolved via aphrody_context7_resolve_library)"),
			topic: z.string().optional(),
			tokens: z.number().int().min(1000).max(50000).default(5000),
		},
		annotations: { readOnlyHint: true },
	},
	async ({ library_id, topic, tokens }) => {
		const r = await context7Fetch("POST", "/mcp", {
			jsonrpc: "2.0",
			id: 1,
			method: "tools/call",
			params: {
				name: "get-library-docs",
				arguments: {
					context7CompatibleLibraryID: library_id,
					...(topic ? { topic } : {}),
					tokens,
				},
			},
		});
		return asTextContent(r);
	},
);

// ---------------------------------------------------------------------------
// `--list-tools` CLI mode (matches bxc-mcp UX)
// ---------------------------------------------------------------------------

if (process.argv.includes("--list-tools")) {
	const internal = (server as unknown as {
		_registeredTools?: Record<
			string,
			{ description: string; inputSchema: unknown }
		>;
	})._registeredTools;
	const catalog = internal
		? Object.entries(internal).map(([name, t]) => ({
				name,
				description: t.description,
			}))
		: [];
	process.stdout.write(`${JSON.stringify(catalog, null, 2)}\n`);
	process.exit(0);
}

// ---------------------------------------------------------------------------
// Stdio transport startup
// ---------------------------------------------------------------------------

const transport = new StdioServerTransport();
await server.connect(transport);

// Graceful shutdown of the bxc-mcp child on parent SIGINT/SIGTERM.
for (const sig of ["SIGINT", "SIGTERM"] as const) {
	process.on(sig, () => {
		if (bxcMcp) {
			try {
				bxcMcp.proc.kill();
			} catch {
				/* ignore */
			}
		}
		process.exit(0);
	});
}
