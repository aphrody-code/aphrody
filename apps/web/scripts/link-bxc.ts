// SPDX-License-Identifier: Apache-2.0
//
// Ensure `@aphrody/bxc` resolves from this app.
//
// bxc (`@aphrody/bxc`) is a sibling monorepo *root* (it has its own
// `workspaces`), so a normal `file:`/`workspace:` dependency makes bun try to
// resolve bxc's internal `workspace:*` packages against aphrody and fail. The
// reliable way to consume bxc here is a plain node_modules symlink, recreated by
// this postinstall step so a clean `bun install` (or a wiped node_modules) keeps
// the `@aphrody/bxc/*` import specifiers working. No-ops cleanly when bxc is
// absent (e.g. CI without the sibling checkout).

import { existsSync, lstatSync, mkdirSync, rmSync, symlinkSync } from "node:fs";
import { dirname, resolve } from "node:path";

// Sibling layout on the VPS: <root>/aphrody/apps/web and <root>/bxc.
const BXC_DIR = resolve(import.meta.dir, "../../../../bxc");
const LINK = resolve(import.meta.dir, "../node_modules/@aphrody/bxc");

if (!existsSync(BXC_DIR)) {
  console.log(`[link-bxc] bxc not found at ${BXC_DIR}; skipping (import will be unavailable)`);
  process.exit(0);
}

mkdirSync(dirname(LINK), { recursive: true });

// Replace any stale entry (broken symlink, wrong target, leftover dir).
try {
  if (lstatSync(LINK, { throwIfNoEntry: false })) {
    rmSync(LINK, { recursive: true, force: true });
  }
} catch {
  /* nothing to clean */
}

symlinkSync(BXC_DIR, LINK, "dir");
console.log(`[link-bxc] linked @aphrody/bxc -> ${BXC_DIR}`);
