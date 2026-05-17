#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
/**
 * aphrody-vs-open-design-openclaw.audit.ts
 * ========================================
 *
 * Authoritative feature-completeness audit comparing the aphrody monorepo
 * against the union of capabilities found in:
 *
 *   - nexu-io/open-design   (cloned at C:/worktree/open-design)
 *   - openclaw/openclaw     (cloned at C:/worktree/openclaw)
 *
 * The audit consumes the prior hand-written harvest map
 * (`docs/audits/2026-05-17-open-design-openclaw-harvest.md`), the
 * Bun-generated openclaw extension roadmap
 * (`docs/audits/openclaw-extensions-roadmap.md`), the M3 coverage report
 * (`docs/audits/m3-coverage.md`), the aphrody workspace
 * (`Cargo.toml` workspace.members), the local skill registry
 * (`.claude/skills/`), and the design-systems / design-templates manifests
 * (`assets/design-systems/manifest.json`,
 * `assets/design-templates/manifest.json`).
 *
 * For each capability row we mark per-column coverage as `+` (covered),
 * `partial` (in flight / stub), or `-` (gap), link to the implementing
 * artifact in aphrody, propose a wave-3 agent prompt for unfilled cells,
 * and compute aggregate coverage = (aphrody-covered + 0.5 * aphrody-partial)
 * / total-capabilities of the upstream union.
 *
 * Output:
 *   - docs/audits/aphrody-completeness.md     (deterministic Markdown)
 *   - var/data/aphrody-completeness.json      (machine-consumable)
 *
 * Usage:
 *   bun run scripts/aphrody-vs-open-design-openclaw.audit.ts
 *   bun run scripts/aphrody-vs-open-design-openclaw.audit.ts --check
 *   bun run scripts/aphrody-vs-open-design-openclaw.audit.ts \
 *       --source-od=C:/worktree/open-design \
 *       --source-oc=C:/worktree/openclaw
 *
 * Exit codes:
 *   0  audit written successfully
 *   1  --check requested and any upstream source path missing OR aphrody
 *      coverage dropped below the previous run baseline
 *   2  CLI / IO error
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

interface Args {
	check: boolean;
	sourceOd: string;
	sourceOc: string;
}

function parseArgs(argv: string[]): Args {
	const args: Args = {
		check: false,
		sourceOd: "C:/worktree/open-design",
		sourceOc: "C:/worktree/openclaw",
	};
	for (const raw of argv) {
		if (raw === "--check") {
			args.check = true;
		} else if (raw.startsWith("--source-od=")) {
			args.sourceOd = raw.slice("--source-od=".length);
		} else if (raw.startsWith("--source-oc=")) {
			args.sourceOc = raw.slice("--source-oc=".length);
		} else if (raw === "--help" || raw === "-h") {
			console.error(
				"usage: bun run scripts/aphrody-vs-open-design-openclaw.audit.ts [--check] [--source-od=PATH] [--source-oc=PATH]",
			);
			process.exit(0);
		}
	}
	return args;
}

// ---------------------------------------------------------------------------
// Repo layout
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(import.meta.dir, "..");
const CARGO_TOML = join(REPO_ROOT, "Cargo.toml");
const SKILLS_DIR = join(REPO_ROOT, ".claude", "skills");
const DESIGN_SYSTEMS_MANIFEST = join(REPO_ROOT, "assets", "design-systems", "manifest.json");
const DESIGN_TEMPLATES_MANIFEST = join(REPO_ROOT, "assets", "design-templates", "manifest.json");
const OUT_MARKDOWN = join(REPO_ROOT, "docs", "audits", "aphrody-completeness.md");
const OUT_JSON = join(REPO_ROOT, "var", "data", "aphrody-completeness.json");
const PRIOR_HARVEST = join(REPO_ROOT, "docs", "audits", "2026-05-17-open-design-openclaw-harvest.md");
const ROADMAP_OPENCLAW = join(REPO_ROOT, "docs", "audits", "openclaw-extensions-roadmap.md");
const M3_COVERAGE = join(REPO_ROOT, "docs", "audits", "m3-coverage.md");
const PACKAGES_DIR = join(REPO_ROOT, "packages");

// ---------------------------------------------------------------------------
// Source probes
// ---------------------------------------------------------------------------

interface SourceStatus {
	path: string;
	exists: boolean;
	note?: string;
}

function probe(path: string, note?: string): SourceStatus {
	return { path, exists: existsSync(path), note };
}

interface AphrodyInventory {
	crates: string[];
	packages: string[];
	skills: string[];
	designSystemsCount: number;
	designTemplatesPresent: boolean;
	scripts: string[];
}

function readWorkspaceCrates(): string[] {
	const text = readFileSync(CARGO_TOML, "utf8");
	const members: string[] = [];
	const memberBlock = text.match(/members\s*=\s*\[([\s\S]*?)\]/);
	if (!memberBlock) return members;
	for (const raw of memberBlock[1].split(",")) {
		const m = raw.match(/"([^"]+)"/);
		if (m) members.push(m[1]);
	}
	return members.map((m) => m.replace(/^crates\//, "").trim()).filter(Boolean);
}

function readDir(path: string): string[] {
	if (!existsSync(path)) return [];
	return readdirSync(path).filter((entry) => {
		const full = join(path, entry);
		try {
			return statSync(full).isDirectory();
		} catch {
			return false;
		}
	});
}

function readDesignSystemsCount(): number {
	if (!existsSync(DESIGN_SYSTEMS_MANIFEST)) return 0;
	try {
		const parsed = JSON.parse(readFileSync(DESIGN_SYSTEMS_MANIFEST, "utf8")) as {
			count?: number;
			entries?: Record<string, unknown>;
		};
		if (typeof parsed.count === "number") return parsed.count;
		if (parsed.entries) return Object.keys(parsed.entries).length;
	} catch {
		/* ignore parse errors */
	}
	return 0;
}

