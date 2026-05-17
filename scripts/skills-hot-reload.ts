// SPDX-License-Identifier: Apache-2.0
/**
 * skills-hot-reload.ts
 * ====================
 *
 * Hot-reload watcher for `.claude/skills/` (and `.claude/agents/`) that
 * mirrors the behaviour observed during the Bun 3 harvest pass — Claude
 * Code's runtime auto-reloaded freshly imported skills mid-session. This
 * makes that pipeline explicit and observable for any contributor:
 *
 *     bun run scripts/skills-hot-reload.ts
 *
 * On every change the script writes a flat JSON manifest of every parsed
 * skill / agent and touches a "reload signal" file. Any downstream
 * consumer (Claude Code, the openclaw daemon, a hypothetical
 * `aphrody-runtime` crate) can `fs.watch` that single signal file as a
 * cheap reload trigger without re-scanning the skill tree itself.
 *
 * Flags:
 *   --once                run a single full scan, then exit (CI mode)
 *   --reload-signal=PATH  signal file (default: var/data/skills-reload.signal)
 *   --manifest=PATH       manifest output  (default: var/data/skills-manifest.json)
 *
 * Validation: each SKILL.md (and any sibling `.od.yaml` from Bun 1) must
 * carry YAML frontmatter exposing `name + description + (when_to_use |
 * triggers)`. A failing skill is reported and skipped — the watcher itself
 * never crashes.
 */

