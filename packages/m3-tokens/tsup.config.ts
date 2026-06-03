import { defineConfig } from "tsup";

// NOTE: `.d.ts` files are emitted separately by `tsc` (see the `build`
// script). tsup's rollup-based dts pipeline trips over the `baseUrl`
// deprecation in TypeScript 6, so we keep `dts: false` here — same pattern as
// @aphrody/m3-react.
export default defineConfig({
  entry: ["src/theme-to-tokens.ts", "src/dynamic-color.ts", "src/breakpoints.ts"],
  format: ["esm"],
  target: "es2022",
  dts: false,
  clean: true,
  outDir: "dist",
  treeshake: true,
});
