---
title: "Dépendances"
nav_order: 11
---

# Audit dépendances

> Revue : 2026-05-29. Méthode : `bun pm view` + build/typecheck/test empiriques.
> Toolchain **bun-only** ; voir [`STACK.md`](./STACK.md) pour les choix d'outils.

Le `catalog` (single source of truth des versions partagées, `package.json` racine)
est **frais** sur l'essentiel : `react`/`react-dom` 19.2.6, `lit` 3.3.3, `@lit/react`
1.0.8, `@lit/context` 1.1.6, `motion` 12.40, `turbo` 2.9, `typescript` 6.0.3
(+ `@typescript/native-preview`/`tsgo` pour le typecheck), `tsup` 8.5,
`@material/material-color-utilities` 0.4.0 (dernier publié). `@react-three/fiber`
9.6.x + `drei` 9.121.x sont consommés **depuis l'upstream npm** (les forks
« React Canary » ont été retirés — React 19.2 stable les supporte).

## Bump appliqué

### `three` 0.170 → 0.184 (+ `@types/three`) ✅

**Fait.** R3F déclare `three >=0.156` (fiber 9.6.x) / `>=0.137` (drei 9.12x) → 0.184
est dans la plage. Validé : `tsc` material-web **0 erreur** (les composants 3D —
`canvas3d`/`globe3d`/`card3d`/`M3Theme3D` — compilent contre `@types/three` 0.184),
build showcase (R3F + three 0.184), gate bxc 17/17. Gain : `WebGPURenderer` r175+,
NodeMaterial redesigné.

## Bump appliqué (suite)

### `ai` (Vercel AI SDK) 4.1 → 6.0 + `@ai-sdk/google` 1 → 3 — `doc-ai` ✅

**Fait.** Usage minimal (`generateText({model: google(id), prompt, abortSignal})` → `{text}`) stable v4→v6. Validé : typecheck doc-ai 0 + 79 tests pass (client mocké derrière `LlmClient`). Le chemin live Gemini reste couvert seulement par le typecheck (pas de clé en CI) ; doc-ai est un outil interne (non shippé).

## À conserver tel quel

`@material/material-color-utilities` 0.4.0 — dernier publié ; le **patch ESM**
(`patches/@material%2Fmaterial-color-utilities@0.4.0.patch`) reste nécessaire
(bug specifier non corrigé upstream). `@webgpu/types` 0.1.70 est patché à vide
(`patches/`) car TypeScript 6 fournit nativement les types WebGPU dans `lib.dom`.

## Nettoyé cette passe

Retiré du `catalog` (zéro consommateur après restructuration) : `next`,
`@types/scheduler` (ex-exemple Next.js), `playwright`, `sass` (remplacé par
`sass-embedded`). Forks vendorisés supprimés : `lit`, `gts`, `wasm-physics`,
`@react-three/fiber`, `@react-three/drei`.
