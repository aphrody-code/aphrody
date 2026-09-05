import { defineConfig } from "tsup";
import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

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
  entry: ["src/index.ts"],
  format: ["esm"],
  target: "es2022",
  platform: "browser",
  dts: false,
  sourcemap: true,
  clean: true,
  splitting: true,
  treeshake: true,
  external: ["react", "react-dom", "react/jsx-runtime", "motion", "motion/react"],
  async onSuccess() {
    await prependUseClient("dist");
  },
});
