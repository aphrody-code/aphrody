---
title: "@material/web (Google)"
nav_order: 5
---

# `@material/web` — Material Web Components de Google

Portée : ce document décrit `@material/web` (MWC), l'implémentation web officielle de Material Design 3 par Google, sous forme de web components Lit, son statut de maintenance en 2026, l'inventaire des composants présents dans le repo local `material-web/` (un fork enrichi), le theming M3 par tokens CSS, la conformité M3, l'intégration frameworks, et les ajouts spécifiques du fork local « aphrody-code ».

---

## 1. Nature

`@material/web` est une bibliothèque de [web components](https://developer.mozilla.org/en-US/docs/Web/Web_Components) construite sur [Lit](https://lit.dev/) qui implémente [Material 3](https://m3.material.io/), le design system open-source de Google. Source : `material-web/README.md`, `material-web/package.json` (`"name": "@material/web"`, `"version": "2.4.1"`).

Caractéristiques :

- **Framework-agnostic.** Ce sont des Custom Elements standards (`<md-*>`) : ils fonctionnent en HTML pur et dans Lit, React, Vue, Svelte, Angular, Eleventy, WordPress, Rails, etc. (`material-web/docs/intro.md`).
- **Basé sur Lit + standards de plateforme.** Shadow DOM pour l'encapsulation, `ElementInternals` pour l'association aux formulaires, Popover API. MWC a été un terrain de pionnier pour ces standards.
- **Dépendances minimales.** `package.json` ne dépend que de `lit` (`^2.8.0 || ^3.0.0`), `@lit/context` et `tslib`.
- **Import par side-effect.** Importer un module enregistre l'élément via le décorateur `@customElement`. On peut importer composant par composant (recommandé en prod) ou tout d'un coup via `all.js`.

### Usage — exemple buildless (CDN, depuis le README)

```html
<head>
  <link
    href="https://fonts.googleapis.com/css2?family=Roboto:wght@400;500;700&display=swap"
    rel="stylesheet"
  />
  <script type="importmap">
    { "imports": { "@material/web/": "https://esm.run/@material/web/" } }
  </script>
  <script type="module">
    import "@material/web/all.js";
    import { styles as typescaleStyles } from "@material/web/typography/md-typescale-styles.js";
    document.adoptedStyleSheets.push(typescaleStyles.styleSheet);
  </script>
</head>
<body>
  <md-checkbox></md-checkbox>
  <md-outlined-text-field label="Favorite color" value="Purple"></md-outlined-text-field>
  <md-outlined-button type="reset">Reset</md-outlined-button>
</body>
```

### Usage — import ciblé en production

```js
// Importer seulement ce qu'on utilise (tree-shaking, bundle plus léger)
import "@material/web/button/filled-button.js";
import "@material/web/checkbox/checkbox.js";
```

```html
<md-filled-button>Click</md-filled-button>
```

---

## 2. Statut & roadmap 2026 — projet en maintenance

**MWC est en mode maintenance depuis juin 2024, sans changement en 2026.** Le README local le mentionne explicitement :

> « **MWC is in maintenance mode pending new maintainers** » — `material-web/README.md`, ligne 21, et `material-web/docs/roadmap.md`.

Annonce officielle : [Discussion #5642 — « MWC is in maintenance mode »](https://github.com/material-components/material-web/discussions/5642) (10 juin 2024).

### Ce que ça signifie (factuel, d'après l'annonce et le roadmap local)

- **Raison.** L'équipe Material Design a réaffecté les ingénieurs de MWC vers le framework interne de Google (« Wiz »). Le projet n'est plus activement staffé.
- **Pas déprécié, mais figé.** MWC « n'est ni déprécié ni supprimé », mais **aucun nouveau composant ni nouvelle feature n'est planifié** (`docs/roadmap.md` § Current/Future).
- **PRs.** Les pull requests ne sont plus acceptées par défaut ; petits correctifs au cas par cas, sur base bénévole.
- **Recherche de mainteneurs.** Google « investigue des moyens de poursuivre le développement, y compris identifier de nouveaux mainteneurs » — d'où le « pending new maintainers ».
- **Angular non affecté.** L'équipe redirige désormais explicitement les utilisateurs Angular vers [Angular Material](https://material.angular.io/), qui continue son propre développement (note dans le README local, § Quick start). MUI (React) est également indépendant.

### Complétude vs Android / Flutter / Angular Material

L'implémentation web canonique de Google est **moins complète** que les bibliothèques natives (Jetpack Compose Material 3, Flutter Material) : le roadmap **listait comme « non encore construits »** une famille entière de composants (`docs/roadmap.md` § Future › New components) qui n'ont jamais été livrés stables en amont :

> Autocomplete, Badge, Banner, Bottom app bar, Bottom sheet, Card, Data table, Date picker, Navigation bar, Navigation drawer, Navigation rail, Search, Segmented button, Snackbar, Time picker, Top app bar, Tooltip.

Ces composants sont **gelés au stade `labs/` ou totalement absents** dans l'upstream. C'est précisément ce gap que le fork local comble (voir § 7). Les fondations stables (couleur, typographie, ~21 familles de composants) ont été achevées « Material 1.0 » au Q3 2023, accessibilité complète au Q1 2024 (`docs/roadmap.md` § Past).

---

## 3. Inventaire des composants

Le repo local définit **94 custom elements `<md-*>`** au total (comptés via les décorateurs `@customElement`), répartis entre trois tiers : **stable upstream**, **`labs/` upstream** (preview), et **ajouts du fork aphrody**. La distinction stable/labs vient de l'upstream Google ; les ajouts viennent du fork (cf. `material-web/APHRODY-M3.md`, qui annonce « 94 `md-*` tags total »).

### 3.1 Stable upstream (Material 1.0, documentés dans `docs/components/`)

21 familles documentées dans `material-web/docs/components/*.md`. Tags `<md-*>` correspondants :

| Catégorie           | Tags                                                                                                                                                   | Chemin repo                                                  |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------ |
| Boutons             | `md-elevated-button`, `md-filled-button`, `md-filled-tonal-button`, `md-outlined-button`, `md-text-button`                                             | `material-web/button/`                                       |
| FAB                 | `md-fab`, `md-branded-fab`                                                                                                                             | `material-web/fab/`                                          |
| Icon buttons        | `md-icon-button`, `md-filled-icon-button`, `md-filled-tonal-icon-button`, `md-outlined-icon-button`                                                    | `material-web/iconbutton/`                                   |
| Sélection           | `md-checkbox`, `md-radio`, `md-switch`, `md-slider`                                                                                                    | `material-web/{checkbox,radio,switch,slider}/`               |
| Chips               | `md-chip-set`, `md-assist-chip`, `md-filter-chip`, `md-input-chip`, `md-suggestion-chip`                                                               | `material-web/chips/`                                        |
| Champs / saisie     | `md-filled-text-field`, `md-outlined-text-field`, `md-filled-field`, `md-outlined-field`, `md-filled-select`, `md-outlined-select`, `md-select-option` | `material-web/{textfield,field,select}/`                     |
| Listes & menus      | `md-list`, `md-list-item`, `md-menu`, `md-menu-item`, `md-sub-menu`, `md-menu-group`                                                                   | `material-web/{list,menu}/`                                  |
| Conteneurs / divers | `md-dialog`, `md-divider`, `md-elevation`, `md-focus-ring`, `md-ripple`, `md-icon`, `md-item`                                                          | `material-web/{dialog,divider,elevation,focus,ripple,icon}/` |
| Progress            | `md-circular-progress`, `md-linear-progress`                                                                                                           | `material-web/progress/`                                     |
| Onglets             | `md-tabs`, `md-primary-tab`, `md-secondary-tab`                                                                                                        | `material-web/tabs/`                                         |

Le bundle de commodité `material-web/all.ts` (→ `all.js`) ré-exporte l'ensemble du tier stable.

### 3.2 `labs/` upstream (preview, jamais stabilisé)

Composants en zone expérimentale upstream (`material-web/labs/`), promus pour usage par le fork via `aphrody-labs.ts` :

| Tags                                                                | Chemin repo                                          |
| ------------------------------------------------------------------- | ---------------------------------------------------- |
| `md-badge`                                                          | `material-web/labs/badge/`                           |
| `md-elevated-card`, `md-filled-card`, `md-outlined-card`, `md-card` | `material-web/labs/card/`                            |
| `md-navigation-bar`, `md-navigation-tab`                            | `material-web/labs/navigationbar/`, `navigationtab/` |
| `md-navigation-drawer`, `md-navigation-drawer-modal`                | `material-web/labs/navigationdrawer/`                |
| `md-outlined-segmented-button`, `md-outlined-segmented-button-set`  | `material-web/labs/segmentedbutton(set)/`            |
| `md-split-button`                                                   | `material-web/labs/splitbutton/`                     |

### 3.3 Ajouts du fork (aphrody) — voir § 7 pour le détail

Les ~50 tags restants (snackbar, app-bars, navigation rail, sheets, carousel, date/time picker, button group, FAB menu, scaffold/panes, tooltip, table, stepper, tree, autocomplete, paginator, expansion, grid-list, virtual-scroller, `md-type`, `md-webgpu-canvas`…) sont des **ajouts du fork local**, non présents dans l'upstream Google.

---

## 4. Theming M3

Le theming repose entièrement sur des **CSS custom properties** ; pas d'API JS de thème. Trois niveaux de tokens.

### 4.1 Couleur — `--md-sys-color-*`

`material-web/docs/theming/color.md`. Convention : `--md-sys-color-<role>` + son pendant `--md-sys-color-on-<role>` pour le contenu accessible. Posés sur `:root` :

```css
:root {
  --md-sys-color-primary: #006a6a;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-primary-container: #6ff7f6;
  --md-sys-color-on-primary-container: #002020;
  /* secondary, tertiary, error, surface, surface-container-*, outline… */
}
```

Génération du schéma : plugin Figma _Material Theme Builder_ ou la lib `@material/material-color-utilities` (runtime). Liste exhaustive : `material-web/tokens/_md-sys-color.scss`.

### 4.2 Dark mode

On bascule l'intégralité des tokens `--md-sys-color-*` vers leur version sombre, typiquement via `@media (prefers-color-scheme: dark)` ou une classe de thème :

```css
@media (prefers-color-scheme: dark) {
  :root {
    --md-sys-color-primary: #4cdada;
    --md-sys-color-surface: #0e1414;
    /* … palette dark complète */
  }
}
```

### 4.3 Shape — `--md-sys-shape-corner-*`

`material-web/docs/theming/shape.md`. Échelle de rayons : `--md-sys-shape-corner-none`, `-extra-small`, `-small`, `-medium`, `-large`, `-extra-large`, `-full`.

### 4.4 Typographie — `--md-sys-typescale-*` + classes utilitaires

`material-web/docs/theming/typography.md`. Les classes `md-typescale-*` (ex. `md-typescale-display-large`, `md-typescale-body-medium`) s'activent en adoptant la feuille :

```js
import { styles as typescaleStyles } from "@material/web/typography/md-typescale-styles.js";
document.adoptedStyleSheets.push(typescaleStyles.styleSheet);
```

```html
<h1 class="md-typescale-display-medium">Hello Material!</h1>
```

### 4.5 Tokens niveau composant — `--md-<comp>-*`

Pour surcharger un composant précis sans toucher le schéma global. Exemples (`material-web/docs/components/button.md`) :

```css
md-filled-button {
  --md-filled-button-container-color: #006a6a;
  --md-filled-button-container-shape: 8px;
  --md-filled-button-label-text-font: "Roboto";
}
```

Hiérarchie : `--md-ref-*` (référence) → `--md-sys-*` (système, le niveau usuel d'un thème) → `--md-comp/<comp>-*` (composant). Définir les `--md-sys-*` suffit dans la grande majorité des cas.

---

## 5. Conformité M3

`@material/web` est **l'implémentation web canonique et de référence de Material Design 3 par Google** : même éditeur (équipe Material Design), tokens alignés sur le système M3 officiel (`m3.material.io`), même nomenclature de tokens (`--md-sys-*`) que les specs et le Material Theme Builder. C'est le pendant web de Jetpack Compose Material 3 (Android) et du widget set Material de Flutter.

Limite de conformité importante : **la couverture de composants reste partielle** vis-à-vis de la spec M3 complète (cf. la liste « non construits » du § 2). En pratique, MWC stable couvre les fondations et les composants de base, mais pas toute la surface M3 — d'où l'existence de bibliothèques tierces (MDUI) et, ici, du fork aphrody pour compléter le catalogue.

---

## 6. Intégration frameworks

`material-web/docs/intro.md` confirme le support multi-framework (Lit, React, Vue, Svelte, Eleventy, WordPress, Rails).

### React

React (avant 19) ne gère pas nativement les Custom Elements (events custom, props complexes). Deux approches :

- **Wrappers via `@lit/react`** (`createComponent`) — la méthode recommandée par Lit pour générer un composant React typé autour de chaque élément :

```jsx
import React from "react";
import { createComponent } from "@lit/react";
import { MdFilledButton } from "@material/web/button/filled-button.js";

export const FilledButton = createComponent({
  tagName: "md-filled-button",
  elementClass: MdFilledButton,
  react: React,
  events: { onClick: "click" },
});
```

- **React 19+** : prise en charge native des Custom Elements (props et events) ; on peut utiliser `<md-*>` directement en JSX.

Le fork local fournit d'ailleurs des wrappers React prêts à l'emploi (cf. § 7, `apps/m3-react` / `@aphrody-code/m3-react`).

### Angular

Officiellement, **Google recommande Angular Material plutôt que MWC en Angular** (README local, § Quick start). Si l'on tient à MWC, Angular consomme les Custom Elements via `CUSTOM_ELEMENTS_SCHEMA` dans le module/composant.

### Vue / Svelte

Support natif des Custom Elements : `<md-*>` utilisables directement dans les templates. Vue nécessite de marquer les tags `md-*` comme custom elements dans la config du compilateur (`isCustomElement`).

---

## 7. Fork local (aphrody-code)

Le repo local **n'est pas l'upstream tel quel** : c'est un fork (remote `upstream` = `material-components/material-web`, branche locale `main` portant le commit fork). Commit de tête :

```
172bd1383 feat: complete M3 catalog + Angular Material parity + modern web platform
```

Le manifeste du fork est `material-web/APHRODY-M3.md`. Objectif déclaré : **compléter le catalogue M3 que l'upstream n'a jamais livré stable**, ajouter la **parité avec Angular Material (+ CDK)**, et moderniser via les features récentes de la plateforme web.

### 7.1 Architecture du fork

- **`aphrody-components.ts`** — tous les nouveaux composants self-contained (chacun consomme les tokens `--md-sys-*` directement, peint ses propres state/elevation layers ; **pas de pipeline SASS** requis, build `tsc` + `lit` seul).
- **`aphrody-labs.ts`** — promeut les composants `labs/` upstream jugés stables (badge, cards, navigation bar/drawer/tab, segmented button set). Ceux-ci réutilisent les styles SASS upstream.
- **`all.ts`** ré-exporte les deux.
- Build dédié : scripts npm `build:aphrody` (`bun run aphrody-build.ts`) et `typecheck:aphrody` (`tsc -p tsconfig.aphrody.json`), config `tsconfig.aphrody.json`. Bundle Bun natif, ~102 KB, lit gardé externe ; sortie git-ignorée dans `dist-aphrody/`. Minifier CSS-in-JS optionnel via `aphrody-css-minify.ts` (`--css-transpile`).
- Wrappers React : `apps/m3-react` (`@aphrody-code/m3-react`), un par élément.

### 7.2 Nouveaux composants ajoutés (24 custom elements self-contained)

D'après `APHRODY-M3.md` § New components — comble précisément la liste « non construits » de l'upstream :

| Catégorie          | Tags                                                                                                                       | Chemin repo                                                 |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| Communication      | `md-snackbar` (Popover API, top layer), `md-loading-indicator` (M3 Expressive)                                             | `material-web/snackbar/`, `loadingindicator/`               |
| Navigation         | `md-navigation-rail` (+ `-item`), `md-top-app-bar` (scroll-driven CSS), `md-bottom-app-bar`, `md-search-bar`, `md-toolbar` | `material-web/{navigationrail,appbar,search,toolbar}/`      |
| Conteneurs         | `md-bottom-sheet`, `md-side-sheet`, `md-carousel` (+ `-item`)                                                              | `material-web/{sheet,carousel}/`                            |
| Action / sélection | `md-button-group`, `md-fab-menu` (+ `-item`), `md-date-picker`, `md-time-picker`                                           | `material-web/{buttongroup,fabmenu,datepicker,timepicker}/` |
| Layout adaptatif   | `md-scaffold`, `md-pane`, `md-list-detail`, `md-supporting-pane` (ResizeObserver → `size-class`)                           | `material-web/layout/`                                      |
| Typographie        | `md-type` (M3 type scale + axes Google Sans Flex animables, mode `code` → Google Sans Code)                                | `material-web/typography/`                                  |
| Effets             | `md-webgpu-canvas` (WGSL spectrum-shift / sparkle / glimmer, fallback CSS)                                                 | `material-web/effects/`                                     |

### 7.3 Parité Angular Material (+ CDK)

Équivalents `<md-*>` ajoutés pour chaque composant `angular/components` `src/material/*` (`APHRODY-M3.md` § Angular Material parity) :

| Tags                                           | Chemin repo                   |
| ---------------------------------------------- | ----------------------------- |
| `md-tooltip`                                   | `material-web/tooltip/`       |
| `md-expansion-panel`, `md-accordion`           | `material-web/expansion/`     |
| `md-grid-list`, `md-grid-tile`                 | `material-web/gridlist/`      |
| `md-table` (tri de colonnes)                   | `material-web/table/`         |
| `md-paginator`                                 | `material-web/paginator/`     |
| `md-stepper`, `md-step`                        | `material-web/stepper/`       |
| `md-autocomplete`                              | `material-web/autocomplete/`  |
| `md-tree`, `md-tree-item`                      | `material-web/tree/`          |
| `md-virtual-scroller` (gap CDK virtual-scroll) | `material-web/virtualscroll/` |

Voir `material-web/docs/design/angular-material-parity.md`. Le manifeste annonce « **94 `md-*` tags total ; `tsc` + `bun build` green** » — vérifié : 94 décorateurs `@customElement` recensés dans le repo.

### 7.4 Plateforme web moderne & tokens

- **`md-snackbar` → Popover API** (`popover="manual"`, top layer, `@starting-style` + `transition-behavior: allow-discrete`), fallback inline.
- **`md-top-app-bar` → scroll-driven CSS** (`animation-timeline: scroll(...)`, compositor-driven, guardé par `@supports`), fallback JS pour Firefox.
- Politique navigateurs : moteurs modernes (WebGPU, Shadow DOM), feature detection + dégradation gracieuse, **sans polyfills**.
- Internals partagés : `internal/motion/easing-and-duration.ts` (7 easings + 16 durations M3), `typography/internal/google-sans-flex-axes.ts` (6 axes variables), `layout/internal/scaffold.ts` (window-size-class, breakpoints 600/840/1200/1600). Valeurs tracées à une source Rust `crates/m3-tokens`.

> Note : le fork porte un en-tête `Copyright 2026 Google LLC` dans ses fichiers (`aphrody-components.ts`) — convention reprise de l'upstream, mais ces fichiers sont des ajouts aphrody, pas du code Google officiel.

---

## 8. Forces / limites / quand l'utiliser

### Forces

- **Référence M3 canonique pour le web** : tokens et nomenclature officiels, fidélité au design system.
- **Framework-agnostic, standards-based** : Custom Elements + Shadow DOM, encapsulation forte, pas de lock-in framework.
- **Accessibilité de premier plan** (VoiceOver, TalkBack, JAWS, NVDA, ChromeVox — complété Q1 2024).
- **Theming par CSS variables** : simple, surchargeable à trois niveaux, dark mode trivial.
- **Léger** : dépend uniquement de Lit ; tree-shakable par import ciblé.

### Limites

- **Maintenance mode** : pas de nouvelles features, PRs upstream non acceptées par défaut, pérennité dépendante de futurs mainteneurs.
- **Catalogue upstream incomplet** : familles entières (date picker, snackbar, app bars, navigation, tooltip…) jamais livrées stables — coincées en `labs/` ou absentes.
- **React < 19 nécessite des wrappers** (`@lit/react`).
- **Angular : déconseillé par Google** au profit d'Angular Material.

### Quand l'utiliser

- **Oui** : app web qui veut un look M3 canonique, en HTML/Lit/Vue/Svelte, sur navigateurs modernes ; projet à l'aise avec un socle en maintenance mais stable et standard.
- **Plutôt non (upstream seul)** : besoin du catalogue M3 complet → utiliser un fork qui complète (comme le local aphrody), MDUI, ou les libs natives. En Angular → Angular Material. Si l'on a besoin d'un projet activement staffé par Google côté web, MWC n'en est plus un.
- **Le fork local (aphrody)** lève la limite « catalogue incomplet » : 94 tags, parité Angular Material, plateforme web moderne — au prix d'être un fork hors upstream (pas de releases npm Google sur ces ajouts).

---

## Sources

- Repo local : `material-web/README.md`, `material-web/package.json`, `material-web/docs/roadmap.md`, `material-web/docs/intro.md`, `material-web/docs/theming/{color,shape,typography}.md`, `material-web/docs/components/`, `material-web/APHRODY-M3.md`, `material-web/all.ts`, `material-web/aphrody-{components,labs}.ts`.
- [Discussion #5642 — MWC is in maintenance mode (GitHub)](https://github.com/material-components/material-web/discussions/5642)
- [material-components/material-web (GitHub)](https://github.com/material-components/material-web)
- [@material/web (npm)](https://www.npmjs.com/package/@material/web)
- [Material 3 (m3.material.io)](https://m3.material.io/)
- [Angular Material](https://material.angular.io/)
