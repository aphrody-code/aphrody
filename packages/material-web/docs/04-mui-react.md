---
title: "MUI & React"
nav_order: 6
---

# MUI (Material UI, React) et son rapport à Material Design 3

> Portée : ce document décrit la librairie de composants React **MUI / Material UI** telle qu'elle existe dans le monorepo local `material-ui/` (version **9.0.1**, avril 2026) et fait le point — sources à l'appui — sur l'état réel du support de **Material Design 3 / Material You** en 2026. Conclusion d'entrée de jeu : **Material UI implémente Material Design 2, pas M3, et aucune migration M3 n'est planifiée.**

---

## 1. Nature : librairie de composants React, monorepo

Material UI est une librairie open-source (MIT) de composants React qui implémente **une réimplémentation indépendante** du système Material Design de Google. Le dépôt est un **monorepo pnpm + Lerna + Nx**.

- Racine : `material-ui/package.json` → `"name": "@mui/monorepo"`, `"version": "9.0.1"`, gestionnaire forcé `pnpm` (`preinstall: npx only-allow pnpm`).
- Build orchestré par Lerna (`lerna run build`) ; cache Nx (`nx.json`) ; workspaces (`pnpm-workspace.yaml`).
- Description officielle (`packages/mui-material/package.json`) : _« Material UI is an open-source React component library that implements Google's Material Design. »_
- Le README pointe explicitement vers **`m2.material.io`** (Material Design **2**) comme système de référence : `material-ui/README.md`.

### Packages clés

Inventaire réel dans `material-ui/packages/` :

| Package                                                    | Chemin                                                 | Version      | Rôle                                                                                   |
| ---------------------------------------------------------- | ------------------------------------------------------ | ------------ | -------------------------------------------------------------------------------------- |
| `@mui/material`                                            | `packages/mui-material`                                | 9.0.1        | Composants Material Design (le cœur grand public).                                     |
| `@mui/system`                                              | `packages/mui-system`                                  | 9.0.1        | Utilitaires CSS-in-JS de bas niveau : `sx`, `styled`, `Box`, `Stack`, accès au thème.  |
| `@mui/styled-engine`                                       | `packages/mui-styled-engine`                           | 9.0.0        | Wrapper `styled()` au-dessus de **Emotion** (moteur de style par défaut).              |
| `@mui/styled-engine-sc`                                    | `packages/mui-styled-engine-sc`                        | —            | Variante du moteur basée sur **styled-components**.                                    |
| `@mui/private-theming`                                     | `packages/mui-private-theming`                         | 9.0.1        | Contexte React du thème (interne).                                                     |
| `@mui/lab`                                                 | `packages/mui-lab`                                     | 9.0.0-beta.3 | Laboratoire de composants instables.                                                   |
| `@mui/icons-material`                                      | `packages/mui-icons-material`                          | 9.0.1        | Icônes Material Design en SVG React (plusieurs milliers).                              |
| `@mui/material-nextjs`                                     | `packages/mui-material-nextjs`                         | 9.0.1        | Intégration SSR Next.js (App Router / Pages).                                          |
| `@mui/material-pigment-css`                                | `packages/mui-material-pigment-css`                    | 9.0.1        | Wrapper exposant les API `styled`/theming de Material UI au-dessus de **Pigment CSS**. |
| `pigment-css-react` / `pigment-react`                      | `packages/pigment-css-react`, `packages/pigment-react` | alpha        | Moteur de style **zero-runtime** (voir §6, **en pause**).                              |
| `@mui/codemod`, `@mui/envinfo`, `@mui/types`, `@mui/utils` | `packages/mui-*`                                       | —            | Outillage : codemods de migration, diagnostics, types partagés, utilitaires.           |

> ⚠️ **Absents de ce monorepo en v9** : `@mui/base` (**Base UI**) et `@mui/joy` (**Joy UI**).
>
> - **Base UI** a quitté ce dépôt et vit désormais dans son propre projet (`@base-ui-components/react`), stabilisé **v1.1 début janvier 2026**. Material UI v9 commence à le consommer (voir §3).
> - **Joy UI** : _« Joy UI code and docs ont été retirés du dépôt »_ lors de la maintenance v9. Projet **en pause**, sans plan ni calendrier.
> - **MUI X** (DataGrid, Date Pickers, Charts, Tree View, Scheduler, Chat…) est un dépôt séparé (`mui/mui-x`), aligné sur le même numéro de version majeure que Material UI depuis v9.

