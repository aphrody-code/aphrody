// SPDX-License-Identifier: Apache-2.0
// #!/usr/bin/env bun
/**
 * edge-mass-scrape.ts
 * ===================
 *
 * Windows-native JS-rendering mass-scrape via Microsoft Edge headless mode.
 * Companion to scripts/bxc-mass-scrape.ts — used when the target requires
 * client-side JS hydration (SPAs like design.google, material.angular.dev,
 * shadcn-ui's interactive examples) and bxc's `profile=fast|stealth|max`
 * are unavailable on Windows (Lightpanda is Linux/macOS only).
 *
 * Implementation: spawns `msedge --headless=new --dump-dom <url>` per URL.
 * Edge runs the full Chromium pipeline (HTML parse, JS exec, layout) and
 * prints the post-hydration DOM serialization to stdout — exactly what we
 * need for SPA scrape.
 *
 * Same URL list format + same manifest schema as bxc-mass-scrape, so callers
 * can A/B compare static vs JS-rendered byte sizes per URL.
 *
 * Usage:
 *   bun run scripts/edge-mass-scrape.ts --urls=scripts/bxc-mass-scrape.angular-material.urls.json
 *   bun run scripts/edge-mass-scrape.ts --concurrency=4 --timeout=45000
 *   bun run scripts/edge-mass-scrape.ts --edge="/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
 *
 * Output: <cache>/<sha256>.html + manifest.json (schema v1, "engine":"edge").
 */

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_EDGE = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const DEFAULT_URLS = join(REPO_ROOT, "scripts", "bxc-mass-scrape.urls.json");
const DEFAULT_CACHE = join(REPO_ROOT, "var", "data", "edge-cache");
const FAIL_THRESHOLD = 0.5;

interface Args {
	edgePath: string;
	urlsFile: string;
	cacheDir: string;
	concurrency: number;
	timeoutMs: number;
	retry: number;
	force: boolean;
	virtualTimeBudgetMs: number;
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = {
		edgePath: DEFAULT_EDGE,
		urlsFile: DEFAULT_URLS,
		cacheDir: DEFAULT_CACHE,
		concurrency: 4,
		timeoutMs: 45_000,
		retry: 1,
		force: false,
		virtualTimeBudgetMs: 8_000,
	};
	for (const raw of argv) {
		if (raw.startsWith("--edge=")) out.edgePath = raw.slice("--edge=".length);
		else if (raw.startsWith("--urls=")) out.urlsFile = raw.slice("--urls=".length);
		else if (raw.startsWith("--cache=")) out.cacheDir = raw.slice("--cache=".length);
		else if (raw.startsWith("--concurrency=")) {
			const n = Number(raw.slice("--concurrency=".length));
			if (!Number.isInteger(n) || n < 1 || n > 16) {
				throw new Error(`invalid --concurrency=${raw} (1..16)`);
			}
			out.concurrency = n;
		} else if (raw.startsWith("--timeout=")) {
			const n = Number(raw.slice("--timeout=".length));
			if (!Number.isFinite(n) || n <= 0) throw new Error(`invalid --timeout=${raw}`);
			out.timeoutMs = n;
		} else if (raw.startsWith("--retry=")) {
			const n = Number(raw.slice("--retry=".length));
			if (!Number.isInteger(n) || n < 0 || n > 4) {
				throw new Error(`invalid --retry=${raw} (0..4)`);
			}
			out.retry = n;
		} else if (raw.startsWith("--virtual-time=")) {
			const n = Number(raw.slice("--virtual-time=".length));
			if (!Number.isFinite(n) || n < 0) throw new Error(`invalid --virtual-time=${raw}`);
			out.virtualTimeBudgetMs = n;
		} else if (raw === "--force") {
			out.force = true;
		}
	}
	return out;
}

interface UrlEntry {
	url: string;
	category: string;
	tags?: readonly string[];
}

