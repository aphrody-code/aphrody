// SPDX-License-Identifier: Apache-2.0
//
// Bun-native build for the aphrody Material 3 extension bundle. Replaces the
// upstream wireit → SASS → css-to-ts → tsc pipeline with a single `Bun.build`
// pass:
//   • the built-in `ts` loader transpiles the components (decorators honored
//     via tsconfig `experimentalDecorators`);
//   • the JS minifier (whitespace + syntax + identifiers) shrinks the bundle,
//     with `keepNames` so component class names stay readable in DevTools and
//     `drop: ['debugger']` for clean production output;
//   • the optional `aphrody-css-in-js` plugin routes every Lit `css` literal
//     through Bun's LightningCSS port. NOTE: this *transpiles for older
//     browsers* (vendor-prefixing, color fallbacks, logical-property lowering),
//     which slightly INCREASES bundle size. Our components target modern
//     engines (WebGPU, Shadow DOM), so it's opt-in via `--css-transpile`.
// Refs: https://bun.com/docs/bundler , /bundler/minifier , /bundler/plugins , /bundler/css.
//
// Usage:  bun run aphrody-build.ts                 (production: minified JS)
//         bun run aphrody-build.ts --no-min        (readable JS, for debugging)
//         bun run aphrody-build.ts --css-transpile (widen CSS browser support)

import { cssMinifyPlugin } from "./aphrody-css-minify.js";
import { sassRustPlugin as sassPlugin } from "../bun-rs/src/index.ts";
import { resolve } from "node:path";

const minify = !Bun.argv.includes("--no-min");
const cssTranspile = Bun.argv.includes("--css-transpile");

// Native Sass compile via Bun's plugin system (dart-sass under the hood), so a
// `.scss` import resolves to a Lit CSSResult without the separate sass CLI +
// css-to-ts pass. loadPaths mirror the wireit build:sass invocation.
const sass = sassPlugin({
  loadPaths: [
    resolve(import.meta.dir, "node_modules"),
    resolve(import.meta.dir, "../../node_modules"),
  ],
});

const result = await Bun.build({
  entrypoints: ["./aphrody-components.ts"],
  outdir: "./dist-aphrody",
  target: "browser",
  format: "esm",
  // `lit` stays external so consuming apps dedupe it with their own copy.
  external: ["lit", "lit/*", "@lit/*"],
  minify: minify ? { whitespace: true, syntax: true, identifiers: true, keepNames: true } : false,
  drop: ["debugger"],
  sourcemap: "linked",
  plugins: cssTranspile ? [sass, cssMinifyPlugin] : [sass],
});

if (!result.success) {
  for (const log of result.logs) {
    console.error(log);
  }
  process.exit(1);
}

// Report the shipped JS size (exclude the .map, which is ~4x larger and not loaded at runtime).
const jsBytes = result.outputs
  .filter((o) => o.path.endsWith(".js"))
  .reduce((n, o) => n + o.size, 0);
console.log(
  `aphrody-components → ${(jsBytes / 1024).toFixed(1)} KB js` +
    `${minify ? " (min)" : ""}${cssTranspile ? " + css-transpile" : ""}`,
);
