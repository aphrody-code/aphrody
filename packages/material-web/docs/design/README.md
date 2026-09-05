<!-- SPDX-License-Identifier: Apache-2.0 -->

# Material Design 3 — corpus de référence

Specs M3 paraphrasées (valeurs numériques : dp, rayons, rôles de couleur,
niveaux d'élévation, durées/courbes de motion) servant de **source de vérité
design** pour `packages/material-web`, `packages/m3-tokens`, `packages/m3-motion`
et le kit de migration MUI (`migration/`). Aucune phrase n'est recopiée de la
documentation Google ; seules les valeurs (faits non protégeables) le sont.
Source : <https://m3.material.io>.

> Ce corpus est distillé depuis `design.google` / `m3.material.io`. Il ne fait
> pas partie du build : c'est une référence pour implémenter et vérifier les
> tokens et composants.

## Fondations

| Doc                                            | Contenu                                                     |
| ---------------------------------------------- | ----------------------------------------------------------- |
| [`m3-foundations.md`](./m3-foundations.md)     | Accessibilité, états d'interaction, state layers, principes |
| [`m3-glossary.md`](./m3-glossary.md)           | Glossaire A–Z des termes M3                                 |
| [`m3-design-tokens.md`](./m3-design-tokens.md) | Les 3 niveaux de tokens (ref / sys / comp)                  |
| [`M3-FRAMEWORK.md`](./M3-FRAMEWORK.md)         | Vue d'ensemble architecture M3                              |

## Styles & tokens

| Doc                                                      | Contenu                                                                     |
| -------------------------------------------------------- | --------------------------------------------------------------------------- |
| [`m3-styles.md`](./m3-styles.md)                         | Color (HCT), typescale, **shape (10 niveaux)**, elevation, motion, spacing  |
| [`m3-motion.md`](./m3-motion.md)                         | Courbes d'easing (7) + durées (16) — confronté à `packages/m3-motion`       |
| [`m3-layout.md`](./m3-layout.md)                         | Window size classes (600/840/1200/1600), panes, navigation adaptive, marges |
| [`aphrody-m3-tokens.md`](./aphrody-m3-tokens.md)         | Exemple de thème (rôles dark/light dérivés d'un seed HCT)                   |
| [`aphrody-m3-theme.json`](./aphrody-m3-theme.json)       | Le même thème en JSON (seed + palettes)                                     |
| [`fusion-tokens.sample.css`](./fusion-tokens.sample.css) | Bloc `:root { --md-sys-* }` baseline M3 complet                             |

## Composants & couverture

| Doc                                                          | Contenu                                             |
| ------------------------------------------------------------ | --------------------------------------------------- |
| [`m3-components-spec.md`](./m3-components-spec.md)           | Audit de couverture M3 × `md-*`                     |
| [`m3-components.md`](./m3-components.md)                     | Catalogue des composants M3 (heights, radii, rôles) |
| [`m3-web-update.md`](./m3-web-update.md)                     | Synthèse des 35 composants M3, plan d'update        |
| [`angular-material-parity.md`](./angular-material-parity.md) | Table de parité Angular Material ↔ `md-*`           |

## Références (contexte brand Gemini / Google Sans)

`gemini/` et `references/` contiennent des specs réutilisables (thème Gemini dark,
famille Google Sans, transparent screens) **incluant du contexte brand aphrody/Gemini**.
À extraire selon le scope.

| Doc                                                                                        | Contenu                                                   |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| [`gemini/`](./gemini/)                                                                     | Thème Gemini (tokens dark pixel-exact, mapping de rôles)  |
| [`references/google-sans-family.md`](./references/google-sans-family.md)                   | Google Sans / Sans Flex / Sans Code                       |
| [`references/gemini-visual-language.md`](./references/gemini-visual-language.md)           | Langage visuel Gemini (gradients, motion, sparkle)        |
| [`references/transparent-screens-glimmer.md`](./references/transparent-screens-glimmer.md) | Écrans transparents (Compose Glimmer), a11y, optical size |
| [`references/reinfurt-true-not-new.md`](./references/reinfurt-true-not-new.md)             | Philosophie design « vrai vs nouveau »                    |

## Cas d'usage

- **Définir/vérifier un composant** → `m3-components-spec.md` + `m3-styles.md`
- **Tokens couleur (seed → rôles)** → `packages/m3-tokens/dynamic-color` ; exemple : `aphrody-m3-theme.json`
- **Layout adaptive (breakpoints/panes/nav)** → `m3-layout.md` ↔ `packages/m3-tokens/src/breakpoints.ts`
- **Motion (easing/durées)** → `m3-motion.md` ↔ `packages/m3-motion`
- **Parité de couverture** → `angular-material-parity.md`, `m3-components-spec.md`

## État de conformité (vérifié 2026-05-29)

- **Couleur** : 47 rôles `--md-sys-color-*` (light+dark) dérivés runtime depuis un seed (`dynamic-color`). Conforme.
- **Shape** : échelle 10 niveaux exposée en runtime (`m3-tokens.css`), incl. les 3 ajouts M3 Expressive.
- **Layout** : breakpoints 600/840/1200/1600, marges 16/24, panes 1/1/2/2/3, nav bar→rail→rail-expanded. Conforme (`breakpoints.ts`).
- **Motion** : 7 courbes + 16 durées exactes (`m3-motion`). Conforme.
- **Composants** : 35/35 catégories M3 couvertes et wrappées React. Variantes M3 Expressive (tailles boutons/FAB, slider vertical, progress wavy) en attente de stabilisation upstream Google.
