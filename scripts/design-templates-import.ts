// SPDX-License-Identifier: Apache-2.0
// #!/usr/bin/env bun
/**
 * design-templates-import.ts
 * ==========================
 *
 * Lazy-pull the per-template `SKILL.md` catalogue from
 * `C:/worktree/open-design/design-templates/<name>/SKILL.md` into the
 * aphrody repo under `assets/design-templates/<name>/`. SKILL.md is
 * copied verbatim (open-design schema is authoritative). Sibling files
 * (`example.html`, `template.html`, `styles.css`, `preview.png`,
 * `README.md`, `template.json`, `LICENSE`) are copied when present, and
 * the `references/` subdirectory is mirrored recursively.
 *
 * A `manifest.json` is maintained at the destination root so any agent
 * can ask "do you have template=dashboard?" with a single file read.
 *
 * Usage:
 *   bun run scripts/design-templates-import.ts                  # all
 *   bun run scripts/design-templates-import.ts --top=10         # top 10
 *   bun run scripts/design-templates-import.ts --only=dashboard,blog-post
 *   bun run scripts/design-templates-import.ts --check          # CI gate
 *   bun run scripts/design-templates-import.ts --source=<path>  # override
 */

import {
	copyFileSync,
	existsSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { basename, join, relative, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_SOURCE = "C:/worktree/open-design/design-templates";
const DEST_ROOT = join(REPO_ROOT, "assets", "design-templates");
const MANIFEST_PATH = join(DEST_ROOT, "manifest.json");
const AUDIT_PATH = join(REPO_ROOT, "docs", "audits", "design-templates-import.md");

// Curated priority list — see task spec. Slugs absent from the source
// dir are warned and skipped (kept verbatim for spec fidelity).
const TOP_PRIORITY: readonly string[] = [
	"dashboard",
	"blog-post",
	"docs-page",
	"email-marketing",
	"github-dashboard",
	"finance-report",
	"eng-runbook",
	"design-brief",
	"html-ppt",
	"audio-jingle",
] as const;

// Slugs to skip: anything starting with `_` or `.`, plus the top-level
// `AGENTS.md` (not a slug — open-design index file).
function isImportableSlug(slug: string): boolean {
	if (slug.startsWith("_") || slug.startsWith(".")) return false;
	if (slug === "AGENTS.md") return false;
	return true;
}

// Sibling files copied verbatim when present at the slug root.
const SIBLING_FILES = [
	"example.html",
	"template.html",
	"index.html",
	"styles.css",
	"preview.png",
	"README.md",
	"template.json",
	"LICENSE",
] as const;

// Sibling directories mirrored recursively when present.
const SIBLING_DIRS = ["references"] as const;

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
				"Usage: bun run scripts/design-templates-import.ts " +
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
		const skillMd = join(sourceDir, e.name, "SKILL.md");
		if (!existsSync(skillMd)) continue;
		slugs.push(e.name);
	}
	slugs.sort();
	return slugs;
}

// Mirror `srcDir` into `destDir` recursively. Returns the list of file
// paths copied, expressed relative to `destDir`.
function copyDirRecursive(srcDir: string, destDir: string): string[] {
	const copied: string[] = [];
	mkdirSync(destDir, { recursive: true });
	const stack: string[] = [srcDir];
	while (stack.length > 0) {
		const cur = stack.pop();
		if (!cur) break;
		for (const entry of readdirSync(cur, { withFileTypes: true })) {
			const srcPath = join(cur, entry.name);
			const rel = relative(srcDir, srcPath);
			const dstPath = join(destDir, rel);
			if (entry.isDirectory()) {
				mkdirSync(dstPath, { recursive: true });
				stack.push(srcPath);
			} else if (entry.isFile()) {
				mkdirSync(resolve(dstPath, ".."), { recursive: true });
				copyFileSync(srcPath, dstPath);
				copied.push(rel.split(/[\\/]/).join("/"));
			}
		}
	}
	return copied;
}

interface FrontMatter {
	name: string;
	description: string;
	mode: string;
	platform: string;
	scenario: string;
}

