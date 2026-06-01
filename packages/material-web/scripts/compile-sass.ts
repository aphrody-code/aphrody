// SPDX-License-Identifier: Apache-2.0
//
// Sass build. Compiles every non-partial `*.scss` to a sibling `*.css`.
//
// Prefers the native Rust grass compiler via the @aphrody-code/bun-rs FFI
// (fastest), but that native lib only builds on a machine that has the
// out-of-tree `m3-tokens` crate. When `libbun_rs.so` is missing (CI, fresh
// clone) we transparently fall back to `sass-embedded` (dart-sass), which is
// already installed. Set `SASS_FORCE_EMBEDDED=1` to force the fallback.
//

import { resolve, basename } from "node:path";

const PKG = resolve(import.meta.dir, "..");
const loadPaths = [
  resolve(PKG, "node_modules"),
  resolve(PKG, "node_modules/sass-true/sass"),
  resolve(PKG, "../../node_modules"),
  resolve(PKG, "../../node_modules/sass-true/sass"),
];

/** Compiles one SCSS file to compressed CSS. */
type Compiler = (abs: string) => string | Promise<string>;

async function selectCompiler(): Promise<{ compile: Compiler; engine: string }> {
  if (Bun.env.SASS_FORCE_EMBEDDED !== "1") {
    try {
      const { compileSassFile } = await import("../../bun-rs/src/index.ts");
      // The native lib is dlopen'd lazily on first use. Probe-compile a
      // trivial, self-contained sheet so a missing libbun_rs.so surfaces here
      // (where we can fall back) instead of crashing mid-run. A valid one-liner
      // can only fail by failing to load the native library.
      const probe = resolve(PKG, ".sass-probe.scss");
      await Bun.write(probe, ".probe{color:red}");
      try {
        compileSassFile(probe, loadPaths, "compressed", true);
      } finally {
        await Bun.file(probe)
          .unlink()
          .catch(() => {});
      }
      return {
        compile: (abs) => compileSassFile(abs, loadPaths, "compressed", true),
        engine: "Grass Rust FFI",
      };
    } catch {
      // fall through to sass-embedded
    }
  }
  const sass = await import("sass-embedded");
  return {
    compile: async (abs) => (await sass.compileAsync(abs, { loadPaths, style: "compressed" })).css,
    engine: "sass-embedded",
  };
}

// Entry stylesheets = non-partial .scss (partials start with `_`).
const entries: string[] = [];
for await (const rel of new Bun.Glob("**/*.scss").scan({ cwd: PKG })) {
  if (rel.includes("node_modules")) continue;
  if (basename(rel).startsWith("_")) continue;
  entries.push(rel);
}

const { compile, engine } = await selectCompiler();

const CONCURRENCY = 24;
let i = 0;
let done = 0;

async function worker() {
  while (i < entries.length) {
    const rel = entries[i++];
    const abs = resolve(PKG, rel);
    const cssPath = abs.replace(/\.scss$/, ".css");
    try {
      const css = await compile(abs);
      await Bun.write(cssPath, css);
      done++;
    } catch (e: any) {
      console.error(`Failed to compile ${rel}:`, e.message);
      process.exit(1);
    }
  }
}

const t0 = Bun.nanoseconds();
await Promise.all(Array.from({ length: CONCURRENCY }, worker));
console.log(
  `sass: ${done} stylesheets in ${((Bun.nanoseconds() - t0) / 1e9).toFixed(2)}s (${engine}, x${CONCURRENCY})`,
);
