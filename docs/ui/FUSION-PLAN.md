# Plan de fusion UI — Tailwind CSS + shadcn/ui + Material Design 3

Synthèse des trois audits ([audit-tailwindcss](audit-tailwindcss.md), [audit-shadcn](audit-shadcn.md), [audit-material-web](audit-material-web.md)). Objectif : fusionner les trois systèmes UI des forks `packages/{tailwindcss,ui,material-web}` en un système unifié aphrody.

## Constat : un dénominateur commun unique

Les trois audits convergent indépendamment sur le même pivot : **les CSS custom properties**. C'est la seule couche que les trois systèmes partagent nativement et qui traverse aussi bien le light DOM (Tailwind, shadcn/React) que le Shadow DOM (web components MD3, par héritage).

| Système | Modèle de tokens | Langage / runtime |
|---------|------------------|-------------------|
| Material Web (`packages/material-web`) | `--md-sys-color-*`, `--md-sys-typescale-*`, `--md-sys-shape-*`, `--md-sys-elevation-*` (3 niveaux : ref/sys/comp) | Lit + web components (`md-*`) |
| Tailwind v4 (`packages/tailwindcss`) | `@theme` → `--color-*`, `--spacing`, `--text-*` en `oklch()` (plus de `tailwind.config.js`) | CSS-native + moteur Rust Oxide |
| shadcn/ui (`packages/ui`) | variables sémantiques `--primary`/`--background`/… en OKLCH, `@theme inline`, `:root`/`.dark` | React + Radix/Base UI + Tailwind v4 |

## Source de vérité : `crates/m3-tokens` (Rust)

Point clé relevé par l'audit material-web : le crate Rust **`m3-tokens`** calcule déjà la palette tonale HCT (`dynamic::seed_to_palette`) et son `export_css` émet des `--md-sys-color-*` **au format exact attendu par Material Web**. C'est le générateur unique, et il est aligné avec la cible #1 Rust du projet.

```
m3-tokens (Rust, seed → HCT → tonal palettes)
        │  export_css
        ▼
  tokens.css : :root { --md-sys-color-*; --md-sys-typescale-*; --md-sys-shape-* }
        │
        ├──────────────► Material Web : hérite DIRECTEMENT (aucun mapping)
        │
        ├──► alias shadcn : --primary: var(--md-sys-color-primary);
        │                   --background: var(--md-sys-color-surface); … (registry:theme)
        │
        └──► Tailwind @theme inline : --color-primary: var(--md-sys-color-primary); …
                                      (pattern déjà utilisé par shadcn v4)
```

## Étapes concrètes

1. **Générateur** : étendre `m3-tokens::export_css` pour émettre, en plus des `--md-sys-*`, deux feuilles d'alias dérivées :
   - `aliases-shadcn.css` : `--primary`, `--secondary`, `--background`, `--foreground`, `--border`, `--ring`… → `var(--md-sys-color-*)` (mapping sémantique M3 → shadcn).
   - `aliases-tailwind.css` : bloc `@theme inline { --color-*: var(--md-sys-color-*); }`.
2. **Material Web** : consomme `tokens.css` tel quel (héritage Shadow DOM). Aucun changement de composant.
3. **shadcn/ui** : publier un item `registry:theme` `@aphrody` qui injecte `tokens.css` + `aliases-shadcn.css` à la place du bloc `:root` OKLCH hardcodé. Les composants React inchangés.
4. **Tailwind v4** : importer `aliases-tailwind.css` via `@theme inline` ; les utilitaires (`bg-primary`, `text-foreground`) pointent alors vers les tokens M3.
5. **Composants** : wrapper les web components `<md-*>` pour React (option A : `@lit/react` ; option B alignée §0, wasm-bindgen/Rust). Les composants shadcn restent la couche React riche ; MD3 fournit les primitives canoniques + le thème.
6. **Cascade** : isoler chaque système par `@layer` (reset / md3 / tailwind-utilities / shadcn-components) pour un ordre de spécificité déterministe.

## Conflits à gérer (relevés par les audits)

- **Shadow DOM** : les classes utilitaires Tailwind ne traversent pas le Shadow DOM des `md-*` ; seules les custom properties héritées passent → d'où le choix « tokens, pas classes » comme contrat d'intégration.
- **Vocabulaire** : M3 (`--md-sys-color-primary`) vs shadcn (`--primary`/`--primary-foreground`) → résolu par la feuille d'alias générée.
- **Couleur** : M3 calcule en HCT, shadcn/Tailwind expriment en OKLCH → `m3-tokens` émet directement en `oklch()` pour un espace commun.
- **Gestionnaires** : material-web (npm/wireit), ui + tailwindcss (pnpm/turbo) → orchestration via `turbo` global (déjà installé) ; `m3-tokens` (cargo) en amont du pipeline.

## Pipeline outillage (synergie avec le reste)

`m3-tokens` (cargo) → `tokens.css` → forks UI. Lint/format JS/TS des forks via **oxlint + oxfmt** (cf. `packages/gts` comme référence de config oxc Google-style), migration bun via **n2b**.
