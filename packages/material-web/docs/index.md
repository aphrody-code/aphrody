---
title: Accueil
nav_order: 1
description: "Material Design 3 pour le web : web components Lit + wrappers React, cible de migration MUI / MUI X."
permalink: /
---

# Material 3 for the Web

{: .fs-9 }

Un système de composants **Material Design 3** complet pour le web moderne : des web components **Lit** (`<md-*>`) avec des **wrappers React** de première classe. Pensé comme la **cible de migration de MUI + MUI X (Community)**.
{: .fs-6 .fw-300 }

[Démarrer](#démarrage){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[Migration MUI → M3](https://github.com/aphrody-code/material-web/tree/feat/m3-monorepo/migration){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## Ce que c'est

- **Lib Lit self-contained** — chaque `<md-*>` consomme les rôles `--md-sys-color-*` (≈47, light + dark) émis au runtime ; le reste (typescale, shape, elevation, motion, state) est résolu au compile-time Sass.
- **Wrappers React** (`@aphrody/m3-react`) — un wrapper `@lit/react` par élément, events custom typés.
- **Material You runtime** (`@aphrody/m3-tokens`) — dérive les rôles de couleur depuis n'importe quelle couleur seed (7 variantes de scheme + niveaux de contraste WCAG).
- **Couverture MUI + MUI X Community** — Data Grid, charts, pickers, tree, scheduler, et la surface `@mui/material`.

## Démarrage

```bash
# Toolchain : bun uniquement
bun add @aphrody/m3-react @aphrody/material-web
```

```tsx
import { MdFilledButton, MdOutlinedTextField } from "@aphrody/m3-react";

export function Demo() {
  return (
    <form>
      <MdOutlinedTextField label="Email" type="email" />
      <MdFilledButton>Envoyer</MdFilledButton>
    </form>
  );
}
```

## Documentation

| Guide                                                                    | Sujet                                              |
| ------------------------------------------------------------------------ | -------------------------------------------------- |
| [État des lieux MD3]({% link 00-README.md %})                            | Panorama Material Design 3 sur le web              |
| [Fondations MD3]({% link 01-md3-spec-foundations.md %})                  | Référence des fondations de la spec                |
| [Tokens & Theming]({% link 02-tokens-theming-web.md %})                  | Tokens `--md-sys-*` et theming                     |
| [@material/web (Google)]({% link 03-material-web-google.md %})           | Les Material Web Components                        |
| [MUI & React]({% link 04-mui-react.md %})                                | Rapport à MUI / Material UI                        |
| [Tailwind & MD3]({% link 05-tailwind-ecosystem-md3.md %})                | L'écosystème Tailwind face à M3                    |
| [Panorama & Recommandations]({% link 06-landscape-recommendations.md %}) | Recommandations d'architecture                     |
| [Lit — Bonnes pratiques]({% link lit-best-practices.md %})               | Référentiel Lit                                    |
| [Stack 2026]({% link STACK.md %})                                        | Toolchain Bun-native, Rust tooling, cross-platform |
| [Dépendances]({% link DEPENDENCIES.md %})                                | Audit et roadmap d'upgrade                         |

## Corpus de référence Material Design 3

Les specs M3 distillées (foundations, tokens, styles, layout, motion, composants, parité Angular Material) servant de source de vérité design vivent dans [`docs/design/`](https://github.com/aphrody-code/material-web/tree/feat/m3-monorepo/docs/design) (index : [`docs/design/README.md`](https://github.com/aphrody-code/material-web/blob/feat/m3-monorepo/docs/design/README.md)).

## Migration MUI → Material 3

Le kit de migration complet vit dans le dossier [`migration/`](https://github.com/aphrody-code/material-web/tree/feat/m3-monorepo/migration) :

- **Conventions & mapping** : `00-CONVENTIONS.md`, `01-component-mapping.md`, theming (`02`), intégration React (`03`), playbook (`04`), gap-analysis (`05`), Tailwind (`06`).
- **Couverture** : `07-coverage-mui.md` (`@mui/material` v9), `08-coverage-mui-x.md` (MUI X Community), `09-coverage-tailwind.md`.
- **Cas réel mesuré** : [`10-case-study-rpbey.md`](https://github.com/aphrody-code/material-web/blob/feat/m3-monorepo/migration/10-case-study-rpbey.md) — un dashboard Next.js MUI v9 passé au codemod (chiffres réels ; le coût dominant est le styling `sx`, pas la couverture composant).
- **Icônes / Material Symbols** : [`11-material-symbols.md`](https://github.com/aphrody-code/material-web/blob/feat/m3-monorepo/migration/11-material-symbols.md) — axes variables de `md-icon`, chargement de police, optimisation, et le codemod `@mui/icons-material` → Material Symbols (**96 % automatique**).
- **Outillage** : [`mui-m3-map.json`](https://github.com/aphrody-code/material-web/blob/feat/m3-monorepo/migration/mui-m3-map.json) (mapping consolidé machine-readable) + codemods jscodeshift dans [`migration/codemods/`](https://github.com/aphrody-code/material-web/tree/feat/m3-monorepo/migration/codemods).
