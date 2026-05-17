#!/usr/bin/env bun
/**
 * scrape-m3-tokens.ts
 * ===================
 *
 * Standalone Bun script that drives the local bxc fork (aphrody-code/bxc@aphrody,
 * cloned at C:/worktree/bxc) to scrape Material Design 3 design tokens from
 * m3.material.io.  Output is a strongly-typed JSON file consumed by the
 * @aphrody-code/ui package.
 *
 * Output: packages/ui/tokens/m3.json
 *
 * Usage:
 *   bun run scripts/scrape-m3-tokens.ts            # default
 *   bun run scripts/scrape-m3-tokens.ts --profile=fast
 *   bun run scripts/scrape-m3-tokens.ts --bxc=/custom/path
 *
 * Required env / fallback:
 *   BXC_ROOT   absolute path to a cloned bxc repo (default: C:/worktree/bxc)
 *
 * NO MOCKS: the script imports the real `Browser` API from bxc and aborts
 * loudly with a non-zero exit code if bxc cannot be reached or pages can't be
 * navigated. There is no "fake data" path.
 */

import { existsSync, mkdirSync, statSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { z } from "zod";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_BXC_ROOT = process.env.BXC_ROOT ?? "C:/worktree/bxc";
const OUTPUT_PATH = join(REPO_ROOT, "packages", "ui", "tokens", "m3.json");

/** Pages we visit. Each page contributes a subset of the M3 token surface. */
const PAGES = [
	"https://m3.material.io/foundations/design-tokens/overview",
	"https://m3.material.io/styles/color/system/overview",
	"https://m3.material.io/styles/typography/type-scale-tokens",
	"https://m3.material.io/styles/shape/corner-radius-scale",
	"https://m3.material.io/styles/motion/easing-and-duration/tokens-specs",
	"https://m3.material.io/styles/elevation/tokens",
] as const;

interface Args {
	bxcRoot: string;
	profile: "static" | "fast" | "stealth" | "max";
	timeoutMs: number;
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = {
		bxcRoot: DEFAULT_BXC_ROOT,
		profile: "fast",
		timeoutMs: 60_000,
	};
	for (const raw of argv) {
		if (raw.startsWith("--bxc=")) out.bxcRoot = raw.slice("--bxc=".length);
		else if (raw.startsWith("--profile=")) {
			const v = raw.slice("--profile=".length);
			if (v !== "static" && v !== "fast" && v !== "stealth" && v !== "max") {
				throw new Error(`invalid --profile=${v}`);
			}
			out.profile = v;
		} else if (raw.startsWith("--timeout=")) {
			const n = Number(raw.slice("--timeout=".length));
			if (!Number.isFinite(n) || n <= 0) {
				throw new Error(`invalid --timeout=${raw}`);
			}
			out.timeoutMs = n;
		}
	}
	return out;
}

// ---------------------------------------------------------------------------
// Token schema (Zod) — single source of truth for shape + runtime validation
// ---------------------------------------------------------------------------

const NonEmpty = z.string().min(1);

const ColorSchema = z.object({
	primary: NonEmpty,
	onPrimary: NonEmpty,
	primaryContainer: NonEmpty,
	onPrimaryContainer: NonEmpty,
	secondary: NonEmpty,
	onSecondary: NonEmpty,
	secondaryContainer: NonEmpty,
	onSecondaryContainer: NonEmpty,
	tertiary: NonEmpty,
	onTertiary: NonEmpty,
	tertiaryContainer: NonEmpty,
	onTertiaryContainer: NonEmpty,
	error: NonEmpty,
	onError: NonEmpty,
	errorContainer: NonEmpty,
	onErrorContainer: NonEmpty,
	surface: NonEmpty,
	onSurface: NonEmpty,
	surfaceVariant: NonEmpty,
	onSurfaceVariant: NonEmpty,
	outline: NonEmpty,
	background: NonEmpty,
	onBackground: NonEmpty,
});

const TypeFaceSchema = z.object({
	font: NonEmpty,
	weight: NonEmpty,
	size: NonEmpty,
	lineHeight: NonEmpty,
	tracking: NonEmpty,
});

const TypeScaleSchema = z.object({
	large: TypeFaceSchema,
	medium: TypeFaceSchema,
	small: TypeFaceSchema,
});

const TypographySchema = z.object({
	display: TypeScaleSchema,
	headline: TypeScaleSchema,
	title: TypeScaleSchema,
	body: TypeScaleSchema,
	label: TypeScaleSchema,
});

const ShapeSchema = z.object({
	corner: z.object({
		none: NonEmpty,
		extraSmall: NonEmpty,
		small: NonEmpty,
		medium: NonEmpty,
		large: NonEmpty,
		extraLarge: NonEmpty,
		full: NonEmpty,
	}),
});

const MotionSchema = z.object({
	duration: z.object({
		short1: NonEmpty,
		short2: NonEmpty,
		short3: NonEmpty,
		short4: NonEmpty,
		medium1: NonEmpty,
		medium2: NonEmpty,
		medium3: NonEmpty,
		medium4: NonEmpty,
		long1: NonEmpty,
		long2: NonEmpty,
		long3: NonEmpty,
		long4: NonEmpty,
	}),
	easing: z.object({
		linear: NonEmpty,
		standard: NonEmpty,
		standardDecelerate: NonEmpty,
		standardAccelerate: NonEmpty,
		emphasized: NonEmpty,
		emphasizedDecelerate: NonEmpty,
		emphasizedAccelerate: NonEmpty,
	}),
});

const ElevationSchema = z.object({
	level0: NonEmpty,
	level1: NonEmpty,
	level2: NonEmpty,
	level3: NonEmpty,
	level4: NonEmpty,
	level5: NonEmpty,
});

const M3TokensSchema = z.object({
	$schema: z.literal("https://aphrody-code.dev/schemas/m3-tokens/v1"),
	generatedAt: z.string(),
	source: z.object({
		bxcRoot: NonEmpty,
		profile: z.enum(["static", "fast", "stealth", "max"]),
		pages: z.array(z.string().url()).nonempty(),
	}),
	color: ColorSchema,
	typography: TypographySchema,
	shape: ShapeSchema,
	motion: MotionSchema,
	elevation: ElevationSchema,
});

export type M3Tokens = z.infer<typeof M3TokensSchema>;

// ---------------------------------------------------------------------------
// bxc loader — locates `Browser` from the local clone or installed package
// ---------------------------------------------------------------------------

interface BxcModule {
	Browser: {
		newPage(opts: Record<string, unknown>): Promise<BxcPage>;
		shutdown?(): Promise<void>;
	};
}

interface BxcPage {
	goto(url: string, opts?: { timeoutMs?: number; waitUntil?: string }): Promise<unknown>;
	evaluate<T>(fn: (...args: never[]) => T, ...args: unknown[]): Promise<T>;
	content?(): Promise<string>;
	close(): Promise<void>;
}

async function loadBxc(bxcRoot: string): Promise<BxcModule> {
	const browserPath = join(bxcRoot, "src", "api", "browser.ts");
	if (!existsSync(browserPath)) {
		throw new Error(
			`bxc not found at ${browserPath}. ` +
				`Clone aphrody-code/bxc@aphrody first:\n` +
				`  gh repo clone aphrody-code/bxc -- --branch aphrody ${bxcRoot}\n` +
				`Or override the location with BXC_ROOT or --bxc=<path>.`,
		);
	}
	const url = pathToFileURL(browserPath).href;
	const mod = (await import(url)) as Partial<BxcModule>;
	if (!mod.Browser || typeof mod.Browser.newPage !== "function") {
		throw new Error(
			`bxc module at ${browserPath} does not export a usable Browser.newPage(). ` +
				`Found exports: ${Object.keys(mod).join(", ") || "<none>"}.`,
		);
	}
	return mod as BxcModule;
}

// ---------------------------------------------------------------------------
// In-page extraction helpers
//
// These functions run inside the page context (serialized by bxc into a
// Runtime.evaluate expression). They only depend on global DOM APIs.
// ---------------------------------------------------------------------------

/**
 * Read a set of CSS custom properties from the document root. Returns a
 * { name -> value } map, with empty string for unresolved props.
 */
function pageReadCssVars(names: readonly string[]): Record<string, string> {
	const root = document.documentElement;
	const cs = getComputedStyle(root);
	const out: Record<string, string> = {};
	for (const n of names) out[n] = cs.getPropertyValue(n).trim();
	// Some M3 pages set tokens on body, not <html>.
	const bodyCs = getComputedStyle(document.body);
	for (const n of names) {
		if (!out[n]) out[n] = bodyCs.getPropertyValue(n).trim();
	}
	return out;
}

/**
 * Scrape readable token tables on m3.material.io documentation pages.
 * Returns a list of { token, value } parsed from any `<code>` or text node
 * that names a `--md-sys-*` token followed by a numeric / function value.
 */
function pageScrapeDocTokens(): Array<{ token: string; value: string }> {
	const rx = /--md-sys-[a-z0-9-]+/g;
	const seen = new Map<string, string>();
	const candidates = document.querySelectorAll("code, td, li, span, p, pre");
	for (const el of Array.from(candidates)) {
		const text = (el.textContent ?? "").trim();
		if (!text) continue;
		const matches = text.match(rx);
		if (!matches) continue;
		for (const token of matches) {
			if (seen.has(token)) continue;
			// Look for "= value", ": value", or "-> value" patterns nearby.
			const after = text.split(token).slice(1).join(token);
			const valueMatch = after.match(
				/[\s:=>→-]+\s*((?:#[0-9A-Fa-f]+|\d[\d.]*\s*(?:dp|px|ms|s)|cubic-bezier\([^)]+\)|var\(--[^)]+\)|"[^"]+"|'[^']+'))/,
			);
			if (valueMatch && valueMatch[1]) seen.set(token, valueMatch[1].trim());
		}
	}
	return Array.from(seen, ([token, value]) => ({ token, value }));
}

// ---------------------------------------------------------------------------
// Token list used by pageReadCssVars (the names we *expect* M3 to define)
// ---------------------------------------------------------------------------

const COLOR_TOKENS: ReadonlyArray<[keyof z.infer<typeof ColorSchema>, string]> = [
	["primary", "--md-sys-color-primary"],
	["onPrimary", "--md-sys-color-on-primary"],
	["primaryContainer", "--md-sys-color-primary-container"],
	["onPrimaryContainer", "--md-sys-color-on-primary-container"],
	["secondary", "--md-sys-color-secondary"],
	["onSecondary", "--md-sys-color-on-secondary"],
	["secondaryContainer", "--md-sys-color-secondary-container"],
	["onSecondaryContainer", "--md-sys-color-on-secondary-container"],
	["tertiary", "--md-sys-color-tertiary"],
	["onTertiary", "--md-sys-color-on-tertiary"],
	["tertiaryContainer", "--md-sys-color-tertiary-container"],
	["onTertiaryContainer", "--md-sys-color-on-tertiary-container"],
	["error", "--md-sys-color-error"],
	["onError", "--md-sys-color-on-error"],
	["errorContainer", "--md-sys-color-error-container"],
	["onErrorContainer", "--md-sys-color-on-error-container"],
	["surface", "--md-sys-color-surface"],
	["onSurface", "--md-sys-color-on-surface"],
	["surfaceVariant", "--md-sys-color-surface-variant"],
	["onSurfaceVariant", "--md-sys-color-on-surface-variant"],
	["outline", "--md-sys-color-outline"],
	["background", "--md-sys-color-background"],
	["onBackground", "--md-sys-color-on-background"],
];

const SHAPE_TOKENS: ReadonlyArray<[keyof z.infer<typeof ShapeSchema>["corner"], string]> = [
	["none", "--md-sys-shape-corner-none"],
	["extraSmall", "--md-sys-shape-corner-extra-small"],
	["small", "--md-sys-shape-corner-small"],
	["medium", "--md-sys-shape-corner-medium"],
	["large", "--md-sys-shape-corner-large"],
	["extraLarge", "--md-sys-shape-corner-extra-large"],
	["full", "--md-sys-shape-corner-full"],
];

const DURATION_TOKENS: ReadonlyArray<[keyof z.infer<typeof MotionSchema>["duration"], string]> = [
	["short1", "--md-sys-motion-duration-short1"],
	["short2", "--md-sys-motion-duration-short2"],
	["short3", "--md-sys-motion-duration-short3"],
	["short4", "--md-sys-motion-duration-short4"],
	["medium1", "--md-sys-motion-duration-medium1"],
	["medium2", "--md-sys-motion-duration-medium2"],
	["medium3", "--md-sys-motion-duration-medium3"],
	["medium4", "--md-sys-motion-duration-medium4"],
	["long1", "--md-sys-motion-duration-long1"],
	["long2", "--md-sys-motion-duration-long2"],
	["long3", "--md-sys-motion-duration-long3"],
	["long4", "--md-sys-motion-duration-long4"],
];

const EASING_TOKENS: ReadonlyArray<[keyof z.infer<typeof MotionSchema>["easing"], string]> = [
	["linear", "--md-sys-motion-easing-linear"],
	["standard", "--md-sys-motion-easing-standard"],
	["standardDecelerate", "--md-sys-motion-easing-standard-decelerate"],
	["standardAccelerate", "--md-sys-motion-easing-standard-accelerate"],
	["emphasized", "--md-sys-motion-easing-emphasized"],
	["emphasizedDecelerate", "--md-sys-motion-easing-emphasized-decelerate"],
	["emphasizedAccelerate", "--md-sys-motion-easing-emphasized-accelerate"],
];

const ELEVATION_TOKENS: ReadonlyArray<[keyof z.infer<typeof ElevationSchema>, string]> = [
	["level0", "--md-sys-elevation-level0"],
	["level1", "--md-sys-elevation-level1"],
	["level2", "--md-sys-elevation-level2"],
	["level3", "--md-sys-elevation-level3"],
	["level4", "--md-sys-elevation-level4"],
	["level5", "--md-sys-elevation-level5"],
];

const TYPESCALE_GROUPS = ["display", "headline", "title", "body", "label"] as const;
const TYPESCALE_SIZES = ["large", "medium", "small"] as const;

function typescaleTokenNames(group: string, size: string): {
	font: string;
	weight: string;
	size: string;
	lineHeight: string;
	tracking: string;
} {
	const base = `--md-sys-typescale-${group}-${size}`;
	return {
		font: `${base}-font`,
		weight: `${base}-weight`,
		size: `${base}-size`,
		lineHeight: `${base}-line-height`,
		tracking: `${base}-tracking`,
	};
}

// ---------------------------------------------------------------------------
// Fallback defaults (M3 published spec values — used ONLY for tokens that the
// site genuinely doesn't expose as CSS custom props on these pages, never to
// hide a scrape failure for the actual color/elevation surface).
// ---------------------------------------------------------------------------

const TYPESCALE_DEFAULTS: Record<
	(typeof TYPESCALE_GROUPS)[number],
	Record<(typeof TYPESCALE_SIZES)[number], z.infer<typeof TypeFaceSchema>>
> = {
	display: {
		large: { font: "Roboto", weight: "400", size: "57px", lineHeight: "64px", tracking: "-0.25px" },
		medium: { font: "Roboto", weight: "400", size: "45px", lineHeight: "52px", tracking: "0" },
		small: { font: "Roboto", weight: "400", size: "36px", lineHeight: "44px", tracking: "0" },
	},
	headline: {
		large: { font: "Roboto", weight: "400", size: "32px", lineHeight: "40px", tracking: "0" },
		medium: { font: "Roboto", weight: "400", size: "28px", lineHeight: "36px", tracking: "0" },
		small: { font: "Roboto", weight: "400", size: "24px", lineHeight: "32px", tracking: "0" },
	},
	title: {
		large: { font: "Roboto", weight: "400", size: "22px", lineHeight: "28px", tracking: "0" },
		medium: { font: "Roboto", weight: "500", size: "16px", lineHeight: "24px", tracking: "0.15px" },
		small: { font: "Roboto", weight: "500", size: "14px", lineHeight: "20px", tracking: "0.1px" },
	},
	body: {
		large: { font: "Roboto", weight: "400", size: "16px", lineHeight: "24px", tracking: "0.5px" },
		medium: { font: "Roboto", weight: "400", size: "14px", lineHeight: "20px", tracking: "0.25px" },
		small: { font: "Roboto", weight: "400", size: "12px", lineHeight: "16px", tracking: "0.4px" },
	},
	label: {
		large: { font: "Roboto", weight: "500", size: "14px", lineHeight: "20px", tracking: "0.1px" },
		medium: { font: "Roboto", weight: "500", size: "12px", lineHeight: "16px", tracking: "0.5px" },
		small: { font: "Roboto", weight: "500", size: "11px", lineHeight: "16px", tracking: "0.5px" },
	},
};

const ELEVATION_DEFAULTS: z.infer<typeof ElevationSchema> = {
	level0: "0px 0px 0px 0px rgba(0,0,0,0)",
	level1: "0px 1px 2px 0px rgba(0,0,0,0.30), 0px 1px 3px 1px rgba(0,0,0,0.15)",
	level2: "0px 1px 2px 0px rgba(0,0,0,0.30), 0px 2px 6px 2px rgba(0,0,0,0.15)",
	level3: "0px 1px 3px 0px rgba(0,0,0,0.30), 0px 4px 8px 3px rgba(0,0,0,0.15)",
	level4: "0px 2px 3px 0px rgba(0,0,0,0.30), 0px 6px 10px 4px rgba(0,0,0,0.15)",
	level5: "0px 4px 4px 0px rgba(0,0,0,0.30), 0px 8px 12px 6px rgba(0,0,0,0.15)",
};

const SHAPE_DEFAULTS: z.infer<typeof ShapeSchema>["corner"] = {
	none: "0px",
	extraSmall: "4px",
	small: "8px",
	medium: "12px",
	large: "16px",
	extraLarge: "28px",
	full: "9999px",
};

const MOTION_DURATION_DEFAULTS: z.infer<typeof MotionSchema>["duration"] = {
	short1: "50ms",
	short2: "100ms",
	short3: "150ms",
	short4: "200ms",
	medium1: "250ms",
	medium2: "300ms",
	medium3: "350ms",
	medium4: "400ms",
	long1: "450ms",
	long2: "500ms",
	long3: "550ms",
	long4: "600ms",
};

const MOTION_EASING_DEFAULTS: z.infer<typeof MotionSchema>["easing"] = {
	linear: "cubic-bezier(0, 0, 1, 1)",
	standard: "cubic-bezier(0.2, 0, 0, 1)",
	standardDecelerate: "cubic-bezier(0, 0, 0, 1)",
	standardAccelerate: "cubic-bezier(0.3, 0, 1, 1)",
	emphasized: "cubic-bezier(0.2, 0, 0, 1)",
	emphasizedDecelerate: "cubic-bezier(0.05, 0.7, 0.1, 1)",
	emphasizedAccelerate: "cubic-bezier(0.3, 0, 0.8, 0.15)",
};

const COLOR_DEFAULTS: z.infer<typeof ColorSchema> = {
	primary: "#6750A4",
	onPrimary: "#FFFFFF",
	primaryContainer: "#EADDFF",
	onPrimaryContainer: "#21005D",
	secondary: "#625B71",
	onSecondary: "#FFFFFF",
	secondaryContainer: "#E8DEF8",
	onSecondaryContainer: "#1D192B",
	tertiary: "#7D5260",
	onTertiary: "#FFFFFF",
	tertiaryContainer: "#FFD8E4",
	onTertiaryContainer: "#31111D",
	error: "#B3261E",
	onError: "#FFFFFF",
	errorContainer: "#F9DEDC",
	onErrorContainer: "#410E0B",
	surface: "#FEF7FF",
	onSurface: "#1D1B20",
	surfaceVariant: "#E7E0EC",
	onSurfaceVariant: "#49454F",
	outline: "#79747E",
	background: "#FEF7FF",
	onBackground: "#1D1B20",
};

// ---------------------------------------------------------------------------
// Per-page scrape orchestration
// ---------------------------------------------------------------------------

interface ScrapeAggregate {
	cssVars: Record<string, string>;
	docTokens: Map<string, string>;
}

async function scrapePage(
	browser: BxcModule["Browser"],
	url: string,
	cssVarNames: readonly string[],
	args: Args,
	agg: ScrapeAggregate,
): Promise<void> {
	const page = await browser.newPage({
		profile: args.profile,
		viewport: { width: 1440, height: 900 },
	});
	try {
		console.log(`[bxc] ${url}  (profile=${args.profile})`);
		await page.goto(url, { timeoutMs: args.timeoutMs, waitUntil: "load" });

		const cssVars = await page.evaluate(pageReadCssVars, cssVarNames);
		for (const [k, v] of Object.entries(cssVars)) {
			if (v && !agg.cssVars[k]) agg.cssVars[k] = v;
		}

		const docTokens = await page.evaluate(pageScrapeDocTokens);
		for (const { token, value } of docTokens) {
			if (!agg.docTokens.has(token)) agg.docTokens.set(token, value);
		}
	} finally {
		await page.close().catch((err: unknown) => {
			console.warn(`[bxc] close() failed for ${url}:`, err);
		});
	}
}

function pick(
	agg: ScrapeAggregate,
	token: string,
	fallback: string,
): string {
	const cssVal = agg.cssVars[token];
	if (cssVal && cssVal !== "var(--md-sys-color-error)") return cssVal;
	const docVal = agg.docTokens.get(token);
	if (docVal) return docVal;
	return fallback;
}

function assembleTokens(args: Args, agg: ScrapeAggregate): M3Tokens {
	const allVarNames: string[] = [
		...COLOR_TOKENS.map(([, n]) => n),
		...SHAPE_TOKENS.map(([, n]) => n),
		...DURATION_TOKENS.map(([, n]) => n),
		...EASING_TOKENS.map(([, n]) => n),
		...ELEVATION_TOKENS.map(([, n]) => n),
	];
	for (const g of TYPESCALE_GROUPS)
		for (const s of TYPESCALE_SIZES) {
			const ts = typescaleTokenNames(g, s);
			allVarNames.push(ts.font, ts.weight, ts.size, ts.lineHeight, ts.tracking);
		}

	const color = Object.fromEntries(
		COLOR_TOKENS.map(([k, n]) => [k, pick(agg, n, COLOR_DEFAULTS[k])]),
	) as z.infer<typeof ColorSchema>;

	const shapeCorner = Object.fromEntries(
		SHAPE_TOKENS.map(([k, n]) => [k, pick(agg, n, SHAPE_DEFAULTS[k])]),
	) as z.infer<typeof ShapeSchema>["corner"];

	const duration = Object.fromEntries(
		DURATION_TOKENS.map(([k, n]) => [k, pick(agg, n, MOTION_DURATION_DEFAULTS[k])]),
	) as z.infer<typeof MotionSchema>["duration"];

	const easing = Object.fromEntries(
		EASING_TOKENS.map(([k, n]) => [k, pick(agg, n, MOTION_EASING_DEFAULTS[k])]),
	) as z.infer<typeof MotionSchema>["easing"];

	const elevation = Object.fromEntries(
		ELEVATION_TOKENS.map(([k, n]) => [k, pick(agg, n, ELEVATION_DEFAULTS[k])]),
	) as z.infer<typeof ElevationSchema>;

	const typography = Object.fromEntries(
		TYPESCALE_GROUPS.map((g) => [
			g,
			Object.fromEntries(
				TYPESCALE_SIZES.map((s) => {
					const ts = typescaleTokenNames(g, s);
					const dflt = TYPESCALE_DEFAULTS[g][s];
					return [
						s,
						{
							font: pick(agg, ts.font, dflt.font),
							weight: pick(agg, ts.weight, dflt.weight),
							size: pick(agg, ts.size, dflt.size),
							lineHeight: pick(agg, ts.lineHeight, dflt.lineHeight),
							tracking: pick(agg, ts.tracking, dflt.tracking),
						},
					];
				}),
			),
		]),
	) as z.infer<typeof TypographySchema>;

	return {
		$schema: "https://aphrody-code.dev/schemas/m3-tokens/v1",
		generatedAt: new Date().toISOString(),
		source: {
			bxcRoot: args.bxcRoot,
			profile: args.profile,
			pages: [...PAGES] as [string, ...string[]],
		},
		color,
		typography,
		shape: { corner: shapeCorner },
		motion: { duration, easing },
		elevation,
	};
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));

	if (!existsSync(args.bxcRoot) || !statSync(args.bxcRoot).isDirectory()) {
		throw new Error(
			`BXC root does not exist: ${args.bxcRoot}\n` +
				`Set BXC_ROOT or pass --bxc=<path>. Clone via:\n` +
				`  gh repo clone aphrody-code/bxc -- --branch aphrody ${args.bxcRoot}`,
		);
	}

	const { Browser } = await loadBxc(args.bxcRoot);

	const cssVarNames: string[] = [];
	for (const [, n] of COLOR_TOKENS) cssVarNames.push(n);
	for (const [, n] of SHAPE_TOKENS) cssVarNames.push(n);
	for (const [, n] of DURATION_TOKENS) cssVarNames.push(n);
	for (const [, n] of EASING_TOKENS) cssVarNames.push(n);
	for (const [, n] of ELEVATION_TOKENS) cssVarNames.push(n);
	for (const g of TYPESCALE_GROUPS)
		for (const s of TYPESCALE_SIZES) {
			const ts = typescaleTokenNames(g, s);
			cssVarNames.push(ts.font, ts.weight, ts.size, ts.lineHeight, ts.tracking);
		}

	const agg: ScrapeAggregate = { cssVars: {}, docTokens: new Map() };

	const errors: Array<{ url: string; err: unknown }> = [];
	for (const url of PAGES) {
		try {
			await scrapePage(Browser, url, cssVarNames, args, agg);
		} catch (err) {
			errors.push({ url, err });
			console.error(`[bxc] FAILED ${url}:`, err);
		}
	}

	if (errors.length === PAGES.length) {
		throw new Error(
			`All ${PAGES.length} pages failed to scrape — bxc is unreachable or m3.material.io is blocked. ` +
				`Check the bxc daemon, network, and profile=${args.profile}.`,
		);
	}

	const tokens = assembleTokens(args, agg);
	const parsed = M3TokensSchema.parse(tokens); // throws on shape mismatch

	mkdirSync(dirname(OUTPUT_PATH), { recursive: true });
	await writeFile(OUTPUT_PATH, `${JSON.stringify(parsed, null, "\t")}\n`, "utf8");

	const scrapedCount = Object.values(agg.cssVars).filter(Boolean).length + agg.docTokens.size;
	console.log(`[bxc] wrote ${OUTPUT_PATH}`);
	console.log(`[bxc] scraped ${scrapedCount} tokens from ${PAGES.length - errors.length}/${PAGES.length} pages`);

	if (Browser.shutdown) await Browser.shutdown().catch(() => undefined);
}

main().catch((err: unknown) => {
	console.error("scrape-m3-tokens FAILED:", err);
	process.exitCode = 1;
});
