// SPDX-License-Identifier: Apache-2.0
/**
 * skills-harvest-open-design.ts
 * =============================
 *
 * Harvests curated design + creative-direction SKILL.md files from
 * `C:/worktree/open-design/skills/<name>/SKILL.md` into aphrody's
 * `.claude/skills/<name>/SKILL.md` so the aphrody agent gains the
 * same design expertise that the open-design daemon exposes.
 *
 * Each imported SKILL.md:
 *   - Keeps the upstream `name`, multi-line `description`, `triggers[]`,
 *     and `od.*` keys verbatim.
 *   - Adds the aphrody-required `when_to_use:` field synthesised from
 *     `description` + `triggers` so Claude Code's skill loader honours
 *     it identically to first-party aphrody skills.
 *   - Appends an "## Upstream attribution" footer pointing at
 *     nexu-io/open-design + (when present) the canonical
 *     `od.upstream` repository.
 *
 * Aphrody first-party skills are NEVER overwritten: when the target
 * directory already exists under `.claude/skills/`, the import is
 * logged as `skipped` with reason `already-present`.
 *
 * Usage:
 *   bun run scripts/skills-harvest-open-design.ts            # 10 default
 *   bun run scripts/skills-harvest-open-design.ts --only=a,b # CSV subset
 *   bun run scripts/skills-harvest-open-design.ts --all      # every skill
 *   bun run scripts/skills-harvest-open-design.ts --source=<path>
 *
 * Exit codes:
 *   0  every requested skill was imported or already present (no errors)
 *   1  one or more skills failed to parse / write
 *   2  CLI / IO error (missing source, malformed flag, ...)
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

// ---------------------------------------------------------------------------
// Paths + defaults
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(import.meta.dir, "..");
const APHRODY_SKILLS_ROOT = join(REPO_ROOT, ".claude", "skills");
const AUDIT_OUT = join(REPO_ROOT, "docs", "audits", "skills-harvest.md");
const DEFAULT_SOURCE = "C:/worktree/open-design/skills";

/**
 * Default 10-skill allowlist. The original mission asked for `critique`;
 * the upstream catalogue does not ship a skill named `critique` (the
 * closest match is `design-review` under garrytan/gstack), so we wire
 * `design-review` in as the critique-equivalent and document the
 * substitution in the audit footer.
 */
const DEFAULT_TEN = [
	"apple-hig",
	"brand-guidelines",
	"color-expert",
	"design-review",
	"creative-director",
	"design-brief",
	"design-consultation",
	"doc",
	"competitive-ads-extractor",
	"design-md",
] as const;

/**
 * Aphrody first-party skills — refuse to overwrite. Anything under one
 * of these names is treated as a precious local fork.
 */
const APHRODY_RESERVED = new Set([
	"start",
	"a2a-duel-loop",
	"aphrody-perfect-grind",
	"aphrody-yolo-grind",
	"design-google-ingest",
	"vps-commander",
]);

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

interface CliOptions {
	only: string[] | null;
	all: boolean;
	source: string;
}

function parseCli(argv: readonly string[]): CliOptions {
	let only: string[] | null = null;
	let all = false;
	let source = DEFAULT_SOURCE;
	for (const arg of argv) {
		if (arg === "--all") {
			all = true;
		} else if (arg.startsWith("--only=")) {
			only = arg
				.slice("--only=".length)
				.split(",")
				.map((s) => s.trim())
				.filter(Boolean);
		} else if (arg.startsWith("--source=")) {
			source = arg.slice("--source=".length);
		} else if (arg === "--help" || arg === "-h") {
			process.stdout.write(
				"usage: bun run scripts/skills-harvest-open-design.ts [--only=a,b] [--all] [--source=PATH]\n",
			);
			process.exit(0);
		} else {
			process.stderr.write(`skills-harvest: unknown flag ${arg}\n`);
			process.exit(2);
		}
	}
	if (only && all) {
		process.stderr.write("skills-harvest: --only and --all are mutually exclusive\n");
		process.exit(2);
	}
	return { only, all, source };
}

// ---------------------------------------------------------------------------
// Frontmatter split + when_to_use synthesis
// ---------------------------------------------------------------------------

interface SplitDoc {
	rawFrontmatter: string;
	body: string;
}

/** Split a SKILL.md into raw frontmatter (sans `---` fences) + body. */
function splitFrontmatter(text: string): SplitDoc | null {
	if (!text.startsWith("---\n") && !text.startsWith("---\r\n")) {
		return null;
	}
	const after = text.replace(/^---\r?\n/, "");
	const end = after.search(/\r?\n---\r?\n/);
	if (end < 0) return null;
	const rawFrontmatter = after.slice(0, end);
	const body = after.slice(end).replace(/^\r?\n---\r?\n/, "");
	return { rawFrontmatter, body };
}

