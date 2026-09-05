---
title: "État des lieux MD3"
nav_order: 2
---

# Material Design 3 sur le web — état des lieux (mai 2026)

Documentation de référence sur l'état réel de **Material Design 3 (Material You)** pour le développement web en 2026 : la spécification, son implémentation en tokens/CSS, et les briques disponibles dans cet espace de travail (`/home/ubuntu/md3`).

Rédigée à partir d'une exploration en lecture seule des repos clonés (`material-web`, `material-ui`, `material-tailwind`, `shadcn-ui`, `tailwindcss`) et de recherches sur les sources officielles (m3.material.io, material-web.dev, dépôts GitHub Google, mui.com).

## Sommaire

| #   | Document                                                               | Contenu                                                                                                                                                                                                                                                            |
| --- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 01  | [`01-md3-spec-foundations.md`](./01-md3-spec-foundations.md)           | **Fondations de la spec MD3** : couleur HCT & dynamic color, design tokens (`ref`/`sys`/`comp`), type scale, élévation, shape, motion, state layers, window size classes, a11y. M3 Expressive (2025).                                                              |
| 02  | [`02-tokens-theming-web.md`](./02-tokens-theming-web.md)               | **Tokens & theming sur le web** : du token aux CSS custom properties `--md-sys-*`, `material-color-utilities` (HCT, `themeFromSourceColor`, `applyTheme`), Material Theme Builder, dynamic color web, dark mode & contrast, intégration CSS/Tailwind/material-web. |
| 03  | [`03-material-web-google.md`](./03-material-web-google.md)             | **`@material/web` (Google, Lit)** : web components `<md-*>`, statut _maintenance mode_, inventaire des composants, theming M3, intégration frameworks, et section dédiée au fork local `aphrody`.                                                                  |
| 04  | [`04-mui-react.md`](./04-mui-react.md)                                 | **MUI (React)** : v9.0.1, packages, Emotion/Pigment CSS, et le point central — MUI reste **Material 2**, aucune roadmap M3 ; pivot vers Base UI.                                                                                                                   |
| 05  | [`05-tailwind-ecosystem-md3.md`](./05-tailwind-ecosystem-md3.md)       | **Écosystème Tailwind face à MD3** : exploration de `material-tailwind` (= Material 2, gelé), Tailwind v4 comme moteur, shadcn/ui (non-Material mais bon modèle), mapping `--md-sys-*` → `@theme`.                                                                 |
| 06  | [`06-landscape-recommendations.md`](./06-landscape-recommendations.md) | **Panorama & recommandations** : tableau comparatif maître, le constat clé, 4 scénarios d'architecture pour un projet « md3 » et la reco finale par profil.                                                                                                        |

## TL;DR — les 5 constats

1. **Aucune implémentation web first-party complète et activement maintenue de MD3 n'existe en 2026.** Les chantiers M3 actifs de Google sont **Compose (Android)** et **Flutter** ; le web est en retrait.
2. **`@material/web` est l'unique porteur des tokens M3 canoniques** (`--md-sys-color/typescale/shape/elevation/motion`) — mais le projet est en **maintenance mode** depuis juin 2024 (ingénieurs réaffectés vers Wiz) et **M3 Expressive (2025) n'y est pas implémenté**.
3. **MUI ≠ M3.** MUI v9.0.1 (avril 2026) reste sur la palette **Material 2** (pas de rôles `tertiary`/`container`/`surface-variant`), sans roadmap M3.
4. **material-tailwind = Material 2** (preuve interne dans le repo), maintenance gelée, pas de support Tailwind v4. **shadcn/ui n'est pas Material du tout** — mais son modèle (registry copy-paste + tokens CSS + primitives Radix) est le meilleur véhicule pour bâtir un MD3 _custom_.
5. **Le pont universel = les tokens `--md-sys-*`** : `material-web/tokens`, Material Theme Builder et `material-color-utilities` convergent tous vers ce contrat de nommage, ce qui rend l'interop directe entre tous les écosystèmes.

## Briques disponibles dans cet espace de travail

| Repo                 | Quoi                                        | Version                     | Rapport à MD3                                          |
| -------------------- | ------------------------------------------- | --------------------------- | ------------------------------------------------------ |
| `material-web/`      | Web components Lit (fork `aphrody` enrichi) | `@material/web` 2.4.1       | **MD3 canonique** (tokens + composants), upstream gelé |
| `material-ui/`       | Composants React                            | `@mui/material` 9.0.1       | Material **2**, pas de M3                              |
| `material-tailwind/` | Composants React/HTML + Tailwind            | 2.1.10 / 2.3.2 (v3 en beta) | Material **2**, gelé                                   |
| `shadcn-ui/`         | Registry React copy-paste (Radix)           | CLI 4.7.0                   | **Non-Material** ; bon socle DIY                       |
| `tailwindcss/`       | Moteur CSS utility-first (Rust oxide)       | 4.3.0                       | Neutre ; point d'ancrage `@theme`                      |

> Pour le détail de chaque brique et les scénarios d'assemblage, voir [`06-landscape-recommendations.md`](./06-landscape-recommendations.md).
