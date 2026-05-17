# node2bun report

- mode : `check`
- racine : `/home/ubuntu/rsbun/wasm/binaryen`

## `.github/workflows/ci.yml`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 86:9 | `cli/npm-install` | bun install → bun install | `bun install` |
| 141:9 | `cli/npm-install` | bun install → bun install | `bun install` |
| 171:9 | `cli/npm-install` | bun install → bun install | `bun install` |
| 209:9 | `cli/npm-install` | bun install → bun install | `bun install` |
| 294:9 | `cli/npm-install` | bun install → bun install | `bun install` |
| 332:9 | `cli/npm-install` | bun install → bun install | `bun install` |

## `scripts/benchmarking/bench.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 23:21 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |
| 137:19 | `api/performance-now` | Bun.nanoseconds() offre une horloge haute précision (retourne nanosecondes depuis démarrage) |  |
| 139:25 | `api/performance-now` | Bun.nanoseconds() offre une horloge haute précision (retourne nanosecondes depuis démarrage) |  |

## `scripts/fuzz_shell.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 22:25 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |

## `scripts/test/node-esm-loader.mjs`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 4:22 | `imports/node-prefix` | préfixer 'process' avec 'node:' (recommandé) | `node:process` |
| 3:19 | `imports/node-prefix` | préfixer 'path' avec 'node:' (recommandé) | `node:path` |

## `scripts/validation_shell.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 10:21 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |

## `test/lit/passes/string-lowering.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 2:46 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |

## `test/lit/wasm-split/call_exports.mjs`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 8:22 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |
| 25:1 | `api/fs-writeFileSync` | remplacer fs.writeFileSync(path, data) par await Bun.write(path, data) | `await Bun.write(outFile, profileData)` |

## `test/unit/input/asyncify.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 9:19 | `imports/node-prefix` | préfixer 'fs' avec 'node:' (recommandé) | `node:fs` |
