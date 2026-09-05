import { describe, expect, test } from "bun:test";

// Size budgets that lock in the tree-shaking guarantee: a deep family import
// must stay tiny, and the barrel must stay light. Built from source with React
// and @aphrody/material-web kept external (they resolve in the consumer / load the
// element code on demand), minified + gzipped — the figures a real consumer
// pays for the wrapper layer. Guards against accidental bloat or a regression
// of the per-family subpath split.

const EXTERNAL = [
  "react",
  "react-dom",
  "react/jsx-runtime",
  "@lit/react",
  "@aphrody/material-web",
  "@aphrody/material-web/*",
];

async function gzipBytes(entry: string): Promise<number> {
  const result = await Bun.build({
    entrypoints: [new URL(`../${entry}`, import.meta.url).pathname],
    external: EXTERNAL,
    minify: true,
    target: "browser",
  });
  if (!result.success) {
    throw new Error(`build failed for ${entry}: ${result.logs.map((l) => l.message).join("; ")}`);
  }
  let total = 0;
  for (const output of result.outputs) {
    total += Bun.gzipSync(Buffer.from(await output.text())).length;
  }
  return total;
}

describe("bundle-size budgets (gzip)", () => {
  test("a single family deep-import stays under 1 KB", async () => {
    const size = await gzipBytes("wrappers/button.ts");
    expect(size, `wrappers/button.ts = ${size} B gzip`).toBeLessThan(1024);
  });

  test("a chart family deep-import stays under 1.5 KB", async () => {
    const size = await gzipBytes("wrappers/charts.ts");
    expect(size, `wrappers/charts.ts = ${size} B gzip`).toBeLessThan(1536);
  });

  test("the full barrel (all 115 wrappers) stays under 7 KB", async () => {
    const size = await gzipBytes("index.ts");
    expect(size, `index.ts = ${size} B gzip`).toBeLessThan(7168);
  });

  test("the transitions entry stays under 2.5 KB", async () => {
    const size = await gzipBytes("transitions/index.ts");
    expect(size, `transitions/index.ts = ${size} B gzip`).toBeLessThan(2560);
  });

  test("a deep family import is at least 5x lighter than the barrel", async () => {
    const family = await gzipBytes("wrappers/button.ts");
    const barrel = await gzipBytes("index.ts");
    expect(barrel / family).toBeGreaterThan(5);
  });
});