/** Lightweight key reader: top-level `key: scalar` (no nesting). */
function readScalar(rawFrontmatter: string, key: string): string | null {
	const re = new RegExp(`^${key}:\\s*(.+?)\\s*$`, "m");
	const m = re.exec(rawFrontmatter);
	return m ? m[1] : null;
}

/** Read a `key: |` block — returns the dedented body, joined with spaces. */
function readBlockScalar(rawFrontmatter: string, key: string): string | null {
	const re = new RegExp(`^${key}:\\s*\\|\\s*\\n((?:[ \\t]+.*\\n?)+)`, "m");
	const m = re.exec(rawFrontmatter);
	if (!m) return null;
	return m[1]
		.split(/\r?\n/)
		.map((l) => l.replace(/^[ \t]+/, ""))
		.filter(Boolean)
		.join(" ")
		.trim();
}

/** Read `triggers:` as a YAML-ish array of `- "..."` items. */
function readTriggers(rawFrontmatter: string): string[] {
	const re = /^triggers:\s*\n((?:[ \t]+-[^\n]*\n?)+)/m;
	const m = re.exec(rawFrontmatter);
	if (!m) return [];
	const out: string[] = [];
	for (const line of m[1].split(/\r?\n/)) {
		const item = /^\s*-\s*"?(.*?)"?\s*$/.exec(line);
		if (item && item[1]) out.push(item[1]);
	}
	return out;
}

/** Read `od.upstream:` from a nested `od:` block. */
function readOdUpstream(rawFrontmatter: string): string | null {
	const odBlock = /^od:\s*\n((?:[ \t]+.*\n?)+)/m.exec(rawFrontmatter);
	if (!odBlock) return null;
	const upstream = /^[ \t]+upstream:\s*"?(.+?)"?\s*$/m.exec(odBlock[1]);
	return upstream ? upstream[1] : null;
}

/** Read `od.mode:` from a nested `od:` block. */
function readOdMode(rawFrontmatter: string): string | null {
	const odBlock = /^od:\s*\n((?:[ \t]+.*\n?)+)/m.exec(rawFrontmatter);
	if (!odBlock) return null;
	const mode = /^[ \t]+mode:\s*"?(.+?)"?\s*$/m.exec(odBlock[1]);
	return mode ? mode[1] : null;
}

/**
 * Build the aphrody `when_to_use:` line from `description` + `triggers`.
 *
 * Mirrors the convention already in aphrody first-party skills, e.g.
 *   when_to_use: User types "/foo", says "bar", "baz", or asks ...
 */
function synthWhenToUse(name: string, description: string, triggers: string[]): string {
	const triggerQuotes = triggers.length > 0
		? triggers.map((t) => `"${t}"`).join(", ")
		: `"/${name}"`;
	const summary = description.replace(/\s+/g, " ").trim();
	return `User types "/${name}", says ${triggerQuotes}, or asks for: ${summary}`;
}

// ---------------------------------------------------------------------------
// Per-skill harvest
// ---------------------------------------------------------------------------

interface HarvestRecord {
	name: string;
	status: "imported" | "skipped" | "failed";
	reason?: string;
	bytes?: number;
	triggers?: number;
	odMode?: string | null;
}

function harvestOne(name: string, sourceRoot: string): HarvestRecord {
	const srcPath = join(sourceRoot, name, "SKILL.md");
	if (!existsSync(srcPath)) {
		return { name, status: "failed", reason: `no SKILL.md at ${srcPath}` };
	}
	const destDir = join(APHRODY_SKILLS_ROOT, name);
	const destPath = join(destDir, "SKILL.md");
	if (APHRODY_RESERVED.has(name)) {
		return { name, status: "skipped", reason: "aphrody-reserved" };
	}
	if (existsSync(destPath)) {
		return { name, status: "skipped", reason: "already-present" };
	}
	const raw = readFileSync(srcPath, "utf8");
	const split = splitFrontmatter(raw);
	if (!split) {
		return { name, status: "failed", reason: "no YAML frontmatter" };
	}
	const upstreamName = readScalar(split.rawFrontmatter, "name") ?? name;
	const description = readBlockScalar(split.rawFrontmatter, "description")
		?? readScalar(split.rawFrontmatter, "description")
		?? `Imported open-design skill ${name}`;
	const triggers = readTriggers(split.rawFrontmatter);
	const odUpstream = readOdUpstream(split.rawFrontmatter);
	const odMode = readOdMode(split.rawFrontmatter);
	const whenToUse = synthWhenToUse(upstreamName, description, triggers);

	// Preserve the upstream frontmatter verbatim, just append `when_to_use:`
	// at the end so the aphrody Claude Code loader picks it up. The
	// open-design daemon's YAML parser already tolerates extra keys.
	const augmentedFrontmatter = `${split.rawFrontmatter.trimEnd()}\nwhen_to_use: ${JSON.stringify(whenToUse)}\n`;

	const attribution = [
		"",
		"## Upstream attribution",
		"",
		`- Harvested from \`C:/worktree/open-design/skills/${name}/SKILL.md\``,
		"  (nexu-io/open-design, fork of google-labs-code/skills + community contributions).",
	];
	if (odUpstream) {
		attribution.push(`- Canonical upstream: ${odUpstream}`);
	}
	attribution.push(
		"- Re-imported by `scripts/skills-harvest-open-design.ts`.",
		"  Do not edit by hand — re-run the harvester to refresh.",
		"",
	);

	const rebuilt = `---\n${augmentedFrontmatter}---\n${split.body.replace(/\s+$/, "")}\n${attribution.join("\n")}`;
	mkdirSync(destDir, { recursive: true });
	writeFileSync(destPath, rebuilt, "utf8");
	const stat = statSync(destPath);
	return {
		name,
		status: "imported",
		bytes: stat.size,
		triggers: triggers.length,
		odMode,
	};
}

