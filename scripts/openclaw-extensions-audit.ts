// SPDX-License-Identifier: Apache-2.0
// #!/usr/bin/env bun
/**
 * openclaw-extensions-audit.ts
 * ===========================
 *
 * Enumerates every extension under `C:/worktree/openclaw/extensions/`,
 * extracts metadata (name, description, version, LOC, test count, deps),
 * scores each by port-priority for aphrody, and emits a ranked roadmap.
 *
 * Inputs
 * ------
 *   --source=<path>   Override extensions source dir (default:
 *                     C:/worktree/openclaw/extensions).
 *   --json            Write JSON to stdout (no files written).
 *   --top=<N>         Restrict roadmap rows to top-N (default: all).
 *
 * Outputs (when --json absent)
 * ----------------------------
 *   var/data/openclaw-extensions.json           machine-readable, gitignored.
 *   docs/audits/openclaw-extensions-roadmap.md  ranked table + checklist.
 *
 * Port-priority heuristic
 * -----------------------
 *   high   : name matches /memory|gateway|voice|sdk/
 *   medium : name matches a known channel adapter (discord, slack, telegram,
 *            whatsapp, signal, imessage, irc, teams, matrix, feishu, line,
 *            mattermost, nextcloud, nostr, synology, tlon, twitch, zalo,
 *            wechat, qq, webchat).
 *   low    : everything else.
 *
 * The roadmap groups by priority and highlights the 5 highest-LOC items
 * inside `high` as the canonical next-tick port checklist for the aphrody
 * Rust crates (`crates/aphrody-memory`, `crates/aphrody-gateway`,
 * `crates/aphrody-voice`, etc.).
 */

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve, sep } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_SOURCE = "C:/worktree/openclaw/extensions";
const OUT_JSON = join(REPO_ROOT, "var", "data", "openclaw-extensions.json");
const OUT_MD = join(REPO_ROOT, "docs", "audits", "openclaw-extensions-roadmap.md");

const HIGH_RE = /memory|gateway|voice|sdk/i;
const CHANNEL_ADAPTERS = new Set<string>([
	"discord",
	"slack",
	"telegram",
	"whatsapp",
	"signal",
	"imessage",
	"irc",
	"teams",
	"matrix",
	"feishu",
	"line",
	"mattermost",
	"nextcloud",
	"nextcloud-talk",
	"nostr",
	"synology",
	"synology-chat",
	"tlon",
	"twitch",
	"zalo",
	"zalouser",
	"wechat",
	"webchat",
	"qq",
	"qqbot",
	"qa-channel",
]);

const CODE_EXTS = new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"]);
const SKIP_DIRS = new Set([
	"node_modules",
	".git",
	"dist",
	"build",
	"out",
	".turbo",
	".next",
	".cache",
	"coverage",
	".vite",
]);

type Priority = "high" | "medium" | "low";

interface ExtensionRecord {
	dir: string;
	name: string;
	bareName: string;
	description: string;
	version: string;
	keywords: string[];
	loc: number;
	testCount: number;
	depsCount: number;
	deps: string[];
	priority: Priority;
	priorityReason: string;
	hasReadme: boolean;
	hasSrc: boolean;
}

interface CliArgs {
	source: string;
	json: boolean;
	top: number | null;
}

function parseArgs(argv: string[]): CliArgs {
	let source = DEFAULT_SOURCE;
	let json = false;
	let top: number | null = null;

	for (const arg of argv.slice(2)) {
		if (arg === "--json") {
			json = true;
		} else if (arg.startsWith("--source=")) {
			source = arg.slice("--source=".length);
		} else if (arg.startsWith("--top=")) {
			const n = Number.parseInt(arg.slice("--top=".length), 10);
			if (Number.isFinite(n) && n > 0) top = n;
		} else if (arg === "--help" || arg === "-h") {
			printHelp();
			process.exit(0);
		} else {
			process.stderr.write(`warn: unknown arg: ${arg}\n`);
		}
	}

	return { source, json, top };
}

function printHelp(): void {
	process.stdout.write(
		[
			"openclaw-extensions-audit.ts — rank openclaw extensions by port-priority",
			"",
			"  bun run scripts/openclaw-extensions-audit.ts",
			"  bun run scripts/openclaw-extensions-audit.ts --json",
			"  bun run scripts/openclaw-extensions-audit.ts --top=20",
			"  bun run scripts/openclaw-extensions-audit.ts --source=/path/to/extensions",
			"",
			"  Default source: " + DEFAULT_SOURCE,
			"  Default outputs:",
			"    " + OUT_JSON,
			"    " + OUT_MD,
			"",
		].join("\n"),
	);
}

function listExtensionDirs(sourceDir: string): string[] {
	if (!existsSync(sourceDir)) {
		throw new Error(`source directory not found: ${sourceDir}`);
	}
	const out: string[] = [];
	for (const entry of readdirSync(sourceDir, { withFileTypes: true })) {
		if (!entry.isDirectory()) continue;
		const dir = join(sourceDir, entry.name);
		if (existsSync(join(dir, "package.json"))) {
			out.push(dir);
		}
	}
	return out.sort();
}