function loadUrls(file: string): UrlEntry[] {
	if (!existsSync(file)) {
		throw new Error(`URL list not found: ${file}`);
	}
	const parsed = JSON.parse(readFileSync(file, "utf8")) as { urls: UrlEntry[] };
	if (!Array.isArray(parsed.urls) || parsed.urls.length === 0) {
		throw new Error(`URL list at ${file} must contain { "urls": [...] } with >=1 entries.`);
	}
	for (const e of parsed.urls) {
		if (typeof e.url !== "string" || !/^https?:\/\//.test(e.url)) {
			throw new Error(`invalid URL entry: ${JSON.stringify(e)}`);
		}
		if (typeof e.category !== "string" || e.category.length === 0) {
			throw new Error(`URL entry missing category: ${e.url}`);
		}
	}
	return parsed.urls;
}

function sha256Hex(s: string): string {
	return createHash("sha256").update(s).digest("hex");
}

interface ScrapeResult {
	url: string;
	category: string;
	sha256: string;
	status: "ok" | "cached" | "failed";
	bytes: number;
	ms: number;
	attempt: number;
	error: string | null;
	outputPath: string;
}

/**
 * Spawn Edge headless and capture --dump-dom output for one URL.
 *
 * Edge flags:
 *   --headless=new                : new headless mode (post-Chromium 109)
 *   --disable-gpu                 : avoid GPU init in headless context
 *   --no-sandbox                  : required in some CI/container contexts
 *   --disable-dev-shm-usage       : avoid /dev/shm issues
 *   --disable-extensions          : clean profile, no addons
 *   --hide-scrollbars             : cleaner DOM output
 *   --disable-translate           : skip translate prompt
 *   --no-first-run                : skip first-run dialog
 *   --no-default-browser-check    : skip default-browser prompt
 *   --user-data-dir=<tmp>         : isolated profile per spawn
 *   --virtual-time-budget=N       : run timers up to N ms then freeze (deterministic SPA capture)
 *   --dump-dom <url>              : print DOM serialization to stdout
 */
