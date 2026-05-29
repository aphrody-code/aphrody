#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
//
// Publish aphrody's first-party / forked npm packages to **GitHub Packages**
// (registry npm.pkg.github.com, org `aphrody-code`).
//
// Why a script: the material-web fork keeps the upstream name `@material/web`
// in-tree so it stays a drop-in during development. GitHub Packages requires
// the package scope to match the org (`@aphrody-code`), so this script rewrites
// the name to `@aphrody-code/material-web` **only at publish time** (in a temp
// copy of package.json) and restores the original afterwards — the working tree
// is never left renamed.
//
// Auth: needs a token with `write:packages` in $GITHUB_TOKEN. The current
// `gh` token only has `gist read:org repo workflow`; add the scope with:
//   gh auth refresh -s write:packages && export GITHUB_TOKEN="$(gh auth token)"
//
// Usage:
//   bun scripts/publish-github-packages.ts            # dry-run (default, safe)
//   bun scripts/publish-github-packages.ts --publish  # real publish
//   bun scripts/publish-github-packages.ts --only m3-react

import {join} from 'node:path';
import {readFileSync, writeFileSync, existsSync, unlinkSync} from 'node:fs';

const ROOT = join(import.meta.dir, '..');

interface Target {
  /** Directory relative to repo root. */
  dir: string;
  /** The scoped name to publish under (GitHub Packages requires @aphrody-code). */
  publishName: string;
  /** Build command to run before publishing (optional). */
  build?: string;
}

// The genuinely publishable aphrody packages. The upstream monorepo forks
// (lit / ui / tailwindcss / gts) are consumed in-tree and are NOT published
// individually here — each is a multi-package monorepo that would need
// per-subpackage rescoping (a separate effort), so publishing the private
// root manifests would be meaningless.
// material-web's upstream `npm run build` (wireit) is bash-only and breaks under
// Windows cmd (`$(ls -d */ | grep ...)`). We run the publishable steps directly
// via bash (git-bash on Windows): sass → css-to-ts → tsc. tsc emits .js even
// with the env-only bun-types/Timeout type warnings.
const MW_BUILD =
  "node_modules/.bin/sass --style=compressed --load-path=node_modules --load-path=node_modules/sass-true/sass $(ls -d */ | grep -vE 'node_modules|catalog') " +
  "&& (find . \\( -path ./.wireit -o -path ./node_modules -o -path ./catalog \\) -prune -o -name '*.css' -print | xargs -L1 node scripts/css-to-ts.js --suffix=.cssresult) " +
  '&& node_modules/.bin/tsc --pretty; true';

const TARGETS: Target[] = [
  {dir: 'packages/material-web', publishName: '@aphrody-code/material-web', build: MW_BUILD},
  {dir: 'apps/m3-react', publishName: '@aphrody-code/m3-react'},
  {dir: 'packages/skills', publishName: '@aphrody-code/skills'},
  {dir: 'packages/x', publishName: '@aphrody-code/x'},
];

const args = new Set(Bun.argv.slice(2));
const dryRun = !args.has('--publish');
const onlyIdx = Bun.argv.indexOf('--only');
const only = onlyIdx !== -1 ? Bun.argv[onlyIdx + 1] : null;

function log(msg: string) {
  console.error(`[publish] ${msg}`);
}

if (!process.env['GITHUB_TOKEN']) {
  log('WARNING: $GITHUB_TOKEN is not set — a real publish will fail auth.');
  log('  gh auth refresh -s write:packages && export GITHUB_TOKEN="$(gh auth token)"');
}
log(dryRun ? 'DRY-RUN (pass --publish to publish for real)' : 'PUBLISHING for real');

let failures = 0;
for (const t of TARGETS) {
  if (only && !t.dir.includes(only) && !t.publishName.includes(only)) {
    continue;
  }
  const abs = join(ROOT, t.dir);
  const pkgPath = join(abs, 'package.json');
  if (!existsSync(pkgPath)) {
    log(`SKIP ${t.dir} (no package.json)`);
    continue;
  }
  const original = readFileSync(pkgPath, 'utf8');
  const pkg = JSON.parse(original);
  const restore = () => writeFileSync(pkgPath, original);

  try {
    if (t.build && !dryRun) {
      log(`${t.dir}: building …`);
      const b = Bun.spawnSync(['bash', '-lc', t.build], {cwd: abs, stdout: 'inherit', stderr: 'inherit'});
      if (b.exitCode !== 0) {
        throw new Error(`build failed (${b.exitCode})`);
      }
    }
    // Rewrite name + ensure GitHub Packages publishConfig for this publish.
    pkg.name = t.publishName;
    pkg.publishConfig = {...pkg.publishConfig, registry: 'https://npm.pkg.github.com'};
    delete pkg.private;
    writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');

    // npm only reads .npmrc from cwd/home/global — packages in nested git repos
    // (e.g. the material-web fork) don't inherit the repo-root .npmrc, so write a
    // scoped, authed .npmrc into the package dir for the publish, then remove it.
    const npmrcPath = join(abs, '.npmrc');
    const hadNpmrc = existsSync(npmrcPath);
    const npmrcBak = hadNpmrc ? readFileSync(npmrcPath, 'utf8') : null;
    const token = process.env['GITHUB_TOKEN'] ?? 'dummy_token';
    writeFileSync(
      npmrcPath,
      `@aphrody-code:registry=https://npm.pkg.github.com\n//npm.pkg.github.com/:_authToken=${token}\n`,
    );
    try {
      const cmd = ['bun', 'publish', ...(dryRun ? ['--dry-run'] : [])];
      log(`${t.dir} → ${t.publishName} : ${cmd.join(' ')}`);
      const p = Bun.spawnSync(cmd, {cwd: abs, stdout: 'inherit', stderr: 'inherit'});
      if (p.exitCode !== 0) {
        throw new Error(`bun publish exited ${p.exitCode}`);
      }
      log(`${t.publishName} ✓`);
    } finally {
      // Never leave the token-bearing .npmrc behind.
      if (hadNpmrc && npmrcBak !== null) {
        writeFileSync(npmrcPath, npmrcBak);
      } else {
        try {
          unlinkSync(npmrcPath);
        } catch {
          /* ignore */
        }
      }
    }
  } catch (err) {
    failures++;
    log(`${t.publishName} FAILED: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    restore();
  }
}

if (failures > 0) {
  log(`${failures} package(s) failed`);
  process.exit(1);
}
log('done');
