// SPDX-License-Identifier: Apache-2.0
/**
 * skill-schema-align.ts
 * =====================
 *
 * Bridges aphrody's `.claude/skills/<name>/SKILL.md` files (Anthropic
 * skill-loader schema: `name` + `description` + `when_to_use`) to the
 * open-design daemon's schema (`name` + multi-line `description` +
 * `triggers[]` + `od.*` block) by emitting a companion `.od.yaml`
 * sidecar next to every existing SKILL.md. The original SKILL.md is
 * NEVER modified — Claude Code keeps loading the skill from SKILL.md;
 * the open-design daemon (which scans the same directory) now also
 * resolves the skill via `.od.yaml`.
 *
 * Trigger extraction: open-design expects authored phrases in
 * `triggers:`. Aphrody's `when_to_use` field already enumerates the
 * phrases in double quotes (`User types "/start", says "start", "go", ...`).
 * We extract every `"..."` substring and use those as triggers, in source
 * order, deduplicated case-insensitively. When `when_to_use` has zero
 * quoted phrases we synthesise one trigger from the skill name (`/<name>`)
 * so the skill still has at least one usable phrase.
 *
 * Usage:
 *   bun run scripts/skill-schema-align.ts          # emit .od.yaml + audit
 *   bun run scripts/skill-schema-align.ts --check  # exit 1 if any skill failed to align
 *
 * Exit codes:
 *   0  every skill emitted a .od.yaml AND --check (when passed) found no failures
 *   1  --check found one or more skills with `status == failed`
 *   2  CLI / IO error
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(import.meta.dir, "..");
const SKILLS_ROOT = join(REPO_ROOT, ".claude", "skills");
const AUDIT_OUT = join(REPO_ROOT, "docs", "audits", "skill-schema-alignment.md");

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Frontmatter shape we expect inside aphrody SKILL.md files. */
interface AphrodyFrontmatter {
	name?: string;
	description?: string;
	when_to_use?: string;
	// Permit forward-compat unknown keys without aborting the parser.
	[key: string]: string | undefined;
}

/** Per-skill alignment status. */
type AlignmentStatus = "aligned" | "partial" | "failed";

interface SkillReport {
	skill: string;
	relPath: string;
	status: AlignmentStatus;
	triggerCount: number;
	missingFields: string[];
	notes: string[];
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const args = new Set(process.argv.slice(2));
const CHECK = args.has("--check");

// ---------------------------------------------------------------------------
// Frontmatter parser (minimal, scalar-only)
// ---------------------------------------------------------------------------

/**
 * Parse aphrody's flat YAML frontmatter — only scalar `key: value`
 * pairs and a single optional block scalar via `|` are needed today.
 * Aphrody SKILL.md frontmatter never nests, so we deliberately do
 * NOT pull in a full YAML parser (keeps the script bun-vendored-only
 * with zero npm install). If a richer SKILL.md schema lands later
 * (`triggers:` block lists, nested `od:`), we revisit this — until
 * then this parser is exact on the in-repo files.
 */
function parseAphrodyFrontmatter(raw: string): { data: AphrodyFrontmatter; body: string } {
	// Strip a UTF-8 BOM (﻿) if present — at least one in-repo SKILL.md
	// ships with one, which would otherwise break the leading `^---` anchor.
	const normalised = raw.charCodeAt(0) === 0xfeff ? raw.slice(1) : raw;
	const FM = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/;
	const match = normalised.match(FM);
	if (!match) return { data: {}, body: raw };
	const [, fmText, bodyText] = match;
	const data: AphrodyFrontmatter = {};
	const lines = fmText.split(/\r?\n/);
	let i = 0;
	while (i < lines.length) {
		const line = lines[i];
		if (line.trim().length === 0) {
			i += 1;
			continue;
		}
		const kv = line.match(/^([A-Za-z_][A-Za-z0-9_]*):\s*(\|[+-]?)?\s*(.*)$/);
		if (!kv) {
			i += 1;
			continue;
		}
		const [, key, blockMarker, rest] = kv;
		if (blockMarker) {
			// Block-scalar: keep consuming subsequent indented lines until
			// a non-indented line (or EOF) ends the block.
			const buf: string[] = [];
			let j = i + 1;
			while (j < lines.length) {
				const next = lines[j];
				if (next.length === 0) {
					buf.push("");
					j += 1;
					continue;
				}
				const indent = next.match(/^(\s+)(.*)$/);
				if (!indent) break;
				buf.push(indent[2]);
				j += 1;
			}
			data[key] = buf.join("\n").trim();
			i = j;
			continue;
		}
		// Inline scalar — strip optional surrounding quotes.
		let val = rest.trim();
		if (
			(val.startsWith('"') && val.endsWith('"')) ||
			(val.startsWith("'") && val.endsWith("'"))
		) {
			val = val.slice(1, -1);
		}
		data[key] = val;
		i += 1;
	}
	return { data, body: bodyText };
}

// ---------------------------------------------------------------------------
// Trigger extraction
// ---------------------------------------------------------------------------

/**
 * Pull every double-quoted substring out of `when_to_use`. Dedupes
 * case-insensitively while preserving the first-seen casing (so
 * "/start" stays before "start" if both appear). Returns a freshly
 * allocated list so the caller can safely sort / mutate.
 */
function extractTriggers(whenToUse: string | undefined, skillName: string): string[] {
	const out: string[] = [];
	const seen = new Set<string>();
	const QUOTED = /"([^"]+)"/g;
	if (whenToUse) {
		let m: RegExpExecArray | null;
		while ((m = QUOTED.exec(whenToUse)) !== null) {
			const phrase = m[1].trim();
			if (phrase.length === 0) continue;
			const key = phrase.toLowerCase();
			if (seen.has(key)) continue;
			seen.add(key);
			out.push(phrase);
		}
	}
	if (out.length === 0 && skillName) {
		// Fallback so open-design always has at least one phrase to match.
		out.push(`/${skillName}`);
	}
	return out;
}

