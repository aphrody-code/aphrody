# node2bun report

- mode : `check`
- racine : `/home/ubuntu/rsbun/wasm/wasm-pack`

## `.github/workflows/test.yml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 37:9 | `ci/setup-node` | actions/setup-node → oven-sh/setup-bun@v2 | `uses: oven-sh/setup-bun@v2` |

## `Cargo.toml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `ecosystem/anyhow` | crate Rust `anyhow` détecté (anyhow (error handling)) — doc : https://docs.rs/anyhow | `https://docs.rs/anyhow` |
| 1:1 | `ecosystem/serde` | crate Rust `serde` détecté (Serde (ser/deserialize)) — doc : https://serde.rs/ | `https://serde.rs/` |
| 1:1 | `ecosystem/serde-json` | crate Rust `serde_json` détecté (serde_json) — doc : https://docs.rs/serde_json | `https://docs.rs/serde_json` |
| 1:1 | `ecosystem/clap` | crate Rust `clap` détecté (clap (CLI parser)) — doc : https://docs.rs/clap | `https://docs.rs/clap` |
| 1:1 | `ecosystem/ureq` | crate Rust `ureq` détecté (ureq (sync HTTP)) — doc : https://github.com/algesten/ureq | `https://github.com/algesten/ureq` |

## `npm/binary.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:27 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |
| 3:21 | `imports/node-prefix` | préfixer 'os' avec 'node:' (recommandé) | `node:os` |

## `npm/install.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `shebang/node` | shebang 'node' → 'bun' | `#!/usr/bin/env bun` |

## `npm/run.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `shebang/node` | shebang 'node' → 'bun' | `#!/usr/bin/env bun` |

## `npm/yarn.lock`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `lock/rival` | lockfile concurrent 'yarn.lock' présent — exécuter 'bun install' puis supprimer ce fichier |  |
