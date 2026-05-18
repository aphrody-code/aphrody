// SPDX-License-Identifier: Apache-2.0
//
// Copy the Google Sans Flex variable font from assets/fonts/google-sans-flex/
// into public/fonts/google-sans-flex/ so Next.js can serve it at /fonts/...
// without bundling. Idempotent — re-runs are safe (mtime preserved on copy).

import { promises as fs } from "node:fs";
import { dirname, join, resolve } from "node:path";

const FONT_FILENAME = "GoogleSansFlex-VariableFont_GRAD,ROND,opsz,slnt,wdth,wght.ttf";

async function main(): Promise<void> {
  // packages/gemini-app-aphrody/scripts/copy-fonts.ts
  const here = dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1"));
  const pkgDir = resolve(here, "..");
  const repoRoot = resolve(pkgDir, "..", "..");
  const src = join(repoRoot, "assets", "fonts", "google-sans-flex", FONT_FILENAME);
  const destDir = join(pkgDir, "public", "fonts", "google-sans-flex");
  const dest = join(destDir, FONT_FILENAME);

  await fs.mkdir(destDir, { recursive: true });

  let needCopy = true;
  try {
    const [srcStat, dstStat] = await Promise.all([fs.stat(src), fs.stat(dest)]);
    if (dstStat.size === srcStat.size && dstStat.mtimeMs >= srcStat.mtimeMs) {
      needCopy = false;
    }
  } catch {
    // dest missing → copy
  }

  if (needCopy) {
    try {
      await fs.copyFile(src, dest);
      process.stdout.write(`copy-fonts: ${dest}\n`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      process.stderr.write(`copy-fonts: WARN ${msg}\n`);
      process.stderr.write(`copy-fonts: app will run with the system fallback font.\n`);
      // Non-fatal: dev can still proceed; @font-face will simply 404.
    }
  } else {
    process.stdout.write(`copy-fonts: up-to-date (${dest})\n`);
  }

  // Also copy the OFL license alongside, per SIL OFL 1.1 redistribution rules.
  try {
    const oflSrc = join(repoRoot, "assets", "fonts", "google-sans-flex", "OFL.txt");
    const oflDst = join(destDir, "OFL.txt");
    await fs.copyFile(oflSrc, oflDst);
  } catch {
    // ignore — license file missing in dev checkout shouldn't break dev server
  }
}

main().catch((err: unknown) => {
  const msg = err instanceof Error ? err.message : String(err);
  process.stderr.write(`copy-fonts: FATAL ${msg}\n`);
  process.exit(1);
});
