// SPDX-License-Identifier: Apache-2.0
/**
 * check-worktrees.ts
 * ==================
 *
 * CI / pre-commit gate. Walks every aphrody source file that hardcodes a
 * worktree path and verifies the path resolves on the running machine.
 *
 * Exit code:
 *   0 — every referenced worktree exists.
 *   1 — at least one missing worktree.  Tells the user which
 *       `bun run scripts/setup-worktrees.ts --only=<slug>` to fix it.
 *
 * Usage:
 *   bun run scripts/check-worktrees.ts
 *   bun run scripts/check-worktrees.ts --json
 *   bun run scripts/check-worktrees.ts --root=/path/to/worktree
 */

import { existsSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

interface ExpectedWorktree {
	readonly slug: string;
	readonly dirName: string;
	readonly consumers: readonly string[];
}

const EXPECTED: readonly ExpectedWorktree[] = [
	{ slug: "bxc", dirName: "bxc", consumers: ["scripts/bxc-mass-scrape.ts", "scripts/scrape-m3-tokens.ts"] },
	{ slug: "n2b", dirName: "n2b", consumers: ["Cargo.toml"] },
	{ slug: "open-design", dirName: "open-design", consumers: [
		"packages/aphrody-skills/src/sources.ts",
		"scripts/design-systems-import.ts",
		"scripts/design-templates-import.ts",
		"scripts/skills-harvest-open-design.ts",
	] },
	{ slug: "openclaw", dirName: "openclaw", consumers: [
		"packages/aphrody-skills/src/sources.ts",
		"packages/plugin-package-contract/",
		"scripts/openclaw-extensions-audit.ts",
	] },
	{ slug: "gemini-cli", dirName: "gemini-cli", consumers: [
		"packages/aphrody-skills/src/sources.ts",
		"packages/gemini-live-aphrody/src/auth.ts",
	] },
	{ slug: "components", dirName: "components", consumers: ["docs/audits/2026-05-17-angular-material-scrape.md"] },
	{ slug: "whisper", dirName: "whisper", consumers: ["crates/aphrody-voice-stt/src/local_whisper.rs"] },
	{ slug: "live-api-web-console", dirName: "live-api-web-console", consumers: ["packages/gemini-live-aphrody/"] },
	{ slug: "design.md", dirName: "design.md", consumers: ["DESIGN.md"] },
	{ slug: "agent-browser", dirName: "agent-browser", consumers: ["docs/audits/2026-05-17-vercel-agent-browser-vs-bxc.md"] },
	{ slug: "vercel-agent-skills", dirName: "vercel-agent-skills", consumers: ["packages/aphrody-skills/src/sources.ts"] },
	{ slug: "vercel-skills", dirName: "vercel-skills", consumers: ["packages/aphrody-skills/src/sources.ts"] },
	{ slug: "open-agents", dirName: "open-agents", consumers: ["packages/aphrody-skills/src/sources.ts"] },
];

interface Args {
	root: string;
	json: boolean;
}

function defaultRoot(): string {
	if (process.platform === "win32") return "C:/worktree";
	const home = process.env["HOME"] ?? process.env["USERPROFILE"] ?? ".";
	return join(home, "worktree");
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = { root: defaultRoot(), json: false };
	for (const raw of argv) {
		if (raw.startsWith("--root=")) out.root = raw.slice("--root=".length);
		else if (raw === "--json") out.json = true;
		else if (raw === "--help" || raw === "-h") {
			console.log("check-worktrees.ts — verify every aphrody-referenced worktree exists");
			console.log("Flags: --root=<path>  --json  --help");
			process.exit(0);
		}
	}
	return out;
}

interface CheckResult {
	slug: string;
	dirName: string;
	path: string;
	exists: boolean;
	consumers: readonly string[];
}

function main(): void {
	const args = parseArgs(process.argv.slice(2));

	const results: CheckResult[] = EXPECTED.map((spec) => {
		const path = join(args.root, spec.dirName);
		const exists = existsSync(path) && statSync(path).isDirectory();
		return { slug: spec.slug, dirName: spec.dirName, path, exists, consumers: spec.consumers };
	});

	const missing = results.filter((r) => !r.exists);

	if (args.json) {
		console.log(JSON.stringify({
			$schema: "https://aphrody-code.dev/schemas/check-worktrees/v1",
			generatedAt: new Date().toISOString(),
			root: args.root,
			total: EXPECTED.length,
			present: results.length - missing.length,
			missing: missing.length,
			results,
		}, null, 2));
	} else {
		console.log(`[check-worktrees] root=${args.root}  total=${EXPECTED.length}  present=${results.length - missing.length}  missing=${missing.length}`);
		for (const r of results) {
			const mark = r.exists ? "ok    " : "MISS  ";
			console.log(`  ${mark} ${r.slug.padEnd(22)} ${r.path}`);
		}
		if (missing.length > 0) {
			console.log("");
			console.log("Fix: bun run scripts/setup-worktrees.ts --only=" + missing.map((m) => m.slug).join(","));
		}
	}

	if (missing.length > 0) {
		process.exitCode = 1;
	}
}

main();
