# node2bun report

- mode : `check`
- racine : `/home/ubuntu/rsbun/wasm/wabt`

## `docs/demo/libwabt.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:844 | `api/fileURLToPath` | Bun.fileURLToPath() est équivalent (ou utiliser import.meta.dir/path) |  |
| 1:3562 | `api/new-url-import-meta` | utiliser import.meta.dir ou path.join(import.meta.dir, ...) plutôt que new URL(..., import.meta.url) |  |

## `docs/demo/third_party.bundle.js`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:33990 | `api/process-env` | Bun.env est un alias plus court de process.env (préférence stylistique) |  |

## `docs/demo/third_party/package-lock.json`

| ligne | règle | message | remplacement |
| --- | --- | --- | --- |
| 1:1 | `lock/rival` | lockfile concurrent 'package-lock.json' présent — exécuter 'bun install' puis supprimer ce fichier |  |
