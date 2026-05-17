#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
/**
 * runtimes-detect.ts
 * ==================
 *
 * Detect every AI coding-agent CLI installed on PATH plus aphrody's own
 * toolchain dependencies. The runtimes table mirrors the 16 agents that
 * open-design's daemon auto-detects (see
 * `C:/worktree/open-design/apps/daemon/src/runtimes/defs/`) and adds the
 * 8 aphrody-internal binaries we rely on (bun, cargo, gh, gemini, msedge,
 * lightpanda, bxc, n2b).
 *
 * Produces a structured JSON manifest so `aphrody runtimes` can render it
 * and downstream tooling can consume it without re-probing PATH.
 *
 * Usage:
 *   bun run scripts/runtimes-detect.ts                            # pretty table
 *   bun run scripts/runtimes-detect.ts --json                     # JSON to stdout
 *   bun run scripts/runtimes-detect.ts --output=path/manifest.json
 *   bun run scripts/runtimes-detect.ts --check                    # exit 1 if bun/cargo/gh missing
 *
 * Default output path: var/data/runtimes/manifest.json (gitignored cache).
 */

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_OUTPUT = join(REPO_ROOT, "var", "data", "runtimes", "manifest.json");
const PROBE_TIMEOUT_MS = 5_000;
const MANDATORY_FOR_CHECK = ["bun", "cargo", "gh"] as const;

type Category = "coding-agent" | "toolchain" | "browser" | "aphrody-internal";

interface RuntimeSpec {
	id: string;
	name: string;
	bin: string;
	fallbackBins?: string[];
	versionArgs: string[];
	category: Category;
	source: "open-design" | "aphrody";
}