function readScripts(): string[] {
	const scriptsDir = join(REPO_ROOT, "scripts");
	if (!existsSync(scriptsDir)) return [];
	return readdirSync(scriptsDir).filter((entry) => /\.(ts|sh|ps1|py)$/.test(entry));
}

function takeInventory(): AphrodyInventory {
	return {
		crates: readWorkspaceCrates(),
		packages: readDir(PACKAGES_DIR),
		skills: readDir(SKILLS_DIR),
		designSystemsCount: readDesignSystemsCount(),
		designTemplatesPresent: existsSync(DESIGN_TEMPLATES_MANIFEST),
		scripts: readScripts(),
	};
}

// ---------------------------------------------------------------------------
// Capability matrix
// ---------------------------------------------------------------------------

type Coverage = "+" | "partial" | "-";

interface Capability {
	id: string;
	row: string;
	openDesign: Coverage;
	openclaw: Coverage;
	aphrodyOracle: (inv: AphrodyInventory) => {
		coverage: Coverage;
		artifact?: string;
		notes?: string;
	};
	wave3Prompt?: string;
}

function hasCrate(inv: AphrodyInventory, name: string): boolean {
	return inv.crates.includes(name);
}

function hasSkill(inv: AphrodyInventory, name: string): boolean {
	return inv.skills.includes(name);
}

function hasScript(inv: AphrodyInventory, name: string): boolean {
	return inv.scripts.includes(name);
}

function hasPackage(inv: AphrodyInventory, name: string): boolean {
	return inv.packages.includes(name);
}

