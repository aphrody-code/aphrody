// SPDX-License-Identifier: Apache-2.0
// #!/usr/bin/env bun
/**
 * m3-coverage-audit.ts
 * ====================
 *
 * Static workspace audit that compares the canonical Material Design 3
 * specification surface against what aphrody actually implements in:
 *
 *   - `crates/m3-tokens/src/`           (design-token surfaces)
 *   - `crates/shadcn-bridge/src/`       (component bridge wrappers)
 *   - `crates/aphrody-wasm/examples/`   (pixel-perfect HTML showcases)
 *
 * Emits a deterministic Markdown report at `docs/audits/m3-coverage.md` and
 * an optional JSON dump on stdout for CI integration.
 *
 * Pure Bun + node:fs/path. No external deps. No Rust toolchain required —
 * the script greps `pub const` / `pub fn` / `pub struct *Props` /
 * `pub fn create_` / `#[wasm_bindgen]` directly from source text. Counts
 * are therefore a syntactic lower bound (a regression guard, not a typechecker).
 *
 * Usage:
 *   bun run scripts/m3-coverage-audit.ts            # write Markdown report
 *   bun run scripts/m3-coverage-audit.ts --check    # exit 1 if surfaces missing
 *   bun run scripts/m3-coverage-audit.ts --json     # emit JSON to stdout
 *
 * Exit codes:
 *   0  report written (or JSON emitted) successfully and no required-surface gap
 *   1  --check passed and at least one required token / component surface is missing
 *   2  CLI / IO error (bad args, unreadable source, write failure)
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

// ---------------------------------------------------------------------------
// Repo layout
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(import.meta.dir, "..");
const M3_TOKENS_DIR = join(REPO_ROOT, "crates", "m3-tokens", "src");
const SHADCN_BRIDGE_DIR = join(REPO_ROOT, "crates", "shadcn-bridge", "src");
const WASM_EXAMPLES_DIR = join(REPO_ROOT, "crates", "aphrody-wasm", "examples");
const OUT_MARKDOWN = join(REPO_ROOT, "docs", "audits", "m3-coverage.md");

// ---------------------------------------------------------------------------
// Canonical M3 surfaces
// ---------------------------------------------------------------------------

/**
 * The eight design-token surfaces the M3 spec defines under
 * https://m3.material.io/styles. The first six are implemented today;
 * `tonal` and `dynamic` are tracked here so the audit reports the gap.
 */
const M3_TOKEN_SURFACES: readonly string[] = [
	"color",
	"typography",
	"elevation",
	"motion",
	"shape",
	"state",
	"tonal",
	"dynamic",
];

/**
 * Canonical M3 component catalogue (m3.material.io/components). Each entry
 * is the spec slug; the audit cross-references against shadcn-bridge files
 * and against `<md-*>` custom elements in the pixel-perfect HTML demo.
 */
const M3_COMPONENT_CATALOGUE: readonly string[] = [
	"badges",
	"bottom-app-bar",
	"bottom-sheets",
	"buttons-common",
	"buttons-extended-fab",
	"buttons-fab",
	"buttons-icon-button",
	"buttons-segmented",
	"cards",
	"carousel",
	"checkbox",
	"chips",
	"date-pickers",
	"dialogs",
	"divider",
	"fab",
	"lists",
	"menus",
	"navigation-bar",
	"navigation-drawer",
	"navigation-rail",
	"progress-indicators",
	"radio",
	"search",
	"sliders",
	"snackbars",
	"switch",
	"tabs",
	"text-fields",
	"time-pickers",
	"tooltips",
	"top-app-bar",
];

/**
 * Map shadcn-bridge source filenames → which M3 catalogue slugs they cover.
 * One bridge file may cover multiple spec entries (e.g. `button.rs` covers
 * the whole common-button family).
 */
