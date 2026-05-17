#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
/**
 * design-google-ingest.ts
 * =======================
 *
 * URL-discovery orchestrator for the `design-google-ingest` skill. Re-scrapes
 * `https://design.google/library` for the freshest article list, unions with
 * the canonical seed set (home + about + events + products + library), then
 * writes `scripts/design-google.urls.json` in the v1 schema consumed by
 * `scripts/edge-mass-scrape.ts`.
 *
 * Single-purpose: discovery only. The skill then invokes the Edge headless
 * scraper itself, then dispatches the design-google-curator agent.
 *
 * Usage:
 *   bun run scripts/design-google-ingest.ts                       # default
 *   bun run scripts/design-google-ingest.ts --force-refresh       # bypass cache discovery
 *   bun run scripts/design-google-ingest.ts --cache=var/data/edge-cache
 *
 * Output:
 *   scripts/design-google.urls.json (v1)
 */

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_EDGE = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const DEFAULT_CACHE = join(REPO_ROOT, "var", "data", "edge-cache");
const OUTPUT_JSON = join(REPO_ROOT, "scripts", "design-google.urls.json");
const LIBRARY_URL = "https://design.google/library";
const VIRTUAL_TIME_MS = 20_000;

interface Args {
	edgePath: string;
	cacheDir: string;
	forceRefresh: boolean;
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = {
		edgePath: process.env.EDGE_PATH ?? DEFAULT_EDGE,
		cacheDir: DEFAULT_CACHE,
		forceRefresh: false,
	};
	for (const raw of argv) {
		if (raw.startsWith("--edge=")) out.edgePath = raw.slice("--edge=".length);
		else if (raw.startsWith("--cache=")) out.cacheDir = raw.slice("--cache=".length);
		else if (raw === "--force-refresh") out.forceRefresh = true;
	}
	return out;
}

interface UrlEntry {
	url: string;
	category: string;
	tags: string[];
}

/**
 * Canonical site pages — always present in the URL list regardless of
 * whether the library index links to them.
 */
const SEED_SITE_PAGES: UrlEntry[] = [
	{ url: "https://design.google/", category: "site", tags: ["home"] },
	{ url: "https://design.google/about", category: "site", tags: ["about"] },
	{ url: "https://design.google/events", category: "site", tags: ["events"] },
	{ url: "https://design.google/products", category: "site", tags: ["products"] },
	{ url: "https://design.google/library", category: "site", tags: ["library-index"] },
];

/**
 * Known-good library articles. Kept as a seed even when the library page
 * fails to scrape — so the manifest never silently shrinks if the SPA
 * shell-detection fingerprint changes upstream.
 */
const SEED_LIBRARY_ARTICLES: UrlEntry[] = [
	{ url: "https://design.google/library/gemini-ai-visual-design", category: "library", tags: ["gemini"] },
	{ url: "https://design.google/library/google-sans-flex-font", category: "library", tags: ["brand", "type"] },
	{ url: "https://design.google/library/expressive-material-design-google-research", category: "library", tags: ["m3"] },
	{ url: "https://design.google/library/design-notes-material-3-expressive-liam-spradlin", category: "library", tags: ["m3"] },
	{ url: "https://design.google/library/material-design-3", category: "library", tags: ["m3"] },
	{ url: "https://design.google/library/material-3-design-tokens", category: "library", tags: ["m3", "tokens"] },
	{ url: "https://design.google/library/color-theory-ruxandra-duru", category: "library", tags: ["foundations", "color"] },
	{ url: "https://design.google/library/david-reinfurt-teaches-design", category: "library", tags: ["foundations"] },
	{ url: "https://design.google/library/hardware-and-software-can-work-together", category: "library", tags: ["foundations"] },
	{ url: "https://design.google/library/open-source-custom-fonts-google", category: "library", tags: ["brand", "type"] },
	{ url: "https://design.google/library/transparent-screens", category: "library", tags: ["brand"] },
	{ url: "https://design.google/library/ux-design-system-dance", category: "library", tags: ["foundations", "ux"] },
	{ url: "https://design.google/library/accessibility", category: "library", tags: ["foundations", "a11y"] },
	{ url: "https://design.google/library/design-systems", category: "library", tags: ["foundations"] },
];

function sha256Hex(s: string): string {
	return createHash("sha256").update(s).digest("hex");
}

/**
 * Re-fetch the library index page via Edge headless and extract every
 * `/library/<slug>` href into a fresh `UrlEntry[]`.
 */
