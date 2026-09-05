// Bun-native smoke test: boot the fullstack server, fetch the HTML entry and
// the bundled client module, assert they resolve. Uses only Bun.serve + the
// standard fetch/Response Web APIs.
import index from "./index.html";
import { join } from "node:path";

const server = Bun.serve({
  port: 0, // ephemeral
  development: { hmr: false },
  routes: {
    "/bun_rs_bg.wasm": new Response(
      Bun.file(join(import.meta.dir, "../../../packages/bun-rs/pkg/bun_rs_bg.wasm")),
      {
        headers: { "Content-Type": "application/wasm" },
      },
    ),
    "/*": index,
  },
});

let failed = false;
const check = (ok: boolean, msg: string) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${msg}`);
  if (!ok) failed = true;
};

try {
  const html = await fetch(new URL("/", server.url)).then((r) => r.text());
  check(html.includes('<div class="shell">'), "html entry serves shell mount point");

  const scriptMatch = html.match(/src="([^"]+\.js)"/);
  check(Boolean(scriptMatch), "html references a bundled client script");

  if (scriptMatch) {
    const js = await fetch(new URL(scriptMatch[1], server.url));
    check(js.ok, `bundled client script responds ${js.status}`);
    const body = await js.text();
    check(body.length > 1000, "bundled client script is non-trivial");
  }

  const wasmRes = await fetch(new URL("/bun_rs_bg.wasm", server.url));
  check(wasmRes.ok, `WASM binary serves with status ${wasmRes.status}`);
  check(
    wasmRes.headers.get("Content-Type") === "application/wasm",
    "WASM binary has correct MIME type",
  );
  const wasmBytes = await wasmRes.arrayBuffer();
  check(wasmBytes.byteLength > 100000, "WASM binary is non-trivial");
} finally {
  server.stop(true);
}

if (failed) process.exit(1);
console.log("m3-showcase smoke passed");
