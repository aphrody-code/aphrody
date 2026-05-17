#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
/**
 * bxc-mass-scrape.ts
 * ==================
 *
 * Native Windows-first mass-scrape orchestrator built on the local bxc clone
 * (aphrody-code/bxc@aphrody at C:/worktree/bxc). Walks a JSON URL list, runs
 * N concurrent bxc pages, writes one HTML file per URL into the cache dir,
 * and emits a manifest with timing + status per URL.
 *
 * No mocks. Aborts loudly if bxc is missing or > FAIL_THRESHOLD of pages fail.
 *
 * Usage:
 *   bun run scripts/bxc-mass-scrape.ts                                          # defaults
 *   bun run scripts/bxc-mass-scrape.ts --urls=scripts/bxc-mass-scrape.urls.json
 *   bun run scripts/bxc-mass-scrape.ts --concurrency=8 --profile=stealth
 *   bun run scripts/bxc-mass-scrape.ts --cache=var/data/bxc-cache --retry=2
 *   bun run scripts/bxc-mass-scrape.ts --mode=full --timeout=90000
 *
 * Env / fallback:
 *   BXC_ROOT     absolute path to a cloned bxc repo (default: C:/worktree/bxc)
 *
 * Output:
 *   <cache>/<sha256-of-url>.html       per-URL DOM snapshot
 *   <cache>/manifest.json              aggregated run report (see ManifestSchema)
 */

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_BXC_ROOT = process.env.BXC_ROOT ?? "C:/worktree/bxc";
const DEFAULT_URLS = join(REPO_ROOT, "scripts", "bxc-mass-scrape.urls.json");
const DEFAULT_CACHE = join(REPO_ROOT, "var", "data", "bxc-cache");
const FAIL_THRESHOLD = 0.5; // abort run if > 50 % of URLs fail

type Profile = "static" | "http" | "fast" | "stealth" | "max";
type Mode = "static" | "full";

interface Args {
	bxcRoot: string;
	urlsFile: string;
	cacheDir: string;
	concurrency: number;
	profile: Profile;
	mode: Mode;
	timeoutMs: number;
	retry: number;
	force: boolean;
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = {
		bxcRoot: DEFAULT_BXC_ROOT,
		urlsFile: DEFAULT_URLS,
		cacheDir: DEFAULT_CACHE,
		concurrency: 6,
		// Default profile = "static" — in-process StaticDomTransport, no binary
		// dependency.  Use --profile=http for curl-impersonate (fastest, no DOM)
		// or --profile=fast/stealth/max for full Chrome/bxc-engine.
		profile: "static",
		mode: "static",
		timeoutMs: 60_000,
		retry: 2,
		force: false,
	};
	for (const raw of argv) {
		if (raw.startsWith("--bxc=")) out.bxcRoot = raw.slice("--bxc=".length);
		else if (raw.startsWith("--urls=")) out.urlsFile = raw.slice("--urls=".length);
		else if (raw.startsWith("--cache=")) out.cacheDir = raw.slice("--cache=".length);
		else if (raw.startsWith("--concurrency=")) {
			const n = Number(raw.slice("--concurrency=".length));
			if (!Number.isInteger(n) || n < 1 || n > 32) {
				throw new Error(`invalid --concurrency=${raw} (1..32)`);
			}
			out.concurrency = n;
		} else if (raw.startsWith("--profile=")) {
			const v = raw.slice("--profile=".length);
			if (
				v !== "static" &&
				v !== "http" &&
				v !== "fast" &&
				v !== "stealth" &&
				v !== "max"
			) {
				throw new Error(`invalid --profile=${v} (static|http|fast|stealth|max)`);
			}
			out.profile = v;
		} else if (raw.startsWith("--mode=")) {
			const v = raw.slice("--mode=".length);
			if (v !== "static" && v !== "full") {
				throw new Error(`invalid --mode=${v} (static|full)`);
			}
			out.mode = v;
		} else if (raw.startsWith("--timeout=")) {
			const n = Number(raw.slice("--timeout=".length));
			if (!Number.isFinite(n) || n <= 0) throw new Error(`invalid --timeout=${raw}`);
			out.timeoutMs = n;
		} else if (raw.startsWith("--retry=")) {
			const n = Number(raw.slice("--retry=".length));
			if (!Number.isInteger(n) || n < 0 || n > 8) {
				throw new Error(`invalid --retry=${raw} (0..8)`);
			}
			out.retry = n;
		} else if (raw === "--force") {
			out.force = true;
		}
	}
	return out;
}