async function fetchViaEdge(
	edgePath: string,
	url: string,
	timeoutMs: number,
	virtualTimeBudgetMs: number,
): Promise<string> {
	const userDataDir = `${Bun.env.TEMP ?? "/tmp"}/edge-mass-scrape-${process.pid}-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
	const args = [
		"--headless=new",
		"--disable-gpu",
		"--no-sandbox",
		"--disable-dev-shm-usage",
		"--disable-extensions",
		"--hide-scrollbars",
		"--disable-translate",
		"--no-first-run",
		"--no-default-browser-check",
		`--user-data-dir=${userDataDir}`,
	];
	if (virtualTimeBudgetMs > 0) {
		args.push(`--virtual-time-budget=${virtualTimeBudgetMs}`);
	}
	args.push("--dump-dom", url);

	const proc = Bun.spawn([edgePath, ...args], {
		stdout: "pipe",
		stderr: "pipe",
	});

	const killer = setTimeout(() => {
		try {
			proc.kill("SIGKILL");
		} catch {
			// noop
		}
	}, timeoutMs);

	try {
		const [stdout, exitCode] = await Promise.all([
			new Response(proc.stdout).text(),
			proc.exited,
		]);
		if (exitCode !== 0) {
			const stderr = await new Response(proc.stderr).text();
			throw new Error(`edge exit ${exitCode}: ${stderr.slice(0, 300)}`);
		}
		if (stdout.length === 0) {
			throw new Error("edge returned empty DOM");
		}
		return stdout;
	} finally {
		clearTimeout(killer);
	}
}

async function scrapeOne(
	edgePath: string,
	entry: UrlEntry,
	cacheDir: string,
	args: Args,
): Promise<ScrapeResult> {
	const sha = sha256Hex(entry.url);
	const outputPath = join(cacheDir, `${sha}.html`);
	const base = { url: entry.url, category: entry.category, sha256: sha, outputPath };

	if (!args.force && existsSync(outputPath) && statSync(outputPath).size > 0) {
		const sz = statSync(outputPath).size;
		return { ...base, status: "cached", bytes: sz, ms: 0, attempt: 0, error: null };
	}

	let lastErr: unknown = null;
	for (let attempt = 1; attempt <= args.retry + 1; attempt++) {
		const t0 = performance.now();
		try {
			const html = await fetchViaEdge(edgePath, entry.url, args.timeoutMs, args.virtualTimeBudgetMs);
			await writeFile(outputPath, html, "utf8");
			const ms = Math.round(performance.now() - t0);
			console.log(`[edge] ok   ${entry.url}  ${html.length}B  ${ms}ms  attempt=${attempt}`);
			return { ...base, status: "ok", bytes: html.length, ms, attempt, error: null };
		} catch (err) {
			lastErr = err;
			const ms = Math.round(performance.now() - t0);
			console.warn(
				`[edge] fail ${entry.url}  attempt=${attempt}/${args.retry + 1}  ${ms}ms  ${String(err).slice(0, 200)}`,
			);
			if (attempt <= args.retry) {
				await new Promise((r) => setTimeout(r, 500 * 2 ** (attempt - 1)));
			}
		}
	}
	return {
		...base,
		status: "failed",
		bytes: 0,
		ms: 0,
		attempt: args.retry + 1,
		error: String(lastErr).slice(0, 500),
	};
}

async function runPool<T, R>(
	items: readonly T[],
	concurrency: number,
	worker: (item: T, idx: number) => Promise<R>,
): Promise<R[]> {
	const results: R[] = new Array(items.length);
	let cursor = 0;
	async function runOne(): Promise<void> {
		while (true) {
			const i = cursor++;
			if (i >= items.length) return;
			const item = items[i];
			if (item === undefined) return;
			results[i] = await worker(item, i);
		}
	}
	const lanes = Array.from({ length: Math.min(concurrency, items.length) }, () => runOne());
	await Promise.all(lanes);
	return results;
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));

	if (!existsSync(args.edgePath)) {
		throw new Error(
			`Edge not found at ${args.edgePath}. Pass --edge=<path-to-msedge.exe>.`,
		);
	}
	mkdirSync(args.cacheDir, { recursive: true });
	mkdirSync(dirname(join(args.cacheDir, "manifest.json")), { recursive: true });

	const urls = loadUrls(args.urlsFile);
	console.log(
		`[edge-mass-scrape] urls=${urls.length}  concurrency=${args.concurrency}  ` +
			`virtualTime=${args.virtualTimeBudgetMs}ms  cache=${args.cacheDir}`,
	);

	const t0 = performance.now();
	const results = await runPool(urls, args.concurrency, (entry) =>
		scrapeOne(args.edgePath, entry, args.cacheDir, args),
	);
	const totalMs = Math.round(performance.now() - t0);

	const ok = results.filter((r) => r.status === "ok").length;
	const cached = results.filter((r) => r.status === "cached").length;
	const failed = results.filter((r) => r.status === "failed").length;
	const totalBytes = results.reduce((acc, r) => acc + r.bytes, 0);

	const manifest = {
		$schema: "https://aphrody-code.dev/schemas/bxc-mass-scrape/v1",
		engine: "edge",
		generatedAt: new Date().toISOString(),
		args: {
			edgePath: args.edgePath,
			urlsFile: args.urlsFile,
			cacheDir: args.cacheDir,
			concurrency: args.concurrency,
			timeoutMs: args.timeoutMs,
			retry: args.retry,
			force: args.force,
			virtualTimeBudgetMs: args.virtualTimeBudgetMs,
		},
		stats: {
			total: urls.length,
			ok,
			cached,
			failed,
			totalBytes,
			totalMs,
			failureRate: failed / urls.length,
		},
		results,
	};
	const manifestPath = join(args.cacheDir, "manifest.json");
	await writeFile(manifestPath, `${JSON.stringify(manifest, null, "\t")}\n`, "utf8");

	console.log(
		`[edge-mass-scrape] done  ok=${ok}  cached=${cached}  failed=${failed}  ` +
			`total=${urls.length}  ${(totalBytes / 1024).toFixed(1)}KB  ${totalMs}ms`,
	);
	console.log(`[edge-mass-scrape] manifest: ${manifestPath}`);

	if (failed / urls.length > FAIL_THRESHOLD) {
		throw new Error(
			`failure rate ${(100 * (failed / urls.length)).toFixed(1)}% exceeds threshold ${(100 * FAIL_THRESHOLD).toFixed(0)}%`,
		);
	}
}

main().catch((err: unknown) => {
	console.error("edge-mass-scrape FAILED:", err);
	process.exitCode = 1;
});