const SHADCN_TO_M3: Record<string, readonly string[]> = {
	"app_bar.rs": ["top-app-bar", "bottom-app-bar"],
	"avatar.rs": [],
	"badge.rs": ["badges"],
	"bottom_sheet.rs": ["bottom-sheets"],
	"button.rs": ["buttons-common"],
	"card.rs": ["cards"],
	"checkbox.rs": ["checkbox"],
	"chip.rs": ["chips"],
	"date_picker.rs": ["date-pickers"],
	"dialog.rs": ["dialogs"],
	"fab.rs": ["fab", "buttons-extended-fab", "buttons-fab"],
	"icon_button.rs": ["buttons-icon-button"],
	"input.rs": ["text-fields"],
	"list.rs": ["lists"],
	"navigation_bar.rs": ["navigation-bar"],
	"navigation_drawer.rs": ["navigation-drawer"],
	"navigation_rail.rs": ["navigation-rail"],
	"progress.rs": ["progress-indicators"],
	"radio_group.rs": ["radio"],
	"search_bar.rs": ["search"],
	"segmented_button.rs": ["buttons-segmented"],
	"select.rs": ["menus"],
	"slider.rs": ["sliders"],
	"switch.rs": ["switch"],
	"tabs.rs": ["tabs"],
	"time_picker.rs": ["time-pickers"],
	"toast.rs": ["snackbars"],
	"tooltip.rs": ["tooltips"],
};

/**
 * Map `<md-*>` custom element tag → M3 catalogue slug it satisfies.
 * Tags absent from this table do not contribute to component coverage
 * (e.g. `<md-elevation>` is a styling primitive, not a component).
 */
const MD_TAG_TO_M3: Record<string, string> = {
	"md-filled-button": "buttons-common",
	"md-outlined-button": "buttons-common",
	"md-text-button": "buttons-common",
	"md-filled-tonal-button": "buttons-common",
	"md-elevated-button": "buttons-common",
	"md-fab": "fab",
	"md-branded-fab": "fab",
	"md-icon-button": "buttons-icon-button",
	"md-filled-icon-button": "buttons-icon-button",
	"md-outlined-icon-button": "buttons-icon-button",
	"md-filled-tonal-icon-button": "buttons-icon-button",
	"md-outlined-text-field": "text-fields",
	"md-filled-text-field": "text-fields",
	"md-dialog": "dialogs",
	"md-tabs": "tabs",
	"md-primary-tab": "tabs",
	"md-secondary-tab": "tabs",
	"md-outlined-select": "menus",
	"md-filled-select": "menus",
	"md-select-option": "menus",
	"md-menu": "menus",
	"md-menu-item": "menus",
	"md-sub-menu": "menus",
	"md-checkbox": "checkbox",
	"md-radio": "radio",
	"md-switch": "switch",
	"md-slider": "sliders",
	"md-chip-set": "chips",
	"md-assist-chip": "chips",
	"md-filter-chip": "chips",
	"md-input-chip": "chips",
	"md-suggestion-chip": "chips",
	"md-divider": "divider",
	"md-list": "lists",
	"md-list-item": "lists",
	"md-circular-progress": "progress-indicators",
	"md-linear-progress": "progress-indicators",
	"md-navigation-bar": "navigation-bar",
	"md-navigation-tab": "navigation-bar",
	"md-navigation-drawer": "navigation-drawer",
	"md-navigation-rail": "navigation-rail",
	"md-search": "search",
	"md-snackbar": "snackbars",
	"md-tooltip": "tooltips",
	"md-top-app-bar": "top-app-bar",
	"md-bottom-app-bar": "bottom-app-bar",
	"md-bottom-sheet": "bottom-sheets",
	"md-badge": "badges",
	"md-date-picker": "date-pickers",
	"md-time-picker": "time-pickers",
	"md-carousel": "carousel",
};

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------

interface CliArgs {
	check: boolean;
	json: boolean;
}

function parseArgs(argv: readonly string[]): CliArgs {
	const out: CliArgs = { check: false, json: false };
	for (const raw of argv) {
		if (raw === "--check") out.check = true;
		else if (raw === "--json") out.json = true;
		else if (raw === "--help" || raw === "-h") {
			printHelp();
			process.exit(0);
		} else {
			throw new Error(`unknown flag: ${raw} (try --help)`);
		}
	}
	return out;
}

