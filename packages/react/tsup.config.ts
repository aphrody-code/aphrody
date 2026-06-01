import { defineConfig } from "tsup";
import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

/**
 * Build config for @aphrody-code/m3-react.
 *
 * Ships an ESM `dist/` (`.js` + `.d.ts`) so the package is consumable from a
 * Next.js app (App Router / RSC) without `transpilePackages`.
 *
 * Key constraints:
 *  - Two entry points: the full barrel (`index.ts`) and `transitions/`.
 *  - `react`, `react-dom` and `@aphrody-code/material-web` are EXTERNAL — they must resolve
 *    in the consumer (peer deps / its own node_modules). In particular the deep
 *    `@aphrody-code/material-web/<dir>/<el>.js` imports are kept verbatim so the consumer's
 *    bundler (webpack/turbopack) loads the element definitions client-side.
 *  - Every emitted `.js` chunk gets a leading `'use client';` directive so
 *    Next RSC treats the whole package as client components (web components are
 *    client-only). esbuild strips module-level directives when bundling, so we
 *    re-inject it in `onSuccess` rather than via `banner` (which is ignored).
 *
 * `.d.ts` files are emitted separately by `tsc` (see the `build` script);
 * tsup's rollup-based dts pipeline trips over the `baseUrl` deprecation in
 * TypeScript 6, so we keep `dts: false` here.
 */
const USE_CLIENT = "'use client';\n";

async function prependUseClient(dir: string): Promise<void> {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      await prependUseClient(full);
      continue;
    }
    if (!entry.name.endsWith(".js")) continue;
    const src = await readFile(full, "utf8");
    if (src.startsWith(USE_CLIENT)) continue;
    await writeFile(full, USE_CLIENT + src);
  }
}

export default defineConfig({
  // The full barrel, the transitions entry, AND every per-family wrapper as its
  // own entry so consumers can deep-import (`@aphrody-code/m3-react/button`) and pull
  // only that family + a shared chunk — real tree-shaking despite `sideEffects`.
  entry: [
    "index.ts",
    "transitions/index.ts",
    "adaptive/index.ts",
    "state/index.ts",
    "wrappers/*.ts",
  ],
  format: ["esm"],
  target: "es2022",
  platform: "browser",
  dts: false,
  sourcemap: true,
  clean: true,
  splitting: true,
  treeshake: true,
  // Keep React and the web-component library out of the bundle.
  external: ["react", "react-dom", "react/jsx-runtime"],
  // `@aphrody-code/material-web` and ALL its deep `.js` subpaths stay external.
  esbuildOptions(options) {
    options.external = [
      ...(options.external ?? []),
      "@aphrody-code/material-web",
      "@aphrody-code/material-web/*",
    ];
  },
  async onSuccess() {
    await prependUseClient("dist");
  },
});
