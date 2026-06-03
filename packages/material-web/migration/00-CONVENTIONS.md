<!-- CONTRAT PARTAGÉ — source de vérité unique pour toute la flotte de migration. -->

# Migration MUI → material-web — Conventions partagées (contrat)

Ce fichier est **la source de vérité unique** du kit de migration MUI (React) → `@material/web` (fork aphrody, web components Lit). Tous les livrables (mapping, codemods, wrappers, doc, exemples, intégration Tailwind) **doivent** respecter ces conventions à la lettre, sans en inventer d'autres ni les contredire. En cas de doute, vérifier dans les repos (`material-ui/`, `material-web/`) — **ne jamais inventer un élément `md-*` ou une prop qui n'existe pas**.

## 0. Faits terrain (vérifiés)

- **MUI** : `material-ui/packages/mui-material/` — `@mui/material@9.0.1`, ~100 composants réels (154 modules), styling Emotion, palette **Material 2**, theming `createTheme` + CSS vars (`createThemeWithVars.js`).
- **material-web (fork)** : `material-web/` — `@material/web@2.4.1`, **93 éléments `<md-*>` réels** (liste : `migration/scripts/md-elements.txt`). Entrées : `all.ts` re-exporte `aphrody-components.ts` + `aphrody-labs.ts` + composants upstream. Composants **self-contained** consommant les tokens `--md-sys-*` directement (avec fallbacks), bundle `tsc`+`lit` (pas de SASS).
- **Wrappers React** : RÉALISÉS dans le monorepo material-web → `material-web/packages/react` (package `@aphrody/m3-react`). Le package tokens vit dans `material-web/packages/tokens` (`@aphrody/m3-tokens`). material-web est un monorepo bun + turbo (`workspaces: ["packages/*", "catalog"]`, `turbo.json`).
- **`@lit/react` n'est pas installé** — le wrapper layer en dépend (`bun add @lit/react`, bun uniquement, jamais npm/pnpm).
- M3 Expressive (2025) non implémenté côté web ; `md-type` porte le type scale M3.

## 1. Layout du kit

```
migration/
├── 00-CONVENTIONS.md          ← CE FICHIER (ne pas modifier sauf l'orchestrateur)
├── 01-component-mapping.md    ← mapping exhaustif MUI→md-* (props, slots, events, gaps)
├── 02-theme-token-migration.md← thème MUI (M2) → tokens --md-sys-* + script de conversion
├── 03-react-integration.md    ← patterns @lit/react : events, forms, refs, controlled, SSR
├── 04-migration-playbook.md   ← stratégie strangler, coexistence, phases, tests, pièges
├── 05-gap-analysis.md         ← composants MUI sans équivalent + shims recommandés
├── 06-tailwind-material-web.md← intégration native Tailwind ⇄ material-web (shadow DOM)
├── codemods/                  ← transforms jscodeshift + règles ast-grep + README
│   (wrappers & tokens DÉPLACÉS dans le monorepo : material-web/packages/{react,tokens})
├── examples/                  ← exemple de migration de bout en bout (écran réel)
└── scripts/                   ← md-elements.txt + utilitaires
```

## 2. Convention de nommage des wrappers React

- Un wrapper React **par élément `md-*`**, nom = PascalCase du tag : `md-filled-button` → `MdFilledButton`, `md-outlined-text-field` → `MdOutlinedTextField`.
- Implémentation via `createComponent` de `@lit/react` :
  ```ts
  import * as React from "react";
  import { createComponent } from "@lit/react";
  import { MdFilledButton as MdFilledButtonElement } from "@material/web/button/filled-button.js";
  export const MdFilledButton = createComponent({
    react: React,
    tagName: "md-filled-button",
    elementClass: MdFilledButtonElement,
    events: { onInput: "input", onChange: "change" },
  });
  ```