import {
	existsSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const SKILLS_ROOT = join(REPO_ROOT, ".claude", "skills");
const AGENTS_ROOT = join(REPO_ROOT, ".claude", "agents");
const DEFAULT_SIGNAL = join(REPO_ROOT, "var", "data", "skills-reload.signal");
const DEFAULT_MANIFEST = join(REPO_ROOT, "var", "data", "skills-manifest.json");
const AUDIT_PATH = join(
	REPO_ROOT,
	"docs",
	"audits",
	"skills-hot-reload.md",
);
const POLL_INTERVAL_MS = 1500;

interface Args {
	once: boolean;
	signalPath: string;
	manifestPath: string;
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = {
		once: false,
		signalPath: DEFAULT_SIGNAL,
		manifestPath: DEFAULT_MANIFEST,
	};
	for (const raw of argv) {
		if (raw === "--once") {
			out.once = true;
		} else if (raw.startsWith("--reload-signal=")) {
			out.signalPath = resolve(raw.slice("--reload-signal=".length));
		} else if (raw.startsWith("--manifest=")) {
			out.manifestPath = resolve(raw.slice("--manifest=".length));
		} else if (raw === "--help" || raw === "-h") {
			console.log(
				"usage: bun run scripts/skills-hot-reload.ts [--once] " +
					"[--reload-signal=PATH] [--manifest=PATH]",
			);
			process.exit(0);
		} else {
			throw new Error(`unknown flag: ${raw}`);
		}
	}
	return out;
}

type SkillKind = "skill" | "agent";

interface SkillEntry {
	kind: SkillKind;
	id: string;
	name: string;
	description: string;
	when_to_use: string | null;
	triggers: string[];
	source: string; // absolute path of SKILL.md / agent .md / .od.yaml
	relSource: string; // path relative to repo root, forward-slash
	dir: string; // absolute containing directory
	mtimeMs: number;
	valid: true;
}

interface SkillError {
	kind: SkillKind;
	id: string;
	source: string;
	relSource: string;
	error: string;
	valid: false;
}

type ScanRecord = SkillEntry | SkillError;

interface ScanResult {
	generated_at: string;
	repo_root: string;
	skills_root: string;
	agents_root: string;
	count: { total: number; valid: number; invalid: number };
	entries: ScanRecord[];
}

// ---------------------------------------------------------------------------
// Frontmatter parser (minimal — scalars, block-literal strings, flat arrays)
// ---------------------------------------------------------------------------

type YamlScalar = string | number | boolean | null;
type YamlValue = YamlScalar | YamlValue[] | { [k: string]: YamlValue };

function stripBom(s: string): string {
	return s.replace(/^﻿/, "");
}

function parseFrontmatter(raw: string): {
	data: Record<string, YamlValue>;
	body: string;
} {
	const text = stripBom(raw);
	// Skip leading comment / blank lines before the opening `---` so .od.yaml
	// files (which carry an SPDX header) parse the same as a SKILL.md.
	const stripped = text.replace(/^(?:[ \t]*#[^\n]*\n|[ \t]*\n)+/, "");
	const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/.exec(stripped);
	if (!m) return { data: {}, body: stripped };
	return { data: parseYaml(m[1] ?? ""), body: m[2] ?? "" };
}

// `.od.yaml` files are pure YAML documents — the leading `---` is the YAML
// document-start marker, not a frontmatter fence (there is no closing
// `---`). Strip SPDX/comment headers, the optional doc-start marker, and
// hand the rest to the YAML subset parser.
function parseYamlOrFrontmatter(raw: string): Record<string, YamlValue> {
	const text = stripBom(raw);
	let stripped = text.replace(/^(?:[ \t]*#[^\n]*\n|[ \t]*\n)+/, "");
	if (stripped.startsWith("---\n") || stripped.startsWith("---\r\n")) {
		stripped = stripped.replace(/^---\r?\n/, "");
	}
	return parseYaml(stripped);
}

function parseYaml(src: string): Record<string, YamlValue> {
	const lines = src.split(/\r?\n/);
	const root: Record<string, YamlValue> = {};
	let currentKey: string | null = null;
	let currentArray: YamlValue[] | null = null;
	let i = 0;
	while (i < lines.length) {
		const raw = lines[i] ?? "";
		if (/^\s*(#.*)?$/.test(raw)) {
			i++;
			continue;
		}
		const indent = raw.match(/^\s*/)?.[0].length ?? 0;
		const line = raw.slice(indent);

		// Array item (only one level of arrays supported — sufficient for triggers).
		if (line.startsWith("- ") && currentArray && currentKey) {
			currentArray.push(coerce(line.slice(2).trim()));
			i++;
			continue;
		}

		const kv = /^([^:]+):\s*(.*)$/.exec(line);
		if (!kv) {
			i++;
			continue;
		}
		const key = (kv[1] ?? "").trim();
		const val = kv[2] ?? "";

		// Block literal `|`, `|-`, `>`, `>-`.
		if (val === "|" || val === "|-" || val === ">" || val === ">-") {
			const collected: string[] = [];
			const childIndent = indent + 2;
			i++;
			while (i < lines.length) {
				const next = lines[i] ?? "";
				if (/^\s*$/.test(next)) {
					collected.push("");
					i++;
					continue;
				}
				const nIndent = next.match(/^\s*/)?.[0].length ?? 0;
				if (nIndent < childIndent) break;
				collected.push(next.slice(childIndent));
				i++;
			}
			const folded = val.startsWith(">")
				? collected.join(" ").replace(/\s+/g, " ").trim()
				: collected.join("\n").trimEnd();
			root[key] = folded;
			currentKey = null;
			currentArray = null;
			continue;
		}

		// Inline flow array `[a, b]`.
		if (val.startsWith("[") && val.endsWith("]")) {
			root[key] = val
				.slice(1, -1)
				.split(",")
				.map((s) => coerce(s.trim()))
				.filter((v) => v !== "");
			currentKey = null;
			currentArray = null;
			i++;
			continue;
		}

		// Empty value -> next non-empty lines may be `- item` array entries.
		if (val === "") {
			const arr: YamlValue[] = [];
			root[key] = arr;
			currentKey = key;
			currentArray = arr;
			i++;
			continue;
		}

		root[key] = coerce(val);
		currentKey = null;
		currentArray = null;
		i++;
	}
	return root;
}

function coerce(raw: string): YamlScalar {
	let v = raw.trim();
	if (
		(v.startsWith('"') && v.endsWith('"')) ||
		(v.startsWith("'") && v.endsWith("'"))
	) {
		return v.slice(1, -1);
	}
	if (v === "true") return true;
	if (v === "false") return false;
	if (v === "null" || v === "~") return null;
	if (/^-?\d+$/.test(v)) return Number(v);
	if (/^-?\d*\.\d+$/.test(v)) return Number(v);
	return v;
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

const NAME_RE = /^[a-z0-9][a-z0-9-]*$/;

function asString(v: YamlValue | undefined): string | null {
	if (typeof v === "string") return v.trim() || null;
	if (typeof v === "number" || typeof v === "boolean") return String(v);
	return null;
}

function asStringArray(v: YamlValue | undefined): string[] {
	if (!Array.isArray(v)) return [];
	const out: string[] = [];
	for (const x of v) {
		if (typeof x === "string" && x.trim()) out.push(x.trim());
		else if (typeof x === "number" || typeof x === "boolean") out.push(String(x));
	}
	return out;
}

function validateRecord(
	source: string,
	kind: SkillKind,
	fallbackId: string,
	data: Record<string, YamlValue>,
	mtimeMs: number,
): ScanRecord {
	const relSource = relative(REPO_ROOT, source).split(/[\\/]/).join("/");
	const dir = dirname(source);
	const id = asString(data.name) ?? fallbackId;
	const fail = (msg: string): SkillError => ({
		kind,
		id,
		source,
		relSource,
		error: msg,
		valid: false,
	});

	const name = asString(data.name);
	if (!name) return fail("missing `name` in frontmatter");
	if (!NAME_RE.test(name))
		return fail(
			`\`name\` "${name}" does not match /^[a-z0-9][a-z0-9-]*$/`,
		);

	const description = asString(data.description);
	if (!description) return fail("missing or empty `description`");

	const whenToUse = asString(data.when_to_use);
	const triggers = asStringArray(data.triggers);
	// SKILL.md / .od.yaml must advertise their trigger surface. Agents are
	// invoked explicitly via the Task tool, so a `description` is enough —
	// the Claude Code harness routes calls by name, not by trigger phrase.
	if (kind === "skill" && !whenToUse && triggers.length === 0) {
		return fail("must declare either `when_to_use` or non-empty `triggers:`");
	}

	return {
		kind,
		id: name,
		name,
		description,
		when_to_use: whenToUse,
		triggers,
		source,
		relSource,
		dir,
		mtimeMs,
		valid: true,
	};
}

// ---------------------------------------------------------------------------
// Scan
// ---------------------------------------------------------------------------

function listSubdirs(root: string): string[] {
	if (!existsSync(root)) return [];
	const out: string[] = [];
	for (const entry of readdirSync(root, { withFileTypes: true })) {
		if (entry.isDirectory()) out.push(join(root, entry.name));
	}
	return out;
}

function scanSkillsRoot(root: string): ScanRecord[] {
	const out: ScanRecord[] = [];
	for (const skillDir of listSubdirs(root)) {
		const fallbackId = basename(skillDir);
		const candidates = [
			join(skillDir, "SKILL.md"),
			join(skillDir, "skill.md"),
			...readdirSync(skillDir, { withFileTypes: true })
				.filter((d) => d.isFile() && d.name.endsWith(".od.yaml"))
				.map((d) => join(skillDir, d.name)),
		];
		for (const file of candidates) {
			if (!existsSync(file)) continue;
			out.push(parseOne(file, "skill", fallbackId));
		}
	}
	return out;
}

function scanAgentsRoot(root: string): ScanRecord[] {
	const out: ScanRecord[] = [];
	if (!existsSync(root)) return out;
	for (const entry of readdirSync(root, { withFileTypes: true })) {
		if (!entry.isFile() || !entry.name.endsWith(".md")) continue;
		const file = join(root, entry.name);
		const fallbackId = basename(entry.name, ".md");
		out.push(parseOne(file, "agent", fallbackId));
	}
	return out;
}

function parseOne(
	file: string,
	kind: SkillKind,
	fallbackId: string,
): ScanRecord {
	const relSource = relative(REPO_ROOT, file).split(/[\\/]/).join("/");
	let raw: string;
	let mtimeMs = 0;
	try {
		raw = readFileSync(file, "utf8");
		mtimeMs = statSync(file).mtimeMs;
	} catch (err) {
		return {
			kind,
			id: fallbackId,
			source: file,
			relSource,
			error: `read failed: ${(err as Error).message}`,
			valid: false,
		};
	}
	const isOdYaml = file.toLowerCase().endsWith(".od.yaml");
	let data: Record<string, YamlValue>;
	try {
		// `.od.yaml` is pure YAML (no `---` fences required); SKILL.md and
		// agent .md files must carry frontmatter delimited by `---`.
		if (isOdYaml) {
			data = parseYamlOrFrontmatter(raw);
		} else {
			const stripped = raw.replace(/^﻿/, "").trimStart();
			if (!stripped.startsWith("---")) {
				return {
					kind,
					id: fallbackId,
					source: file,
					relSource,
					error: "missing YAML frontmatter (file must start with `---`)",
					valid: false,
				};
			}
			data = parseFrontmatter(raw).data;
		}
	} catch (err) {
		return {
			kind,
			id: fallbackId,
			source: file,
			relSource,
			error: `frontmatter parse error: ${(err as Error).message}`,
			valid: false,
		};
	}
	return validateRecord(file, kind, fallbackId, data, mtimeMs);
}

function fullScan(): ScanResult {
	const entries: ScanRecord[] = [
		...scanSkillsRoot(SKILLS_ROOT),
		...scanAgentsRoot(AGENTS_ROOT),
	];
	let valid = 0;
	let invalid = 0;
	for (const e of entries) (e.valid ? valid++ : invalid++);
	return {
		generated_at: new Date().toISOString(),
		repo_root: REPO_ROOT,
		skills_root: SKILLS_ROOT,
		agents_root: AGENTS_ROOT,
		count: { total: entries.length, valid, invalid },
		entries,
	};
}

function ensureDir(path: string): void {
	const d = dirname(path);
	if (!existsSync(d)) mkdirSync(d, { recursive: true });
}

function writeManifest(path: string, result: ScanResult): void {
	ensureDir(path);
	writeFileSync(path, `${JSON.stringify(result, null, 2)}\n`, "utf8");
}

function touchSignal(path: string, reason: string): void {
	ensureDir(path);
	writeFileSync(
		path,
		`${new Date().toISOString()} ${reason}\n`,
		"utf8",
	);
}

// ---------------------------------------------------------------------------
// Audit doc
// ---------------------------------------------------------------------------

function writeAuditDoc(args: Args, result: ScanResult): void {
	ensureDir(AUDIT_PATH);
	const lines = [
		"# `skills-hot-reload` — integration notes",
		"",
		"`scripts/skills-hot-reload.ts` watches `.claude/skills/` and",
		"`.claude/agents/`, re-parses YAML frontmatter on every change, writes",
		"a flat JSON manifest, and touches a single signal file. Downstream",
		"runtimes can `fs.watch` the signal file as a cheap zero-payload",
		"reload trigger.",
		"",
		`Generated: ${result.generated_at}.`,
		"",
		"## Outputs",
		"",
		`- **Manifest** \`${args.manifestPath}\` — \`{generated_at, count, entries[]}\`. Each entry is \`{kind, id, name, description, when_to_use, triggers[], source, relSource, dir, mtimeMs, valid}\`.`,
		`- **Signal**   \`${args.signalPath}\` — one line per touch: \`<iso8601> <reason>\`. Watchers can ignore the content and react on \`mtime\`.`,
		"",
		"## Run modes",
		"",
		"```bash",
		"bun run scripts/skills-hot-reload.ts            # watch (poll 1500 ms)",
		"bun run scripts/skills-hot-reload.ts --once     # CI: one scan, exit",
		"bun run scripts/skills-hot-reload.ts \\",
		"  --reload-signal=var/data/custom.signal \\",
		"  --manifest=var/data/custom-manifest.json",
		"```",
		"",
		"`--once` exits with code `1` when any skill fails validation — wire",
		"it into pre-commit or the CI lint pass to catch broken frontmatter",
		"before it lands.",
		"",
		"## Integration patterns",
		"",
		"### Claude Code (this repo)",
		"",
		"Claude Code already re-scans `.claude/skills/` opportunistically",
		"between turns. Run the watcher in the background of a long session",
		"to surface validation failures the moment a SKILL.md is edited — the",
		"signal file is a passive observation channel; the harness needs no",
		"changes.",
		"",
		"### openclaw daemon",
		"",
		"The daemon's `apps/daemon/src/skills.ts` re-scans on every",
		"`GET /api/skills`. Replace the per-request scan with a single",
		"`fs.watch` against the signal file:",
		"",
		"```ts",
		"import { watch } from 'node:fs';",
		"let cached = await listSkills(roots);",
		`watch('${args.signalPath.replace(/\\/g, "/")}', () => {`,
		"  listSkills(roots).then((next) => { cached = next; });",
		"});",
		"app.get('/api/skills', (_, res) => res.json(cached));",
		"```",
		"",
		"### Hypothetical `aphrody-runtime` crate",
		"",
		"In Rust, use `notify`'s `RecommendedWatcher` against the signal file",
		"and reload an `ArcSwap<Vec<Skill>>` on each event. The signal file",
		"is short enough to fit a single `read_to_string`; the manifest is",
		"the source of truth for the parse output.",
		"",
		"```rust",
		"let (tx, rx) = std::sync::mpsc::channel();",
		"let mut w = notify::recommended_watcher(tx)?;",
		`w.watch(Path::new("${args.signalPath.replace(/\\/g, "/")}"), RecursiveMode::NonRecursive)?;`,
		"for _evt in rx { reload_manifest(&manifest_path)?; }",
		"```",
		"",
		"## Validation rules",
		"",
		"- File must begin with `---` (YAML frontmatter delimiter).",
		"- `name` must match `/^[a-z0-9][a-z0-9-]*$/`.",
		"- `description` must be a non-empty string.",
		"- Either `when_to_use` (aphrody schema) **or** non-empty `triggers:`",
		"  (open-design schema) must be present.",
		"",
		"A single skill failing validation never crashes the watcher — it is",
		"recorded as `{valid: false, error}` in the manifest and printed to",
		"stdout once. The watcher keeps running.",
		"",
	];
	writeFileSync(AUDIT_PATH, lines.join("\n"), "utf8");
}

// ---------------------------------------------------------------------------
// Watch loop (poll-based — Bun.fs.watch recursive on Windows is unreliable)
// ---------------------------------------------------------------------------

interface FileSnapshot {
	mtimeMs: number;
	size: number;
}

function snapshotTree(roots: readonly string[]): Map<string, FileSnapshot> {
	const out = new Map<string, FileSnapshot>();
	for (const root of roots) {
		if (!existsSync(root)) continue;
		walk(root, out);
	}
	return out;
}

function walk(dir: string, out: Map<string, FileSnapshot>): void {
	let entries: ReturnType<typeof readdirSync>;
	try {
		entries = readdirSync(dir, { withFileTypes: true });
	} catch {
		return;
	}
	for (const entry of entries) {
		const full = join(dir, entry.name);
		if (entry.isDirectory()) {
			walk(full, out);
		} else if (entry.isFile()) {
			try {
				const s = statSync(full);
				out.set(full, { mtimeMs: s.mtimeMs, size: s.size });
			} catch {
				// transient: file gone between readdir and stat
			}
		}
	}
}

function diff(
	prev: Map<string, FileSnapshot>,
	next: Map<string, FileSnapshot>,
): { added: string[]; removed: string[]; changed: string[] } {
	const added: string[] = [];
	const changed: string[] = [];
	const removed: string[] = [];
	for (const [k, v] of next) {
		const p = prev.get(k);
		if (!p) added.push(k);
		else if (p.mtimeMs !== v.mtimeMs || p.size !== v.size) changed.push(k);
	}
	for (const k of prev.keys()) if (!next.has(k)) removed.push(k);
	return { added, removed, changed };
}

function logEvent(event: string, path: string): void {
	const ts = new Date().toISOString();
	const rel = relative(REPO_ROOT, path).split(/[\\/]/).join("/");
	console.log(`[${ts}] ${event.padEnd(8)} ${rel}`);
}

function logScan(result: ScanResult): void {
	console.log(
		`[${result.generated_at}] scan     total=${result.count.total} ` +
			`valid=${result.count.valid} invalid=${result.count.invalid}`,
	);
	for (const e of result.entries) {
		if (!e.valid) console.log(`  INVALID ${e.relSource}: ${e.error}`);
	}
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));
	const roots = [SKILLS_ROOT, AGENTS_ROOT];

	const result = fullScan();
	writeManifest(args.manifestPath, result);
	touchSignal(args.signalPath, "initial-scan");
	writeAuditDoc(args, result);
	logScan(result);

	if (args.once) {
		process.exit(result.count.invalid === 0 ? 0 : 1);
	}

	let snapshot = snapshotTree(roots);
	console.log(
		`[${new Date().toISOString()}] watching ${roots.join(" + ")} ` +
			`(poll=${POLL_INTERVAL_MS}ms)`,
	);

	const tick = (): void => {
		const next = snapshotTree(roots);
		const { added, removed, changed } = diff(snapshot, next);
		if (added.length || removed.length || changed.length) {
			for (const p of added) logEvent("ADDED", p);
			for (const p of changed) logEvent("CHANGED", p);
			for (const p of removed) logEvent("REMOVED", p);
			snapshot = next;
			const rescan = fullScan();
			writeManifest(args.manifestPath, rescan);
			const reason = added.length
				? `added=${added.length}`
				: changed.length
					? `changed=${changed.length}`
					: `removed=${removed.length}`;
			touchSignal(args.signalPath, reason);
			logScan(rescan);
		}
	};

	const handle = setInterval(tick, POLL_INTERVAL_MS);
	const shutdown = (sig: string): void => {
		clearInterval(handle);
		console.log(`[${new Date().toISOString()}] ${sig} — exiting`);
		process.exit(0);
	};
	process.on("SIGINT", () => shutdown("SIGINT"));
	process.on("SIGTERM", () => shutdown("SIGTERM"));
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