function printHelp(): void {
	process.stdout.write(
		"m3-coverage-audit — compare aphrody implementation against the M3 spec\n\n" +
			"Usage:\n" +
			"  bun run scripts/m3-coverage-audit.ts [--check] [--json]\n\n" +
			"Flags:\n" +
			"  --check   exit 1 if any required token/component surface is missing\n" +
			"  --json    print JSON report to stdout (no Markdown write)\n" +
			"  --help    show this help text\n",
	);
}

// ---------------------------------------------------------------------------
// Source-text helpers (no Rust toolchain needed — purely textual)
// ---------------------------------------------------------------------------

function readIfExists(path: string): string | null {
	if (!existsSync(path)) return null;
	try {
		return readFileSync(path, "utf8");
	} catch (err) {
		throw new Error(`failed to read ${path}: ${(err as Error).message}`);
	}
}

function countMatches(source: string, pattern: RegExp): number {
	const matches = source.match(pattern);
	return matches ? matches.length : 0;
}

/** Strips // line comments and /* block * / comments so counts are real. */
function stripComments(source: string): string {
	// Remove block comments first (non-greedy, multi-line).
	let s = source.replace(/\/\*[\s\S]*?\*\//g, "");
	// Then line comments.
	s = s.replace(/^[ \t]*\/\/.*$/gm, "");
	return s;
}

// ---------------------------------------------------------------------------
// Dimension 1 — Token surfaces
// ---------------------------------------------------------------------------

interface TokenSurfaceReport {
	surface: string;
	file: string;
	present: boolean;
	pubConst: number;
	pubFn: number;
}

function auditTokenSurfaces(): TokenSurfaceReport[] {
	const out: TokenSurfaceReport[] = [];
	for (const surface of M3_TOKEN_SURFACES) {
		const file = `${surface}.rs`;
		const path = join(M3_TOKENS_DIR, file);
		const src = readIfExists(path);
		if (src === null) {
			out.push({ surface, file, present: false, pubConst: 0, pubFn: 0 });
			continue;
		}
		const stripped = stripComments(src);
		// `pub const NAME:` — anchored to a line beginning so commented-out
		// or doc-test code inside /// blocks does not inflate the count.
		const pubConst = countMatches(stripped, /^\s*pub\s+const\s+[A-Z_][A-Z0-9_]*\s*:/gm);
		// `pub fn name(` — same anchoring; counts both plain and generic fns.
		const pubFn = countMatches(stripped, /^\s*pub\s+(?:async\s+)?fn\s+[a-z_][a-z0-9_]*\s*[<(]/gm);
		out.push({ surface, file, present: true, pubConst, pubFn });
	}
	return out;
}

// ---------------------------------------------------------------------------
// Dimension 2 — Component surfaces
// ---------------------------------------------------------------------------

interface ComponentSurfaceReport {
	file: string;
	present: boolean;
	hasPropsStruct: boolean;
	hasCreateFn: boolean;
	hasWasmBindgen: boolean;
}

function auditComponentSurfaces(): ComponentSurfaceReport[] {
	const out: ComponentSurfaceReport[] = [];
	if (!existsSync(SHADCN_BRIDGE_DIR)) return out;
	const files = readdirSync(SHADCN_BRIDGE_DIR)
		.filter((f) => f.endsWith(".rs") && f !== "lib.rs")
		.sort();
	for (const file of files) {
		const path = join(SHADCN_BRIDGE_DIR, file);
		const src = readIfExists(path);
		if (src === null) {
			out.push({
				file,
				present: false,
				hasPropsStruct: false,
				hasCreateFn: false,
				hasWasmBindgen: false,
			});
			continue;
		}
		const stripped = stripComments(src);
		const hasPropsStruct = /pub\s+struct\s+[A-Z][A-Za-z0-9_]*Props\b/.test(stripped);
		const hasCreateFn = /pub\s+fn\s+create_[a-z_][a-z0-9_]*\s*\(/.test(stripped);
		// `#[wasm_bindgen]` attribute — keep the literal brackets, escape them.
		const hasWasmBindgen = /#\[wasm_bindgen[\s\S]*?\]/.test(stripped);
		out.push({ file, present: true, hasPropsStruct, hasCreateFn, hasWasmBindgen });
	}
	return out;
}

// ---------------------------------------------------------------------------
// Dimension 3 — M3 spec catalogue cross-reference
// ---------------------------------------------------------------------------

interface CatalogueReport {
	covered: string[];
	missing: string[];
	coverageByBridge: Map<string, string[]>;
}

function auditCatalogueCoverage(components: ComponentSurfaceReport[]): CatalogueReport {
	const coveredSet = new Set<string>();
	const coverageByBridge = new Map<string, string[]>();
	for (const c of components) {
		const slugs = SHADCN_TO_M3[c.file] ?? [];
		coverageByBridge.set(c.file, [...slugs]);
		for (const slug of slugs) coveredSet.add(slug);
	}
	const covered = M3_COMPONENT_CATALOGUE.filter((s) => coveredSet.has(s));
	const missing = M3_COMPONENT_CATALOGUE.filter((s) => !coveredSet.has(s));
	return { covered, missing, coverageByBridge };
}

// ---------------------------------------------------------------------------
// Dimension 4 — Pixel-perfect HTML coverage
// ---------------------------------------------------------------------------

interface HtmlCoverageReport {
	files: string[];
	mdTagsUsed: string[];
	mdTagsUnrecognised: string[];
	componentsCovered: string[];
	componentsMissing: string[];
}

function auditHtmlCoverage(): HtmlCoverageReport {
	const candidates = ["m3-shadcn-pixel-perfect.html", "m3-shadcn-pixel-perfect-v2.html"];
	const filesPresent: string[] = [];
	const tagSet = new Set<string>();
	for (const f of candidates) {
		const path = join(WASM_EXAMPLES_DIR, f);
		const src = readIfExists(path);
		if (src === null) continue;
		filesPresent.push(f);
		// `<md-foo>` opening tags (with optional whitespace + attributes). The
		// regex captures the tag name only; `g` flag plus `matchAll` for the
		// global iterator.
		const re = /<(md-[a-z][a-z0-9-]*)\b/g;
		for (const m of src.matchAll(re)) {
			const tag = m[1];
			if (tag !== undefined) tagSet.add(tag);
		}
	}
	const mdTagsUsed = [...tagSet].sort();
	const mdTagsUnrecognised = mdTagsUsed.filter((t) => !(t in MD_TAG_TO_M3));
	const coveredSet = new Set<string>();
	for (const t of mdTagsUsed) {
		const slug = MD_TAG_TO_M3[t];
		if (slug !== undefined) coveredSet.add(slug);
	}
	const componentsCovered = M3_COMPONENT_CATALOGUE.filter((s) => coveredSet.has(s));
	const componentsMissing = M3_COMPONENT_CATALOGUE.filter((s) => !coveredSet.has(s));
	return {
		files: filesPresent,
		mdTagsUsed,
		mdTagsUnrecognised,
		componentsCovered,
		componentsMissing,
	};
}

// ---------------------------------------------------------------------------
// Dimension 5 — Verdict
// ---------------------------------------------------------------------------

interface Verdict {
	tokenSurfacePct: number;
	componentBridgePct: number;
	specCataloguePct: number;
	htmlCataloguePct: number;
	overallPct: number;
	missingTokenSurfaces: string[];
	missingComponents: string[];
	missingHtml: string[];
	topMissing: string[];
}

function pct(num: number, den: number): number {
	if (den === 0) return 0;
	return Math.round((1000 * num) / den) / 10;
}

function computeVerdict(
	tokens: TokenSurfaceReport[],
	components: ComponentSurfaceReport[],
	catalogue: CatalogueReport,
	html: HtmlCoverageReport,
): Verdict {
	const tokensPresent = tokens.filter((t) => t.present).length;
	const tokenSurfacePct = pct(tokensPresent, tokens.length);

	const compFull = components.filter(
		(c) => c.present && c.hasPropsStruct && c.hasCreateFn && c.hasWasmBindgen,
	).length;
	const componentBridgePct = pct(compFull, components.length);

	const specCataloguePct = pct(catalogue.covered.length, M3_COMPONENT_CATALOGUE.length);
	const htmlCataloguePct = pct(html.componentsCovered.length, M3_COMPONENT_CATALOGUE.length);

	// Weighted overall: tokens 25 %, bridge 25 %, catalogue 25 %, html 25 %.
	const overallPct = Math.round(
		(tokenSurfacePct + componentBridgePct + specCataloguePct + htmlCataloguePct) / 4 * 10,
	) / 10;

	const missingTokenSurfaces = tokens.filter((t) => !t.present).map((t) => t.surface);
	// The intersection of catalogue-missing and html-missing is the truly
	// most painful gap — surface up to 10 entries.
	const trulyMissing = catalogue.missing.filter((s) => html.componentsMissing.includes(s));
	const topMissing = trulyMissing.slice(0, 10);

	return {
		tokenSurfacePct,
		componentBridgePct,
		specCataloguePct,
		htmlCataloguePct,
		overallPct,
		missingTokenSurfaces,
		missingComponents: catalogue.missing,
		missingHtml: html.componentsMissing,
		topMissing,
	};
}

// ---------------------------------------------------------------------------
// Markdown rendering
// ---------------------------------------------------------------------------

function yesNo(b: boolean): string {
	return b ? "yes" : "no";
}

function renderMarkdown(
	tokens: TokenSurfaceReport[],
	components: ComponentSurfaceReport[],
	catalogue: CatalogueReport,
	html: HtmlCoverageReport,
	verdict: Verdict,
): string {
	const lines: string[] = [];
	lines.push("<!-- SPDX-License-Identifier: Apache-2.0 -->");
	lines.push("<!-- GENERATED by scripts/m3-coverage-audit.ts — DO NOT EDIT BY HAND. -->");
	lines.push("");
	lines.push("# M3 ↔ aphrody Coverage Audit");
	lines.push("");
	lines.push(
		"Static, deterministic audit comparing the canonical Material Design 3" +
			" specification (m3.material.io) against what the `aphrody`",
	);
	lines.push(
		"workspace actually implements in `crates/m3-tokens/`, `crates/shadcn-bridge/`," +
			" and `crates/aphrody-wasm/examples/`.",
	);
	lines.push("");
	lines.push(
		"Regenerate with `bun run scripts/m3-coverage-audit.ts`. Pass `--check`" +
			" to fail CI when a required surface goes missing.",
	);
	lines.push("");

	// 1. Token surfaces ------------------------------------------------------
	lines.push("## 1. Token surfaces (`crates/m3-tokens/src/`)");
	lines.push("");
	lines.push("| Surface | File | Present | `pub const` | `pub fn` |");
	lines.push("|---------|------|---------|-------------|----------|");
	for (const t of tokens) {
		lines.push(`| ${t.surface} | \`${t.file}\` | ${yesNo(t.present)} | ${t.pubConst} | ${t.pubFn} |`);
	}
	lines.push("");
	lines.push(`Coverage: **${verdict.tokenSurfacePct} %** ` +
		`(${tokens.filter((t) => t.present).length} / ${tokens.length} surfaces present)`);
	if (verdict.missingTokenSurfaces.length > 0) {
		lines.push("");
		lines.push(`Missing surfaces: ${verdict.missingTokenSurfaces.map((s) => `\`${s}\``).join(", ")}`);
	}
	lines.push("");

	// 2. Component surfaces --------------------------------------------------
	lines.push("## 2. Component surfaces (`crates/shadcn-bridge/src/`)");
	lines.push("");
	lines.push("| File | Present | `*Props` struct | `create_*` fn | `#[wasm_bindgen]` |");
	lines.push("|------|---------|-----------------|---------------|-------------------|");
	for (const c of components) {
		lines.push(
			`| \`${c.file}\` | ${yesNo(c.present)} | ${yesNo(c.hasPropsStruct)} | ` +
				`${yesNo(c.hasCreateFn)} | ${yesNo(c.hasWasmBindgen)} |`,
		);
	}
	lines.push("");
	lines.push(`Coverage: **${verdict.componentBridgePct} %** ` +
		`(files where all 3 facets — Props + create_ + wasm_bindgen — are present)`);
	lines.push("");

	// 3. Catalogue cross-reference -------------------------------------------
	lines.push("## 3. M3 component catalogue cross-reference");
	lines.push("");
	lines.push(
		"Canonical M3 component list (m3.material.io/components) cross-" +
			"referenced against shadcn-bridge files via `SHADCN_TO_M3`.",
	);
	lines.push("");
	lines.push("| M3 component | Covered by bridge |");
	lines.push("|--------------|-------------------|");
	const bridgeBySlug = new Map<string, string[]>();
	for (const [file, slugs] of catalogue.coverageByBridge) {
		for (const slug of slugs) {
			const arr = bridgeBySlug.get(slug) ?? [];
			arr.push(file);
			bridgeBySlug.set(slug, arr);
		}
	}
	for (const slug of M3_COMPONENT_CATALOGUE) {
		const files = bridgeBySlug.get(slug) ?? [];
		const cell = files.length === 0 ? "_(none)_" : files.map((f) => `\`${f}\``).join(", ");
		lines.push(`| ${slug} | ${cell} |`);
	}
	lines.push("");
	lines.push(
		`Coverage: **${verdict.specCataloguePct} %** ` +
			`(${catalogue.covered.length} / ${M3_COMPONENT_CATALOGUE.length} catalogue slugs covered)`,
	);
	if (catalogue.missing.length > 0) {
		lines.push("");
		lines.push("Missing components:");
		for (const m of catalogue.missing) lines.push(`- ${m}`);
	}
	lines.push("");

	// 4. Pixel-perfect HTML --------------------------------------------------
	lines.push("## 4. Pixel-perfect HTML coverage (`crates/aphrody-wasm/examples/`)");
	lines.push("");
	if (html.files.length === 0) {
		lines.push("_No pixel-perfect HTML file found._");
	} else {
		lines.push("Files scanned:");
		for (const f of html.files) lines.push(`- \`${f}\``);
		lines.push("");
		lines.push(`Distinct \`<md-*>\` custom elements used: **${html.mdTagsUsed.length}**`);
		lines.push("");
		lines.push("<details><summary>Full tag list</summary>");
		lines.push("");
		for (const t of html.mdTagsUsed) lines.push(`- \`${t}\``);
		lines.push("");
		lines.push("</details>");
		if (html.mdTagsUnrecognised.length > 0) {
			lines.push("");
			lines.push("Unrecognised tags (not mapped to an M3 catalogue slug — review):");
			for (const t of html.mdTagsUnrecognised) lines.push(`- \`${t}\``);
		}
	}
	lines.push("");
	lines.push(
		`Coverage: **${verdict.htmlCataloguePct} %** ` +
			`(${html.componentsCovered.length} / ${M3_COMPONENT_CATALOGUE.length} catalogue slugs touched)`,
	);
	if (html.componentsMissing.length > 0) {
		lines.push("");
		lines.push("Catalogue slugs NOT yet demonstrated in HTML:");
		for (const m of html.componentsMissing) lines.push(`- ${m}`);
	}
	lines.push("");

	// 5. Verdict -------------------------------------------------------------
	lines.push("## 5. Verdict");
	lines.push("");
	lines.push("| Dimension | Coverage |");
	lines.push("|-----------|----------|");
	lines.push(`| Token surfaces | ${verdict.tokenSurfacePct} % |`);
	lines.push(`| Component bridges | ${verdict.componentBridgePct} % |`);
	lines.push(`| Spec catalogue | ${verdict.specCataloguePct} % |`);
	lines.push(`| Pixel-perfect HTML | ${verdict.htmlCataloguePct} % |`);
	lines.push(`| **Overall (mean)** | **${verdict.overallPct} %** |`);
	lines.push("");
	if (verdict.topMissing.length > 0) {
		lines.push(`Top ${verdict.topMissing.length} missing (no bridge + no HTML demo):`);
		for (const m of verdict.topMissing) lines.push(`- ${m}`);
	} else {
		lines.push("No M3 components are simultaneously missing from both the bridge and the HTML demo.");
	}
	lines.push("");
	lines.push("---");
	lines.push("");
	lines.push("_Audit produced by `scripts/m3-coverage-audit.ts`. Counts are syntactic_");
	lines.push("_(grep-based) and act as a regression lower bound, not a typechecker._");
	lines.push("");
	return lines.join("\n");
}

// ---------------------------------------------------------------------------
// JSON shape
// ---------------------------------------------------------------------------

interface JsonReport {
	$schema: string;
	generatedAt: string;
	tokenSurfaces: TokenSurfaceReport[];
	componentSurfaces: ComponentSurfaceReport[];
	catalogue: { covered: string[]; missing: string[] };
	html: HtmlCoverageReport;
	verdict: Verdict;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function ensureDir(path: string): void {
	if (!existsSync(path)) mkdirSync(path, { recursive: true });
}

function main(): void {
	let args: CliArgs;
	try {
		args = parseArgs(process.argv.slice(2));
	} catch (err) {
		process.stderr.write(`m3-coverage-audit: ${(err as Error).message}\n`);
		process.exit(2);
	}

	// Surface validation: refuse to silently produce an empty report.
	if (!existsSync(M3_TOKENS_DIR)) {
		process.stderr.write(`m3-coverage-audit: missing dir ${M3_TOKENS_DIR}\n`);
		process.exit(2);
	}
	if (!existsSync(SHADCN_BRIDGE_DIR)) {
		process.stderr.write(`m3-coverage-audit: missing dir ${SHADCN_BRIDGE_DIR}\n`);
		process.exit(2);
	}
	// statSync sanity check — confirms read permission too.
	try {
		statSync(M3_TOKENS_DIR);
		statSync(SHADCN_BRIDGE_DIR);
	} catch (err) {
		process.stderr.write(`m3-coverage-audit: stat failure: ${(err as Error).message}\n`);
		process.exit(2);
	}

	const tokens = auditTokenSurfaces();
	const components = auditComponentSurfaces();
	const catalogue = auditCatalogueCoverage(components);
	const html = auditHtmlCoverage();
	const verdict = computeVerdict(tokens, components, catalogue, html);

	if (args.json) {
		const json: JsonReport = {
			$schema: "https://aphrody-code.dev/schemas/m3-coverage-audit/v1",
			generatedAt: new Date().toISOString(),
			tokenSurfaces: tokens,
			componentSurfaces: components,
			catalogue: { covered: catalogue.covered, missing: catalogue.missing },
			html,
			verdict,
		};
		process.stdout.write(`${JSON.stringify(json, null, "\t")}\n`);
	} else {
		const md = renderMarkdown(tokens, components, catalogue, html, verdict);
		try {
			ensureDir(dirname(OUT_MARKDOWN));
			writeFileSync(OUT_MARKDOWN, md, "utf8");
		} catch (err) {
			process.stderr.write(`m3-coverage-audit: write failed: ${(err as Error).message}\n`);
			process.exit(2);
		}
		process.stdout.write(`m3-coverage-audit: wrote ${OUT_MARKDOWN}\n`);
		process.stdout.write(
			`  token=${verdict.tokenSurfacePct}%  bridge=${verdict.componentBridgePct}%  ` +
				`catalogue=${verdict.specCataloguePct}%  html=${verdict.htmlCataloguePct}%  ` +
				`overall=${verdict.overallPct}%\n`,
		);
	}

	if (args.check) {
		// Required surfaces: every surface present in M3_TOKEN_SURFACES that
		// has been promoted out of the "planned" set MUST exist on disk. We
		// codify the required set explicitly here so removing a file is a
		// CI-visible regression.
		const requiredTokens = new Set([
			"color",
			"typography",
			"elevation",
			"motion",
			"shape",
			"state",
		]);
		const tokenGap = tokens.filter((t) => requiredTokens.has(t.surface) && !t.present);
		const requiredComponents = new Set(Object.keys(SHADCN_TO_M3));
		const componentGap = components.filter(
			(c) => requiredComponents.has(c.file) && (!c.hasPropsStruct || !c.hasCreateFn),
		);
		if (tokenGap.length > 0 || componentGap.length > 0) {
			process.stderr.write("m3-coverage-audit --check: REGRESSION\n");
			for (const t of tokenGap) {
				process.stderr.write(`  missing token surface: ${t.surface} (${t.file})\n`);
			}
			for (const c of componentGap) {
				process.stderr.write(
					`  incomplete component: ${c.file} ` +
						`(props=${c.hasPropsStruct} create=${c.hasCreateFn})\n`,
				);
			}
			process.exit(1);
		}
		process.stdout.write("m3-coverage-audit --check: OK\n");
	}
}

main();