---

## 2. Version actuelle et architecture de styling

### Version

- **`@mui/material` 9.0.1** (monorepo `@mui/monorepo@9.0.1`).
- **Material UI v9.0 publiée le 8 avril 2026**. Saut direct **v7 → v9** (pas de v8 public) pour **réaligner la majeure** avec MUI X, déjà en v9. Désormais Material UI et MUI X partagent la même majeure.
- Compatibilité React : `^17 || ^18 || ^19` (peerDeps de `packages/mui-material/package.json`).
- Apports v9 : nouveaux composants **NumberField** et **Menubar** (bâtis sur Base UI), thème étendu aux `color-mix()`, ~**3 % de bundle en moins** vs v7, **`sx` jusqu'à +30 %** de perf, nettoyage de props dépréciées (`component`, `componentsProps`, props système des composants de layout), améliorations a11y (Tabs, Menu, roving tabindex). **Aucune mention de Material Design 3.**

### Moteur de style

- Par défaut : **Emotion** (`@emotion/react` + `@emotion/styled`), via `@mui/styled-engine`.
- Les peerDeps Emotion / Pigment sont **optionnelles** (`peerDependenciesMeta` de `packages/mui-material/package.json`) : on peut swapper vers **styled-components** (`@mui/styled-engine-sc`) ou **Pigment CSS** (`@mui/material-pigment-css`).
- Trois piliers d'écriture de styles :
  1. **`sx`** — prop ad hoc résolue contre le thème, idéale pour les overrides ponctuels.
  2. **`styled()`** — composants stylés réutilisables.
  3. **`theme`** — source de vérité centralisée (palette, typo, spacing, breakpoints, `components` overrides).

```tsx
import { styled } from "@mui/material/styles";
import Button from "@mui/material/Button";

// sx : override ponctuel résolu contre le thème
<Button sx={{ bgcolor: "primary.main", borderRadius: 2, px: 3 }}>OK</Button>;

// styled() : composant réutilisable
const Pill = styled(Button)(({ theme }) => ({
  borderRadius: 999,
  paddingInline: theme.spacing(3),
  backgroundColor: theme.palette.primary.main,
}));
```

---

## 3. M2 vs M3 — le point central

### Ce que MUI implémente aujourd'hui

Material UI est, depuis l'origine, une implémentation de **Material Design 2**. Le README renvoie à `m2.material.io`, les composants (Button `contained`/`outlined`/`text`, FAB, AppBar, élévations `elevation`, ripple, etc.) suivent les specs M2, et il n'existe **aucun** package, thème ou variante M3 dans le monorepo (`material-ui/packages/` — pas de `m3`, pas de tokens M3, pas de dynamic color).

### État officiel du support Material Design 3 / Material You en 2026

**Factuel : il n'y a pas de support M3, ni de roadmap M3.**

- Le billet **« MUI Update: What we've been working on (and why) »** (`https://mui.com/blog/2026-and-beyond/`) détaille le statut des projets (Base UI = focus, Pigment CSS = en pause, Joy UI = en pause, Toolpad = non maintenu) **sans aucune mention de Material Design 3 / Material You**.
- Le billet d'annonce **v9** (`https://mui.com/blog/introducing-material-ui-v9/`) — pourtant la dernière majeure — **ne mentionne pas M3** : v9 est une release de _fondations_ (a11y, theming, perf), pas un redesign visuel.
- Historiquement, l'idée d'un support M3 avait été discutée côté MUI via le chantier **Material You / nouvelle architecture de styling** ; en pratique l'effort a été **réorienté vers Base UI** (primitives headless, sans opinion visuelle) et **Pigment CSS** (perf), puis ce dernier **mis en pause**. La stratégie de fond est désormais : _Base UI = socle headless accessible_, sur lequel des « surfaces » Material UI sont reconstruites (NumberField, Menubar en v9). Material Design 3 n'est pas un livrable annoncé sur ce chemin.

