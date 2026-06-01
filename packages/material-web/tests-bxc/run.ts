/**
 * Browser test gate (real Chromium, no heavy test deps).
 *
 * Serves the package with Bun.serve, loads tests-bxc/index.html in a headless
 * system Chrome (the self-contained `dist-aphrody/aphrody-components.js` bundle
 * defines every component and self-checks customElements + shadowRoot + console
 * errors), reads the `#bxc-result` verdict, and exits non-zero on any failure.
 *
 * Run: bun run build:aphrody && bun run test:browser
 * Chrome is resolved from BXC_CHROME_BIN | CHROME_BIN | PATH.
 */
import { resolve } from "node:path";

const PKG_ROOT = resolve(import.meta.dir, "..");

// Build a fully self-contained bundle for the smoke (lit inlined, unlike the
// shipped dist-aphrody bundle which keeps lit external for consumer dedupe) so
// it loads in a bare browser with no import map.
const built = await Bun.build({
  entrypoints: [`${PKG_ROOT}/aphrody-components.ts`],
  target: "browser",
  minify: true,
});
if (!built.success) {
  console.error("smoke bundle build failed");
  for (const log of built.logs) console.error(log);
  process.exit(1);
}
await Bun.write(`${PKG_ROOT}/dist-aphrody/aphrody-components.smoke.js`, built.outputs[0]);

function findChrome(): string | null {
  const explicit = Bun.env.BXC_CHROME_BIN ?? Bun.env.CHROME_BIN;
  if (explicit) return explicit;
  for (const c of ["google-chrome", "chromium", "chromium-browser", "google-chrome-stable"]) {
    const p = Bun.which(c);
    if (p) return p;
  }
  return null;
}

const chrome = findChrome();
if (!chrome) {
  console.error("no Chrome/Chromium found (set BXC_CHROME_BIN)");
  process.exit(1);
}

const server = Bun.serve({
  port: 0,
  async fetch(req) {
    let p = new URL(req.url).pathname;
    if (p === "/") p = "/tests-bxc/index.html";
    const f = Bun.file(PKG_ROOT + p);
    if (await f.exists()) return new Response(f);
    return new Response("404", { status: 404 });
  },
});

const url = `http://localhost:${server.port}/tests-bxc/index.html`;
console.log(`bxc gate: ${chrome} -> ${url}`);

const proc = Bun.spawn(
  [
    chrome,
    "--headless",
    "--no-sandbox",
    "--disable-gpu",
    "--virtual-time-budget=15000",
    "--run-all-compositor-stages-before-draw",
    "--dump-dom",
    url,
  ],
  { stdout: "pipe", stderr: "ignore" },
);
const dom = await new Response(proc.stdout).text();
await proc.exited;
server.stop(true);

// Parse the verdict written into <pre id="bxc-result">DEFINED:n/N SHADOW:n/N ERRORS:k</pre>
const m = dom.match(/id="bxc-result"[^>]*>([^<]*)</);
const verdict = m?.[1]?.trim() ?? "";
console.log(`verdict: ${verdict || "(empty — page did not settle)"}`);

const defined = verdict.match(/DEFINED:(\d+)\/(\d+)/);
const errors = verdict.match(/ERRORS:(\d+)/);
let ok = false;
if (defined && errors) {
  const [, got, total] = defined;
  const errCount = Number(errors[1]);
  ok = got === total && errCount === 0;
  console.log(`${ok ? "PASS" : "FAIL"} — defined ${got}/${total}, errors ${errCount}`);
} else {
  console.log("FAIL — could not parse verdict");
}

process.exit(ok ? 0 : 1);
