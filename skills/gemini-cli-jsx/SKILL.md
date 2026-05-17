---
name: gemini-cli-jsx
description: Migre les 414 .tsx Ink/React de gemini-cli vers le runtime JSX natif de Bun — supprime esbuild/vitest-jsx, configure tsconfig + bunfig, gère les pièges Ink (raw mode TTY, useStdin, alt-screen)
---

# gemini-cli — JSX migration runbook

Cible : `tests/targets/gemini-cli/packages/cli/src/**/*.tsx` (414 fichiers Ink + React).
Bun a un runtime JSX natif (transpileur Zig, pas oxc) qui remplace esbuild/swc/tsc pour le JSX. Ce skill liste les étapes mécaniques + les 4 pièges Ink connus.

## Pré-requis

- `n2b` build release dispo (`cargo build --release -p n2b`)
- Bun ≥ 1.3 (pour `--hot` stable + Fast Refresh)
- Cible : monorepo `gemini-cli` cloné dans `tests/targets/gemini-cli/`

## Étape 1 — Audit des patterns JSX existants

```bash
n2b tests/targets/gemini-cli/packages/cli --report=json \
  | jq '.files[].findings[] | select(.rule_id | startswith("api/") or startswith("imports/")) | .rule_id' \
  | sort -u
```

Patterns attendus (Ink-specific) :
- `import { render, Box, Text, useInput, useStdin } from 'ink'`
- `import React, { useState, useEffect, useMemo } from 'react'`
- `import Spinner from 'ink-spinner'`, `Gradient from 'ink-gradient'`, etc.

## Étape 2 — tsconfig pour le runtime JSX natif Bun

Bun honore `compilerOptions.jsx` du `tsconfig.json` racine. Pour Ink/React 19 :

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "Preserve",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "jsxImportSource": "react",
    "allowImportingTsExtensions": true,
    "verbatimModuleSyntax": true,
    "noEmit": true,
    "types": ["bun-types"]
  }
}
```

Points clés :
- `jsx: "react-jsx"` → automatic runtime, pas besoin de `import React from 'react'` dans chaque .tsx
- `jsxImportSource: "react"` → Bun importe `react/jsx-runtime` automatiquement
- **Ne pas** mettre `jsxImportSource: "ink"` — Ink n'a pas de jsx-runtime, il consomme React tel quel
- Retirer `esbuild.config.js` et `tsconfig.build.json` (devenus inutiles)

## Étape 3 — bunfig.toml (optionnel mais recommandé)

À la racine du monorepo :

```toml
[run]
# Hot reload : utile pour `bun --hot ./packages/cli/src/index.ts`
hot = false  # défaut, opt-in via CLI

[install]
# gemini-cli a un .npmrc Artifact Registry — on garde
# scopes via NPM_CONFIG_REGISTRY env si besoin

[loader]
# Bun infère .tsx → tsx loader. Pas besoin de configurer.
```

## Étape 4 — Pièges Ink connus sous Bun

### 4.1 `useStdin` + raw mode

Ink utilise `process.stdin.setRawMode(true)`. Bun supporte mais via `Bun.stdin` qui ne *re-export* pas `setRawMode`. Solution : Ink détecte `process.stdin` natif → fonctionne. **Ne pas remplacer `process.stdin` par `Bun.stdin`** dans les composants Ink.

```tsx
// CORRECT — laisser tel quel
import { useStdin } from 'ink';
const { stdin, setRawMode } = useStdin();

// INCORRECT — Bun.stdin n'a pas setRawMode
import { stdin } from 'bun';
```

### 4.2 Alt-screen / `enterAltScreen`

`ink` envoie `\x1b[?1049h`/`\x1b[?1049l` directement à `process.stdout.write`. Bun route ça correctement. Aucune action.

### 4.3 Fast Refresh sous `bun --hot`

Bun 1.3+ implémente React Fast Refresh (vu dans `upstream/bun/src/bundler/p.zig`). Pour Ink, le re-render full-tree est OK car Ink reconstruit son framebuffer. Activer :

```bash
bun --hot ./packages/cli/src/index.ts
```

### 4.4 `import.meta.resolve` vs `require.resolve`

gemini-cli a quelques `require.resolve('@google/gemini-cli-core/...')` (rule `globals/require-dynamic`). Sous ESM Bun :

```ts
// Avant
const corePath = require.resolve('@google/gemini-cli-core');
// Après
const corePath = import.meta.resolve('@google/gemini-cli-core');
```

`n2b --aggressive` propose le rewrite automatiquement (rule `globals/require-dynamic`).

## Étape 5 — Lancement dev (hot reload)

```bash
cd tests/targets/gemini-cli/packages/cli
bun --hot ./src/gemini.tsx
```

Si TTY n'est pas alloué (ex. exécution dans un sub-process non-tty), Ink throw `Raw mode is not supported`. Forcer un PTY :

```bash
bun --hot ./src/gemini.tsx </dev/tty
# ou via script-pty si CI
```

## Étape 6 — Build production (single-file executable)

Bundler Bun avec `--compile` produit un binaire portable :

```bash
bun build ./packages/cli/src/gemini.tsx \
  --compile --minify --sourcemap \
  --target=bun-linux-x64 \
  --outfile gemini
```

Pour cross-compile (mac/windows) : voir le skill **`gemini-cli-cli`** (étape "Single-file executable matrix").

## Étape 7 — Vérification

```bash
# Type-check (Bun n'exécute pas le type-check au runtime)
bun tsc --noEmit

# Lance l'app
./gemini --help

# n2b doit voir 0 finding api/* sur le code post-migration
n2b tests/targets/gemini-cli/packages/cli --report=text | rg -e 'api/|imports/'
```

## Anti-patterns à ne PAS introduire

- **Ne pas** wrapper React dans un loader Bun custom — le runtime JSX natif est ~5× plus rapide qu'esbuild.
- **Ne pas** activer `jsxFactory: "h"` ou autre — Ink consomme React, le factory doit rester `react-jsx`.
- **Ne pas** mixer ESM + CommonJS dans un même package — gemini-cli est pure ESM (`"type": "module"`), garder.

## Sources

- `docs/bun/runtime/jsx.mdx` — runtime natif
- `docs/bun/bundler/loaders.mdx` — règles d'inférence par extension
- `upstream/bun/src/bundler/p.zig` — implémentation Fast Refresh
- `tests/targets/gemini-cli/packages/cli/src/test-utils/render.tsx` — exemple Ink réel
