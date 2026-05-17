// SPDX-License-Identifier: Apache-2.0
/**
 * setup-worktrees.ts
 * ==================
 *
 * Bootstraps every external worktree the aphrody repo depends on at runtime
 * but does NOT vendor (per repo-light policy). Runs `gh repo clone` for each
 * missing dependency.  Idempotent: re-run any time to fetch new arrivals.
 *
 * Why this script exists
 * ----------------------
 * Aphrody references ~12 upstream repos via absolute paths (C:/worktree/* on
 * Windows, ~/worktree/* elsewhere) — bxc, open-design, openclaw, gemini-cli,
 * angular/components, whisper, etc.  Vendoring them would bloat the repo to
 * > 1 GB; relying on individual contributors to remember the clone list is
 * fragile.  This script is the single source of truth: clone everything in
 * one command so the rest of the repo "just works".
 *
 * Usage
 * -----
 *   bun run scripts/setup-worktrees.ts                # clone every missing
 *   bun run scripts/setup-worktrees.ts --dry-run      # plan only
 *   bun run scripts/setup-worktrees.ts --root=/path   # override C:/worktree
 *   bun run scripts/setup-worktrees.ts --only=bxc,n2b # restrict to allowlist
 *   bun run scripts/setup-worktrees.ts --update       # `git pull` existing
 *   bun run scripts/setup-worktrees.ts --json         # machine-readable plan
 *
 * Output
 * ------
 *   var/data/worktrees-manifest.json (gitignored cache)
 */

