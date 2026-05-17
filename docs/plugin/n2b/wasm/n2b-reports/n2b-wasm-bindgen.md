# node2bun report

- mode : `check`
- racine : `/home/ubuntu/rsbun/wasm/wasm-bindgen`

## `.github/workflows/codspeed.yml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 32:9 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 34:11 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |

## `.github/workflows/main.yml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 123:7 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 146:7 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 163:7 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 180:7 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 198:9 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 213:9 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 232:7 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 254:7 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 324:7 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 386:7 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |
| 125:9 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 148:9 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 165:9 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 182:9 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 200:11 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 215:11 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 234:9 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 256:9 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 326:9 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 388:9 | `ci/node-version` | remplacer 'node-version' par 'bun-version: latest' | `bun-version: latest` |
| 363:12 | `cli/npm-install` | bun install → bun install | `bun install` |
| 392:12 | `cli/npm-install` | bun install → bun install | `bun install` |
| 235:12 | `cli/npm-i` | npm i → bun install | `bun install` |
| 236:12 | `cli/npm-i` | npm i → bun install | `bun install` |
| 395:12 | `cli/npm-test` | npm test → bun test | `bun test` |
| 366:12 | `cli/pnpm-install` | bun install → bun install | `bun install` |
| 393:12 | `cli/pnpm-install` | bun install → bun install | `bun install` |
| 368:12 | `cli/pnpm-run` | pnpm run → bun run | `bun run ` |

## `Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/serde` | crate Rust `serde` détecté (Serde (ser/deserialize)) — doc : https://serde.rs/ | `https://serde.rs/` |
| 1:1 | `ecosystem/serde-json` | crate Rust `serde_json` détecté (serde_json) — doc : https://docs.rs/serde_json | `https://docs.rs/serde_json` |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `benchmarks/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `benchmarks/wcodspeed/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/anyhow` | crate Rust `anyhow` détecté (anyhow (error handling)) — doc : https://docs.rs/anyhow | `https://docs.rs/anyhow` |
| 1:1 | `ecosystem/serde` | crate Rust `serde` détecté (Serde (ser/deserialize)) — doc : https://serde.rs/ | `https://serde.rs/` |
| 1:1 | `ecosystem/serde-json` | crate Rust `serde_json` détecté (serde_json) — doc : https://docs.rs/serde_json | `https://docs.rs/serde_json` |

## `crates/cli-support/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/anyhow` | crate Rust `anyhow` détecté (anyhow (error handling)) — doc : https://docs.rs/anyhow | `https://docs.rs/anyhow` |
| 1:1 | `ecosystem/serde` | crate Rust `serde` détecté (Serde (ser/deserialize)) — doc : https://serde.rs/ | `https://serde.rs/` |
| 1:1 | `ecosystem/serde-json` | crate Rust `serde_json` détecté (serde_json) — doc : https://docs.rs/serde_json | `https://docs.rs/serde_json` |

## `crates/cli/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/anyhow` | crate Rust `anyhow` détecté (anyhow (error handling)) — doc : https://docs.rs/anyhow | `https://docs.rs/anyhow` |
| 1:1 | `ecosystem/clap` | crate Rust `clap` détecté (clap (CLI parser)) — doc : https://docs.rs/clap | `https://docs.rs/clap` |
| 1:1 | `ecosystem/serde` | crate Rust `serde` détecté (Serde (ser/deserialize)) — doc : https://serde.rs/ | `https://serde.rs/` |
| 1:1 | `ecosystem/serde-json` | crate Rust `serde_json` détecté (serde_json) — doc : https://docs.rs/serde_json | `https://docs.rs/serde_json` |
| 1:1 | `ecosystem/ureq` | crate Rust `ureq` détecté (ureq (sync HTTP)) — doc : https://github.com/algesten/ureq | `https://github.com/algesten/ureq` |

## `crates/cli/tests/reference/import-target-deno.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 108:17 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/import-target-experimental-nodejs-module.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 109:17 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/import-target-nodejs.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 110:28 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |

## `crates/cli/tests/reference/import-target-web.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 193:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-deno-atomics.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 59:17 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-deno-mvp.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 26:17 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-deno.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 35:17 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-experimental-nodejs-module-atomics.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 73:25 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-experimental-nodejs-module-mvp.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 28:17 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-experimental-nodejs-module.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 37:17 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-nodejs-atomics.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 78:27 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |
| 106:14 | `imports/node-prefix` | préfixer 'worker_threads' avec 'node:' (recommandé) | `node:worker_threads` |

