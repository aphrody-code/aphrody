import { defineConfig } from "tsup";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const USE_CLIENT = "'use client';\n";

export default defineConfig({
  entry: ["src/index.ts", "src/react.tsx", "src/tauri.tsx"],
  format: ["esm"],
  target: "es2022",
  dts: false,
  clean: true,
  outDir: "dist",
  treeshake: true,
  external: ["react", "react-dom", "react/jsx-runtime"],
  esbuildOptions(options) {
    options.external = [
      ...(options.external ?? []),
      "@aphrody/m3-tokens",
      "@aphrody/m3-tokens/*",
    ];
  },
  async onSuccess() {
    for (const name of ["react.js", "tauri.js"]) {
      try {
        const file = join("dist", name);
        const src = await readFile(file, "utf8");
        if (!src.startsWith(USE_CLIENT)) {
          await writeFile(file, USE_CLIENT + src);
        }
      } catch (e) {
        // Ignore if file doesn't exist
      }
    }
  },
});
