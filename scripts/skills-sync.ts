// SPDX-License-Identifier: Apache-2.0
// #!/usr/bin/env bun
/**
 * skills-sync.ts — pull a remote SKILL.md catalog into ./.claude/skills/.
 *
 * Usage:
 *     bun run scripts/skills-sync.ts <github-owner>/<repo>[#<ref>]
 *
 * Examples:
 *     bun run scripts/skills-sync.ts vercel-labs/agent-skills
 *     bun run scripts/skills-sync.ts anthropics/skills#main
 *
 * Strategy:
 *   1. Shallow-clone the repo into a temp directory (bun shell + git).
 *   2. Discover every `SKILL.md` (recursive) and copy its containing folder
 *      into `.claude/skills/<skill-name>/` — `<skill-name>` is taken from
 *      the SKILL.md YAML frontmatter `name:` field, falling back to the
 *      directory basename.
 *   3. Skip skills that already exist locally (no overwrite without --force).
 *   4. Print a summary table.
 *
 * No Node dependency — uses Bun stdlib (Bun.file, Bun.$, fs/promises).
 */

import { $ } from "bun";
import { mkdir, readdir, rm, stat, copyFile, readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, basename, dirname, relative } from "node:path";
import { tmpdir } from "node:os";

const REPO_ROOT = join(import.meta.dir, "..");
const TARGET_ROOT = join(REPO_ROOT, ".claude", "skills");

const FORCE = process.argv.includes("--force");
const args = process.argv.slice(2).filter((a) => !a.startsWith("--"));
const source = args[0];

if (!source) {
    console.error("usage: bun run scripts/skills-sync.ts <owner>/<repo>[#<ref>] [--force]");
    process.exit(64);
}

const [repoSpec, ref = "HEAD"] = source.split("#");
if (!repoSpec.includes("/")) {
    console.error(`Invalid source "${source}" — expected <owner>/<repo>.`);
    process.exit(64);
}

const url = `https://github.com/${repoSpec}.git`;
const workDir = await Bun.file(join(tmpdir(), `skills-sync-${Date.now()}`)).name;
await mkdir(workDir, { recursive: true });

console.log(`▸ Cloning ${url}#${ref} (shallow) ...`);
const cloneResult = await $`git clone --depth 1 --branch ${ref === "HEAD" ? "main" : ref} ${url} ${workDir}`
    .nothrow();
if (cloneResult.exitCode !== 0) {
    // fallback: clone default branch
    await rm(workDir, { recursive: true, force: true });
    await mkdir(workDir, { recursive: true });
    await $`git clone --depth 1 ${url} ${workDir}`;
}

console.log("▸ Discovering SKILL.md files ...");
const candidates: string[] = [];
async function walk(dir: string) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
        if (entry.name === ".git" || entry.name === "node_modules") continue;
        const full = join(dir, entry.name);
        if (entry.isDirectory()) await walk(full);
        else if (entry.isFile() && entry.name === "SKILL.md") candidates.push(full);
    }
}
await walk(workDir);
console.log(`  found ${candidates.length} skill(s).`);

await mkdir(TARGET_ROOT, { recursive: true });
const report: { name: string; status: "installed" | "skipped" | "overwritten"; from: string }[] = [];

for (const skillMd of candidates) {
    const text = await readFile(skillMd, "utf8");
    const frontmatter = text.match(/^---\s*\n([\s\S]*?)\n---/);
    let name = basename(dirname(skillMd));
    if (frontmatter) {
        const m = frontmatter[1].match(/^name:\s*(.+?)\s*$/m);
        if (m) name = m[1].trim();
    }
    const dest = join(TARGET_ROOT, name);
    const srcDir = dirname(skillMd);
    if (existsSync(dest) && !FORCE) {
        report.push({ name, status: "skipped", from: relative(workDir, srcDir) });
        continue;
    }
    if (existsSync(dest)) await rm(dest, { recursive: true, force: true });
    await copyDir(srcDir, dest);
    report.push({
        name,
        status: existsSync(dest) && FORCE ? "overwritten" : "installed",
        from: relative(workDir, srcDir),
    });
}

await rm(workDir, { recursive: true, force: true });

console.log("");
console.log("Summary");
console.log("-------");
for (const r of report) {
    const tag = r.status === "skipped" ? "SKIP" : r.status === "overwritten" ? "REPL" : "NEW ";
    console.log(`  [${tag}] ${r.name.padEnd(40)}  ← ${r.from}`);
}
console.log("");
console.log(`Done. ${report.filter((r) => r.status !== "skipped").length} skill(s) written to ${TARGET_ROOT}.`);

async function copyDir(src: string, dest: string): Promise<void> {
    await mkdir(dest, { recursive: true });
    for (const entry of await readdir(src, { withFileTypes: true })) {
        const s = join(src, entry.name);
        const d = join(dest, entry.name);
        if (entry.isDirectory()) await copyDir(s, d);
        else if (entry.isFile()) await copyFile(s, d);
    }
}