async function fetchLibraryHrefs(args: Args): Promise<UrlEntry[]> {
	mkdirSync(args.cacheDir, { recursive: true });

	const sha = sha256Hex(LIBRARY_URL);
	const cachePath = join(args.cacheDir, `${sha}.html`);

	let html: string;
	const cacheFresh =
		!args.forceRefresh &&
		existsSync(cachePath) &&
		statSync(cachePath).size > 8_000 &&
		Date.now() - statSync(cachePath).mtimeMs < 24 * 60 * 60 * 1_000;

	if (cacheFresh) {
		console.log(`[discover] using cached library index (${statSync(cachePath).size} B)`);
		html = readFileSync(cachePath, "utf8");
	} else {
		if (!existsSync(args.edgePath)) {
			throw new Error(`Edge not found at ${args.edgePath}. Pass --edge=<path>.`);
		}
		console.log(`[discover] fetching ${LIBRARY_URL} via Edge headless...`);
		const userDataDir = `${process.env.TEMP ?? "/tmp"}/dg-discover-${process.pid}-${Date.now()}`;
		const proc = Bun.spawn(
			[
				args.edgePath,
				"--headless=new",
				"--disable-gpu",
				"--no-sandbox",
				"--disable-dev-shm-usage",
				"--disable-extensions",
				"--hide-scrollbars",
				"--no-first-run",
				"--no-default-browser-check",
				`--user-data-dir=${userDataDir}`,
				`--virtual-time-budget=${VIRTUAL_TIME_MS}`,
				"--dump-dom",
				LIBRARY_URL,
			],
			{ stdout: "pipe", stderr: "pipe" },
		);
		const killer = setTimeout(() => {
			try {
				proc.kill("SIGKILL");
			} catch {
				// noop
			}
		}, 60_000);
		try {
			const [stdout, exitCode] = await Promise.all([
				new Response(proc.stdout).text(),
				proc.exited,
			]);
			if (exitCode !== 0) {
				const stderr = await new Response(proc.stderr).text();
				throw new Error(`edge exit ${exitCode}: ${stderr.slice(0, 300)}`);
			}
			html = stdout;
			await writeFile(cachePath, html, "utf8");
			console.log(`[discover] cached ${cachePath} (${html.length} B)`);
		} finally {
			clearTimeout(killer);
		}
	}

	const hrefRe = /href="(\/library\/[a-z0-9-]+)"/gi;
	const seen = new Set<string>();
	for (const m of html.matchAll(hrefRe)) {
		const path = m[1];
		if (path !== undefined) {
			seen.add(`https://design.google${path}`);
		}
	}

	const entries: UrlEntry[] = [];
	for (const url of [...seen].sort()) {
		entries.push({ url, category: "library", tags: ["discovered"] });
	}
	console.log(`[discover] extracted ${entries.length} /library/* hrefs`);
	return entries;
}

/**
 * Union the seed set with the discovered set. The seed wins category +
 * tags on conflict (curated metadata beats the auto-discovered default).
 */
function unionEntries(...sources: UrlEntry[][]): UrlEntry[] {
	const map = new Map<string, UrlEntry>();
	for (const src of sources) {
		for (const e of src) {
			const key = e.url.toLowerCase();
			if (!map.has(key)) {
				map.set(key, e);
			}
		}
	}
	const out = [...map.values()];
	out.sort((a, b) => a.url.localeCompare(b.url));
	return out;
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));

	const discovered = await fetchLibraryHrefs(args);

	const merged = unionEntries(
		SEED_SITE_PAGES,
		SEED_LIBRARY_ARTICLES,
		discovered,
	);

	const document = {
		$schema: "https://aphrody-code.dev/schemas/bxc-mass-scrape-urls/v1",
		version: "1.0.0",
		description:
			"design.google end-to-end ingest URL list — discovered via " +
			"scripts/design-google-ingest.ts (Edge headless virtual-time=" +
			`${VIRTUAL_TIME_MS}) + seed catalogue. Consumed by the ` +
			"design-google-ingest skill via scripts/edge-mass-scrape.ts.",
		urls: merged,
	};

	await writeFile(OUTPUT_JSON, `${JSON.stringify(document, null, "\t")}\n`, "utf8");

	const byCategory = merged.reduce<Record<string, number>>((acc, e) => {
		acc[e.category] = (acc[e.category] ?? 0) + 1;
		return acc;
	}, {});

	console.log(`[ingest] wrote ${OUTPUT_JSON}`);
	console.log(`[ingest] total URLs: ${merged.length}`);
	for (const [cat, n] of Object.entries(byCategory).sort()) {
		console.log(`[ingest]   ${cat.padEnd(8)} ${n}`);
	}
}

main().catch((err: unknown) => {
	console.error("design-google-ingest FAILED:", err);
	process.exitCode = 1;
});