// ---------------------------------------------------------------------------
// URL list schema
// ---------------------------------------------------------------------------

interface UrlEntry {
	url: string;
	category: string;
	tags?: readonly string[];
}

function loadUrls(file: string): UrlEntry[] {
	if (!existsSync(file)) {
		throw new Error(
			`URL list not found: ${file}. Run scripts/bxc-mass-scrape.ps1 first ` +
				"or pass --urls=<path-to-json>.",
		);
	}
	const raw = readFileSync(file, "utf8");
	const parsed = JSON.parse(raw) as { urls: UrlEntry[] };
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

// ---------------------------------------------------------------------------
// bxc loader — same pattern as scripts/scrape-m3-tokens.ts
// ---------------------------------------------------------------------------

interface BxcModule {
	Browser: {
		newPage(opts: Record<string, unknown>): Promise<BxcPage>;
		shutdown?(): Promise<void>;
	};
}

interface BxcPage {
	goto(url: string, opts?: { timeoutMs?: number; waitUntil?: string }): Promise<unknown>;
	content?(): Promise<string>;
	evaluate<T>(fn: (...args: never[]) => T, ...args: unknown[]): Promise<T>;
	close(): Promise<void>;
}

async function loadBxc(bxcRoot: string): Promise<BxcModule> {
	const browserPath = join(bxcRoot, "src", "api", "browser.ts");
	if (!existsSync(browserPath)) {
		throw new Error(
			`bxc not found at ${browserPath}. Clone via:\n` +
				`  gh repo clone aphrody-code/bxc -- --branch aphrody ${bxcRoot}\n` +
				"Or set BXC_ROOT / pass --bxc=<path>.",
		);
	}
	const url = pathToFileURL(browserPath).href;
	const mod = (await import(url)) as Partial<BxcModule>;
	if (!mod.Browser || typeof mod.Browser.newPage !== "function") {
		throw new Error(
			`bxc module at ${browserPath} missing Browser.newPage(). ` +
				`Exports: ${Object.keys(mod).join(", ") || "<none>"}.`,
		);
	}
	return mod as BxcModule;
}

// ---------------------------------------------------------------------------
// Per-URL scrape with retry + cache check
// ---------------------------------------------------------------------------

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

function sha256Hex(s: string): string {
	return createHash("sha256").update(s).digest("hex");
}

async function fetchPage(
	browser: BxcModule["Browser"],
	url: string,
	args: Args,
): Promise<string> {
	const page = await browser.newPage({
		profile: args.profile,
		mode: args.mode,
		viewport: { width: 1440, height: 900 },
	});
	try {
		await page.goto(url, { timeoutMs: args.timeoutMs, waitUntil: "load" });
		if (typeof page.content === "function") {
			return await page.content();
		}
		// Fallback: evaluate documentElement.outerHTML.
		return await page.evaluate(() => document.documentElement.outerHTML);
	} finally {
		await page.close().catch((err: unknown) => {
			console.warn(`[bxc] close() failed for ${url}:`, err);
		});
	}
}

async function scrapeOne(
	browser: BxcModule["Browser"],
	entry: UrlEntry,
	args: Args,
): Promise<ScrapeResult> {
	const sha = sha256Hex(entry.url);
	const outputPath = join(args.cacheDir, `${sha}.html`);
	const baseResult = {
		url: entry.url,
		category: entry.category,
		sha256: sha,
		outputPath,
	};

	if (!args.force && existsSync(outputPath) && statSync(outputPath).size > 0) {
		const sz = statSync(outputPath).size;
		return { ...baseResult, status: "cached", bytes: sz, ms: 0, attempt: 0, error: null };
	}

	let lastErr: unknown = null;
	for (let attempt = 1; attempt <= args.retry + 1; attempt++) {
		const t0 = performance.now();
		try {
			const html = await fetchPage(browser, entry.url, args);
			if (typeof html !== "string" || html.length === 0) {
				throw new Error("empty body");
			}
			await writeFile(outputPath, html, "utf8");
			const ms = Math.round(performance.now() - t0);
			console.log(`[bxc] ok   ${entry.url}  ${html.length}B  ${ms}ms  attempt=${attempt}`);
			return {
				...baseResult,
				status: "ok",
				bytes: html.length,
				ms,
				attempt,
				error: null,
			};
		} catch (err) {
			lastErr = err;
			const ms = Math.round(performance.now() - t0);
			console.warn(
				`[bxc] fail ${entry.url}  attempt=${attempt}/${args.retry + 1}  ${ms}ms  ${String(err).slice(0, 200)}`,
			);
			// Exponential backoff: 250ms, 500ms, 1000ms, ...
			if (attempt <= args.retry) {
				await new Promise((r) => setTimeout(r, 250 * 2 ** (attempt - 1)));
			}
		}
	}
	return {
		...baseResult,
		status: "failed",
		bytes: 0,
		ms: 0,
		attempt: args.retry + 1,
		error: String(lastErr).slice(0, 500),
	};
}

// ---------------------------------------------------------------------------
// Concurrency pool (no external deps — chunked Promise.all)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));

	if (!existsSync(args.bxcRoot) || !statSync(args.bxcRoot).isDirectory()) {
		throw new Error(
			`bxc root does not exist: ${args.bxcRoot}\n` +
				"Set BXC_ROOT or pass --bxc=<path>. Clone via:\n" +
				`  gh repo clone aphrody-code/bxc -- --branch aphrody ${args.bxcRoot}`,
		);
	}

	mkdirSync(args.cacheDir, { recursive: true });
	mkdirSync(dirname(join(args.cacheDir, "manifest.json")), { recursive: true });

	const urls = loadUrls(args.urlsFile);
	console.log(
		`[bxc-mass-scrape] urls=${urls.length}  concurrency=${args.concurrency}  ` +
			`profile=${args.profile}  mode=${args.mode}  cache=${args.cacheDir}`,
	);

	const { Browser } = await loadBxc(args.bxcRoot);

	const t0 = performance.now();
	const results = await runPool(urls, args.concurrency, (entry) =>
		scrapeOne(Browser, entry, args),
	);
	const totalMs = Math.round(performance.now() - t0);

	const ok = results.filter((r) => r.status === "ok").length;
	const cached = results.filter((r) => r.status === "cached").length;
	const failed = results.filter((r) => r.status === "failed").length;
	const totalBytes = results.reduce((acc, r) => acc + r.bytes, 0);

	const manifest = {
		$schema: "https://aphrody-code.dev/schemas/bxc-mass-scrape/v1",
		generatedAt: new Date().toISOString(),
		args: {
			bxcRoot: args.bxcRoot,
			urlsFile: args.urlsFile,
			cacheDir: args.cacheDir,
			concurrency: args.concurrency,
			profile: args.profile,
			mode: args.mode,
			timeoutMs: args.timeoutMs,
			retry: args.retry,
			force: args.force,
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
		`[bxc-mass-scrape] done  ok=${ok}  cached=${cached}  failed=${failed}  ` +
			`total=${urls.length}  ${(totalBytes / 1024).toFixed(1)}KB  ${totalMs}ms`,
	);
	console.log(`[bxc-mass-scrape] manifest: ${manifestPath}`);

	if (Browser.shutdown) {
		await Browser.shutdown().catch(() => undefined);
	}

	if (failed / urls.length > FAIL_THRESHOLD) {
		throw new Error(
			`failure rate ${(100 * (failed / urls.length)).toFixed(1)}% exceeds threshold ${(100 * FAIL_THRESHOLD).toFixed(0)}%`,
		);
	}
}

main().catch((err: unknown) => {
	console.error("bxc-mass-scrape FAILED:", err);
	process.exitCode = 1;
});
