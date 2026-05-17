#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
/**
 * design-google-curate.ts
 * =======================
 *
 * Reads the edge-cache produced by the design-google-ingest skill, extracts
 * structured metadata (title, description, first prose paragraph, color
 * stops, pull-quotes) per article, and assembles docs/DESIGN.md per the
 * design-google-curator agent's output schema.
 *
 * This is the deterministic fallback path. The agent variant is preferred
 * when narrative tuning matters; this script gives us a CI-friendly,
 * reproducible bare-minimum baseline.
 */

import { existsSync, readFileSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const URLS_JSON = join(REPO_ROOT, "scripts", "design-google.urls.json");
const CACHE_DIR = join(REPO_ROOT, "var", "data", "edge-cache");
const MANIFEST_JSON = join(CACHE_DIR, "manifest.json");
const COVERAGE_MD = join(REPO_ROOT, "docs", "audits", "m3-coverage.md");
const OUTPUT_MD = join(REPO_ROOT, "docs", "DESIGN-GOOGLE.md");

interface ManifestResult {
	url: string;
	category: string;
	sha256: string;
	status: "ok" | "cached" | "failed";
	bytes: number;
	ms: number;
	outputPath: string;
	error: string | null;
}

interface UrlEntry {
	url: string;
	category: string;
	tags: string[];
}

interface Article {
	url: string;
	bytes: number;
	status: ManifestResult["status"];
	title: string;
	desc: string;
	excerpt: string;
	colors: string[];
	quote: string;
	tags: string[];
	slug: string;
}

function decodeEntities(s: string): string {
	return s
		.replace(/&amp;/g, "&")
		.replace(/&lt;/g, "<")
		.replace(/&gt;/g, ">")
		.replace(/&quot;/g, '"')
		.replace(/&apos;/g, "'")
		.replace(/&#39;/g, "'")
		.replace(/&#x27;/g, "'")
		.replace(/&mdash;/g, "—")
		.replace(/&ndash;/g, "–")
		.replace(/&hellip;/g, "…")
		.replace(/&nbsp;/g, " ");
}

function stripTags(s: string): string {
	return decodeEntities(s.replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim());
}

function extract(html: string): Pick<Article, "title" | "desc" | "excerpt" | "colors" | "quote"> {
	const titleMatch = html.match(/<title>([^<]+)<\/title>/i);
	const descMatch =
		html.match(/<meta\s+name="description"\s+content="([^"]+)"/i) ||
		html.match(/<meta\s+property="og:description"\s+content="([^"]+)"/i);
	const ogTitleMatch = html.match(/<meta\s+property="og:title"\s+content="([^"]+)"/i);

	let title = "";
	if (titleMatch && titleMatch[1] !== undefined) {
		title = decodeEntities(titleMatch[1]).trim();
	}
	if ((!title || title === "Avant de continuer") && ogTitleMatch && ogTitleMatch[1] !== undefined) {
		title = decodeEntities(ogTitleMatch[1]).trim();
	}

	const desc = descMatch && descMatch[1] !== undefined ? decodeEntities(descMatch[1]).trim() : "";

	const paragraphs: string[] = [];
	for (const m of html.matchAll(/<p[^>]*>([\s\S]*?)<\/p>/gi)) {
		if (m[1] !== undefined) {
			const stripped = stripTags(m[1]);
			if (stripped.length >= 80 && stripped.length <= 600 && !stripped.startsWith("Sorry, your browser")) {
				paragraphs.push(stripped);
			}
		}
	}
	const excerpt = paragraphs[0] ? paragraphs[0].slice(0, 320) : "";

	const seen = new Set<string>();
	const colors: string[] = [];
	for (const m of html.matchAll(/#([0-9A-Fa-f]{6})\b/g)) {
		if (m[1] !== undefined) {
			const lower = m[1].toLowerCase();
			if (!seen.has(lower)) {
				seen.add(lower);
				colors.push("#" + m[1].toUpperCase());
				if (colors.length >= 12) break;
			}
		}
	}

	// Pull-quote: first short blockquote-ish piece.
	let quote = "";
	const bq = html.match(/<blockquote[^>]*>([\s\S]*?)<\/blockquote>/i);
	if (bq && bq[1] !== undefined) {
		const stripped = stripTags(bq[1]);
		if (stripped.length > 0 && stripped.length <= 240) quote = stripped;
	}
	if (!quote) {
		// Heuristic: a <p> that contains quote marks and an em-dash attribution.
		for (const p of paragraphs) {
			if ((p.includes("“") || p.includes('"')) && (p.includes("—") || p.includes("--"))) {
				if (p.length <= 240) {
					quote = p;
					break;
				}
			}
		}
	}

	return { title, desc, excerpt, colors, quote };
}

function slugOf(url: string): string {
	const path = url.replace(/^https?:\/\/design\.google/i, "").replace(/\/$/, "") || "/";
	return path === "/" ? "home" : path.replace(/^\/library\//, "").replace(/^\//, "");
}

function bucketOf(url: string, tags: readonly string[]): string {
	const slug = slugOf(url).toLowerCase();
	if (["home", "about", "events", "products", "library"].includes(slug)) return "site";
	if (slug === "gemini-ai-visual-design" || slug.includes("gemini")) return "gemini";
	if (
		slug.includes("google-sans") ||
		slug.includes("custom-fonts") ||
		slug.includes("transparent-screens") ||
		slug.includes("font")
	) {
		return "brand";
	}
	if (
		slug.includes("material-3") ||
		slug.includes("material-design") ||
		slug.includes("design-tokens") ||
		slug.includes("expressive-material")
	) {
		return "m3";
	}
	if (
		slug.includes("color-theory") ||
		slug.includes("david-reinfurt") ||
		slug.includes("ux-design-system") ||
		slug.includes("accessibility") ||
		slug.includes("design-systems") ||
		(tags as string[]).includes("foundations")
	) {
		return "foundations";
	}
	return "other";
}

function isSpaShell(bytes: number): boolean {
	// design.google's empty SPA shell is exactly 28028 B (verified in
	// docs/audits/2026-05-17-design-google-scrape.md). bxc-static returns
	// this; Edge headless should always exceed it post-hydration.
	return bytes > 0 && bytes <= 35_000;
}

function readCoverageSnapshot(): string {
	if (!existsSync(COVERAGE_MD)) return "_coverage report not yet generated_";
	const text = readFileSync(COVERAGE_MD, "utf8");
	const m = text.match(/token=([\d.]+)\s*%\s*bridge=([\d.]+)\s*%\s*catalogue=([\d.]+)\s*%\s*html=([\d.]+)\s*%\s*overall=([\d.]+)\s*%/i);
	if (!m) return "_coverage table marker not found_";
	const [, token, bridge, catalogue, html, overall] = m;
	return [
		`| Metric | Value | Source |`,
		`|---|---|---|`,
		`| M3 token coverage  | ${token} % | \`docs/audits/m3-coverage.md\` |`,
		`| M3 bridge coverage | ${bridge} % | \`docs/audits/m3-coverage.md\` |`,
		`| M3 catalogue cover | ${catalogue} % | \`docs/audits/m3-coverage.md\` |`,
		`| Pixel-perfect HTML | ${html} % | \`docs/audits/m3-coverage.md\` |`,
		`| **Overall**        | ${overall} % | \`docs/audits/m3-coverage.md\` |`,
	].join("\n");
}

function articleSection(a: Article): string {
	const lines: string[] = [];
	lines.push(`### ${a.title || a.slug}`);
	lines.push("");
	lines.push(`- **Source:** <${a.url}>`);
	lines.push(`- **Bytes captured:** ${a.bytes.toLocaleString("en-US")}`);
	if (a.desc) lines.push(`- **Description:** ${a.desc}`);
	if (a.excerpt) lines.push(`- **Excerpt:** ${a.excerpt}`);
	if (a.colors.length > 0) {
		lines.push(`- **Color stops:** ${a.colors.map((c) => "`" + c + "`").join(", ")}`);
	}
	if (a.quote) lines.push(`- **Pull quote:** > ${a.quote}`);
	lines.push("");
	return lines.join("\n");
}

async function main(): Promise<void> {
	if (!existsSync(URLS_JSON)) throw new Error(`URL list not found: ${URLS_JSON}`);
	if (!existsSync(MANIFEST_JSON)) throw new Error(`manifest not found: ${MANIFEST_JSON}`);

	const urls: UrlEntry[] = (JSON.parse(readFileSync(URLS_JSON, "utf8")) as { urls: UrlEntry[] }).urls;
	const manifest = JSON.parse(readFileSync(MANIFEST_JSON, "utf8")) as { results: ManifestResult[] };
	const resultsByUrl = new Map(manifest.results.map((r) => [r.url, r]));

	const articles: Article[] = [];
	const failures: Article[] = [];

	for (const entry of urls) {
		const result = resultsByUrl.get(entry.url);
		if (!result) {
			failures.push({
				url: entry.url,
				bytes: 0,
				status: "failed",
				title: "",
				desc: "",
				excerpt: "",
				colors: [],
				quote: "",
				tags: entry.tags,
				slug: slugOf(entry.url),
			});
			continue;
		}
		if (result.status === "failed" || !existsSync(result.outputPath)) {
			failures.push({
				url: entry.url,
				bytes: result.bytes,
				status: "failed",
				title: "",
				desc: result.error ?? "",
				excerpt: "",
				colors: [],
				quote: "",
				tags: entry.tags,
				slug: slugOf(entry.url),
			});
			continue;
		}
		const html = readFileSync(result.outputPath, "utf8");
		const extracted = extract(html);
		const article: Article = {
			url: entry.url,
			bytes: result.bytes || html.length,
			status: result.status,
			...extracted,
			tags: entry.tags,
			slug: slugOf(entry.url),
		};
		if (isSpaShell(article.bytes)) {
			failures.push(article);
		} else {
			articles.push(article);
		}
	}

	const buckets: Record<string, Article[]> = {
		foundations: [],
		m3: [],
		gemini: [],
		brand: [],
		other: [],
		site: [],
	};
	for (const a of articles) {
		const b = bucketOf(a.url, a.tags);
		buckets[b]?.push(a);
	}
	for (const arr of Object.values(buckets)) {
		arr.sort((x, y) => x.title.localeCompare(y.title));
	}

	const ts = new Date().toISOString();
	const okCount = articles.length;
	const failCount = failures.length;

	const lines: string[] = [];
	lines.push("<!-- SPDX-License-Identifier: Apache-2.0 -->");
	lines.push("<!-- GENERATED by .claude/skills/design-google-ingest + scripts/design-google-curate.ts");
	lines.push("     DO NOT EDIT BY HAND — re-run /design-google-ingest. -->");
	lines.push("");
	lines.push("# Aphrody Design Reference");
	lines.push("");
	lines.push(`Last refreshed: \`${ts}\``);
	lines.push("Source: design.google + `scripts/edge-mass-scrape.ts` (Edge headless, virtual-time=15000)");
	lines.push(`Articles ingested: **${okCount} / ${urls.length}**`);
	if (failCount > 0) lines.push(`SPA-shell / failed: ${failCount} (see §8)`);
	lines.push("");
	lines.push("## 1. Quick-reference index");
	lines.push("");
	lines.push("| Section | Title | URL | Bytes |");
	lines.push("|---|---|---|---|");
	for (const [bucket, arr] of Object.entries(buckets)) {
		for (const a of arr) {
			lines.push(
				`| ${bucket} | ${a.title || a.slug} | <${a.url}> | ${a.bytes.toLocaleString("en-US")} |`,
			);
		}
	}
	lines.push("");

	const sectionOrder: Array<[string, string]> = [
		["foundations", "## 2. Foundations"],
		["m3", "## 3. Material 3"],
		["gemini", "## 4. Gemini visual identity"],
		["brand", "## 5. Brand assets"],
		["other", "## 6. Other library articles"],
		["site", "## 7. Site pages"],
	];
	for (const [key, heading] of sectionOrder) {
		lines.push(heading);
		lines.push("");
		const arr = buckets[key] ?? [];
		if (arr.length === 0) {
			lines.push("_No entries in this bucket._");
			lines.push("");
			continue;
		}
		for (const a of arr) lines.push(articleSection(a));
	}

	lines.push("## 8. Raw failures (SPA shell or scrape error)");
	lines.push("");
	if (failures.length === 0) {
		lines.push("_None — every URL returned a fully-hydrated DOM._");
		lines.push("");
	} else {
		for (const f of failures) {
			lines.push(`- <${f.url}> — captured ${f.bytes.toLocaleString("en-US")} B (${f.status}). ` +
				(f.desc ? `Reason: ${f.desc}. ` : "") +
				"Re-run with `--force --virtual-time=30000` or escalate to `--profile=max` once `bxc-engine` lands on Windows.");
		}
		lines.push("");
	}

	lines.push("## 9. Audit cross-reference");
	lines.push("");
	lines.push(readCoverageSnapshot());
	lines.push("");
	lines.push("Aphrody crate cross-references in lock-step with design.google intel:");
	lines.push("");
	lines.push("- `crates/m3-tokens/src/gemini_brand.rs` ← `gemini-ai-visual-design`");
	lines.push("- `crates/m3-tokens/src/google_sans_flex.rs` ← `google-sans-flex-font`");
	lines.push("- `crates/m3-tokens/src/color.rs` ← `material-3-design-tokens`");
	lines.push("- `crates/m3-tokens/src/{shape,state,tonal,motion,elevation,typography}.rs` ← M3 spec");
	lines.push("- `crates/shadcn-bridge/src/gemini.rs` ← `gemini-ai-visual-design` composables");
	lines.push("- `crates/aphrody-wasm/examples/gemini-clone-pixel-perfect.html` ← full Gemini clone");
	lines.push("- `crates/aphrody-wasm/examples/m3-shadcn-pixel-perfect-v2.html` ← 30-component M3 demo");
	lines.push("");
	lines.push("## 10. Open follow-ups");
	lines.push("");
	lines.push("- Re-run with `--virtual-time=30000` after Edge ships a longer hydration budget so all SPA shell hits in §8 resolve.");
	lines.push("- Port full CAM16 HCT pipeline so the dynamic palette in `crates/m3-tokens/src/dynamic.rs` matches Material Color Utilities round-trip within <1 sRGB unit (currently 5 round-trip tests `#[ignore]`).");
	lines.push("- Re-scrape weekly via `/loop 7d /design-google-ingest` once a CI runner with Edge is provisioned.");
	lines.push("");
	lines.push("---");
	lines.push("");
	lines.push(
		"_Skill: `.claude/skills/design-google-ingest/SKILL.md` · " +
		"Agent: `.claude/agents/design-google-curator.md` · " +
		"Generator: `scripts/design-google-curate.ts`_",
	);
	lines.push("");

	await writeFile(OUTPUT_MD, lines.join("\n"), "utf8");
	console.log(`design-google-curate: wrote ${OUTPUT_MD}`);
	console.log(
		`  ok=${okCount}  failed=${failCount}  bytes=${articles.reduce((acc, a) => acc + a.bytes, 0).toLocaleString("en-US")}`,
	);
	for (const [bucket, arr] of Object.entries(buckets)) {
		console.log(`  ${bucket.padEnd(12)} ${arr.length}`);
	}
}

main().catch((err: unknown) => {
	console.error("design-google-curate FAILED:", err);
	process.exitCode = 1;
});