## `crates/cli/tests/reference/targets-target-nodejs-experimental-reset-state-function-atomics.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 103:27 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |
| 131:14 | `imports/node-prefix` | préfixer 'worker_threads' avec 'node:' (recommandé) | `node:worker_threads` |

## `crates/cli/tests/reference/targets-target-nodejs-experimental-reset-state-function-mvp.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 51:28 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |

## `crates/cli/tests/reference/targets-target-nodejs-experimental-reset-state-function.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 60:28 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |

## `crates/cli/tests/reference/targets-target-nodejs-mvp.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 28:28 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |

## `crates/cli/tests/reference/targets-target-nodejs.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 37:28 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |

## `crates/cli/tests/reference/targets-target-web-atomics.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 148:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-web-experimental-reset-state-function-atomics.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 172:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-web-experimental-reset-state-function-mvp.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 123:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-web-experimental-reset-state-function.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 133:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-web-mvp.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 101:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/targets-target-web.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 111:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/wasm-export-colon.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 798:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/cli/tests/reference/wasm-export-types.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 211:26 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `crates/futures/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `crates/js-sys/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `crates/js-sys/tests/wasm/Array.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `crates/js-sys/tests/wasm/Symbol.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `crates/js-sys/tests/wasm/WebAssembly.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:34 | `imports/node-prefix` | préfixer 'util' avec 'node:' (recommandé) | `node:util` |

## `crates/macro/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `crates/msrv/lib/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `crates/msrv/resolver/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `crates/test-macro/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/tokio` | crate Rust `tokio` détecté (tokio (async runtime)) — doc : https://tokio.rs/ | `https://tokio.rs/` |

## `crates/test/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/serde` | crate Rust `serde` détecté (Serde (ser/deserialize)) — doc : https://serde.rs/ | `https://serde.rs/` |
| 1:1 | `ecosystem/serde-json` | crate Rust `serde_json` détecté (serde_json) — doc : https://docs.rs/serde_json | `https://docs.rs/serde_json` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `crates/test/sample/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `crates/typescript-tests/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/serde` | crate Rust `serde` détecté (Serde (ser/deserialize)) — doc : https://serde.rs/ | `https://serde.rs/` |
| 1:1 | `ecosystem/serde-wasm` | crate Rust `serde-wasm-bindgen` détecté (serde → WASM) — doc : https://github.com/RReverser/serde-wasm-bindgen | `https://github.com/RReverser/serde-wasm-bindgen` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `crates/typescript-tests/package.json`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `pkg/jest-script` | script "test"='NODE_OPTIONS=--experimental-vm-modules jest --config ./jest.config.cjs' utilise jest — préférer 'bun test' (compatible describe/test/expect ; utiliser --preload reflect-metadata pour les décorateurs) | `bun test` |

## `crates/typescript-tests/run.sh`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 22:3 | `cli/npm-install` | bun install → bun install | `bun install` |
| 25:1 | `cli/npm-run` | npm run → bun run | `bun run ` |
| 59:1 | `cli/npm-run` | npm run → bun run | `bun run ` |
| 61:1 | `cli/npm-test` | npm test → bun test | `bun test` |

## `crates/typescript-tests/src/enums.ts`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:37 | `imports/bun-native` | remplacer '@jest/globals' par bun:test — importer depuis bun:test à la place | `bun:test` |

## `crates/typescript-tests/src/function_attrs.ts`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 3:31 | `imports/bun-native` | remplacer '@jest/globals' par bun:test — importer depuis bun:test à la place | `bun:test` |

## `crates/typescript-tests/src/simple_struct.ts`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:37 | `imports/bun-native` | remplacer '@jest/globals' par bun:test — importer depuis bun:test à la place | `bun:test` |

## `crates/typescript-tests/src/typescript_type.ts`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:37 | `imports/bun-native` | remplacer '@jest/globals' par bun:test — importer depuis bun:test à la place | `bun:test` |

## `crates/typescript-tests/tsconfig.json`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `tsconfig/module-legacy` | module='commonjs' — 'ESNext' ou 'Preserve' est recommandé pour Bun (ESM natif) |  |
| 1:1 | `tsconfig/target-legacy` | target='es6' — Bun supporte ESNext/ES2022+, downlevel inutile |  |
| 1:1 | `tsconfig/module-detection` | compilerOptions.moduleDetection absent — 'force' garantit que chaque fichier est ESM (évite les .js traités comme CJS) | `"force"` |