| Aspect                  | Material Design 2 (ce que fait MUI)      | Material Design 3 / Material You                                                              |
| ----------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------- |
| Statut dans MUI v9      | ✅ implémenté                            | ❌ non implémenté, non planifié                                                               |
| Couleur                 | Palette fixe (primary/secondary…)        | Color roles + **HCT** + **dynamic color** (palette dérivée d'une couleur source)              |
| Tokens                  | `theme.palette`, `theme.typography` (M2) | Design tokens M3 (`md.sys.color.*`, `md.sys.typescale.*`)                                     |
| Formes                  | `shape.borderRadius` global              | Échelle de formes M3 (`shape.corner.*`) par catégorie                                         |
| Composants signature M3 | —                                        | Segmented buttons, FAB étendus M3, Navigation rail/bar M3, Search bar… non fournis tels quels |

**Référence à citer pour un projet** : si la conformité M3 stricte est requise, MUI n'est **pas** un véhicule M3 prêt à l'emploi en 2026 ; il faut soit l'utiliser comme M2, soit construire une couche de tokens M3 par-dessus son thème (voir §4, avec limites), soit envisager `Material Web` (composants Web M3 officiels de Google) ou des libs tierces M3.

---

## 4. Theming : `createTheme`, palette, approximer des tokens M3

Le système de thème vit dans `material-ui/packages/mui-material/src/styles/` :

- `createTheme.ts`, `createThemeWithVars.js` (mode CSS variables), `createThemeNoVars.js`.
- `createPalette.js` (rôles de couleur M2), `createTypography.js` (échelle typo M2), `createColorScheme.ts` (light/dark).

### CSS variables et `color-mix()` (v9)

Material UI génère un thème basé sur **CSS custom properties** (`createThemeWithVars`) ; v9 ajoute la génération de valeurs **`color-mix()`** par-dessus les variables, pour dériver overlays et surfaces de façon plus précise. C'est la mécanique la plus proche d'un « token system » moderne disponible nativement.

```tsx
import { createTheme, ThemeProvider } from "@mui/material/styles";

// Approximation de color roles M3 dans un thème MUI (M2 sous le capot)
const theme = createTheme({
  cssVariables: true, // active les CSS vars + color-mix (v9)
  colorSchemes: {
    light: {
      palette: {
        primary: { main: "#6750A4", contrastText: "#FFFFFF" }, // ~ md.sys.color.primary
        secondary: { main: "#625B71" }, // ~ md.sys.color.secondary
        error: { main: "#B3261E" }, // ~ md.sys.color.error
        background: { default: "#FFFBFE", paper: "#FFFBFE" }, // ~ surface
      },
    },
    dark: { palette: { primary: { main: "#D0BCFF" } } },
  },
  shape: { borderRadius: 12 }, // approx d'une corner scale M3
  typography: { fontFamily: "Roboto, system-ui, sans-serif" },
});
```

### Limites pour reproduire M3

- **Pas de HCT ni de dynamic color natif** : la palette MUI est statique. Pour générer une palette M3 dérivée d'une couleur source, il faut un outil externe (`@material/material-color-utilities`) et **injecter manuellement** les hex dans la palette/les CSS vars.
- **Color roles incomplets** : M3 distingue `primary`, `on-primary`, `primary-container`, `on-primary-container`, `surface`, `surface-variant`, `outline`, etc. MUI n'a qu'un sous-ensemble (`main`/`light`/`dark`/`contrastText`). Les rôles « container » et `surface-variant` doivent être ajoutés en **tokens custom** dans le thème (et typés via module augmentation TS).
- **Type scale** : la typo MUI (`h1…caption`, `body1/2`) ne mappe pas 1:1 sur la `md.sys.typescale` M3 (`display/headline/title/body/label` × `large/medium/small`).
- **Élévation/state layers** : M2 (ombres `elevation`) vs M3 (tonal elevation + **state layers** d'opacité). `color-mix()` en v9 aide à émuler les state layers, mais sans automatisme.

---

## 5. Pigment CSS / zero-runtime — où ça en est

- **Pigment CSS** = librairie CSS-in-JS **zero-runtime** (compilation au build, compatible **React Server Components**), pensée pour dépasser les limites de perf d'Emotion (recalcul client à chaque re-render).
- Packages locaux : `packages/pigment-css-react`, `packages/pigment-react`, et le pont `packages/mui-material-pigment-css` (mêmes API `styled`/theming que Material UI).
- **Statut 2026 : en pause (alpha).** Citation officielle (`https://mui.com/blog/2026-and-beyond/`) : _« the underlying problems were not fully solved yet »_ — l'équipe a **dépriorisé** Pigment CSS pour concentrer ses efforts sur **Base UI**. _« Development is paused »_, sans calendrier.
- Conséquence : **Emotion reste le moteur par défaut et recommandé** ; Pigment CSS n'est pas un choix de production fiable aujourd'hui.

---

## 6. Inventaire de composants

Mesuré dans le monorepo (`material-ui/packages/mui-material/src/`) :

- **~154 répertoires de composants/sous-composants**, **~139 exports** depuis `src/index.js`.
- Catégories principales :

| Catégorie      | Exemples (`packages/mui-material/src/`)                                                        |
| -------------- | ---------------------------------------------------------------------------------------------- |
| Inputs         | Button, ButtonGroup, Checkbox, Radio, Switch, Slider, TextField, Select, Autocomplete, Rating  |
| Data display   | Avatar, Badge, Chip, Divider, List, Table, Tooltip, Typography, Icon                           |
| Feedback       | Alert, Backdrop, CircularProgress, LinearProgress, Skeleton, Snackbar, Dialog                  |
| Surfaces       | AppBar, Accordion, Card, Paper                                                                 |
| Navigation     | BottomNavigation, Breadcrumbs, Drawer, Link, Menu, Pagination, Stepper, Tabs, SpeedDial        |
| Layout / utils | Box, Container, Grid, Stack, ImageList, CssBaseline, ClickAwayListener, Modal, Popover, Popper |

- **`@mui/lab`** (`packages/mui-lab/src/`) ajoute des composants instables : Timeline, Masonry, LoadingButton, TabContext/TabList/TabPanel, anciens pickers, etc.
- **`@mui/icons-material`** : icônes Material en SVG (ordre de grandeur : plusieurs milliers).

---

## 7. Forces / limites pour un projet visant la conformité M3 stricte

### Forces

- Écosystème mûr (>10 ans), très large surface de composants (~140 exports), MUI X pour les composants avancés.
- Theming puissant : CSS variables, `color-mix()` (v9), modes light/dark, overrides par composant.
- Accessibilité en progrès (focus v9 : roving tabindex, Tabs/Menu), adoption progressive de **Base UI** (primitives headless accessibles).
- Flexibilité du moteur de style (Emotion / styled-components / Pigment).
- React 19, SSR Next.js (`@mui/material-nextjs`), TypeScript de premier ordre.

### Limites (conformité M3)

- **C'est du Material Design 2**, pas M3 : composants signature M3 absents, design language M2.
- **Pas de dynamic color / HCT** natif : palette statique, génération M3 = outillage externe + injection manuelle.
- **Color roles & type scale M3 incomplets** : nécessite des tokens custom + module augmentation pour combler les rôles `container`, `surface-variant`, `outline`, et le typescale M3.
- **Pas de roadmap M3** : ni v9 ni le billet stratégie 2026 ne l'évoquent ; l'effort va à Base UI (headless) et l'a11y.
- **Pigment CSS en pause** : pas de zero-runtime fiable à court terme.

**Recommandation** : pour une **conformité M3 stricte**, MUI n'est pas le bon véhicule en 2026. Options : (a) **Material Web** (composants Web M3 officiels de Google) ; (b) une lib React orientée M3 ; (c) accepter MUI en **M2** ; ou (d) bâtir une **couche de tokens M3 sur le thème MUI** (CSS vars + `material-color-utilities`), au prix d'un travail conséquent et d'écarts persistants (élévation tonale, state layers, formes par catégorie).

---

## Sources

- Dépôt local : `material-ui/package.json`, `material-ui/README.md`, `material-ui/packages/*/package.json`, `material-ui/packages/mui-material/src/styles/`, `material-ui/packages/mui-material/src/`.
- Annonce v9 : <https://mui.com/blog/introducing-material-ui-v9/>
- Stratégie / statut 2026 (Base UI, Pigment CSS, Joy UI, Toolpad) : <https://mui.com/blog/2026-and-beyond/>
- Pages MUI : <https://mui.com/material-ui/>, <https://mui.com/material-ui/experimental-api/pigment-css/>, <https://mui.com/versions/>
- Material Design 2 (référence MUI) : <https://m2.material.io/>
- Material Design 3 : <https://m3.material.io/>