// Sourced 1:1 from C:/worktree/open-design/apps/daemon/src/runtimes/defs/*.ts
// (claude, codex, copilot, cursor-agent, deepseek, devin, gemini, hermes,
// kilo, kimi, kiro, opencode, pi, qoder, qwen, vibe). Update both sides
// when open-design adds a new runtime.
const OPEN_DESIGN_RUNTIMES: RuntimeSpec[] = [
	{
		id: "claude",
		name: "Claude Code",
		bin: "claude",
		fallbackBins: ["openclaude"],
		versionArgs: ["--version"],
		category: "coding-agent",
		source: "open-design",
	},
	{ id: "codex", name: "Codex CLI", bin: "codex", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "copilot", name: "GitHub Copilot CLI", bin: "copilot", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "cursor-agent", name: "Cursor Agent", bin: "cursor-agent", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "deepseek", name: "DeepSeek TUI", bin: "deepseek", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "devin", name: "Devin for Terminal", bin: "devin", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "gemini", name: "Gemini CLI", bin: "gemini", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "hermes", name: "Hermes", bin: "hermes", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "kilo", name: "Kilo", bin: "kilo", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "kimi", name: "Kimi CLI", bin: "kimi", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "kiro", name: "Kiro CLI", bin: "kiro-cli", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{
		id: "opencode",
		name: "OpenCode",
		bin: "opencode-cli",
		fallbackBins: ["opencode"],
		versionArgs: ["--version"],
		category: "coding-agent",
		source: "open-design",
	},
	{ id: "pi", name: "Pi", bin: "pi", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "qoder", name: "Qoder CLI", bin: "qodercli", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "qwen", name: "Qwen Code", bin: "qwen", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
	{ id: "vibe", name: "Mistral Vibe CLI", bin: "vibe-acp", versionArgs: ["--version"], category: "coding-agent", source: "open-design" },
];

// Aphrody-internal binaries. `gemini` lives in the open-design block above —
// we do not duplicate it here even though it is also an aphrody dep.
const APHRODY_RUNTIMES: RuntimeSpec[] = [
	{ id: "bun", name: "Bun runtime", bin: "bun", versionArgs: ["--version"], category: "toolchain", source: "aphrody" },
	{ id: "cargo", name: "Rust cargo", bin: "cargo", versionArgs: ["--version"], category: "toolchain", source: "aphrody" },
	{ id: "gh", name: "GitHub CLI", bin: "gh", versionArgs: ["--version"], category: "toolchain", source: "aphrody" },
	{
		id: "msedge",
		name: "Microsoft Edge",
		bin: "msedge",
		// On Windows the binary lives in Program Files; Bun.which only checks PATH.
		// We list the alternative basename here for parity — the absolute-path
		// fallback in resolveBin() handles the real install location.
		fallbackBins: ["msedge.exe"],
		versionArgs: ["--version"],
		category: "browser",
		source: "aphrody",
	},
	{ id: "lightpanda", name: "Lightpanda browser", bin: "lightpanda", versionArgs: ["--version"], category: "browser", source: "aphrody" },
	{ id: "bxc", name: "bxc scraper", bin: "bxc", versionArgs: ["--version"], category: "aphrody-internal", source: "aphrody" },
	{ id: "n2b", name: "n2b node→bun migrator", bin: "n2b", versionArgs: ["--version"], category: "aphrody-internal", source: "aphrody" },
];

const RUNTIMES: RuntimeSpec[] = [...OPEN_DESIGN_RUNTIMES, ...APHRODY_RUNTIMES];

// Known absolute fallback paths probed when PATH lookup fails. Keyed by
// runtime id. Keeps the cross-platform contract: the manifest still records
// `found: true` for Edge on Windows users who never `setx PATH` for it.
const ABSOLUTE_FALLBACKS: Record<string, string[]> = {
	msedge: [
		"C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
		"C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
		"/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
		"/usr/bin/microsoft-edge",
	],
};

interface DetectedRuntime {
	id: string;
	name: string;
	bin: string;
	resolvedBin: string | null;
	category: Category;
	source: "open-design" | "aphrody";
	found: boolean;
	path: string | null;
	version: string | null;
	ok: boolean;
	error: string | null;
}

interface Manifest {
	schemaVersion: 1;
	generatedAt: string;
	platform: NodeJS.Platform;
	arch: string;
	runtimeCount: number;
	foundCount: number;
	runtimes: DetectedRuntime[];
}

interface Args {
	emitJson: boolean;
	outputPath: string | null;
	checkMandatory: boolean;
	writeDefault: boolean;
}

function parseArgs(argv: readonly string[]): Args {
	const out: Args = {
		emitJson: false,
		outputPath: null,
		checkMandatory: false,
		writeDefault: false,
	};
	for (const raw of argv) {
		if (raw === "--json") out.emitJson = true;
		else if (raw === "--check") out.checkMandatory = true;
		else if (raw === "--write") {
			out.writeDefault = true;
		} else if (raw.startsWith("--output=")) {
			out.outputPath = resolve(raw.slice("--output=".length));
		} else if (raw === "-h" || raw === "--help") {
			printUsage();
			process.exit(0);
		} else {
			process.stderr.write(`runtimes-detect: unknown argument '${raw}'\n`);
			printUsage();
			process.exit(2);
		}
	}
	return out;
}

function printUsage(): void {
	process.stderr.write(
		[
			"Usage: bun run scripts/runtimes-detect.ts [options]",
			"  --json              Emit manifest JSON to stdout (skips pretty table)",
			"  --output=PATH       Write manifest JSON to PATH",
			"  --write             Write to default path (var/data/runtimes/manifest.json)",
			"  --check             Exit 1 if any of [bun, cargo, gh] is missing",
			"  -h, --help          Show this help",
			"",
		].join("\n"),
	);
}

function resolveBin(spec: RuntimeSpec): string | null {
	const candidates: string[] = [spec.bin, ...(spec.fallbackBins ?? [])];
	for (const cand of candidates) {
		const resolved = Bun.which(cand);
		if (resolved) return resolved;
	}
	const abs = ABSOLUTE_FALLBACKS[spec.id] ?? [];
	for (const candidate of abs) {
		if (Bun.file(candidate).size > 0) {
			return candidate;
		}
	}
	return null;
}

async function probeVersion(
	resolvedBin: string,
	versionArgs: readonly string[],
): Promise<{ version: string | null; ok: boolean; error: string | null }> {
	const ac = new AbortController();
	const timeout = setTimeout(() => ac.abort(), PROBE_TIMEOUT_MS);
	try {
		const proc = Bun.spawn([resolvedBin, ...versionArgs], {
			stdout: "pipe",
			stderr: "pipe",
			stdin: "ignore",
			signal: ac.signal,
		});
		const [stdoutText, stderrText, exitCode] = await Promise.all([
			new Response(proc.stdout).text(),
			new Response(proc.stderr).text(),
			proc.exited,
		]);
		clearTimeout(timeout);
		const blob = `${stdoutText}\n${stderrText}`;
		const firstNonEmpty = blob
			.split(/\r?\n/)
			.map((line) => line.trim())
			.find((line) => line.length > 0);
		if (exitCode !== 0 && !firstNonEmpty) {
			return { version: null, ok: false, error: `exit ${exitCode}` };
		}
		return { version: firstNonEmpty ?? null, ok: exitCode === 0, error: exitCode === 0 ? null : `exit ${exitCode}` };
	} catch (err) {
		clearTimeout(timeout);
		const message = err instanceof Error ? err.message : String(err);
		const isTimeout = ac.signal.aborted || /aborted/i.test(message);
		return { version: null, ok: false, error: isTimeout ? `timeout after ${PROBE_TIMEOUT_MS}ms` : message };
	}
}

async function detectAll(): Promise<DetectedRuntime[]> {
	const probes = RUNTIMES.map(async (spec): Promise<DetectedRuntime> => {
		const resolved = resolveBin(spec);
		if (!resolved) {
			return {
				id: spec.id,
				name: spec.name,
				bin: spec.bin,
				resolvedBin: null,
				category: spec.category,
				source: spec.source,
				found: false,
				path: null,
				version: null,
				ok: false,
				error: "not found on PATH",
			};
		}
		const { version, ok, error } = await probeVersion(resolved, spec.versionArgs);
		return {
			id: spec.id,
			name: spec.name,
			bin: spec.bin,
			resolvedBin: resolved,
			category: spec.category,
			source: spec.source,
			found: true,
			path: resolved,
			version,
			ok,
			error,
		};
	});
	return Promise.all(probes);
}

function buildManifest(runtimes: DetectedRuntime[]): Manifest {
	return {
		schemaVersion: 1,
		generatedAt: new Date().toISOString(),
		platform: process.platform,
		arch: process.arch,
		runtimeCount: runtimes.length,
		foundCount: runtimes.filter((r) => r.found).length,
		runtimes,
	};
}

function pad(s: string, n: number): string {
	if (s.length >= n) return s.slice(0, n);
	return s + " ".repeat(n - s.length);
}

function renderTable(runtimes: readonly DetectedRuntime[]): string {
	const lines: string[] = [];
	const head = `${pad("ID", 20)}| ${pad("bin", 20)}| ${pad("version", 30)}| found`;
	lines.push(head);
	lines.push("-".repeat(20) + "|" + "-".repeat(21) + "|" + "-".repeat(31) + "|------");
	for (const r of runtimes) {
		const version = r.found ? (r.version ?? (r.ok ? "(empty)" : `err: ${r.error ?? "?"}`)) : "-";
		const truncated = version.length > 30 ? `${version.slice(0, 27)}...` : version;
		lines.push(`${pad(r.id, 20)}| ${pad(r.bin, 20)}| ${pad(truncated, 30)}| ${r.found ? "yes" : "no"}`);
	}
	return lines.join("\n");
}

async function writeManifest(path: string, manifest: Manifest): Promise<void> {
	await mkdir(dirname(path), { recursive: true });
	await writeFile(path, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));
	const runtimes = await detectAll();
	const manifest = buildManifest(runtimes);

	const targetOutput = args.outputPath ?? (args.writeDefault ? DEFAULT_OUTPUT : null);
	if (targetOutput) {
		await writeManifest(targetOutput, manifest);
		if (!args.emitJson) {
			process.stderr.write(`runtimes-detect: wrote ${targetOutput}\n`);
		}
	}

	if (args.emitJson) {
		process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
	} else {
		process.stdout.write(`${renderTable(runtimes)}\n`);
		process.stdout.write(
			`\nplatform=${manifest.platform} arch=${manifest.arch} found=${manifest.foundCount}/${manifest.runtimeCount}\n`,
		);
	}

	if (args.checkMandatory) {
		const missing = MANDATORY_FOR_CHECK.filter((id) => {
			const r = runtimes.find((x) => x.id === id);
			return !r || !r.found;
		});
		if (missing.length > 0) {
			process.stderr.write(`runtimes-detect: mandatory runtimes missing: ${missing.join(", ")}\n`);
			process.exit(1);
		}
	}
}

await main();