## `crates/web-sys/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `crates/webidl-tests/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `crates/webidl-tests/globals.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:30 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `crates/webidl/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/anyhow` | crate Rust `anyhow` détecté (anyhow (error handling)) — doc : https://docs.rs/anyhow | `https://docs.rs/anyhow` |
| 1:1 | `ecosystem/clap` | crate Rust `clap` détecté (clap (CLI parser)) — doc : https://docs.rs/clap | `https://docs.rs/clap` |

## `examples/add/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/add/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/canvas/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/canvas/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/char/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/char/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/closures/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/closures/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/console_log/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/console_log/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/dom/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/dom/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/duck-typed-interfaces/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/duck-typed-interfaces/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/explicit-resource-management/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/fetch/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/fetch/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/guide-supported-types-examples/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/guide-supported-types-examples/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/hello_world/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/hello_world/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/import_js/crate/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/import_js/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/julia_set/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/julia_set/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/nodejs-threads/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/nodejs-threads/package.json`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `cli/npm-run` | npm run → bun run | `bun run ` |
| 1:22 | `cli/npm-run` | npm run → bun run | `bun run ` |
| 1:1 | `cli/npm-run` | npm run → bun run | `bun run ` |
| 1:21 | `cli/npm-run` | npm run → bun run | `bun run ` |

## `examples/nodejs-threads/test-esm.mjs`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 13:21 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |
| 14:32 | `imports/node-prefix` | préfixer 'url' avec 'node:' (recommandé) | `node:url` |
| 12:63 | `imports/node-prefix` | préfixer 'worker_threads' avec 'node:' (recommandé) | `node:worker_threads` |
| 55:39 | `api/fileURLToPath` | Bun.fileURLToPath() est équivalent (ou utiliser import.meta.dir/path) |  |
| 89:39 | `api/fileURLToPath` | Bun.fileURLToPath() est équivalent (ou utiliser import.meta.dir/path) |  |

## `examples/nodejs-threads/test.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 12:67 | `imports/node-prefix` | préfixer 'worker_threads' avec 'node:' (recommandé) | `node:worker_threads` |
| 13:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `examples/nodejs_and_deno/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/package.json`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `cli/pnpm-run` | pnpm run → bun run | `bun run ` |

## `examples/paint/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/paint/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/performance/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/performance/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/playwright.spec.ts`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 79:30 | `api/exec` | utiliser le shell Bun ($`cmd`) ou Bun.spawn() à la place de exec |  |
| 129:13 | `api/exec` | utiliser le shell Bun ($`cmd`) ou Bun.spawn() à la place de exec |  |
| 153:13 | `api/exec` | utiliser le shell Bun ($`cmd`) ou Bun.spawn() à la place de exec |  |

## `examples/pnpm-workspace.yaml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `workspace/pnpm-yaml` | pnpm-workspace.yaml présent (1 patterns) — Bun lit "workspaces" dans le package.json racine. Migrer et supprimer ce fichier. | `"workspaces": ["*"]` |

## `examples/raytrace-parallel/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/panic-hook` | crate Rust `console_error_panic_hook` détecté (panic hook → console) — doc : https://github.com/rustwasm/console_error_panic_hook | `https://github.com/rustwasm/console_error_panic_hook` |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/serde-wasm` | crate Rust `serde-wasm-bindgen` détecté (serde → WASM) — doc : https://github.com/RReverser/serde-wasm-bindgen | `https://github.com/RReverser/serde-wasm-bindgen` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/raytrace-parallel/index.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 80:18 | `api/performance-now` | Bun.nanoseconds() offre une horloge haute précision (retourne nanosecondes depuis démarrage) |  |
| 97:17 | `api/performance-now` | Bun.nanoseconds() offre une horloge haute précision (retourne nanosecondes depuis démarrage) |  |