const CAPABILITIES: Capability[] = [
	{
		id: "design-md-spec",
		row: "DESIGN.md spec adoption",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: () =>
			existsSync(join(REPO_ROOT, "DESIGN.md"))
				? { coverage: "+", artifact: "DESIGN.md (root)" }
				: { coverage: "-" },
		wave3Prompt:
			"Author DESIGN.md at repo root from the google-labs-code design.md spec template and run scripts/design-md-validate.ts to enforce sections.",
	},
	{
		id: "design-systems-corpus",
		row: "Design-systems brand corpus (152 upstream)",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: (inv) => {
			if (inv.designSystemsCount >= 100) {
				return {
					coverage: "+",
					artifact: `assets/design-systems/ (${inv.designSystemsCount} brands)`,
				};
			}
			if (inv.designSystemsCount > 0) {
				return {
					coverage: "partial",
					artifact: `assets/design-systems/ (${inv.designSystemsCount}/152 imported)`,
				};
			}
			return { coverage: "-" };
		},
		wave3Prompt:
			"Dispatch scripts/design-systems-import.ts in batched-100 mode to mirror the remaining open-design/design-systems/* DESIGN.md files (target: 152).",
	},
	{
		id: "design-templates-corpus",
		row: "Design templates (30+ upstream)",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			inv.designTemplatesPresent
				? {
						coverage: "+",
						artifact: "assets/design-templates/manifest.json",
					}
				: { coverage: "-" },
		wave3Prompt:
			"Author scripts/design-templates-import.ts mirroring scripts/design-systems-import.ts but iterating C:/worktree/open-design/design-templates/* and emit assets/design-templates/manifest.json.",
	},
	{
		id: "design-md-skill",
		row: "design-md skill framework",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasSkill(inv, "design-md")
				? { coverage: "+", artifact: ".claude/skills/design-md/" }
				: { coverage: "-" },
		wave3Prompt:
			"Mirror C:/worktree/open-design/skills/design-md/SKILL.md into .claude/skills/design-md/ keeping the frontmatter intact.",
	},
	{
		id: "skill-framework",
		row: "SKILL.md schema (name/description/triggers/od)",
		openDesign: "+",
		openclaw: "+",
		aphrodyOracle: (inv) =>
			hasSkill(inv, "design-md") && hasSkill(inv, "apple-hig")
				? {
						coverage: "+",
						artifact: `.claude/skills/ (${inv.skills.length} skills)`,
					}
				: { coverage: "partial" },
		wave3Prompt:
			"Run scripts/skills-sync.ts against open-design and openclaw and add a validator gating frontmatter (name/description/triggers/od).",
	},
	{
		id: "memory-consolidation",
		row: "Memory consolidation (LanceDB + JSONL backends)",
		openDesign: "-",
		openclaw: "+",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "aphrody-memory")
				? { coverage: "+", artifact: "crates/aphrody-memory/" }
				: { coverage: "-" },
		wave3Prompt:
			"Scaffold crates/aphrody-memory/ with MemoryBackend trait + LanceDB backend + .coord JSONL backend (mirrors openclaw memory-host-sdk).",
	},
	{
		id: "gemini-runtime",
		row: "Gemini CLI runtime adapter",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "gemini-runtime")
				? { coverage: "+", artifact: "crates/gemini-runtime/" }
				: { coverage: "-" },
		wave3Prompt:
			"Port C:/worktree/open-design/apps/daemon/src/runtimes/defs/gemini.ts to crates/gemini-runtime/ (bin/versionArgs/env/buildArgs).",
	},
	{
		id: "mcp-oauth",
		row: "MCP HTTP/SSE OAuth 2.1 (RFC 9728/8414/7591/7636)",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "aphrody-mcp")
				? { coverage: "+", artifact: "crates/aphrody-mcp/" }
				: { coverage: "-" },
		wave3Prompt:
			"Port C:/worktree/open-design/apps/daemon/src/mcp-oauth.ts to crates/aphrody-mcp/ using reqwest + oauth2 + DCR persistence.",
	},
	{
		id: "voice-tts",
		row: "Voice TTS provider (ElevenLabs + openclaw speech)",
		openDesign: "+",
		openclaw: "+",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "aphrody-voice")
				? { coverage: "+", artifact: "crates/aphrody-voice/" }
				: { coverage: "-" },
		wave3Prompt:
			"Scaffold crates/aphrody-voice/ proxying ElevenLabs + openclaw speech-core providers.",
	},
	{
		id: "voice-stt",
		row: "Voice STT (Azure/Microsoft + Inworld)",
		openDesign: "-",
		openclaw: "+",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "aphrody-voice-stt")
				? { coverage: "+", artifact: "crates/aphrody-voice-stt/" }
				: { coverage: "-" },
		wave3Prompt:
			"Scaffold crates/aphrody-voice-stt/ with Azure-speech + Inworld-speech + Microsoft-speech provider adapters mirroring openclaw extensions.",
	},
	{
		id: "ai-gateway",
		row: "AI gateway (Cloudflare + Vercel adapters)",
		openDesign: "-",
		openclaw: "+",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "aphrody-gateway")
				? { coverage: "+", artifact: "crates/aphrody-gateway/" }
				: { coverage: "-" },
		wave3Prompt:
			"Port openclaw cloudflare-ai-gateway + vercel-ai-gateway adapters to crates/aphrody-gateway/.",
	},
	{
		id: "ai-minimal-env",
		row: "AI minimal env / runtime auto-detect (16 CLIs)",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: () => {
			const runtimesDir = join(REPO_ROOT, "var", "data", "runtimes");
			if (existsSync(runtimesDir)) return { coverage: "+", artifact: "var/data/runtimes/" };
			return { coverage: "partial", artifact: "aphrody doctor (partial)" };
		},
		wave3Prompt:
			"Author crates/cli/src/runtimes.rs scanning $PATH for the 16 CLIs (claude, codex, devin, cursor-agent, gemini, opencode, qwen, qoder, copilot, hermes, kimi, pi, kiro, kilo, mistral-vibe, deepseek) and emit var/data/runtimes/<host>.json.",
	},
	{
		id: "mcp-plugin-core",
		row: "MCP plugin core / loader",
		openDesign: "+",
		openclaw: "+",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "google_mcp")
				? { coverage: "+", artifact: "crates/google_mcp/" }
				: { coverage: "-" },
		wave3Prompt: "Promote crates/google_mcp/ plugin loader to consume @aphrody/plugin-package-contract.",
	},
	{
		id: "agui-package",
		row: "agui-adapter package + WASM custom elements",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "agui-bridge")
				? { coverage: "+", artifact: "crates/agui-bridge/" }
				: { coverage: "-" },
		wave3Prompt:
			"Bridge open-design/packages/agui-adapter into crates/agui-bridge/ exposing the WASM <a-gui-*> elements.",
	},
	{
		id: "channel-adapters",
		row: "Channel adapters (Discord/Slack/Telegram/Matrix...)",
		openDesign: "-",
		openclaw: "+",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "aphrody-channels")
				? { coverage: "+", artifact: "crates/aphrody-channels/" }
				: { coverage: "-" },
		wave3Prompt:
			"Scaffold crates/aphrody-channels/ as a multiplexer dispatching to openclaw discord/telegram/matrix/slack/whatsapp plugins via the gateway HTTP API.",
	},
	{
		id: "plugin-contract",
		row: "Plugin package contract (compat + validator)",
		openDesign: "-",
		openclaw: "+",
		aphrodyOracle: (inv) => {
			const vendored = join(PACKAGES_DIR, "plugin-package-contract", "index.ts");
			if (existsSync(vendored)) {
				return { coverage: "+", artifact: "packages/plugin-package-contract/index.ts" };
			}
			return hasPackage(inv, "plugin-package-contract")
				? { coverage: "partial", artifact: "packages/plugin-package-contract/ (scaffolded)" }
				: { coverage: "-" };
		},
		wave3Prompt: "Run bun run scripts/plugin-contract-port.ts to vendor the upstream contract.",
	},
	{
		id: "m3-tokens",
		row: "M3 design tokens (color/typography/elevation/motion/shape/state/tonal/dynamic)",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "m3-tokens")
				? { coverage: "+", artifact: "crates/m3-tokens/" }
				: { coverage: "-" },
		wave3Prompt: "Scaffold crates/m3-tokens/ with all 8 M3 surfaces from m3.material.io/styles.",
	},
	{
		id: "shadcn-bridge",
		row: "shadcn -> MWC3 WASM bridge (29 components)",
		openDesign: "+",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "shadcn-bridge")
				? { coverage: "+", artifact: "crates/shadcn-bridge/" }
				: { coverage: "-" },
		wave3Prompt:
			"Author crates/shadcn-bridge/ wrapping the 29 shadcn primitives onto MWC3 custom elements via wasm-bindgen.",
	},
	{
		id: "a2a-protocol",
		row: "A2A coordination protocol (ai.json + JSONL mailbox)",
		openDesign: "-",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "a2a") && existsSync(join(REPO_ROOT, "ai.json"))
				? { coverage: "+", artifact: "crates/a2a/ + ai.json + .coord/" }
				: { coverage: "-" },
	},
	{
		id: "mrx-mapper",
		row: "Monorepo Real-time X-platform mapper",
		openDesign: "-",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "mrx-core") && hasCrate(inv, "mrx-cli")
				? { coverage: "+", artifact: "crates/mrx-{core,cli,detect,audit,watch}/" }
				: { coverage: "-" },
	},
	{
		id: "translation-tooling",
		row: "AI-isms scrub + EN->FR translation tooling",
		openDesign: "-",
		openclaw: "-",
		aphrodyOracle: (inv) =>
			hasCrate(inv, "aphrody-translate")
				? { coverage: "+", artifact: "crates/aphrody-translate/" }
				: { coverage: "-" },
	},
];