function safeReadJson(path: string): Record<string, unknown> | null {
	try {
		const raw = readFileSync(path, "utf8");
		return JSON.parse(raw) as Record<string, unknown>;
	} catch (err) {
		process.stderr.write(`warn: failed to parse ${path}: ${(err as Error).message}\n`);
		return null;
	}
}

function countLinesAndTests(dir: string): { loc: number; testCount: number } {
	let loc = 0;
	let testCount = 0;

	const stack: string[] = [dir];
	while (stack.length > 0) {
		const current = stack.pop();
		if (!current) continue;
		let entries;
		try {
			entries = readdirSync(current, { withFileTypes: true });
		} catch {
			continue;
		}
		for (const entry of entries) {
			const full = join(current, entry.name);
			if (entry.isDirectory()) {
				if (SKIP_DIRS.has(entry.name)) continue;
				stack.push(full);
				continue;
			}
			if (!entry.isFile()) continue;
			const dotIdx = entry.name.lastIndexOf(".");
			if (dotIdx < 0) continue;
			const ext = entry.name.slice(dotIdx).toLowerCase();
			if (!CODE_EXTS.has(ext)) continue;
			if (entry.name.endsWith(".test.ts") || entry.name.endsWith(".test.tsx") || entry.name.endsWith(".test.js")) {
				testCount += 1;
			}
			try {
				const raw = readFileSync(full, "utf8");
				if (raw.length === 0) continue;
				// fast newline count: counts \n; final line without trailing newline adds 1
				let n = 0;
				for (let i = 0; i < raw.length; i++) {
					if (raw.charCodeAt(i) === 10) n += 1;
				}
				loc += raw.endsWith("\n") ? n : n + 1;
			} catch {
				// skip unreadable file
			}
		}
	}

	return { loc, testCount };
}

function classify(bareName: string, keywords: string[]): { priority: Priority; reason: string } {
	if (HIGH_RE.test(bareName)) {
		const m = bareName.match(HIGH_RE);
		return { priority: "high", reason: `name matches /${m?.[0] ?? "high"}/` };
	}
	if (CHANNEL_ADAPTERS.has(bareName)) {
		return { priority: "medium", reason: "channel adapter" };
	}
	// keyword fallback
	const kwLower = keywords.map((k) => k.toLowerCase());
	if (kwLower.some((k) => HIGH_RE.test(k))) {
		return { priority: "high", reason: "keyword match (memory|gateway|voice|sdk)" };
	}
	if (kwLower.some((k) => CHANNEL_ADAPTERS.has(k))) {
		return { priority: "medium", reason: "keyword match (channel)" };
	}
	return { priority: "low", reason: "default" };
}

function stripScope(pkgName: string): string {
	// "@openclaw/memory-core" -> "memory-core"
	if (pkgName.startsWith("@")) {
		const slash = pkgName.indexOf("/");
		return slash >= 0 ? pkgName.slice(slash + 1) : pkgName;
	}
	return pkgName;
}

function audit(args: CliArgs): ExtensionRecord[] {
	const dirs = listExtensionDirs(args.source);
	const records: ExtensionRecord[] = [];

	for (const dir of dirs) {
		const pkgPath = join(dir, "package.json");
		const pkg = safeReadJson(pkgPath);
		if (!pkg) continue;
		const folderName = dir.split(sep).pop() ?? dir.split("/").pop() ?? dir;
		const name = typeof pkg.name === "string" ? pkg.name : folderName;
		const bareName = stripScope(name) || folderName;
		const description = typeof pkg.description === "string" ? pkg.description : "";
		const version = typeof pkg.version === "string" ? pkg.version : "";
		const keywords =
			Array.isArray(pkg.keywords) && pkg.keywords.every((k) => typeof k === "string")
				? (pkg.keywords as string[])
				: [];
		const depsObj =
			pkg.dependencies && typeof pkg.dependencies === "object"
				? (pkg.dependencies as Record<string, string>)
				: {};
		const deps = Object.keys(depsObj).sort();

		const { loc, testCount } = countLinesAndTests(dir);
		const { priority, reason } = classify(bareName, keywords);

		records.push({
			dir,
			name,
			bareName,
			description,
			version,
			keywords,
			loc,
			testCount,
			depsCount: deps.length,
			deps,
			priority,
			priorityReason: reason,
			hasReadme: existsSync(join(dir, "README.md")),
			hasSrc: existsSync(join(dir, "src")),
		});
	}

	// Sort: priority desc (high>med>low), then LOC desc, then name asc.
	const rank: Record<Priority, number> = { high: 3, medium: 2, low: 1 };
	records.sort((a, b) => {
		if (rank[a.priority] !== rank[b.priority]) return rank[b.priority] - rank[a.priority];
		if (a.loc !== b.loc) return b.loc - a.loc;
		return a.name.localeCompare(b.name);
	});

	return records;
}

