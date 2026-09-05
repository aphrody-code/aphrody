// Verifies the Bun Sass plugin compiles a real component `.scss` to a Lit
// CSSResult module through Bun.build (dart-sass under the hood).
import { test, expect } from "bun:test";
import { resolve } from "node:path";
import { sassRustPlugin as sassPlugin } from "../../bun-rs/src/index.ts";

const PKG = resolve(import.meta.dir, "..");

test("sassPlugin compiles .scss to a Lit CSSResult via Bun.build", async () => {
  // Tiny entry that imports a real, self-contained component stylesheet.
  const entry = resolve(PKG, "test/.sass-fixture.ts");
  await Bun.write(
    entry,
    `import { styles } from "../elevation/internal/elevation-styles.scss";\n` +
      `export const css = styles.cssText;\n`,
  );

  const built = await Bun.build({
    entrypoints: [entry],
    target: "browser",
    external: ["lit", "lit/*", "@lit/*"],
    plugins: [
      sassPlugin({
        loadPaths: [resolve(PKG, "node_modules"), resolve(PKG, "../../node_modules")],
      }),
    ],
  });

  expect(built.success).toBe(true);
  const out = await built.outputs[0].text();
  // The compiled CSS (elevation uses box-shadow tokens) is inlined in a css`` literal.
  expect(out).toContain("css`");
  expect(out).toContain("box-shadow");
  expect(out).toContain('"lit"');

  await Bun.file(entry).delete();
});
