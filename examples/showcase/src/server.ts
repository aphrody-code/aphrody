// Bun-native fullstack dev/preview server. No Next.js, no Vite — `Bun.serve`
// bundles the HTML entry (and the .tsx/.css it references) on the fly, with
// hot-module reloading in development. See https://bun.com/docs/runtime/bun-apis
import index from "./index.html";
import { join } from "path";

const server = Bun.serve({
  port: Number(Bun.env.PORT ?? 3000),
  development: {
    hmr: true,
    console: true,
  },
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

console.log(`m3-showcase listening on ${server.url}`);