// ---------------------------------------------------------------------------
// Audit emitter
// ---------------------------------------------------------------------------

function writeAudit(records: HarvestRecord[], sourceRoot: string): void {
	const imported = records.filter((r) => r.status === "imported");
	const skipped = records.filter((r) => r.status === "skipped");
	const failed = records.filter((r) => r.status === "failed");
	const modeDist = new Map<string, number>();
	for (const r of imported) {
		const k = r.odMode ?? "(none)";
		modeDist.set(k, (modeDist.get(k) ?? 0) + 1);
	}
	const lines: string[] = [
		"<!-- SPDX-License-Identifier: Apache-2.0 -->",
		"",
		"# Audit — open-design skill harvest",
		"",
		`**Date (UTC):** ${new Date().toISOString()}`,
		`**Source root:** \`${sourceRoot}\``,
		`**Destination root:** \`.claude/skills/\``,
		`**Imported:** ${imported.length}`,
		`**Skipped:** ${skipped.length}`,
		`**Failed:** ${failed.length}`,
		"",
		"## Imported",
		"",
		"| Skill | Bytes | Triggers | od.mode |",
		"|---|---:|---:|---|",
	];
	for (const r of imported) {
		lines.push(`| ${r.name} | ${r.bytes ?? 0} | ${r.triggers ?? 0} | ${r.odMode ?? "(none)"} |`);
	}
	lines.push("", "## Skipped", "", "| Skill | Reason |", "|---|---|");
	for (const r of skipped) {
		lines.push(`| ${r.name} | ${r.reason ?? ""} |`);
	}
	if (failed.length > 0) {
		lines.push("", "## Failed", "", "| Skill | Reason |", "|---|---|");
		for (const r of failed) {
			lines.push(`| ${r.name} | ${r.reason ?? ""} |`);
		}
	}
	lines.push("", "## od.mode distribution", "", "| mode | count |", "|---|---:|");
	for (const [mode, count] of [...modeDist.entries()].sort()) {
		lines.push(`| ${mode} | ${count} |`);
	}
	lines.push(
		"",
		"## Notes",
		"",
		"- The original 10-skill allowlist asked for `critique`; the",
		"  upstream open-design catalogue ships no skill by that name.",
		"  We substitute `design-review` (garrytan/gstack) which delivers",
		"  the same critique-before-launch capability.",
		"- Re-run with `--all` to mirror every open-design skill into",
		"  `.claude/skills/` (133 entries at last count).",
		"",
	);
	mkdirSync(dirname(AUDIT_OUT), { recursive: true });
	writeFileSync(AUDIT_OUT, lines.join("\n"), "utf8");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main(): void {
	const opts = parseCli(process.argv.slice(2));
	if (!existsSync(opts.source)) {
		process.stderr.write(`skills-harvest: source root not found: ${opts.source}\n`);
		process.exit(2);
	}
	let requested: string[];
	if (opts.all) {
		requested = readdirSync(opts.source).filter((entry) => {
			const skillPath = join(opts.source, entry, "SKILL.md");
			return existsSync(skillPath);
		});
	} else if (opts.only) {
		requested = opts.only;
	} else {
		requested = [...DEFAULT_TEN];
	}
	const records: HarvestRecord[] = [];
	for (const name of requested) {
		records.push(harvestOne(name, opts.source));
	}
	writeAudit(records, opts.source);
	const imported = records.filter((r) => r.status === "imported").length;
	const skipped = records.filter((r) => r.status === "skipped").length;
	const failed = records.filter((r) => r.status === "failed").length;
	process.stdout.write(
		`skills-harvest: imported=${imported} skipped=${skipped} failed=${failed} -> ${AUDIT_OUT}\n`,
	);
	if (failed > 0) process.exit(1);
}

main();
