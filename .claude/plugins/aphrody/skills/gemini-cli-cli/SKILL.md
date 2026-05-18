---
name: gemini-cli-cli
description: Migre la chaîne CLI de gemini-cli (bin/scripts/lockfile/sandbox docker/SEA) vers Bun — replace npm/cross-env/tsx/vitest, configure bin via bun build --compile, gère workspaces + Artifact Registry
---

# gemini-cli — CLI tooling migration runbook

Cible : `tests/targets/gemini-cli/` (monorepo workspace `packages/*` + `scripts/` + `sea/`).
Ce skill couvre tout sauf le rendu JSX (voir **`gemini-cli-jsx`**) : entrées bin, scripts package.json, lockfile, sandbox Docker, tests vitest, build SEA.

## Carte des consommateurs Node à migrer

```
gemini-cli/
├── package.json          ← scripts: 60+ entrées `node`/`npm`/`npx`/`tsx`/`cross-env`
├── packages/
│   ├── cli/              ← bin "gemini" → dist/index.js
│   ├── core/             ← @google/gemini-cli-core (workspace dep)
│   ├── a2a-server/       ← serveur HTTP standalone
│   ├── devtools/         ← outils dev internes
│   ├── sdk/              ← SDK utilisateur
│   └── vscode-ide-companion/  ← extension VSCode (DOIT rester Node)
├── scripts/              ← node + tsx → bun run
├── sea/                  ← Single Executable Application Node — à remplacer
└── esbuild.config.js     ← devient inutile (Bun bundler)
```

**Important** : `vscode-ide-companion` doit rester Node — VSCode héberge son extension dans son propre runtime Node. Ne pas migrer ce package.

## Étape 1 — Audit

```bash
n2b tests/targets/gemini-cli --report=json --report-card \
  | jq '.report_card // {note: "ajouter --migrate pour la report card"}'
```

Cibles attendues :
- `cli/npm-install`, `cli/npm-run`, `cli/npx-tsx` (rules `cli/*`)
- `imports/bun-native` sur `chalk`, `yaml`, `ansi-escapes` (déjà natifs Bun)
- `globals/process-env`, `globals/__dirname`, `globals/require-dynamic`
- `imports/node-prefix` avec `compat: full` (≈22 modules) → migrent automatiquement

## Étape 2 — package.json racine

Avant :
```json
{
  "scripts": {
    "start": "cross-env NODE_ENV=development node scripts/start.js",
    "build": "node scripts/build.js",
    "test": "npm run test --workspaces --if-present",
    "auth:npm": "npx google-artifactregistry-auth"
  },
  "engines": { "node": ">=20.0.0" }
}
```

Après :
```json
{
  "scripts": {
    "start": "NODE_ENV=development bun ./scripts/start.ts",
    "build": "bun ./scripts/build.ts",
    "test": "bun --filter '*' test",
    "auth:npm": "bunx google-artifactregistry-auth"
  },
  "engines": { "bun": ">=1.3.0" },
  "trustedDependencies": ["@google/gemini-cli-core"]
}
```

