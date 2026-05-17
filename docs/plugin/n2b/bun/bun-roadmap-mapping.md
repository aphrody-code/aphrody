# Bun Roadmap #159 — mapping avec nos projets

**Source :** https://github.com/oven-sh/bun/issues/159
**Archivé localement :** `bun-roadmap-159.md`
**Objectif ultime :** compléter les items ouverts de cette roadmap.

## Items où nos projets ont un impact direct

### ✅ Déjà aidé (userland via n2b)

| Item roadmap | Statut Bun | Contribution n2b |
|---|---|---|
| Migration `pnpm-lock.yaml` → `bun.lock` | ✅ done côté Bun | `n2b` scan `lock/rival` + scanner `pnpm_workspace.yaml` + mode `--migrate` |
| Catalogs pnpm support | ✅ done | détection via scanner `pnpm-workspace.yaml` |
| `bun outdated` | ✅ done | scripts root package.json mappent vers `bun outdated` |
| `bun publish` | ✅ done | `@n2b/cli` prêt à publier |
| Text-based lockfile (`bun.lock`) | ✅ done | `n2b` scanners utilisent `bun.lock` |

### 🎯 Items ouverts où on peut livrer du code userland utile

| Item roadmap | Notre angle d'attaque |
|---|---|
| `#7589` **Running scripts with filters, parallel via `bun run`** | On a déjà déployé **Turborepo** dans n2b + md3-ui. On peut documenter le pattern comme bridge en attendant l'implémentation native — et contribuer un benchmark |
| `#947` **REPL support** (remplacer `bun repl` third-party) | Prototype userland via `bun:ffi` + **libreadline** (disponible `/usr/lib/.../libreadline.so.8`). Build-your-own-shell style |
| `#6608` **Nested resolutions/overrides** | Scanner n2b qui détecte les `resolutions`/`overrides` imbriqués dans `package.json` et signale le cas edge |
| `bun update --interactive` #4895 | Potentiel flavor `n2b app init --flavor bun-update-tui` via Ink (déjà là) |
| **Distribute debug/assertion build** | Documenter la procédure `bun run build` (on a `bun-internals/oven-sh-zig` cloné déjà) |
| `bun init` + `engines.bun` | n2b `pkg/engines-pm` détecte déjà l'absence — on pourrait ajouter autofix `bun add engines.bun` |
| **Rewrite `node:http`** (PR #14384) | Hors périmètre userland mais n2b scanner détecte `http.createServer` → `Bun.serve` (déjà fait) |
| **Much more comprehensive N-API coverage** | Notre `n2b bin --flavor native` (napi-rs + bun-native-plugin) peut servir de bench/stress tests |
| **Fast HMR dev server** `#14324` | `n2b ui init --flavor md3-ui` (Next 16 Turbopack) sert déjà |
| **Implement popular framework integration (Next.js)** | md3-ui framework scaffold est pile dans cette direction |
| **Easy & powerful plugin API** | `bun-native-plugin-rs` existe déjà + on a templates `n2b bin --flavor native/mdx/lightningcss` |

### 🔬 Items où notre `bun-ffi-labs` sert de proving ground

- **V8 C++ APIs (canvas, node-pty)** : exploration bun:ffi de libncurses/libreadline pour simuler node-pty
- **N-API test coverage** : 27 tests validés sur bun:ffi + bun:cc — ajouter gypi-style benchmarks
- **Undici override investigation** : on peut builder un HTTP client via bun:ffi→libcurl (déjà dans les plans) et comparer

## Retard Bun sur Node.js (audit 2026-04-17)

### Compat API `node:*` — ~85-87% effectif
Sur ~700 APIs Node.js : 610 OK, 60 partielles, 30 stubs/absentes. 119 TODO/notImplemented répartis sur 27 modules.

| Rang | Module | Statut | Gaps concrets |
|---|---|---|---|
| 1 | `node:test` | 🔴 stub | mocks, snapshots, timers |
| 2 | `node:repl` | 🔴 stub | REPL interactif (cible #947) |
| 3 | `node:http2` | 🟡 95% | `allowHTTP1`, `pushStream`, ALTSVC |
| 4 | `node:worker_threads` | 🟡 70% | stdio options, `markAsUntransferable` |
| 5 | `node:inspector` | 🟡 10% | Debugger/Runtime/Network CDPs absents |
| 6 | `node:child_process` | 🟡 80% | `proc.gid/uid`, IPC socket handles |
| 7 | `node:vm` / `node:tls` | 🟡 | `measureMemory` / `createSecurePair` |

Solides (>90%) : `fs`, `path`, `crypto`, `zlib`, `events`, `os`, `dgram`, `dns`.

### Retard écosystème
- **Stabilité prod** — crashes/memleaks vs Node hardened (ASAN actif depuis v1.2.2)
- **Debugger** — Chrome DevTools absent, DAP VSCode partiel
- **Plateformes** — Windows preview instable, Cloudflare Workers non supporté
- **Dev server HMR natif** — ouvert (#14324), on pallie via Turborepo
- **Test runner** — 85% compat Jest (snapshots/mocks partiels)
- **Workspaces/registries privés** — edge cases non clos

### Points forts Bun (où il mène)
Startup ~3×, install 2-3×, API intégrée (FFI, shell, sqlite), `bun build --compile`.

## Plan d'action

### ✅ Fait (2026-04-17)
1. **n2b monorepo** — refacto Bun workspaces + Cargo workspace committée (`8b83771`)
   - `packages/n2b` (@n2b/core) + `packages/n2b-cli` (@n2b/cli)
   - `turbo.json` + tsconfig composite (project references)
   - Scanners Rust ajoutés : `components_json`, `tauri_conf`, `turbo_json`
   - Nouveau `ui_cmd` (scaffold TUI)
2. Build TS (turbo) + Rust release : OK

### Court terme
- **REPL prototype userland** dans `bun-ffi-labs` (bun:ffi → libreadline) — cible #947
- Push du commit n2b vers `aphrody-code/n2b`
- Commit + push des autres projets en attente (llmstxt-rs, bun-ffi-labs)

### Moyen terme
- Crate `bun-repl-rs` : REPL natif via FFI, publiable avant que Bun n'ait le sien
- Scanner n2b agressif : `node:http` → `Bun.serve`
- Polyfills bun++ pour les 8 gaps critiques canary : `node-sqlite`, `node-util-ext`, `node-domain`, `node-repl`
- Benchmarks N-API via `bun-ffi-labs` (mesurer progrès canary)

### Long terme (contribution upstream)
- Picker 1-5 tests Node.js qui fail dans `src/js/node/*`
- Cibles prioritaires : `http2` (ALTSVC/pushStream), `worker_threads` (stdio), `async_hooks` (v8 promise hooks)
- Itérer sur ~10 tests pour contribuer au % node-compat (passer de 85-87% à 90% cible roadmap)

## Liens utiles

- Roadmap : https://github.com/oven-sh/bun/issues/159
- Bounty program : https://x.com/jarredsumner/status/1914830430811177181
- Bun source : `/home/ubuntu/rsbun/bun-internals/` (fork Zig local)
- nos projets : `n2b/` (monorepo), `llmstxt-rs/`, `bun-ffi-labs/`, `bun++/` (polyfills), `md3-ui` (scaffold)