// ---------------------------------------------------------------------------
// YAML emitter (open-design canonical shape)
// ---------------------------------------------------------------------------

function yamlEscape(value: string): string {
	return value.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

/** Emit a literal-block scalar with two-space indent. */
function emitBlock(key: string, text: string): string {
	const cleaned = text.trim();
	if (cleaned.length === 0) return `${key}: ""`;
	const lines = cleaned.split(/\r?\n/);
	return [`${key}: |`, ...lines.map((ln) => `  ${ln}`)].join("\n");
}

interface OdYamlInput {
	name: string;
	description: string;
	triggers: string[];
	mode: string;
	category: string;
	source: string;
}

function buildOdYaml(input: OdYamlInput): string {
	const lines: string[] = [
		"# SPDX-License-Identifier: Apache-2.0",
		"# GENERATED by scripts/skill-schema-align.ts — DO NOT EDIT BY HAND.",
		"# Source of truth: SKILL.md (Anthropic skill-loader schema).",
		"# Regenerate with: bun run scripts/skill-schema-align.ts",
		"---",
		`name: "${yamlEscape(input.name)}"`,
		emitBlock("description", input.description || input.name),
	];
	if (input.triggers.length > 0) {
		lines.push("triggers:");
		for (const t of input.triggers) {
			lines.push(`  - "${yamlEscape(t)}"`);
		}
	} else {
		lines.push("triggers: []");
	}
	lines.push("od:");
	lines.push(`  mode: ${input.mode}`);
	lines.push(`  category: ${input.category}`);
	lines.push(`  source: "${yamlEscape(input.source)}"`);
	lines.push("");
	return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Mode + category inference
// ---------------------------------------------------------------------------

/**
 * Infer the open-design `od.mode` value from skill body + description.
 * Aphrody skills are mostly orchestration / coordination loops rather
 * than templates or generators, so the default is `prototype` — the
 * widest open-design mode that does not impose a render surface.
 * We promote to `design-system` when the skill clearly authors a
 * DESIGN.md / design tokens.
 */
function inferMode(name: string, description: string, body: string): string {
	const hay = `${name}\n${description}\n${body}`.toLowerCase();
	if (/design[- ]system|design\.md|design tokens|m3|material design/.test(hay)) {
		return "design-system";
	}
	if (/template/.test(hay) && /scaffold|emit one|artifact/.test(hay)) {
		return "template";
	}
	return "prototype";
}

/**
 * Infer the open-design `od.category` (lowercase slug). Aphrody skills
 * fall into three buckets: autonomy loops, A2A coordination, and design
 * ingestion. Anything unmatched falls through to `aphrody-orchestration`
 * so the catalogue stays scannable in the open-design Settings → Skills UI.
 */
function inferCategory(name: string, description: string): string {
	const hay = `${name}\n${description}`.toLowerCase();
	if (/design|m3|material/.test(hay)) return "design-systems";
	if (/duel|a2a|coord/.test(hay)) return "a2a-coordination";
	if (/grind|perfect|yolo|loop|autonomous|start/.test(hay)) return "aphrody-autonomy";
	if (/vps|tunnel|ssh/.test(hay)) return "aphrody-ops";
	return "aphrody-orchestration";
}

// ---------------------------------------------------------------------------
// Per-skill alignment
// ---------------------------------------------------------------------------

interface AlignmentInput {
	skillDir: string;
	skillName: string;
}

function alignSkill({ skillDir, skillName }: AlignmentInput): SkillReport {
	const relPath = `.claude/skills/${skillName}/SKILL.md`;
	const report: SkillReport = {
		skill: skillName,
		relPath,
		status: "failed",
		triggerCount: 0,
		missingFields: [],
		notes: [],
	};

	const skillPath = join(skillDir, "SKILL.md");
	if (!existsSync(skillPath)) {
		report.notes.push("SKILL.md not found in skill directory");
		return report;
	}

	let raw: string;
	try {
		raw = readFileSync(skillPath, "utf8");
	} catch (err) {
		report.notes.push(`read failed: ${(err as Error).message}`);
		return report;
	}

	const { data, body } = parseAphrodyFrontmatter(raw);

	if (!data.name) report.missingFields.push("name");
	if (!data.description) report.missingFields.push("description");
	if (!data.when_to_use) report.missingFields.push("when_to_use");

	const name = data.name ?? skillName;
	const description = data.description ?? "";
	const triggers = extractTriggers(data.when_to_use, name);
	report.triggerCount = triggers.length;

	const mode = inferMode(name, description, body);
	const category = inferCategory(name, description);

	const yaml = buildOdYaml({
		name,
		description,
		triggers,
		mode,
		category,
		source: relPath,
	});

	const outPath = join(skillDir, ".od.yaml");
	try {
		writeFileSync(outPath, yaml, "utf8");
	} catch (err) {
		report.notes.push(`write failed: ${(err as Error).message}`);
		return report;
	}

	// Aligned = every field present + at least one trigger derived from
	// quoted phrases (not the /<name> fallback). Partial = missing some
	// optional field OR only the fallback trigger fires.
	const hasQuotedTrigger =
		data.when_to_use !== undefined && /"[^"]+"/.test(data.when_to_use);
	if (report.missingFields.length === 0 && hasQuotedTrigger) {
		report.status = "aligned";
	} else if (report.missingFields.length === 0) {
		report.status = "partial";
		report.notes.push("no quoted phrases in when_to_use; trigger synthesised from name");
	} else if (data.name && data.description) {
		report.status = "partial";
	} else {
		// Even with missing fields we still emit .od.yaml on best-effort
		// basis; mark failed so --check surfaces the gap to the operator.
		report.status = "failed";
	}

	return report;
}

// ---------------------------------------------------------------------------
// Audit emitter
// ---------------------------------------------------------------------------

function buildAuditMd(reports: SkillReport[]): string {
	const now = new Date().toISOString();
	const aligned = reports.filter((r) => r.status === "aligned").length;
	const partial = reports.filter((r) => r.status === "partial").length;
	const failed = reports.filter((r) => r.status === "failed").length;
	const totalTriggers = reports.reduce((s, r) => s + r.triggerCount, 0);

	const sorted = [...reports].sort((a, b) => a.skill.localeCompare(b.skill));

	const lines: string[] = [
		"<!-- SPDX-License-Identifier: Apache-2.0 -->",
		"<!-- GENERATED by scripts/skill-schema-align.ts — DO NOT EDIT BY HAND. -->",
		"",
		"# Skill schema alignment audit",
		"",
		`Generated: ${now}`,
		`Source script: \`scripts/skill-schema-align.ts\``,
		"",
		"This audit reports the alignment between aphrody's `.claude/skills/*/SKILL.md`",
		"(Anthropic skill-loader schema: `name` + `description` + `when_to_use`) and the",
		"open-design daemon schema (`name` + `description` + `triggers[]` + `od.*`).",
		"The script writes a `.od.yaml` sidecar next to every SKILL.md without modifying",
		"the SKILL.md itself, so both runtimes can resolve the same on-disk skill.",
		"",
		"## Summary",
		"",
		`- Skills processed: **${reports.length}**`,
		`- Aligned: **${aligned}**`,
		`- Partial: **${partial}**`,
		`- Failed: **${failed}**`,
		`- Triggers extracted (total): **${totalTriggers}**`,
		"",
		"## Per-skill report",
		"",
		"| Skill | Status | Triggers | Missing fields | Notes |",
		"|---|---|---|---|---|",
	];
	for (const r of sorted) {
		const missing = r.missingFields.length > 0 ? r.missingFields.join(", ") : "—";
		const notes = r.notes.length > 0 ? r.notes.join("; ") : "—";
		lines.push(
			`| \`${r.skill}\` | ${r.status} | ${r.triggerCount} | ${missing} | ${notes} |`,
		);
	}
	lines.push("");
	lines.push("## How the schemas map");
	lines.push("");
	lines.push("| Aphrody SKILL.md | open-design `.od.yaml` |");
	lines.push("|---|---|");
	lines.push("| `name` | `name` (quoted scalar) |");
	lines.push("| `description` | `description: \\|` (block scalar) |");
	lines.push("| `when_to_use` (double-quoted phrases) | `triggers:` list |");
	lines.push("| _inferred from body_ | `od.mode` (`prototype` / `design-system` / `template`) |");
	lines.push("| _inferred from name+description_ | `od.category` (slug) |");
	lines.push("| `.claude/skills/<name>/SKILL.md` | `od.source` (canonical pointer) |");
	lines.push("");
	lines.push("## Re-running");
	lines.push("");
	lines.push("```bash");
	lines.push("# Re-emit every .od.yaml (idempotent)");
	lines.push("bun run scripts/skill-schema-align.ts");
	lines.push("");
	lines.push("# Gate CI: exit 1 if any skill failed alignment");
	lines.push("bun run scripts/skill-schema-align.ts --check");
	lines.push("```");
	lines.push("");
	return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main(): number {
	if (!existsSync(SKILLS_ROOT)) {
		process.stderr.write(`error: ${SKILLS_ROOT} not found\n`);
		return 2;
	}

	const entries = readdirSync(SKILLS_ROOT, { withFileTypes: true });
	const reports: SkillReport[] = [];
	for (const entry of entries) {
		if (!entry.isDirectory()) continue;
		const skillDir = join(SKILLS_ROOT, entry.name);
		const skillPath = join(skillDir, "SKILL.md");
		if (!existsSync(skillPath) || !statSync(skillPath).isFile()) continue;
		reports.push(alignSkill({ skillDir, skillName: entry.name }));
	}

	// Emit audit.
	const auditDir = dirname(AUDIT_OUT);
	if (!existsSync(auditDir)) mkdirSync(auditDir, { recursive: true });
	writeFileSync(AUDIT_OUT, buildAuditMd(reports), "utf8");

	// Console summary so the operator gets a per-skill line even without
	// reading the audit file.
	const aligned = reports.filter((r) => r.status === "aligned").length;
	const partial = reports.filter((r) => r.status === "partial").length;
	const failed = reports.filter((r) => r.status === "failed").length;
	const totalTriggers = reports.reduce((s, r) => s + r.triggerCount, 0);
	process.stdout.write(
		`skill-schema-align: ${reports.length} skills processed, ` +
			`${aligned} aligned, ${partial} partial, ${failed} failed, ` +
			`${totalTriggers} triggers extracted\n`,
	);
	for (const r of reports) {
		process.stdout.write(
			`  - ${r.skill}: ${r.status} (${r.triggerCount} triggers` +
				(r.missingFields.length > 0
					? `, missing: ${r.missingFields.join(",")}`
					: "") +
				")\n",
		);
	}
	process.stdout.write(`audit written: ${AUDIT_OUT}\n`);

	if (CHECK && failed > 0) {
		process.stderr.write(`--check: ${failed} skill(s) failed alignment\n`);
		return 1;
	}
	return 0;
}

process.exit(main());