- Les wrappers réexportent depuis `material-web/packages/react/index.ts`. Package = `@aphrody/m3-react` (réalise le plan du fork).
- L'import de l'élément se fait depuis son chemin `@material/web/...` (effet de bord d'enregistrement) ; pour les composants du fork, depuis `aphrody-components.ts` / `aphrody-labs.ts` / `all.ts`.

## 3. Mapping canonique MUI → material-web (cœur — à étendre, pas à contredire)

`Button` est **variant-dépendant** :

| MUI                                    | Condition (prop)               | Élément md                                                                               | Wrapper                     |
| -------------------------------------- | ------------------------------ | ---------------------------------------------------------------------------------------- | --------------------------- |
| `Button`                               | `variant="contained"` (défaut) | `md-filled-button`                                                                       | `MdFilledButton`            |
| `Button`                               | `variant="outlined"`           | `md-outlined-button`                                                                     | `MdOutlinedButton`          |
| `Button`                               | `variant="text"`               | `md-text-button`                                                                         | `MdTextButton`              |
| `Button`                               | (M3 elevated)                  | `md-elevated-button`                                                                     | `MdElevatedButton`          |
| `Button`                               | (M3 tonal)                     | `md-filled-tonal-button`                                                                 | `MdFilledTonalButton`       |
| `IconButton`                           | défaut / `color`               | `md-icon-button` (+ filled/outlined/tonal)                                               | `MdIconButton`…             |
| `Fab`                                  | défaut                         | `md-fab`                                                                                 | `MdFab`                     |
| `Fab`                                  | étendu/brandé                  | `md-branded-fab`                                                                         | `MdBrandedFab`              |
| `SpeedDial` (+Action)                  |                                | `md-fab-menu` (+ `md-fab-menu-item`)                                                     | `MdFabMenu`                 |
| `Checkbox`                             |                                | `md-checkbox`                                                                            | `MdCheckbox`                |
| `Radio` / `RadioGroup`                 |                                | `md-radio`                                                                               | `MdRadio`                   |
| `Switch`                               |                                | `md-switch`                                                                              | `MdSwitch`                  |
| `Slider`                               |                                | `md-slider`                                                                              | `MdSlider`                  |
| `TextField`                            | `variant="filled"` (défaut M3) | `md-filled-text-field`                                                                   | `MdFilledTextField`         |
| `TextField`                            | `variant="outlined"`           | `md-outlined-text-field`                                                                 | `MdOutlinedTextField`       |
| `Select`/`NativeSelect`                | filled / outlined              | `md-filled-select` / `md-outlined-select` (+ `md-select-option`)                         | `MdFilledSelect`…           |
| `Autocomplete`                         |                                | `md-autocomplete` (fork)                                                                 | `MdAutocomplete`            |
| `Chip`                                 | assist/filter/input/suggestion | `md-assist-chip`/`md-filter-chip`/`md-input-chip`/`md-suggestion-chip` (+ `md-chip-set`) | `MdAssistChip`…             |
| `Dialog` (+Title/Content/Actions)      |                                | `md-dialog` (slots `headline`/`content`/`actions`)                                       | `MdDialog`                  |
| `Menu`/`MenuList`/`MenuItem`           |                                | `md-menu` / `md-menu-item` (+ `md-sub-menu`, `md-menu-group`)                            | `MdMenu`…                   |
| `List`/`ListItem`/`ListItemText`…      |                                | `md-list` / `md-list-item` (+ `md-item`)                                                 | `MdList`…                   |
| `Divider`                              |                                | `md-divider`                                                                             | `MdDivider`                 |
| `Card` (+Content/Actions/Header/Media) |                                | `md-card` / `md-elevated-card` / `md-filled-card` / `md-outlined-card`                   | `MdCard`…                   |
| `LinearProgress`                       |                                | `md-linear-progress`                                                                     | `MdLinearProgress`          |
| `CircularProgress`                     |                                | `md-circular-progress` (+ `md-loading-indicator`)                                        | `MdCircularProgress`        |
| `Tabs`/`Tab`                           |                                | `md-tabs` + `md-primary-tab`/`md-secondary-tab`                                          | `MdTabs`…                   |
| `Tooltip`                              |                                | `md-tooltip` (fork)                                                                      | `MdTooltip`                 |
| `Snackbar`/`SnackbarContent`           |                                | `md-snackbar` (fork)                                                                     | `MdSnackbar`                |
| `Badge`                                |                                | `md-badge` (fork)                                                                        | `MdBadge`                   |
| `BottomNavigation`(+Action)            |                                | `md-navigation-bar` (fork)                                                               | `MdNavigationBar`           |
| `Drawer`/`SwipeableDrawer`             |                                | `md-navigation-drawer` / `md-navigation-drawer-modal` (fork)                             | `MdNavigationDrawer`        |
| `AppBar`/`Toolbar`                     |                                | `md-top-app-bar` / `md-bottom-app-bar` / `md-toolbar` (fork)                             | `MdTopAppBar`…              |
| `Stepper`/`Step`/`StepLabel`…          |                                | `md-stepper` / `md-step` (fork)                                                          | `MdStepper`                 |
| `Table`/`TableRow`/`TableCell`…        |                                | `md-table` (fork, tri colonnes)                                                          | `MdTable`                   |
| `Pagination`/`TablePagination`         |                                | `md-paginator` (fork)                                                                    | `MdPaginator`               |
| `Accordion`(+Summary/Details)          |                                | `md-accordion` / `md-expansion-panel` (fork)                                             | `MdAccordion`               |
| `ToggleButton`/`Group`                 |                                | `md-outlined-segmented-button` / `-set`                                                  | `MdOutlinedSegmentedButton` |
| `ImageList`/`ImageListItem`            |                                | `md-grid-list` / `md-grid-tile` (fork)                                                   | `MdGridList`                |
| `Icon`/`SvgIcon`                       |                                | `md-icon`                                                                                | `MdIcon`                    |
| `Typography`                           |                                | `md-type` (fork) **ou** classes typescale `--md-sys-typescale-*`                         | `MdType`                    |
| `ButtonBase`                           |                                | composé `md-ripple` + `md-focus-ring`                                                    | (helper)                    |

**Layout MUI → PAS d'élément md** (voir §6, intégration Tailwind) :
`Box`, `Container`, `Stack`, `Grid` → `<div>` + utilitaires Tailwind. `Paper` → `<div>` surface + `md-elevation`.

**Gaps connus (aucun équivalent md — voir `05-gap-analysis.md`)** :
`Avatar`/`AvatarGroup`, `Alert`/`AlertTitle`, `Breadcrumbs`, `Rating`, `Skeleton`, `Backdrop`, `Modal`/`Popover`/`Popper` (primitives — utiliser Popover API / `md-dialog`), `Link` (`<a>` tokenisé), `Collapse`/`Fade`/`Grow`/`Slide`/`Zoom`/`Grow` (transitions → motion tokens), `MobileStepper`, `CssBaseline`/`ScopedCssBaseline` (→ reset + tokens).

## 4. Règles de mapping des props & events

- **Props booléennes/valeurs** : MUI camelCase → attribut/propriété md (souvent identique : `disabled`, `value`, `checked`→`selected`/`checked` selon l'élément, `label`). **Vérifier le nom réel dans l'élément md** (reactive properties Lit) avant de mapper.
- **Controlled components** : MUI `value`+`onChange(e, val)` → md émet des events natifs (`input`, `change`). Les wrappers exposent `onInput`/`onChange` via `events` de `createComponent`. La signature React change : `e.target.value` (pas `(e, value)`). À documenter et à coder dans les codemods/wrappers.
- **`sx` / `className`** : `sx` n'a pas d'équivalent ; convertir vers `style`/classes Tailwind (les utilitaires Tailwind ne traversent PAS le shadow DOM — n'agissent que sur le host / layout). Styling interne = tokens `--md-sys-*`.
- **Slots** : MUI children/props → slots md (`slot="headline"`, `slot="content"`, `slot="action"`…). Les sous-composants MUI (`DialogTitle`, `CardHeader`…) deviennent du contenu slotté.
- **Icônes** : `startIcon`/`endIcon` → `<md-icon slot="icon">` / `slot="start"`/`slot="end"`.

## 5. Règles de mapping des tokens (thème)

Cible : variables CSS `--md-sys-*` (cf. `material-web/tokens/`, `docs/02-tokens-theming-web.md`). Mapping palette MUI (M2) → rôles M3 (best-effort, documenter les pertes) :

| MUI theme                      | → token M3                                             |
| ------------------------------ | ------------------------------------------------------ |
| `palette.primary.main`         | `--md-sys-color-primary`                               |
| `palette.primary.contrastText` | `--md-sys-color-on-primary`                            |
| `palette.secondary.main`       | `--md-sys-color-secondary`                             |
| `palette.error.main`           | `--md-sys-color-error`                                 |
| `palette.background.default`   | `--md-sys-color-background` / `--md-sys-color-surface` |
| `palette.text.primary`         | `--md-sys-color-on-surface`                            |
| `palette.divider`              | `--md-sys-color-outline-variant`                       |
| `shape.borderRadius`           | `--md-sys-shape-corner-*`                              |
| `typography.*`                 | `--md-sys-typescale-*`                                 |

Rôles M3 **sans source MUI** (`tertiary`, `*-container`, `surface-variant`, `outline`…) : générer via `material-color-utilities` à partir d'une couleur source (idéalement `palette.primary.main`). Voir `02-theme-token-migration.md`.

## 6. Intégration native Tailwind ⇄ material-web (baseline pour `06-…`)

Contraintes dures à respecter :

- Les **utilitaires Tailwind ne franchissent pas le Shadow DOM** des éléments `md-*` : ils stylent le _host_ (`display`, `margin`, `width`, `grid`/`flex` autour) et le **layout**, **pas** l'intérieur des composants.
- Le **theming interne** des composants md passe **exclusivement** par les tokens `--md-sys-*`, pas par des classes Tailwind.
- Stratégie d'intégration native à étudier dans `06-…` : (a) Tailwind v4 `@theme` qui **dérive ses couleurs des tokens `--md-sys-*`** (single source of truth partagée) ; (b) layout (ex-`Box/Stack/Grid`) en utilitaires Tailwind ; (c) `::part()` exposés par les éléments md pour un styling ciblé optionnel ; (d) `tailwindcss` v4.3.0 du repo local comme moteur.

## 7. Règles de robustesse (non négociables)

1. **bun uniquement** pour toute commande (install, build, run). Jamais npm/pnpm.
2. **Ne pas inventer** d'élément `md-*`, de prop ou de slot : vérifier dans `material-web/`. Tout ce qui n'existe pas → **gap explicite** dans `05-gap-analysis.md`.
3. Chaque livrable cite ses sources (`repo/chemin:ligne`).
4. **Ne pas commit** (l'admin décide). Laisser l'arbre modifié.
5. Restez dans votre périmètre de fichiers/dossier assigné — pas d'écriture hors zone (évite les conflits entre agents).
6. Français, markdown structuré, blocs de code réels et exécutables.