Substitutions mécaniques (n2b les fait via `--migrate`) :
- `cross-env FOO=bar node X` → `FOO=bar bun X` (Bun lit l'env inline comme bash)
- `npm run X --workspaces` → `bun --filter '*' X`
- `npx Y` → `bunx Y`
- `tsx ./scripts/foo.ts` → `bun ./scripts/foo.ts` (Bun exécute TS direct)
- `node --inspect-brk X` → `bun --inspect-brk X`

## Étape 3 — Workspaces

gemini-cli a `"workspaces": ["packages/*"]` qui marche tel quel sous Bun. Vérifier :

```bash
cd tests/targets/gemini-cli
bun install   # crée bun.lock à partir de package-lock.json
bun pm ls     # liste les workspaces résolus
```

Si Artifact Registry est requis (auth Google), garder `.npmrc` racine — Bun le lit aussi :
```
@google:registry=https://us-west1-npm.pkg.dev/gemini-code-dev/npm-public/
//us-west1-npm.pkg.dev/gemini-code-dev/npm-public/:_authToken=${NPM_TOKEN}
```

## Étape 4 — Bin entry → bun build --compile

Avant : bin "gemini" pointe sur `dist/index.js` (bundle esbuild).
Après : compile-time bundle avec Bun.

```bash
cd packages/cli
bun build ./src/gemini.tsx \
  --compile \
  --minify \
  --sourcemap \
  --target=bun-linux-x64 \
  --outfile=../../dist/gemini-linux-x64
```

**Matrice cross-compile** (Bun supporte natively) :

```bash
for target in bun-linux-x64 bun-linux-arm64 bun-darwin-x64 bun-darwin-arm64 bun-windows-x64; do
  bun build ./src/gemini.tsx --compile --minify \
    --target=$target \
    --outfile=../../dist/gemini-${target#bun-}
done
```

Update `package.json` bin pour pointer sur le binaire de la plateforme courante (script post-install) ou garder un wrapper JS.

## Étape 5 — Tests vitest → bun:test

vitest fonctionne sous Bun mais lentement. Pour migrer :

```ts
// Avant
import { describe, it, expect, vi } from 'vitest';
vi.mock('./foo');

// Après
import { describe, it, expect, mock } from 'bun:test';
mock.module('./foo', () => ({ /* ... */ }));
```

API quasi-compatibles :
- `vi.fn()` → `mock(() => {})`
- `vi.spyOn(obj, 'm')` → `spyOn(obj, 'm')`
- `vi.mock('./mod')` → `mock.module('./mod', () => ({...}))` (note : import statique)
- snapshots `toMatchSnapshot()` identique
- coverage : `bun test --coverage` (lcov natif)

Lance la suite :
```bash
bun --filter '*' test
```

Pour les tests qui appellent `setRawMode` (Ink), garder `import { render } from 'ink-testing-library'` — fonctionne avec `bun:test` car le testing library mock le TTY.

## Étape 6 — SEA (Single Executable Application)

`sea/` utilise `node --experimental-sea-config`. À remplacer par `bun build --compile` (étape 4). Supprimer :
- `sea/sea-config.json`
- `sea/sea-launch.test.js` (devient `compile.test.ts` qui invoke `bun build --compile`)
- script `test:sea-launch`

Le binaire produit par `bun --compile` est :
- Plus petit (~80 MB → ~55 MB pour gemini-cli typique)
- Démarre plus vite (~50 ms vs ~200 ms node SEA)
- Cross-platform sans Node.js installé

## Étape 7 — Sandbox Docker

`scripts/build_sandbox.js` builde `Dockerfile`. Update :

```dockerfile
# Avant
FROM node:20-alpine
RUN npm install -g @google/gemini-cli

# Après
FROM oven/bun:1.3-alpine
RUN bun install -g @google/gemini-cli
# OU mieux : copier le binaire compilé
COPY ./dist/gemini-linux-x64 /usr/local/bin/gemini
```

L'image résultante passe de ~180 MB à ~30 MB (binaire seul, pas de runtime).

## Étape 8 — VSCode companion (NE PAS migrer)

`packages/vscode-ide-companion` reste sur Node. Marquer dans `n2b.json` :

```json
{
  "ignore": ["packages/vscode-ide-companion/**"],
  "rules": {
    "imports/bun-native": "off"
  }
}
```

## Étape 9 — Migration end-to-end via n2b

```bash
cd tests/targets/gemini-cli

# Crée n2b.json (ignore VSCode + a2a-server si nécessaire)
cat > n2b.json <<'EOF'
{
  "ignore": ["packages/vscode-ide-companion/**", "**/dist/**", "third_party/**"],
  "rules": { "globals/process-env": "off" }
}
EOF

# Migre + scaffold polyfills bunpp pour les modules 🔴 (vm2, etc.)
n2b . --migrate --scaffold-polyfills --report=json > migration-report.json

# Vérifie la report card
jq '.report_card' migration-report.json
# {
#   "auto_migratable_pct": 0.87,
#   "manual_residue": [
#     { "rule_id": "imports/node-prefix", "file": "packages/core/src/sandbox.ts",
#       "line": 12, "reason": "node:vm2 missing in Bun (compat=missing)",
#       "suggestion": "bunx n2b bunpp scaffold node-vm2" }
#   ]
# }
```

`.n2b/state.json` track l'avancement (`status: in_progress` → `complete` quand `auto_migratable_pct >= 0.95` et résidu manuel résolu).

## Étape 10 — CI

`.github/workflows/ci.yml` :

```yaml
- uses: oven-sh/setup-bun@v2
  with:
    bun-version: 1.3
- run: bun install --frozen-lockfile
- run: bun --filter '*' test
- run: bun --filter '*' run typecheck
- run: bun build ./packages/cli/src/gemini.tsx --compile --target=bun-linux-x64
```

Retirer `setup-node@v4`, `npm ci`, `cache: npm`.

## Anti-patterns à éviter

- **Ne pas** garder `cross-env` après migration — Bun lit `FOO=bar bun X` natively, comme bash. `cross-env` n'a de sens que sur Windows + npm cmd.exe.
- **Ne pas** exécuter `bun install` dans le bin compilé — `--compile` embed les deps, runtime install casse l'isolation.
- **Ne pas** mixer `npm` et `bun` (ex. `npm test` puis `bun build`) — supprime `package-lock.json` après création de `bun.lock`.
- **Ne pas** retirer `.npmrc` Artifact Registry si l'org en dépend — Bun le respecte.

## Sources

- `tests/targets/gemini-cli/package.json` — état actuel
- `tests/targets/gemini-cli/packages/cli/package.json` — bin entry
- `docs/bun/bundler/executables.mdx` — `--compile` matrix
- `docs/bun/test/writing.mdx` — bun:test API
- `docs/bun/install/workspaces.mdx` — `bun --filter` syntaxe
- `crates/n2b-registry/registry/cli.toml` — règles `cli/*` détectées