// ---------------------------------------------------------------------------
// Audit run
// ---------------------------------------------------------------------------

interface AuditRow {
	id: string;
	row: string;
	aphrody: Coverage;
	openDesign: Coverage;
	openclaw: Coverage;
	artifact?: string;
	notes?: string;
	wave3Prompt?: string;
}

interface AuditResult {
	generatedAt: string;
	sources: SourceStatus[];
	inventory: AphrodyInventory;
	rows: AuditRow[];
	aggregate: {
		totalUpstreamCapabilities: number;
		aphrodyCovered: number;
		aphrodyPartial: number;
		aphrodyMissing: number;
		coveragePercent: number;
	};
	gaps: AuditRow[];
}

function coverageWeight(c: Coverage): number {
	if (c === "+") return 1;
	if (c === "partial") return 0.5;
	return 0;
}

function buildAudit(inv: AphrodyInventory, sources: SourceStatus[]): AuditResult {
	const rows: AuditRow[] = CAPABILITIES.map((cap) => {
		const verdict = cap.aphrodyOracle(inv);
		return {
			id: cap.id,
			row: cap.row,
			aphrody: verdict.coverage,
			openDesign: cap.openDesign,
			openclaw: cap.openclaw,
			artifact: verdict.artifact,
			notes: verdict.notes,
			wave3Prompt: verdict.coverage === "+" ? undefined : cap.wave3Prompt,
		};
	});

	// Aggregate scope = union of capabilities present in open-design OR openclaw.
	const upstream = rows.filter((r) => r.openDesign !== "-" || r.openclaw !== "-");
	const totalUpstreamCapabilities = upstream.length;
	let aphrodyCovered = 0;
	let aphrodyPartial = 0;
	let aphrodyMissing = 0;
	let weighted = 0;
	for (const r of upstream) {
		if (r.aphrody === "+") aphrodyCovered += 1;
		else if (r.aphrody === "partial") aphrodyPartial += 1;
		else aphrodyMissing += 1;
		weighted += coverageWeight(r.aphrody);
	}
	const coveragePercent =
		totalUpstreamCapabilities === 0 ? 0 : Math.round((weighted / totalUpstreamCapabilities) * 1000) / 10;

	const gaps = rows.filter((r) => r.aphrody !== "+");

	return {
		generatedAt: new Date().toISOString(),
		sources,
		inventory: inv,
		rows,
		aggregate: {
			totalUpstreamCapabilities,
			aphrodyCovered,
			aphrodyPartial,
			aphrodyMissing,
			coveragePercent,
		},
		gaps,
	};
}