// Extract a small subset of the YAML front-matter without a full parser.
// We only need `name`, `description` (first line of block scalar),
// `od.mode`, `od.platform`, `od.scenario`.
function extractFrontMatter(md: string, slug: string): FrontMatter {
	const out: FrontMatter = {
		name: slug,
		description: "",
		mode: "unknown",
		platform: "unknown",
		scenario: "unknown",
	};
	const lines = md.split(/\r?\n/);
	if (lines[0]?.trim() !== "---") return out;
	let i = 1;
	let inOd = false;
	let inDescription = false;
	while (i < lines.length) {
		const line = lines[i] ?? "";
		if (line.trim() === "---") break;
		// Top-level scalars
		const nameMatch = line.match(/^name:\s*(.+?)\s*$/);
		if (nameMatch?.[1]) out.name = nameMatch[1];
		const descInline = line.match(/^description:\s*(.+?)\s*$/);
		if (descInline?.[1] && descInline[1] !== "|" && descInline[1] !== ">") {
			out.description = descInline[1].replace(/^["']|["']$/g, "");
		} else if (/^description:\s*[|>]\s*$/.test(line)) {
			inDescription = true;
			i++;
			continue;
		}
		if (inDescription) {
			if (/^\s{2,}\S/.test(line)) {
				const piece = line.replace(/^\s+/, "").trim();
				if (!out.description && piece.length > 0) out.description = piece;
			} else if (line.trim().length > 0 && !line.startsWith(" ")) {
				inDescription = false;
			}
		}
		if (/^od:\s*$/.test(line)) {
			inOd = true;
			i++;
			continue;
		}
		if (inOd) {
			if (/^\S/.test(line)) {
				inOd = false;
			} else {
				const m = line.match(/^\s{2}(\w+):\s*(.+?)\s*$/);
				if (m) {
					const [, key, val] = m;
					const clean = (val ?? "").replace(/^["']|["']$/g, "");
					if (key === "mode") out.mode = clean;
					else if (key === "platform") out.platform = clean;
					else if (key === "scenario") out.scenario = clean;
				}
			}
		}
		i++;
	}
	if (out.description.length > 240) {
		out.description = `${out.description.slice(0, 237)}...`;
	}
	return out;
}

interface ImportResult {
	slug: string;
	name: string;
	description: string;
	mode: string;
	platform: string;
	scenario: string;
	surface: "skill+assets";
	source: "open-design";
	imported_at: string;
	size_bytes: number;
	siblings: string[];
	status: "imported" | "missing";
	reason?: string;
}

function dirSize(dir: string): number {
	let total = 0;
	const stack: string[] = [dir];
	while (stack.length > 0) {
		const cur = stack.pop();
		if (!cur) break;
		for (const entry of readdirSync(cur, { withFileTypes: true })) {
			const p = join(cur, entry.name);
			if (entry.isDirectory()) stack.push(p);
			else if (entry.isFile()) total += statSync(p).size;
		}
	}
	return total;
}

function importOne(slug: string, sourceDir: string): ImportResult {
	const srcDir = join(sourceDir, slug);
	const srcSkill = join(srcDir, "SKILL.md");
	const destDir = join(DEST_ROOT, slug);
	const destSkill = join(destDir, "SKILL.md");
	const importedAt = new Date().toISOString();

	if (!existsSync(srcSkill)) {
		return {
			slug,
			name: slug,
			description: "",
			mode: "unknown",
			platform: "unknown",
			scenario: "unknown",
			surface: "skill+assets",
			source: "open-design",
			imported_at: importedAt,
			size_bytes: 0,
			siblings: [],
			status: "missing",
			reason: `SKILL.md not found at ${srcSkill}`,
		};
	}

	mkdirSync(destDir, { recursive: true });
	copyFileSync(srcSkill, destSkill);
	const md = readFileSync(srcSkill, "utf8");
	const fm = extractFrontMatter(md, slug);

	const siblings: string[] = ["SKILL.md"];
	for (const sib of SIBLING_FILES) {
		const srcSib = join(srcDir, sib);
		if (existsSync(srcSib) && statSync(srcSib).isFile()) {
			copyFileSync(srcSib, join(destDir, sib));
			siblings.push(sib);
		}
	}
	for (const sibDir of SIBLING_DIRS) {
		const srcSibDir = join(srcDir, sibDir);
		if (existsSync(srcSibDir) && statSync(srcSibDir).isDirectory()) {
			const copied = copyDirRecursive(srcSibDir, join(destDir, sibDir));
			for (const rel of copied) siblings.push(`${sibDir}/${rel}`);
		}
	}

	return {
		slug,
		name: fm.name,
		description: fm.description,
		mode: fm.mode,
		platform: fm.platform,
		scenario: fm.scenario,
		surface: "skill+assets",
		source: "open-design",
		imported_at: importedAt,
		size_bytes: dirSize(destDir),
		siblings,
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
		$schema: "https://aphrody-code.dev/schemas/design-templates/v1",
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
	const missing = results.filter((r) => r.status === "missing");
	const totalBytes = imported.reduce((a, r) => a + r.size_bytes, 0);

	// od.mode distribution
	const modeCounts = new Map<string, number>();
	for (const r of imported) {
		modeCounts.set(r.mode, (modeCounts.get(r.mode) ?? 0) + 1);
	}
	const modeRow = [...modeCounts.entries()]
		.sort((a, b) => b[1] - a[1])
		.map(([m, c]) => `${m}=${c}`)
		.join("  ");

	const lines: string[] = [];
	lines.push("<!-- SPDX-License-Identifier: Apache-2.0 -->");
	lines.push("# Design-Templates Import Audit");
	lines.push("");
	lines.push(`- Generated: \`${new Date().toISOString()}\``);
	lines.push(`- Source: \`${args.sourceDir}\``);
	lines.push(`- Destination: \`assets/design-templates/\``);
	lines.push(`- Source slugs available: **${available.length}**`);
	lines.push(`- Processed: **${results.length}**`);
	lines.push(`- Imported: **${imported.length}**`);
	lines.push(`- Missing SKILL.md: **${missing.length}**`);
	lines.push(`- Total bytes copied: **${totalBytes}**`);
	lines.push("");
	lines.push("## od.mode distribution");
	lines.push("");
	lines.push(`\`${modeRow || "(empty)"}\``);
	lines.push("");
	lines.push("## Imported");
	lines.push("");
	lines.push("| Slug | Name | Mode | Platform | Scenario | Bytes | Siblings |");
	lines.push("|------|------|------|----------|----------|-------|----------|");
	for (const r of imported) {
		const sib = r.siblings.length === 0 ? "-" : String(r.siblings.length);
		lines.push(
			`| \`${r.slug}\` | ${r.name} | ${r.mode} | ${r.platform} | ${r.scenario} | ${r.size_bytes} | ${sib} |`,
		);
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
			(slug) => !existsSync(join(DEST_ROOT, slug, "SKILL.md")),
		);
		if (missing.length > 0) {
			console.error(
				`[design-templates-import] CHECK FAIL — ${missing.length} slug(s) absent locally:`,
			);
			for (const s of missing) console.error(`  - ${s}`);
			process.exit(1);
		}
		console.log(
			`[design-templates-import] CHECK OK — all ${available.length} source slugs present.`,
		);
		return;
	}

	let targets: string[];
	if (args.only) {
		targets = args.only.filter((s) => available.includes(s));
		const unknown = args.only.filter((s) => !available.includes(s));
		if (unknown.length > 0) {
			console.warn(
				`[design-templates-import] unknown slugs ignored: ${unknown.join(", ")}`,
			);
		}
	} else if (args.top) {
		const wanted = TOP_PRIORITY.slice(0, args.top);
		targets = wanted.filter((s) => available.includes(s));
		const unknown = wanted.filter((s) => !available.includes(s));
		if (unknown.length > 0) {
			console.warn(
				`[design-templates-import] top-priority slugs absent from source: ${unknown.join(", ")}`,
			);
		}
	} else {
		targets = available;
	}

	if (targets.length === 0) {
		throw new Error("no slugs selected to import");
	}

	console.log(
		`[design-templates-import] source=${args.sourceDir} targets=${targets.length}/${available.length}`,
	);

	const manifest = loadManifest();
	const results: ImportResult[] = [];
	for (const slug of targets) {
		const r = importOne(slug, args.sourceDir);
		results.push(r);
		if (r.status === "imported") {
			manifest.entries[slug] = r;
			console.log(
				`[design-templates-import] ok    ${slug.padEnd(28)} ${String(r.size_bytes).padStart(7)}B  mode=${r.mode.padEnd(10)} siblings=${r.siblings.length}`,
			);
		} else {
			console.warn(`[design-templates-import] miss  ${slug} — ${r.reason}`);
		}
	}

	saveManifest(manifest);
	writeAudit(results, args, available);

	const imported = results.filter((r) => r.status === "imported").length;
	const missing = results.filter((r) => r.status === "missing").length;
	console.log(
		`[design-templates-import] done  imported=${imported}  missing=${missing}  manifest=${MANIFEST_PATH}`,
	);
	console.log(`[design-templates-import] audit=${AUDIT_PATH}`);
}

try {
	main();
} catch (err) {
	console.error(
		"[design-templates-import] FAILED:",
		err instanceof Error ? err.message : String(err),
	);
	process.exit(1);
}

// keep tooling-quiet shim — basename used during dev inspection
void basename;