## `examples/request-animation-frame/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/request-animation-frame/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/synchronous-instantiation/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/todomvc/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/askama` | crate Rust `askama` détecté (Askama (Jinja-like, compile-time)) — doc : https://github.com/askama-rs/askama | `https://github.com/askama-rs/askama` |
| 1:1 | `ecosystem/panic-hook` | crate Rust `console_error_panic_hook` détecté (panic hook → console) — doc : https://github.com/rustwasm/console_error_panic_hook | `https://github.com/rustwasm/console_error_panic_hook` |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/todomvc/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/wasm-audio-worklet/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/panic-hook` | crate Rust `console_error_panic_hook` détecté (panic hook → console) — doc : https://github.com/rustwasm/console_error_panic_hook | `https://github.com/rustwasm/console_error_panic_hook` |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/wasm-in-wasm-imports/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/wasm-in-wasm-imports/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/wasm-in-wasm/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `examples/wasm-in-wasm/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/wasm-in-web-worker/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/panic-hook` | crate Rust `console_error_panic_hook` détecté (panic hook → console) — doc : https://github.com/rustwasm/console_error_panic_hook | `https://github.com/rustwasm/console_error_panic_hook` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/weather_report/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/gloo` | crate Rust `gloo` détecté (gloo (toolkit Rust+WASM)) — doc : https://gloo-rs.web.app/ | `https://gloo-rs.web.app/` |
| 1:1 | `ecosystem/reqwest` | crate Rust `reqwest` détecté (reqwest (HTTP client)) — doc : https://docs.rs/reqwest | `https://docs.rs/reqwest` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/weather_report/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/webaudio/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/webaudio/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/webgl/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/webgl/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/webrtc_datachannel/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/webrtc_datachannel/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/websockets/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/webxr/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/panic-hook` | crate Rust `console_error_panic_hook` détecté (panic hook → console) — doc : https://github.com/rustwasm/console_error_panic_hook | `https://github.com/rustwasm/console_error_panic_hook` |
| 1:1 | `ecosystem/js-sys` | crate Rust `js-sys` détecté (js-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/js_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/js_sys/` |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/wasm-bindgen-futures` | crate Rust `wasm-bindgen-futures` détecté (wasm-bindgen-futures) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/webxr/package.json`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `cli/npm-run` | npm run → bun run | `bun run ` |

## `examples/webxr/webpack.config.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:23 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `examples/without-a-bundler-no-modules/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `examples/without-a-bundler/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |
| 1:1 | `ecosystem/web-sys` | crate Rust `web-sys` détecté (web-sys) — doc : https://rustwasm.github.io/wasm-bindgen/api/web_sys/ | `https://rustwasm.github.io/wasm-bindgen/api/web_sys/` |

## `tests/crates/a/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `tests/crates/b/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `tests/no-std/Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/wasm-bindgen` | crate Rust `wasm-bindgen` détecté (wasm-bindgen) — doc : https://rustwasm.github.io/wasm-bindgen/ | `https://rustwasm.github.io/wasm-bindgen/` |

## `tests/wasm/api.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/arg_names.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/async_vecs.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/bigint.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/char.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/classes.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |
| 36:18 | `imports/node-prefix` | préfixer 'process' avec 'node:' (recommandé) | `node:process` |

## `tests/wasm/closures.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/comments.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:21 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |
| 5:25 | `api/require-resolve` | Bun.resolveSync() remplace require.resolve() (ESM + CJS, plus rapide) |  |

## `tests/wasm/duplicate_deps.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/enum_vecs.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/enums.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/final.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/futures.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/gc.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/getters_and_setters.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/import_class.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/imports.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |
| 3:21 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |
| 100:20 | `api/require-resolve` | Bun.resolveSync() remplace require.resolve() (ESM + CJS, plus rapide) |  |

## `tests/wasm/js_keywords.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/js_namespace_exports.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/js_objects.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/js_vec.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/link_to.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:21 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |
| 2:22 | `imports/node-prefix` | préfixer 'url' avec 'node:' (recommandé) | `node:url` |
| 4:50 | `api/fileURLToPath` | Bun.fileURLToPath() est équivalent (ou utiliser import.meta.dir/path) |  |

## `tests/wasm/math.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/node.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/nullable.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/option.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/optional_primitives.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/reexport.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/result.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/result_jserror.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/rethrow.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/simple.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |
| 27:16 | `imports/node-prefix` | préfixer 'process' avec 'node:' (recommandé) | `node:process` |

## `tests/wasm/slice.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/string_vecs.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/struct_vecs.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/structural.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/usize.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |

## `tests/wasm/validate_prt.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |
| 7:30 | `api/process-env` | Bun.env est un alias plus court de process.env (préférence stylistique) |  |

## `tests/wasm/variadic.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:25 | `imports/node-prefix` | préfixer 'assert' avec 'node:' (recommandé) | `node:assert` |