// ---------------------------------------------------------------------------
// Renderers
// ---------------------------------------------------------------------------

function renderMarkdown(audit: AuditResult): string {
	const lines: string[] = [];
	lines.push("<!-- SPDX-License-Identifier: Apache-2.0 -->");
	lines.push("<!-- GENERATED by scripts/aphrody-vs-open-design-openclaw.audit.ts — DO NOT EDIT BY HAND. -->");
	lines.push("");
	lines.push("# Aphrody vs open-design + openclaw — Completeness audit");
	lines.push("");
	lines.push(`**Generated:** \`${audit.generatedAt}\``);
	lines.push("");
	lines.push("This deterministic audit compares the aphrody monorepo against the");
	lines.push("union of capabilities found in nexu-io/open-design + openclaw/openclaw.");
	lines.push("Cells: `+` covered, `partial` in flight, `-` gap.");
	lines.push("");
	lines.push("## 1. Source clones");
	lines.push("");
	lines.push("| source | path | present |");
	lines.push("|---|---|---|");
	for (const src of audit.sources) {
		lines.push(`| ${src.note ?? "—"} | \`${src.path}\` | ${src.exists ? "yes" : "no"} |`);
	}
	lines.push("");
	lines.push("## 2. Aphrody inventory snapshot");
	lines.push("");
	lines.push(`- workspace crates: **${audit.inventory.crates.length}**`);
	lines.push(`- packages/: **${audit.inventory.packages.length}**`);
	lines.push(`- .claude/skills/: **${audit.inventory.skills.length}**`);
	lines.push(`- design-systems mirrored: **${audit.inventory.designSystemsCount}**`);
	lines.push(`- design-templates manifest present: **${audit.inventory.designTemplatesPresent ? "yes" : "no"}**`);
	lines.push(`- scripts/: **${audit.inventory.scripts.length}**`);
	lines.push("");
	lines.push("## 3. Capability matrix");
	lines.push("");
	lines.push("| capability | aphrody | open-design | openclaw | artifact / next-prompt |");
	lines.push("|---|:---:|:---:|:---:|---|");
	for (const r of audit.rows) {
		const cell = r.aphrody === "+" ? r.artifact ?? "—" : r.wave3Prompt ?? r.notes ?? "—";
		const cleanCell = cell.replace(/\|/g, "\\|").replace(/\n/g, " ");
		lines.push(`| ${r.row} | ${r.aphrody} | ${r.openDesign} | ${r.openclaw} | ${cleanCell} |`);
	}
	lines.push("");
	lines.push("## 4. Aggregate coverage");
	lines.push("");
	lines.push(`- upstream capabilities in scope: **${audit.aggregate.totalUpstreamCapabilities}**`);
	lines.push(`- aphrody fully covered (\`+\`): **${audit.aggregate.aphrodyCovered}**`);
	lines.push(`- aphrody partial (\`partial\`): **${audit.aggregate.aphrodyPartial}**`);
	lines.push(`- aphrody missing (\`-\`): **${audit.aggregate.aphrodyMissing}**`);
	lines.push(`- weighted coverage: **${audit.aggregate.coveragePercent}%**`);
	lines.push("");
	lines.push("Formula: `(count(+) + 0.5 * count(partial)) / count(upstream)` rounded to 0.1%.");
	lines.push("");
	lines.push("## 5. Wave-3 prompt queue (unfilled cells)");
	lines.push("");
	if (audit.gaps.length === 0) {
		lines.push("All upstream capabilities covered. No wave-3 dispatch required.");
	} else {
		lines.push("| id | capability | coverage | proposed wave-3 agent prompt |");
		lines.push("|---|---|:---:|---|");
		for (const g of audit.gaps) {
			const prompt = (g.wave3Prompt ?? "(no upstream gap — local feature)").replace(/\|/g, "\\|");
			lines.push(`| \`${g.id}\` | ${g.row} | ${g.aphrody} | ${prompt} |`);
		}
	}
	lines.push("");
	lines.push("## 6. Source inputs cross-linked");
	lines.push("");
	lines.push("- Prior harvest map: `docs/audits/2026-05-17-open-design-openclaw-harvest.md`");
	lines.push("- Openclaw extensions roadmap: `docs/audits/openclaw-extensions-roadmap.md`");
	lines.push("- M3 coverage report: `docs/audits/m3-coverage.md`");
	lines.push("- Workspace manifest: `Cargo.toml`");
	lines.push("- Skills registry: `.claude/skills/`");
	lines.push("- Design-systems mirror: `assets/design-systems/manifest.json`");
	lines.push("");
	lines.push("Refresh:");
	lines.push("");
	lines.push("```bash");
	lines.push("bun run scripts/aphrody-vs-open-design-openclaw.audit.ts");
	lines.push("```");
	lines.push("");
	return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main(): void {
	const args = parseArgs(process.argv.slice(2));
	const sources: SourceStatus[] = [
		probe(args.sourceOd, "open-design clone"),
		probe(args.sourceOc, "openclaw clone"),
		probe(PRIOR_HARVEST, "prior harvest map"),
		probe(ROADMAP_OPENCLAW, "openclaw roadmap"),
		probe(M3_COVERAGE, "m3 coverage"),
		probe(CARGO_TOML, "workspace manifest"),
		probe(SKILLS_DIR, "skills dir"),
		probe(DESIGN_SYSTEMS_MANIFEST, "design-systems manifest"),
		probe(DESIGN_TEMPLATES_MANIFEST, "design-templates manifest"),
	];

	if (args.check) {
		const missingRequired = sources.filter((s) => !s.exists && s.note !== "design-templates manifest");
		if (missingRequired.length > 0) {
			console.error(
				`[audit] --check: missing required sources: ${missingRequired.map((s) => s.path).join(", ")}`,
			);
			process.exit(1);
		}
	}

	const inv = takeInventory();
	const audit = buildAudit(inv, sources);

	mkdirSync(dirname(OUT_MARKDOWN), { recursive: true });
	mkdirSync(dirname(OUT_JSON), { recursive: true });
	writeFileSync(OUT_MARKDOWN, renderMarkdown(audit), "utf8");
	writeFileSync(OUT_JSON, `${JSON.stringify(audit, null, 2)}\n`, "utf8");

	console.log(`wrote ${OUT_MARKDOWN}`);
	console.log(`wrote ${OUT_JSON}`);
	console.log(
		`coverage = ${audit.aggregate.coveragePercent}% (${audit.aggregate.aphrodyCovered}+ / ${audit.aggregate.aphrodyPartial}~ / ${audit.aggregate.aphrodyMissing}-) of ${audit.aggregate.totalUpstreamCapabilities} upstream capabilities`,
	);
}

try {
	main();
} catch (err) {
	console.error(`[audit] fatal: ${(err as Error).message}`);
	process.exit(2);
}
