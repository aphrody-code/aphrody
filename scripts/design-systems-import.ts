// SPDX-License-Identifier: Apache-2.0
// #!/usr/bin/env bun
/**
 * design-systems-import.ts
 * ========================
 *
 * Lazy-pull the per-brand DESIGN.md catalogue from
 * `C:/worktree/open-design/design-systems/<slug>/DESIGN.md` into the
 * aphrody repo under `assets/design-systems/<slug>/`. Each imported
 * DESIGN.md is linted via `bun x @google/design.md lint` and only
 * accepted when `errors == 0`. Siblings (`tokens.json`, `tokens.css`,
 * `components.html`, `preview.png`, `README.md`) are copied verbatim
 * when present.
 *
 * A `manifest.json` is maintained at the destination root so any agent
 * can ask "do you have brand=airbnb?" with a single file read.
 *
 * Usage:
 *   bun run scripts/design-systems-import.ts                  # all 152
 *   bun run scripts/design-systems-import.ts --top=10         # top 10
 *   bun run scripts/design-systems-import.ts --only=airbnb,apple
 *   bun run scripts/design-systems-import.ts --check          # CI gate
 *   bun run scripts/design-systems-import.ts --source=<path>  # override
 */

import { spawnSync } from "node:child_process";
import {
	copyFileSync,
	existsSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { basename, join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_SOURCE = "C:/worktree/open-design/design-systems";
const DEST_ROOT = join(REPO_ROOT, "assets", "design-systems");
const MANIFEST_PATH = join(DEST_ROOT, "manifest.json");
const AUDIT_PATH = join(REPO_ROOT, "docs", "audits", "design-systems-import.md");

// Curated priority list — see DESIGN.md & task spec.
const TOP_PRIORITY: readonly string[] = [
	"claude",
	"apple",
	"cohere",
	"coinbase",
	"ant",
	"airbnb",
	"canva",
	"brutalism",
	"bento",
	"minimal",
] as const;

// Slugs to skip: `_schema` is the open-design schema folder, `README.md`
// is a top-level file (not a slug). Anything starting with `_` or `.`.
function isImportableSlug(slug: string): boolean {
	if (slug.startsWith("_") || slug.startsWith(".")) return false;
	return true;
}

const SIBLING_FILES = [
	"tokens.json",
	"tokens.css",
	"components.html",
	"preview.png",
	"README.md",
] as const;

interface Args {
	sourceDir: string;
	only: readonly string[] | null;
	top: number | null;
	check: boolean;
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = {
		sourceDir: DEFAULT_SOURCE,
		only: null,
		top: null,
		check: false,
	};
	for (const raw of argv) {
		if (raw.startsWith("--source=")) {
			out.sourceDir = raw.slice("--source=".length);
		} else if (raw.startsWith("--only=")) {
			const slugs = raw
				.slice("--only=".length)
				.split(",")
				.map((s) => s.trim())
				.filter((s) => s.length > 0);
			if (slugs.length === 0) {
				throw new Error("--only= must contain at least one slug");
			}
			out.only = slugs;
		} else if (raw.startsWith("--top=")) {
			const n = Number(raw.slice("--top=".length));
			if (!Number.isInteger(n) || n < 1 || n > 200) {
				throw new Error(`invalid --top=${raw} (1..200)`);
			}
			out.top = n;
		} else if (raw === "--check") {
			out.check = true;
		} else if (raw === "--help" || raw === "-h") {
			console.log(
				"Usage: bun run scripts/design-systems-import.ts " +
					"[--source=<path>] [--only=a,b,c] [--top=N] [--check]",
			);
			process.exit(0);
		} else {
			throw new Error(`unknown arg: ${raw}`);
		}
	}
	return out;
}

function listSourceSlugs(sourceDir: string): string[] {
	if (!existsSync(sourceDir)) {
		throw new Error(`source dir not found: ${sourceDir}`);
	}
	const entries = readdirSync(sourceDir, { withFileTypes: true });
	const slugs: string[] = [];
	for (const e of entries) {
		if (!e.isDirectory()) continue;
		if (!isImportableSlug(e.name)) continue;
		const designMd = join(sourceDir, e.name, "DESIGN.md");
		if (!existsSync(designMd)) continue;
		slugs.push(e.name);
	}
	slugs.sort();
	return slugs;
}

interface LintSummary {
	errors: number;
	warnings: number;
	infos: number;
}

function lintDesignMd(path: string): LintSummary {
	const res = spawnSync("bun", ["x", "@google/design.md", "lint", path], {
		encoding: "utf8",
		shell: true,
	});
	const stdout = res.stdout ?? "";
	// Linter prints a JSON object with `summary`. Be defensive: extract
	// the last valid JSON object from stdout.
	const lastBrace = stdout.lastIndexOf("}");
	const firstBrace = stdout.indexOf("{");
	if (firstBrace < 0 || lastBrace < firstBrace) {
		throw new Error(`lint produced no JSON for ${path}: ${stdout.slice(0, 200)}`);
	}
	const json = stdout.slice(firstBrace, lastBrace + 1);
	const parsed = JSON.parse(json) as { summary?: LintSummary };
	if (!parsed.summary) {
		throw new Error(`lint missing .summary for ${path}`);
	}
	return {
		errors: parsed.summary.errors | 0,
		warnings: parsed.summary.warnings | 0,
		infos: parsed.summary.infos | 0,
	};
}

interface ImportResult {
	slug: string;
	name: string;
	description: string;
	source: "open-design";
	imported_at: string;
	size_bytes: number;
	siblings: string[];
	lint: LintSummary;
	status: "imported" | "rejected" | "missing";
	reason?: string;
}

// Extract a friendly name + first description line from a DESIGN.md.
// Open-design files start with `# Design System Inspired by <Name>` and a
// `> Category: ... / <short description>` blockquote line.
function extractMeta(md: string, slug: string): { name: string; description: string } {
	const lines = md.split(/\r?\n/);
	let name = slug;
	let description = "";
	for (const line of lines) {
		if (!name || name === slug) {
			const m = line.match(/^#\s+(?:Design System (?:Inspired by|for)\s+)?(.+?)\s*$/);
			if (m?.[1]) {
				name = m[1];
			}
		}
		if (!description) {
			const blockquote = line.match(/^>\s*(.+)$/);
			if (blockquote?.[1] && !blockquote[1].toLowerCase().startsWith("category:")) {
				description = blockquote[1].trim();
			}
		}
		if (name !== slug && description) break;
	}
	if (!description) {
		// Fallback: first non-empty, non-heading, non-blockquote line.
		for (const line of lines) {
			const t = line.trim();
			if (t.length === 0 || t.startsWith("#") || t.startsWith(">")) continue;
			description = t.length > 240 ? `${t.slice(0, 237)}...` : t;
			break;
		}
	}
	return { name, description };
}

function importOne(slug: string, sourceDir: string): ImportResult {
	const srcDir = join(sourceDir, slug);
	const srcDesign = join(srcDir, "DESIGN.md");
	const destDir = join(DEST_ROOT, slug);
	const destDesign = join(destDir, "DESIGN.md");
	const importedAt = new Date().toISOString();

	if (!existsSync(srcDesign)) {
		return {
			slug,
			name: slug,
			description: "",
			source: "open-design",
			imported_at: importedAt,
			size_bytes: 0,
			siblings: [],
			lint: { errors: 0, warnings: 0, infos: 0 },
			status: "missing",
			reason: `DESIGN.md not found at ${srcDesign}`,
		};
	}

	const lint = lintDesignMd(srcDesign);
	if (lint.errors > 0) {
		return {
			slug,
			name: slug,
			description: "",
			source: "open-design",
			imported_at: importedAt,
			size_bytes: 0,
			siblings: [],
			lint,
			status: "rejected",
			reason: `lint.errors=${lint.errors}`,
		};
	}

	mkdirSync(destDir, { recursive: true });
	copyFileSync(srcDesign, destDesign);
	const md = readFileSync(srcDesign, "utf8");
	const { name, description } = extractMeta(md, slug);
	const sizeBytes = statSync(destDesign).size;

	const siblings: string[] = [];
	for (const sib of SIBLING_FILES) {
		const srcSib = join(srcDir, sib);
		if (existsSync(srcSib) && statSync(srcSib).isFile()) {
			copyFileSync(srcSib, join(destDir, sib));
			siblings.push(sib);
		}
	}

	return {
		slug,
		name,
		description,
		source: "open-design",
		imported_at: importedAt,
		size_bytes: sizeBytes,
		siblings,
		lint,
		status: "imported",
	};
}

interface Manifest {
	$schema: string;
	generated_at: string;
	source: string;
	count: number;
	entries: Record<string, ImportResult>;
}

function loadManifest(): Manifest {
	if (existsSync(MANIFEST_PATH)) {
		try {
			const parsed = JSON.parse(readFileSync(MANIFEST_PATH, "utf8")) as Manifest;
			if (parsed.entries && typeof parsed.entries === "object") {
				return parsed;
			}
		} catch {
			// fall through to fresh manifest
		}
	}
	return {
		$schema: "https://aphrody-code.dev/schemas/design-systems/v1",
		generated_at: new Date().toISOString(),
		source: "open-design",
		count: 0,
		entries: {},
	};
}

function saveManifest(m: Manifest): void {
	m.generated_at = new Date().toISOString();
	m.count = Object.values(m.entries).filter((e) => e.status === "imported").length;
	mkdirSync(DEST_ROOT, { recursive: true });
	writeFileSync(MANIFEST_PATH, `${JSON.stringify(m, null, "\t")}\n`, "utf8");
}

function writeAudit(results: readonly ImportResult[], args: Args, available: readonly string[]): void {
	const imported = results.filter((r) => r.status === "imported");
	const rejected = results.filter((r) => r.status === "rejected");
	const missing = results.filter((r) => r.status === "missing");
	const totalErrors = results.reduce((a, r) => a + r.lint.errors, 0);
	const totalWarnings = results.reduce((a, r) => a + r.lint.warnings, 0);
	const totalInfos = results.reduce((a, r) => a + r.lint.infos, 0);
	const totalBytes = imported.reduce((a, r) => a + r.size_bytes, 0);

	const lines: string[] = [];
	lines.push("<!-- SPDX-License-Identifier: Apache-2.0 -->");
	lines.push("# Design-Systems Import Audit");
	lines.push("");
	lines.push(`- Generated: \`${new Date().toISOString()}\``);
	lines.push(`- Source: \`${args.sourceDir}\``);
	lines.push(`- Destination: \`assets/design-systems/\``);
	lines.push(`- Source slugs available: **${available.length}**`);
	lines.push(`- Processed: **${results.length}**`);
	lines.push(`- Imported: **${imported.length}**`);
	lines.push(`- Rejected (lint.errors > 0): **${rejected.length}**`);
	lines.push(`- Missing DESIGN.md: **${missing.length}**`);
	lines.push(`- Total bytes copied: **${totalBytes}**`);
	lines.push(
		`- Lint aggregate: errors=${totalErrors}, warnings=${totalWarnings}, infos=${totalInfos}`,
	);
	lines.push("");
	lines.push("## Imported");
	lines.push("");
	lines.push("| Slug | Name | Bytes | Siblings | Lint (E/W/I) |");
	lines.push("|------|------|-------|----------|--------------|");
	for (const r of imported) {
		const sib = r.siblings.length === 0 ? "-" : r.siblings.join(", ");
		lines.push(
			`| \`${r.slug}\` | ${r.name} | ${r.size_bytes} | ${sib} | ${r.lint.errors}/${r.lint.warnings}/${r.lint.infos} |`,
		);
	}
	if (rejected.length > 0) {
		lines.push("");
		lines.push("## Rejected");
		lines.push("");
		for (const r of rejected) {
			lines.push(`- \`${r.slug}\` — ${r.reason ?? "unknown"}`);
		}
	}
	if (missing.length > 0) {
		lines.push("");
		lines.push("## Missing");
		lines.push("");
		for (const r of missing) {
			lines.push(`- \`${r.slug}\` — ${r.reason ?? "unknown"}`);
		}
	}
	lines.push("");
	mkdirSync(resolve(AUDIT_PATH, ".."), { recursive: true });
	writeFileSync(AUDIT_PATH, `${lines.join("\n")}`, "utf8");
}

function main(): void {
	const args = parseArgs(process.argv.slice(2));
	const available = listSourceSlugs(args.sourceDir);

	if (args.check) {
		const missing = available.filter(
			(slug) => !existsSync(join(DEST_ROOT, slug, "DESIGN.md")),
		);
		if (missing.length > 0) {
			console.error(
				`[design-systems-import] CHECK FAIL — ${missing.length} slug(s) absent locally:`,
			);
			for (const s of missing) console.error(`  - ${s}`);
			process.exit(1);
		}
		console.log(
			`[design-systems-import] CHECK OK — all ${available.length} source slugs present.`,
		);
		return;
	}

	let targets: string[];
	if (args.only) {
		targets = args.only.filter((s) => available.includes(s));
		const unknown = args.only.filter((s) => !available.includes(s));
		if (unknown.length > 0) {
			console.warn(`[design-systems-import] unknown slugs ignored: ${unknown.join(", ")}`);
		}
	} else if (args.top) {
		targets = TOP_PRIORITY.slice(0, args.top).filter((s) => available.includes(s));
	} else {
		targets = available;
	}

	if (targets.length === 0) {
		throw new Error("no slugs selected to import");
	}

	console.log(
		`[design-systems-import] source=${args.sourceDir} targets=${targets.length}/${available.length}`,
	);

	const manifest = loadManifest();
	const results: ImportResult[] = [];
	for (const slug of targets) {
		const r = importOne(slug, args.sourceDir);
		results.push(r);
		if (r.status === "imported") {
			manifest.entries[slug] = r;
			console.log(
				`[design-systems-import] ok    ${slug.padEnd(20)} ${r.size_bytes}B  siblings=${r.siblings.length}  lint=${r.lint.errors}/${r.lint.warnings}/${r.lint.infos}`,
			);
		} else if (r.status === "rejected") {
			console.warn(`[design-systems-import] rej   ${slug} — ${r.reason}`);
		} else {
			console.warn(`[design-systems-import] miss  ${slug} — ${r.reason}`);
		}
	}

	saveManifest(manifest);
	writeAudit(results, args, available);

	const imported = results.filter((r) => r.status === "imported").length;
	const rejected = results.filter((r) => r.status === "rejected").length;
	console.log(
		`[design-systems-import] done  imported=${imported}  rejected=${rejected}  manifest=${MANIFEST_PATH}`,
	);
	console.log(`[design-systems-import] audit=${AUDIT_PATH}`);

	if (rejected > 0 && imported === 0) {
		process.exit(2);
	}
}

try {
	main();
} catch (err) {
	console.error(
		"[design-systems-import] FAILED:",
		err instanceof Error ? err.message : String(err),
	);
	process.exit(1);
}

// keep tooling-quiet shim — basename used during dev inspection
void basename;