function ensureDir(path: string): void {
	if (!existsSync(path)) mkdirSync(path, { recursive: true });
}

function renderMarkdown(records: ExtensionRecord[], args: CliArgs): string {
	const dist: Record<Priority, number> = { high: 0, medium: 0, low: 0 };
	for (const r of records) dist[r.priority] += 1;

	const rows = args.top != null ? records.slice(0, args.top) : records;

	const lines: string[] = [];
	lines.push("<!-- SPDX-License-Identifier: Apache-2.0 -->");
	lines.push("");
	lines.push("# Openclaw extensions — port-priority roadmap");
	lines.push("");
	lines.push(`**Source:** \`${args.source}\``);
	lines.push(`**Generated:** ${new Date().toISOString()}`);
	lines.push(`**Total extensions:** ${records.length}`);
	lines.push(
		`**Distribution:** high=${dist.high}, medium=${dist.medium}, low=${dist.low}`,
	);
	lines.push("");
	lines.push(
		"Heuristic: `high` = name/keyword matches /memory|gateway|voice|sdk/; " +
			"`medium` = known channel adapter; `low` = everything else.",
	);
	lines.push("");
	lines.push("## Ranked table");
	lines.push("");
	lines.push("| priority | name | LOC | tests | deps | description |");
	lines.push("|---|---|---:|---:|---:|---|");
	for (const r of rows) {
		const desc = (r.description || "—").replace(/\|/g, "\\|").replace(/\n/g, " ");
		lines.push(
			`| ${r.priority} | \`${r.name}\` | ${r.loc} | ${r.testCount} | ${r.depsCount} | ${desc} |`,
		);
	}
	lines.push("");

	// Per-priority next-tick port checklist: top-5 LOC inside `high`.
	const highSorted = records
		.filter((r) => r.priority === "high")
		.sort((a, b) => b.loc - a.loc)
		.slice(0, 5);

	lines.push("## Next-tick port checklist (top-5 `high` by LOC)");
	lines.push("");
	if (highSorted.length === 0) {
		lines.push("_No `high`-priority extensions found at this source._");
	} else {
		for (const r of highSorted) {
			lines.push(
				`- [ ] **${r.bareName}** (\`${r.name}\`) — ${r.loc} LOC, ` +
					`${r.testCount} tests, ${r.depsCount} deps. ` +
					`Reason: ${r.priorityReason}. ` +
					`Source: \`${r.dir.replace(/\\/g, "/")}\`.`,
			);
		}
	}
	lines.push("");

	// Bonus per-priority short list
	for (const prio of ["medium", "low"] as Priority[]) {
		const top5 = records
			.filter((r) => r.priority === prio)
			.sort((a, b) => b.loc - a.loc)
			.slice(0, 5);
		if (top5.length === 0) continue;
		lines.push(`## Top-5 \`${prio}\` (by LOC)`);
		lines.push("");
		for (const r of top5) {
			lines.push(
				`- **${r.bareName}** — ${r.loc} LOC, ${r.testCount} tests, ${r.depsCount} deps.`,
			);
		}
		lines.push("");
	}

	lines.push("---");
	lines.push("");
	lines.push(
		"_Generator: `scripts/openclaw-extensions-audit.ts`. " +
			"Refresh after upstream openclaw sync._",
	);
	lines.push("");

	return lines.join("\n");
}

function main(): void {
	const args = parseArgs(process.argv);
	const records = audit(args);

	if (args.json) {
		process.stdout.write(JSON.stringify({ source: args.source, count: records.length, records }, null, 2));
		process.stdout.write("\n");
		return;
	}

	ensureDir(dirname(OUT_JSON));
	ensureDir(dirname(OUT_MD));

	const jsonPayload = JSON.stringify(
		{
			source: args.source,
			generatedAt: new Date().toISOString(),
			count: records.length,
			records,
		},
		null,
		2,
	);
	writeFileSync(OUT_JSON, jsonPayload + "\n", "utf8");
	writeFileSync(OUT_MD, renderMarkdown(records, args), "utf8");

	const dist: Record<Priority, number> = { high: 0, medium: 0, low: 0 };
	for (const r of records) dist[r.priority] += 1;
	const top10 = records.slice(0, 10);

	process.stdout.write(
		[
			`wrote ${OUT_JSON}`,
			`wrote ${OUT_MD}`,
			`extensions: ${records.length}`,
			`priority: high=${dist.high} medium=${dist.medium} low=${dist.low}`,
			"top-10 by rank:",
			...top10.map(
				(r, i) =>
					`  ${String(i + 1).padStart(2, " ")}. [${r.priority}] ${r.bareName} ` +
					`(LOC=${r.loc}, tests=${r.testCount}, deps=${r.depsCount})`,
			),
			"",
		].join("\n"),
	);
}

main();
