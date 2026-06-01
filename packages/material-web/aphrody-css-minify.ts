// SPDX-License-Identifier: Apache-2.0
//
// A Bun bundler plugin that runs the CSS embedded in Lit `css` template
// literals through Bun's native CSS bundler (a Rust→Zig port of LightningCSS).
// It strips whitespace AND transpiles for the bundler's default browser
// targets — adding vendor prefixes, color fallbacks, and logical-property
// lowering. That widens browser support at the cost of a slightly larger
// bundle, so the build wires it up only behind `--css-transpile` (off by
// default, since the components target modern engines). Semantics are
// preserved: `calc(100vw - 32px)` keeps its significant spaces, `color-mix`/
// `clamp` stay valid. See https://bun.com/docs/bundler/plugins and /bundler/css.
//
// Why a precompute step: calling `Bun.build` *inside* an `onLoad` callback of
// the parent build deadlocks the single bundler instance. Instead we collect
// every `css` literal up front in `onStart`, minify them all in ONE separate
// `bun build` subprocess, and have `onLoad` replace from the resulting cache
// synchronously. Targets `*-styles.ts` (aphrody's convention); literals with
// `${}` interpolation are skipped (minifying them is unsafe).

import type { BunPlugin } from "bun";
import { mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const LITERAL = /\bcss`([\s\S]*?)`/g;

/** Re-escape a CSS string for safe insertion back into a `css` literal. */
function escapeForTemplate(css: string): string {
  return css.replace(/\\/g, "\\\\").replace(/`/g, "\\`").replace(/\$\{/g, "\\${");
}

/** Extract the minifiable CSS literals (no `${}`) from a source string. */
function extractLiterals(source: string): string[] {
  const out: string[] = [];
  for (const m of source.matchAll(LITERAL)) {
    if (!m[1].includes("${")) {
      out.push(m[1]);
    }
  }
  return out;
}

export const cssMinifyPlugin: BunPlugin = {
  name: "aphrody-css-in-js",
  setup(build) {
    // original CSS → minified CSS, filled in onStart, read in onLoad.
    const cache = new Map<string, string>();

    build.onStart(async () => {
      const glob = new Bun.Glob("**/*-styles.ts");
      const cwd = process.cwd();
      // Collect unique CSS literals keyed by content hash.
      const byHash = new Map<string, string>();
      for await (const rel of glob.scan({ cwd })) {
        if (rel.includes("node_modules")) {
          continue;
        }
        const source = await Bun.file(join(cwd, rel)).text();
        for (const css of extractLiterals(source)) {
          byHash.set(Bun.hash(css).toString(16), css);
        }
      }
      if (byHash.size === 0) {
        return;
      }
      const dir = await mkdtemp(join(tmpdir(), "aphrody-cssin-"));
      try {
        await mkdir(join(dir, "out"), { recursive: true });
        const names: string[] = [];
        for (const [hash, css] of byHash) {
          await Bun.write(join(dir, `${hash}.css`), css);
          names.push(`${hash}.css`);
        }
        // ONE subprocess (cwd = temp dir, relative paths to dodge Windows
        // absolute-outdir resolution). Whitespace-only minify keeps the CSS
        // semantics intact — no logical-property expansion or color folding.
        const proc = Bun.spawn(
          ["bun", "build", ...names, "--outdir", "out", "--minify-whitespace"],
          { cwd: dir, stdout: "ignore", stderr: "ignore" },
        );
        await proc.exited;
        if (proc.exitCode !== 0) {
          return; // leave cache empty → CSS ships unminified, build still ok
        }
        for (const [hash, css] of byHash) {
          try {
            const min = (await readFile(join(dir, "out", `${hash}.css`), "utf8")).trim();
            if (min) {
              cache.set(css, min);
            }
          } catch {
            // missing output for this entry: skip, keep original
          }
        }
      } finally {
        await rm(dir, { recursive: true, force: true }).catch(() => {});
      }
    });

    build.onLoad({ filter: /-styles\.ts$/ }, async ({ path }) => {
      const source = await Bun.file(path).text();
      let changed = false;
      let contents = source;
      for (const css of extractLiterals(source)) {
        const min = cache.get(css);
        if (min && min !== css) {
          contents = contents.replace("css`" + css + "`", "css`" + escapeForTemplate(min) + "`");
          changed = true;
        }
      }
      return changed ? { contents, loader: "ts" } : undefined;
    });
  },
};