import { existsSync, mkdirSync, statSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

interface WorktreeSpec {
	/** Short slug used in `--only` filters and manifest keys. */
	readonly slug: string;
	/** GitHub owner/repo (passed verbatim to `gh repo clone`). */
	readonly repo: string;
	/** Local directory name under the worktree root. Defaults to repo basename. */
	readonly dirName: string;
	/** Branch override, if not the default branch. */
	readonly branch?: string;
	/** Why aphrody needs this clone — for `docs/WORKTREES.md`. */
	readonly reason: string;
	/** Which aphrody files consume this clone (file paths or globs). */
	readonly consumers: readonly string[];
	/** Approximate disk size after clone in MB (used by `--json` size budget). */
	readonly approxMb: number;
	/** `--depth=1 --no-tags --filter=blob:none` recommended for huge repos. */
	readonly shallow?: boolean;
}

const WORKTREES: readonly WorktreeSpec[] = [
	{
		slug: "bxc",
		repo: "aphrody-code/bxc",
		dirName: "bxc",
		branch: "aphrody",
		reason: "In-process browser engine. Drives scripts/bxc-mass-scrape.ts.",
		consumers: ["scripts/bxc-mass-scrape.ts", "scripts/scrape-m3-tokens.ts"],
		approxMb: 60,
	},
	{
		slug: "n2b",
		repo: "aphrody-code/n2b",
		dirName: "n2b",
		branch: "aphrody",
		reason: "Node-to-Bun linter / migrator. Referenced by Cargo.toml workspace deps.",
		consumers: ["Cargo.toml (workspace dep)"],
		approxMb: 40,
	},
	{
		slug: "open-design",
		repo: "nexu-io/open-design",
		dirName: "open-design",
		reason: "152 brand DESIGN.md + 131 SKILL.md + plugin contract + voice + memory refs.",
		consumers: [
			"packages/aphrody-skills/src/sources.ts",
			"scripts/design-systems-import.ts",
			"scripts/design-templates-import.ts",
			"scripts/skills-harvest-open-design.ts",
			"scripts/skill-schema-align.ts",
		],
		approxMb: 310,
	},
	{
		slug: "openclaw",
		repo: "openclaw/openclaw",
		dirName: "openclaw",
		reason: "30+ extensions (memory-core, voice-call, cloudflare/vercel gateway).",
		consumers: [
			"packages/aphrody-skills/src/sources.ts",
			"packages/plugin-package-contract/",
			"scripts/openclaw-extensions-audit.ts",
		],
		approxMb: 240,
	},
	{
		slug: "gemini-cli",
		repo: "google-gemini/gemini-cli",
		dirName: "gemini-cli",
		reason: "Upstream Gemini CLI — 11 SKILL.md + voice surface + OAuth credentials ref.",
		consumers: [
			"packages/aphrody-skills/src/sources.ts",
			"packages/gemini-live-aphrody/src/auth.ts",
			"crates/gemini-runtime/src/lib.rs (reference)",
		],
		approxMb: 80,
		shallow: true,
	},
	{
		slug: "components",
		repo: "angular/components",
		dirName: "components",
		reason: "Angular Material — canonical M3 SCSS tokens reference.",
		consumers: ["crates/m3-tokens/src/* (reference)", "docs/audits/2026-05-17-angular-material-scrape.md"],
		approxMb: 40,
		shallow: true,
	},
	{
		slug: "whisper",
		repo: "openai/whisper",
		dirName: "whisper",
		reason: "Reference STT implementation. Inspires crates/aphrody-voice-stt/.",
		consumers: ["crates/aphrody-voice-stt/src/local_whisper.rs (reference)"],
		approxMb: 30,
		shallow: true,
	},
	{
		slug: "live-api-web-console",
		repo: "google-gemini/live-api-web-console",
		dirName: "live-api-web-console",
		reason: "Upstream of packages/gemini-live-aphrody/ fork.",
		consumers: ["packages/gemini-live-aphrody/README.md (attribution)"],
		approxMb: 20,
		shallow: true,
	},
	{
		slug: "design.md",
		repo: "google-labs-code/design.md",
		dirName: "design.md",
		reason: "DESIGN.md spec source + @google/design.md lint CLI.",
		consumers: ["DESIGN.md (spec compliance gate)"],
		approxMb: 15,
		shallow: true,
	},
	{
		slug: "agent-browser",
		repo: "vercel-labs/agent-browser",
		dirName: "agent-browser",
		reason: "Audited against bxc — alternative browser-agent runtime.",
		consumers: ["docs/audits/2026-05-17-vercel-agent-browser-vs-bxc.md"],
		approxMb: 30,
		shallow: true,
	},
	{
		slug: "vercel-agent-skills",
		repo: "vercel-labs/agent-skills",
		dirName: "vercel-agent-skills",
		reason: "7 SKILL.md (vercel-labs) for aphrody-skills aggregator.",
		consumers: ["packages/aphrody-skills/src/sources.ts"],
		approxMb: 10,
		shallow: true,
	},
	{
		slug: "vercel-skills",
		repo: "vercel-labs/skills",
		dirName: "vercel-skills",
		reason: "1 SKILL.md (find-skills) + skills-host CLI for aphrody-skills.",
		consumers: ["packages/aphrody-skills/src/sources.ts"],
		approxMb: 10,
		shallow: true,
	},
	{
		slug: "open-agents",
		repo: "vercel-labs/open-agents",
		dirName: "open-agents",
		reason: "13 .agents/skills + agent-harness reference for aphrody-skills.",
		consumers: ["packages/aphrody-skills/src/sources.ts"],
		approxMb: 50,
		shallow: true,
	},
];

interface Args {
	root: string;
	only: ReadonlySet<string>;
	dryRun: boolean;
	update: boolean;
	json: boolean;
}

function defaultRoot(): string {
	if (process.platform === "win32") return "C:/worktree";
	const home = process.env["HOME"] ?? process.env["USERPROFILE"] ?? ".";
	return join(home, "worktree");
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = {
		root: defaultRoot(),
		only: new Set<string>(),
		dryRun: false,
		update: false,
		json: false,
	};
	for (const raw of argv) {
		if (raw.startsWith("--root=")) out.root = raw.slice("--root=".length);
		else if (raw.startsWith("--only=")) {
			out.only = new Set(
				raw
					.slice("--only=".length)
					.split(",")
					.map((s) => s.trim())
					.filter((s) => s.length > 0),
			);
		} else if (raw === "--dry-run") out.dryRun = true;
		else if (raw === "--update") out.update = true;
		else if (raw === "--json") out.json = true;
		else if (raw === "--help" || raw === "-h") {
			printHelp();
			process.exit(0);
		}
	}
	return out;
}

function printHelp(): void {
	console.log(
		[
			"setup-worktrees.ts — bootstrap aphrody external worktrees",
			"",
			"Usage:",
			"  bun run scripts/setup-worktrees.ts [flags]",
			"",
			"Flags:",
			"  --root=<path>    Override worktree root (default: C:/worktree on Windows, ~/worktree elsewhere)",
			"  --only=<csv>     Restrict to a comma-separated slug allowlist",
			"  --update         git pull every existing clone (no-op for missing)",
			"  --dry-run        Print plan without cloning",
			"  --json           Emit JSON plan + result to stdout",
			"  --help           This message",
			"",
			`Worktrees catalogued: ${WORKTREES.length} (run with --dry-run to see them).`,
		].join("\n"),
	);
}

interface RunResult {
	slug: string;
	repo: string;
	path: string;
	status: "exists" | "cloned" | "updated" | "skipped" | "failed";
	ms: number;
	error: string | null;
}

async function gitPull(dir: string): Promise<string | null> {
	const proc = Bun.spawn(["git", "-C", dir, "pull", "--ff-only"], {
		stdout: "pipe",
		stderr: "pipe",
	});
	const exit = await proc.exited;
	if (exit !== 0) {
		const stderr = await new Response(proc.stderr).text();
		return stderr.trim().slice(0, 300);
	}
	return null;
}

async function ghClone(spec: WorktreeSpec, root: string): Promise<string | null> {
	const args = ["repo", "clone", spec.repo, spec.dirName, "--"];
	if (spec.shallow) args.push("--depth=1", "--no-tags", "--filter=blob:none");
	if (spec.branch) args.push("--branch", spec.branch);
	const proc = Bun.spawn(["gh", ...args], {
		cwd: root,
		stdout: "pipe",
		stderr: "pipe",
	});
	const exit = await proc.exited;
	if (exit !== 0) {
		const stderr = await new Response(proc.stderr).text();
		return stderr.trim().slice(0, 300);
	}
	return null;
}

async function run(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));

	mkdirSync(args.root, { recursive: true });

	const selected = WORKTREES.filter(
		(w) => args.only.size === 0 || args.only.has(w.slug),
	);

	const results: RunResult[] = [];

	for (const spec of selected) {
		const dir = join(args.root, spec.dirName);
		const exists = existsSync(dir) && statSync(dir).isDirectory();
		const t0 = performance.now();

		if (exists && !args.update) {
			results.push({
				slug: spec.slug,
				repo: spec.repo,
				path: dir,
				status: "exists",
				ms: 0,
				error: null,
			});
			continue;
		}

		if (args.dryRun) {
			results.push({
				slug: spec.slug,
				repo: spec.repo,
				path: dir,
				status: exists ? "skipped" : "cloned",
				ms: 0,
				error: null,
			});
			continue;
		}

		const err = exists ? await gitPull(dir) : await ghClone(spec, args.root);
		const ms = Math.round(performance.now() - t0);

		if (err !== null) {
			results.push({
				slug: spec.slug,
				repo: spec.repo,
				path: dir,
				status: "failed",
				ms,
				error: err,
			});
			console.error(`[setup-worktrees] FAIL ${spec.slug} (${spec.repo}): ${err}`);
			continue;
		}

		results.push({
			slug: spec.slug,
			repo: spec.repo,
			path: dir,
			status: exists ? "updated" : "cloned",
			ms,
			error: null,
		});
		console.log(
			`[setup-worktrees] ${exists ? "updated" : "cloned"} ${spec.slug} (${spec.repo}) in ${ms} ms`,
		);
	}

	const manifestDir = join(resolve(import.meta.dir, ".."), "var", "data");
	mkdirSync(manifestDir, { recursive: true });
	const manifestPath = join(manifestDir, "worktrees-manifest.json");
	const manifest = {
		$schema: "https://aphrody-code.dev/schemas/worktrees-manifest/v1",
		generatedAt: new Date().toISOString(),
		root: args.root,
		args: {
			only: [...args.only].sort(),
			dryRun: args.dryRun,
			update: args.update,
		},
		results,
	};
	await writeFile(manifestPath, `${JSON.stringify(manifest, null, "\t")}\n`, "utf8");

	if (args.json) {
		console.log(JSON.stringify(manifest, null, 2));
	} else {
		const ok = results.filter((r) => r.status === "exists" || r.status === "cloned" || r.status === "updated").length;
		const fail = results.filter((r) => r.status === "failed").length;
		console.log(
			`[setup-worktrees] done  ok=${ok}  failed=${fail}  manifest=${manifestPath}`,
		);
	}

	if (results.some((r) => r.status === "failed")) {
		process.exitCode = 1;
	}
}

run().catch((err: unknown) => {
	console.error("setup-worktrees FAILED:", err);
	process.exitCode = 1;
});
