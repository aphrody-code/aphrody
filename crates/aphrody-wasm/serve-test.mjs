// SPDX-License-Identifier: Apache-2.0
// Minimal bun dev server for manual browser testing of test-browser.html.
// Usage (from workspace root):
//   bun run crates/aphrody-wasm/serve-test.mjs
// Then open http://localhost:8787 in a browser.

import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dir = dirname(fileURLToPath(import.meta.url));
const wasmPkgDir = join(__dir, "..", "..", "target", "aphrody-wasm-pkg");
const htmlFile   = join(__dir, "test-browser.html");

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js":   "application/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".ts":   "application/typescript; charset=utf-8",
};

function mime(path) {
  for (const [ext, type] of Object.entries(MIME)) {
    if (path.endsWith(ext)) return type;
  }
  return "application/octet-stream";
}

Bun.serve({
  port: 8787,
  async fetch(req) {
    const url  = new URL(req.url);
    const path = url.pathname;

    // Root -> test-browser.html
    if (path === "/" || path === "/index.html") {
      const f = Bun.file(htmlFile);
      return new Response(f, { headers: { "Content-Type": mime(".html") } });
    }

    // /pkg/<file> -> target/aphrody-wasm-pkg/<file>
    if (path.startsWith("/pkg/")) {
      const rel  = path.slice("/pkg/".length);
      const full = join(wasmPkgDir, rel);
      const f    = Bun.file(full);
      if (!(await f.exists())) {
        return new Response("not found: " + full, { status: 404 });
      }
      return new Response(f, { headers: { "Content-Type": mime(full) } });
    }

    return new Response("404", { status: 404 });
  },
});

console.log("aphrody-wasm test server: http://localhost:8787");
