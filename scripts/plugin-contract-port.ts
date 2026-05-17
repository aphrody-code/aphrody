// SPDX-License-Identifier: Apache-2.0
// #!/usr/bin/env bun
/**
 * plugin-contract-port.ts
 * =======================
 *
 * Vendors the upstream `@openclaw/plugin-package-contract` TypeScript module
 * as `packages/plugin-package-contract/index.ts` inside aphrody, with a
 * canonical SPDX header, upstream attribution comments, and a companion
 * `README.md` documenting the contract surface + link back to the source
 * path. The vendored copy is byte-faithful to the original interface
 * declarations and helper functions.
 *
 * Why vendor? aphrody integrates the OpenClaw plugin packaging contract as
 * the substrate for its own plugin discovery / compatibility validation
 * (cf. `crates/aphrody-mcp`, `crates/aphrody-gateway`, and the Wave-2
 * AGUI bridge). Path-deps onto `C:/worktree/openclaw` are unsafe for a
 * release-quality artifact — a pinned in-tree copy with SPDX + attribution
 * is the correct supply-chain posture.
 *
 * Usage:
 *   bun run scripts/plugin-contract-port.ts              # write vendored copy
 *   bun run scripts/plugin-contract-port.ts --check      # exit 1 if upstream missing
 *   bun run scripts/plugin-contract-port.ts --source-oc=<path>
 *
 * Exit codes:
 *   0  vendored file + README written successfully
 *   1  --check requested and upstream source unreadable
 *   2  CLI / IO error
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

// ---------------------------------------------------------------------------
// CLI parsing
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
				"usage: bun run scripts/plugin-contract-port.ts [--check] [--source-oc=PATH] [--source-od=PATH]",
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
const OUT_DIR = join(REPO_ROOT, "packages", "plugin-package-contract");
const OUT_TS = join(OUT_DIR, "index.ts");
const OUT_README = join(OUT_DIR, "README.md");
const OUT_PKG_JSON = join(OUT_DIR, "package.json");

// ---------------------------------------------------------------------------
// Vendor pipeline
// ---------------------------------------------------------------------------

interface VendorPlan {
	upstreamPath: string;
	upstreamSource: string;
	upstreamPkgJsonPath: string;
	upstreamPkgJson: string;
}

function loadUpstream(args: Args): VendorPlan {
	const upstreamPath = join(
		args.sourceOc,
		"packages",
		"plugin-package-contract",
		"src",
		"index.ts",
	);
	const upstreamPkgJsonPath = join(
		args.sourceOc,
		"packages",
		"plugin-package-contract",
		"package.json",
	);
	if (!existsSync(upstreamPath)) {
		throw new Error(`upstream contract not found at ${upstreamPath}`);
	}
	if (!existsSync(upstreamPkgJsonPath)) {
		throw new Error(`upstream package.json not found at ${upstreamPkgJsonPath}`);
	}
	return {
		upstreamPath,
		upstreamSource: readFileSync(upstreamPath, "utf8"),
		upstreamPkgJsonPath,
		upstreamPkgJson: readFileSync(upstreamPkgJsonPath, "utf8"),
	};
}

const SPDX = "// SPDX-License-Identifier: Apache-2.0";

const VENDOR_BANNER = `${SPDX}
//
// Vendored from upstream openclaw/openclaw
//   path:    packages/plugin-package-contract/src/index.ts
//   project: https://github.com/openclaw/openclaw
//   licence: MIT (Copyright (c) 2025 Peter Steinberger)
//
// Re-published in aphrody under Apache-2.0 with explicit attribution. The
// upstream MIT licence is compatible with Apache-2.0 redistribution; the
// original copyright notice above must be preserved per the MIT terms.
//
// The TS interface declarations and helper functions below are reproduced
// verbatim from the upstream source as of 2026-05-17. Refresh by re-running
//
//     bun run scripts/plugin-contract-port.ts
//
// DO NOT EDIT BY HAND — regenerate from upstream instead.
// ----------------------------------------------------------------------------
`;

function renderVendored(source: string): string {
	return `${VENDOR_BANNER}\n${source.trimEnd()}\n`;
}

function renderReadme(plan: VendorPlan): string {
	return `<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody/plugin-package-contract

In-tree mirror of [openclaw/openclaw](https://github.com/openclaw/openclaw)'s
\`@openclaw/plugin-package-contract\` package, vendored at
\`${plan.upstreamPath.replace(/\\/g, "/")}\` as of 2026-05-17.

The contract describes the JSON shape an external plugin's \`package.json\`
must expose under its \`openclaw\` block so the host runtime can:

1. Validate that the plugin declares the required compatibility fields.
2. Normalise the declared compatibility into a stable
   \`ExternalPluginCompatibility\` record (used by the registry + gateway).
3. Return per-field validation issues for installer UX.

## Exports

| Symbol | Purpose |
|---|---|
| \`ExternalPluginCompatibility\` | Stable shape returned by \`normalizeExternalPluginCompatibility\`. |
| \`ExternalPluginValidationIssue\` | \`{ fieldPath, message }\` pair surfaced by the validator. |
| \`ExternalCodePluginValidationResult\` | Combined \`{ compatibility, issues }\` payload. |
| \`EXTERNAL_CODE_PLUGIN_REQUIRED_FIELD_PATHS\` | Tuple of dot-paths required for a code plugin. |
| \`normalizeExternalPluginCompatibility(packageJson)\` | Pure normaliser. |
| \`listMissingExternalCodePluginFieldPaths(packageJson)\` | Returns the missing required paths. |
| \`validateExternalCodePluginPackageJson(packageJson)\` | High-level validator. |

## Licence

- Upstream licence: MIT, Copyright (c) 2025 Peter Steinberger.
- Aphrody redistribution: Apache-2.0 with upstream attribution preserved
  in the source banner of \`index.ts\` (see SPDX header).

## Refresh

\`\`\`bash
bun run scripts/plugin-contract-port.ts
\`\`\`

Pass \`--source-oc=<path>\` to override the upstream clone location
(defaults to \`C:/worktree/openclaw\`).

Use \`--check\` in CI to fail-fast when the upstream clone is missing.

## Aphrody consumers

- \`crates/aphrody-mcp\` — plugin registry compatibility gate.
- \`crates/aphrody-gateway\` — provider plugin handshake.
- \`packages/agui-adapter\` (Wave-2) — AGUI runtime plugin loader.
`;
}

function renderPackageJson(): string {
	return `${JSON.stringify(
		{
			name: "@aphrody/plugin-package-contract",
			version: "0.0.0-vendored",
			private: true,
			type: "module",
			description:
				"Vendored mirror of @openclaw/plugin-package-contract — external plugin compatibility shape + validator (MIT upstream, Apache-2.0 redistribution).",
			license: "Apache-2.0",
			exports: {
				".": "./index.ts",
			},
			aphrody: {
				vendoredFrom: "openclaw/openclaw#packages/plugin-package-contract",
				upstreamLicense: "MIT",
				upstreamCopyright: "Copyright (c) 2025 Peter Steinberger",
				refreshScript: "scripts/plugin-contract-port.ts",
			},
		},
		null,
		2,
	)}\n`;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main(): void {
	const args = parseArgs(process.argv.slice(2));
	let plan: VendorPlan;
	try {
		plan = loadUpstream(args);
	} catch (err) {
		if (args.check) {
			console.error(`[plugin-contract-port] check failed: ${(err as Error).message}`);
			process.exit(1);
		}
		console.error(`[plugin-contract-port] error: ${(err as Error).message}`);
		process.exit(2);
	}
	mkdirSync(OUT_DIR, { recursive: true });
	writeFileSync(OUT_TS, renderVendored(plan.upstreamSource), "utf8");
	writeFileSync(OUT_README, renderReadme(plan), "utf8");
	writeFileSync(OUT_PKG_JSON, renderPackageJson(), "utf8");
	const written = [OUT_TS, OUT_README, OUT_PKG_JSON];
	for (const path of written) {
		console.log(`wrote ${path}`);
	}
	console.log(`vendored ${plan.upstreamSource.length} bytes from ${plan.upstreamPath}`);
}

try {
	main();
} catch (err) {
	console.error(`[plugin-contract-port] fatal: ${(err as Error).message}`);
	process.exit(2);
}
