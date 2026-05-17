// SPDX-License-Identifier: Apache-2.0
// #!/usr/bin/env bun
/**
 * bunnize-gemini-cli.ts — JSON-aware Node → Bun migration of every
 * package.json under packages/gemini-cli/.
 *
 * Surgical: only touches the `scripts` object — never dep keys, dep versions,
 * `lint-staged`, `overrides`, or any other field. Idempotent: skips commands
 * already prefixed by `bun ` / `bunx `.
 *
 *     bun run scripts/bunnize-gemini-cli.ts
 *     bun run scripts/bunnize-gemini-cli.ts --dry-run
 */

import { readdir, readFile, writeFile } from "node:fs/promises";
import { join, relative } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const TARGET_ROOT = join(REPO_ROOT, "packages", "gemini-cli");
const DRY = process.argv.includes("--dry-run");

interface Sub { from: RegExp; to: string | ((m: string, ...g: string[]) => string); label: string }

const SUBS: Sub[] = [
    // 1. Drop cross-env wrapper (bun runs scripts via cross-platform shell)
    { from: /\bcross-env\s+/g, to: "", label: "drop cross-env" },
    // 2. tsx X → bun X (bun runs TS natively)
    { from: /\btsx\s+/g, to: "bun ", label: "tsx→bun" },
    // 3. npm run X → bun run X
    { from: /\bnpm\s+run\s+/g, to: "bun run ", label: "npm run→bun run" },
    // 4. npm ci → bun install --frozen-lockfile
    { from: /\bnpm\s+ci\b/g, to: "bun install --frozen-lockfile", label: "npm ci→bun install --frozen" },
    // 5. npm install / npm i → bun install
    { from: /\bnpm\s+install\b/g, to: "bun install", label: "npm install→bun install" },
    { from: /\bnpm\s+i\b/g, to: "bun install", label: "npm i→bun install" },
    // 6. npx X → bunx X
    { from: /\bnpx\s+/g, to: "bunx ", label: "npx→bunx" },
    // 7. Any 'node ' invocation in script command → 'bun '
    //    Boundary: `\b`, only when followed by a flag, path, or word arg.
    //    Excludes 'node:' (module import prefix) and 'node ' inside paths.
    { from: /\bnode\s+(?=--?[\w-]+|[\w./])/g, to: "bun ", label: "node X→bun X" },
    // 8. Bare 'vitest ' command at start of script (or after && / ||)
    //    Only inside script VALUES, never as a dep key. Idempotent: skip
    //    if already prefixed by 'bunx '.
    {
        from: /(^|[\s|&;])(?<!bunx\s)vitest\b/g,
        to: "$1bunx vitest",
        label: "vitest→bunx vitest"
    },
];

interface Report { path: string; counts: Record<string, number>; total: number }
const reports: Report[] = [];

async function walk(dir: string): Promise<string[]> {
    const out: string[] = [];
    for (const entry of await readdir(dir, { withFileTypes: true })) {
        if (entry.name === "node_modules" || entry.name === ".git") continue;
        const full = join(dir, entry.name);
        if (entry.isDirectory()) out.push(...(await walk(full)));
        else if (entry.isFile() && entry.name === "package.json") out.push(full);
    }
    return out;
}

const files = await walk(TARGET_ROOT);
console.log(`Found ${files.length} package.json file(s)\n`);

for (const file of files) {
    const original = await readFile(file, "utf8");
    let pkg: Record<string, unknown>;
    try { pkg = JSON.parse(original); }
    catch { console.warn(`  skip (invalid JSON): ${relative(REPO_ROOT, file)}`); continue; }

    const scripts = pkg.scripts as Record<string, string> | undefined;
    if (!scripts) continue;

    const counts: Record<string, number> = {};
    let totalForFile = 0;

    for (const [name, raw] of Object.entries(scripts)) {
        if (typeof raw !== "string") continue;
        let cur = raw;
        for (const sub of SUBS) {
            const matches = cur.match(sub.from);
            if (matches && matches.length > 0) {
                counts[sub.label] = (counts[sub.label] ?? 0) + matches.length;
                totalForFile += matches.length;
                cur = cur.replace(sub.from, sub.to as string);
            }
        }
        scripts[name] = cur;
    }

    if (totalForFile > 0) {
        const serialized = JSON.stringify(pkg, null, 2) + "\n";
        if (!DRY) await writeFile(file, serialized, "utf8");
        reports.push({ path: relative(REPO_ROOT, file), counts, total: totalForFile });
    }
}

console.log(`${DRY ? "[DRY] " : ""}Bunnization summary`);
console.log("-".repeat(70));
for (const r of reports) {
    console.log(`${r.path}  (${r.total})`);
    for (const [label, n] of Object.entries(r.counts)) {
        console.log(`    ${n.toString().padStart(3)}  ${label}`);
    }
}
console.log("-".repeat(70));
console.log(`${reports.length} file(s) modified, ${reports.reduce((a, r) => a + r.total, 0)} script substitutions total.`);
