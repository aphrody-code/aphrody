<!-- SPDX-License-Identifier: Apache-2.0 -->

# `@aphrody/m3-react`

Wrappers React **par élément `md-*`** du fork aphrody `@material/web` (v2.4.1, 119 éléments),
générés avec [`@lit/react`](https://www.npmjs.com/package/@lit/react) `createComponent`.

Ce package réalise le plan que le fork prévoit (`apps/m3-react`) mais ne livre pas sur disque
(cf. `migration/00-CONVENTIONS.md` §0). Il expose un composant React typé pour chacun des
**119 tags** listés dans `migration/scripts/md-elements.txt`.

- **Statut compilation : 119 / 119 wrappers générés, `tsc --noEmit` → 0 erreur.**
- 1 wrapper React par élément `md-*`, nom = PascalCase du tag (CONVENTIONS §2).
- Events natifs réels mappés par élément (vérifiés dans le code Lit, voir plus bas).

## Install

> **bun uniquement** (jamais npm/pnpm — CONVENTIONS §7.1).

Le package est déjà installé dans ce dossier (`migration/wrappers/`) :

```bash
cd migration/wrappers
bun install            # @lit/react + @material/web (file:../../material-web) + react/react-dom
bun run typecheck      # tsc --noEmit → 0 erreur
bun run generate       # régénère wrappers/ + index.ts depuis md-elements.txt
```

Dans un projet consommateur :

```jsonc
// package.json
{
  "dependencies": {
    "@aphrody/m3-react": "file:../migration/wrappers",
    "react": "^19",
    "react-dom": "^19",
  },
}
```

## Usage

Importer un wrapper enregistre l'élément custom correspondant **comme effet de bord**
(le wrapper importe le module `@material/web/...` qui appelle `customElements.define`).
Aucun import side-effect supplémentaire n'est nécessaire.

```tsx
import { MdFilledButton, MdOutlinedTextField, MdCheckbox } from "@aphrody/m3-react";

function LoginForm() {
  const [email, setEmail] = React.useState("");
  const [accept, setAccept] = React.useState(false);

  return (
    <form>
      {/* controlled : la signature React change — e.target.value, PAS (e, value) */}
      <MdOutlinedTextField
        label="Email"
        value={email}
        onInput={(e) => setEmail((e.target as HTMLInputElement).value)}
      />

      <label>
        <MdCheckbox
          checked={accept}
          onChange={(e) => setAccept((e.target as HTMLInputElement).checked)}
        />
        J'accepte
      </label>

      <MdFilledButton onClick={() => console.log({ email, accept })}>Valider</MdFilledButton>
    </form>
  );
}
```

### Slots & icônes (CONVENTIONS §4)

Les sous-composants MUI (`DialogTitle`, `CardHeader`, `startIcon`…) deviennent du contenu slotté :

```tsx
import { MdDialog, MdFilledButton, MdTextButton, MdIcon } from "@aphrody/m3-react";

<MdDialog open onClosed={(e) => console.log((e.target as any).returnValue)}>
  <div slot="headline">Supprimer ?</div>
  <form slot="content" method="dialog">
    Cette action est irréversible.
  </form>
  <div slot="actions">
    <MdTextButton value="cancel">Annuler</MdTextButton>
    <MdFilledButton value="delete">
      <MdIcon slot="icon">delete</MdIcon>
      Supprimer
    </MdFilledButton>
  </div>
</MdDialog>;
```

> **`MdIcon` exige la police Material Symbols.** La lib ne la charge pas. Une fois,
> au démarrage de l'app :
>
> ```ts
> import { ensureMaterialSymbols } from "@aphrody/material-web/icon/material-symbols.js";
> ensureMaterialSymbols(); // ou { iconNames: ["delete", "search", …] } pour subseter
> ```
>
> Le texte enfant est le **nom du glyphe** (snake_case Material Symbols). Les axes
> variables se pilotent par token : `--md-icon-fill` (0..1), `--md-icon-wght`
> (100..700), `--md-icon-grad`, `--md-icon-opsz`. Migration depuis
> `@mui/icons-material` : codemod `migration/codemods/transforms/icons.ts` (96 % auto).

## Architecture

- `index.ts` — réexporte les 119 wrappers (un `export *` par catégorie).
- `wrappers/<categorie>.ts` — 45 fichiers groupés par dossier source de `material-web/`
  (`button.ts`, `select.ts`, `labs.ts`, …). Chaque export :

  ```ts
  export const MdFilledButton = createComponent({
    react: React,
    tagName: "md-filled-button",
    elementClass: MdFilledButtonElement, // importé depuis @material/web/button/filled-button.js
    events: {}, // events natifs réels (voir tableau)
  });
  ```

- `generate.ts` — générateur **bun** : lit `md-elements.txt`, scanne `material-web/` pour
  résoudre chaque tag → `{fichier d'entrée, classe exportée}` via le motif
  `@customElement('md-x') export class Foo`, applique la table d'events vérifiés, et émet
  les fichiers. Préfère le chemin canonique (bundlé par `all.ts` / `aphrody-components.ts` /
  `aphrody-labs.ts`), puis non-`labs/gb`, sinon `labs/gb` (pour les tags `md-button`,
  `md-card`, `md-menu-group`, `md-split-button` qui n'existent que là).
- `material-web-shims.d.ts` — voir « Détail compilation ».

## Événements mappés (vérifiés dans le code Lit)

Noms d'events extraits par grep de `@fires` / `new (Custom)?Event('…')` dans
`material-web/<dir>/internal/*.ts`. Les composants **upstream** utilisent des noms simples
(`input`, `change`, `opening`…) ; les composants **fork** namespacent les leurs
(`paginator:page`, `table:sort`, `side-sheet:opening`, `tree:select`…).

| Élément                                                         | Events React → event natif                                                |
| --------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `MdCheckbox`, `MdRadio`, `MdSwitch`, `MdSlider`, `Md*TextField` | `onChange:change`, `onInput:input`                                        |
| `MdFilledSelect`, `MdOutlinedSelect`                            | `+onOpening/onOpened/onClosing/onClosed`                                  |
| `MdDialog`                                                      | `onOpen:open`, `onOpened`, `onClose:close`, `onClosed`, `onCancel:cancel` |
| `MdMenu`, `MdSubMenu`                                           | `onOpening/onOpened/onClosing/onClosed`                                   |
| `MdMenuItem`                                                    | `onCloseMenu:close-menu`                                                  |
| `MdSnackbar`                                                    | `onOpening/onOpened/onClosing/onClosed` (bare)                            |
| `MdBottomSheet` / `MdSideSheet`                                 | `onOpening:bottom-sheet:opening` … (fork, namespacés)                     |
| `MdSearchBar`                                                   | `onInput`, `onSearch:search`, `onSearchOpen/onSearchClose`                |
| `MdAutocomplete`                                                | `onInput`, `onSelect:autocomplete:select`                                 |
| `MdNavigationBar`                                               | `onActivated:navigation-bar-activated`                                    |
| `MdNavigationRail`                                              | `onChange:navigation-rail:change`, `onItemActivate`                       |
| `MdNavigationDrawer(Modal)`                                     | `onNavigationDrawerChanged:navigation-drawer-changed`                     |
| `MdInputChip`                                                   | `onRemove:remove`, `onUpdateFocus:update-focus`                           |
| `MdButtonGroup`                                                 | `onChange:button-group:change`                                            |
| `MdFabMenu` / `MdFabMenuItem`                                   | `onOpen/onClose:fab-menu:*`, `onClick:fab-menu-item:click`                |
| `MdExpansionPanel`                                              | `onToggle:expansion:toggle`                                               |
| `MdStepper`                                                     | `onChange:stepper:change`                                                 |
| `MdDatePicker` / `MdTimePicker`                                 | `onChange:date-picker:change` / `time-picker:change`                      |
| `MdTable`                                                       | `onSort:table:sort`, `onSelectionChange:table:selection-change`           |
| `MdPaginator`                                                   | `onPage:paginator:page`                                                   |
| `MdTree` / `MdTreeItem`                                         | `onSelect:tree:select` / `onToggle:tree-item:toggle`, `onActivate`        |
| `MdVirtualScroller`                                             | `onRange:virtual-scroll:range`                                            |
| `MdTooltip`                                                     | `onShow:tooltip-show`, `onHide:tooltip-hide`                              |
| `MdTabs`                                                        | `onChange:change`                                                         |

Les éléments purement présentationnels (`md-icon`, `md-divider`, `md-elevation`, `md-ripple`,
`md-focus-ring`, chips assist/filter/suggestion, cards, fab, boutons, `md-item`, `md-toolbar`,
`md-carousel`, `md-accordion`, `md-loading-indicator`, panes/scaffold…) n'émettent pas d'event
documenté : `events: {}`.

## Détail compilation

`bunx tsc --noEmit` compile **toute la chaîne** (les wrappers _et_ les sources `.ts` de
`material-web/` qu'ils importent). Trois ajustements ont été nécessaires côté `tsconfig.json` /
shims, **sans toucher** à `material-web/src` :

1. **`experimentalDecorators: true` + `useDefineForClassFields: false`** — les éléments Lit du
   fork utilisent les décorateurs _expérimentaux_ (`@property()`, `@customElement()`).
2. **`material-web-shims.d.ts`** — déclare `*.cssresult.js` et `*.css` comme modules ambiants.
   Ces fichiers de styles sont des **artefacts du build SASS** (`build:sass` → `build:css-to-ts`)
   **absents de ce checkout source** du fork. Le wrapper React ne touche jamais ces valeurs
   (les styles voyagent avec l'élément au runtime), d'où un typage `any` — qui évite aussi le
   conflit nominal entre le `lit@2.8.0` imbriqué de `material-web` et le `lit@3.3.3` du package.
3. **`moduleResolution: bundler`** — résout les imports bare `@material/web/x/y.js` vers les
   sources `.ts` (le fork n'a pas de `.js`/`.d.ts` compilés ni de champ `exports`).

> En contexte de **build réel** du fork (`material-web` buildé : `*.cssresult.js` et `*.css`
> présents), les shims deviennent inutiles ; ils ne servent qu'à faire compiler le wrapper
> contre les **sources** du fork tel que livré ici.

### Blocages réels

**Aucun.** Les 119 éléments de `md-elements.txt` résolvent vers un fichier d'entrée réel avec
classe exportée, et compilent. Les 4 tags `md-button`, `md-card`, `md-menu-group`,
`md-split-button` n'existent que sous `labs/gb/components/` (composants « Google Sans » du fork) :
leurs classes exportées ne suivent pas la convention `Md*` (`Button`, `Card`, `MenuGroup`,
`SplitButton`) — le wrapper les ré-alias correctement (`MdButton`, `MdCard`, …).

## Liste complète des wrappers (119)

> Régénérée par `generate.ts` (`.manifest.json`). 0 non résolu(s).

| Wrapper | Tag | Import élément | # events |
| --- | --- | --- | --- |
| `Md3dCanvas` | `md-3d-canvas` | `@aphrody/material-web/canvas3d/canvas-3d.js` | — |
| `Md3dCard` | `md-3d-card` | `@aphrody/material-web/card3d/card-3d.js` | — |
| `Md3dGlobe` | `md-3d-globe` | `@aphrody/material-web/globe3d/globe-3d.js` | — |
| `MdAccordion` | `md-accordion` | `@aphrody/material-web/expansion/accordion.js` | — |
| `MdAlert` | `md-alert` | `@aphrody/material-web/alert/alert.js` | 1 |
| `MdAreaChart` | `md-area-chart` | `@aphrody/material-web/charts/area-chart.js` | — |
| `MdAssistChip` | `md-assist-chip` | `@aphrody/material-web/chips/assist-chip.js` | — |
| `MdAutocomplete` | `md-autocomplete` | `@aphrody/material-web/autocomplete/autocomplete.js` | 2 |
| `MdAvatar` | `md-avatar` | `@aphrody/material-web/avatar/avatar.js` | 1 |
| `MdAvatarGroup` | `md-avatar-group` | `@aphrody/material-web/avatar/avatar-group.js` | — |
| `MdBackdrop` | `md-backdrop` | `@aphrody/material-web/backdrop/backdrop.js` | — |
| `MdBadge` | `md-badge` | `@aphrody/material-web/badge/badge.js` | — |
| `MdBarChart` | `md-bar-chart` | `@aphrody/material-web/charts/bar-chart.js` | — |
| `MdBottomAppBar` | `md-bottom-app-bar` | `@aphrody/material-web/appbar/bottom-app-bar.js` | — |
| `MdBottomSheet` | `md-bottom-sheet` | `@aphrody/material-web/sheet/bottom-sheet.js` | 4 |
| `MdBrandedFab` | `md-branded-fab` | `@aphrody/material-web/fab/branded-fab.js` | — |
| `MdBreadcrumbs` | `md-breadcrumbs` | `@aphrody/material-web/breadcrumbs/breadcrumbs.js` | — |
| `MdButton` | `md-button` | `@aphrody/material-web/labs/gb/components/button/md-button.js` | — |
| `MdButtonGroup` | `md-button-group` | `@aphrody/material-web/buttongroup/button-group.js` | 1 |
| `MdCard` | `md-card` | `@aphrody/material-web/card/card.js` | — |
| `MdCarousel` | `md-carousel` | `@aphrody/material-web/carousel/carousel.js` | 1 |
| `MdCarouselItem` | `md-carousel-item` | `@aphrody/material-web/carousel/carousel-item.js` | — |
| `MdCheckbox` | `md-checkbox` | `@aphrody/material-web/checkbox/checkbox.js` | 2 |
| `MdChipSet` | `md-chip-set` | `@aphrody/material-web/chips/chip-set.js` | — |
| `MdCircularProgress` | `md-circular-progress` | `@aphrody/material-web/progress/circular-progress.js` | — |
| `MdDatePicker` | `md-date-picker` | `@aphrody/material-web/datepicker/date-picker.js` | 1 |
| `MdDateRangePicker` | `md-date-range-picker` | `@aphrody/material-web/datepicker/date-range-picker.js` | 3 |
| `MdDateTimePicker` | `md-date-time-picker` | `@aphrody/material-web/datepicker/date-time-picker.js` | 3 |
| `MdDialog` | `md-dialog` | `@aphrody/material-web/dialog/dialog.js` | 5 |
| `MdDivider` | `md-divider` | `@aphrody/material-web/divider/divider.js` | — |
| `MdElevatedButton` | `md-elevated-button` | `@aphrody/material-web/button/elevated-button.js` | — |
| `MdElevatedCard` | `md-elevated-card` | `@aphrody/material-web/labs/card/elevated-card.js` | — |
| `MdElevation` | `md-elevation` | `@aphrody/material-web/elevation/elevation.js` | — |
| `MdExpansionPanel` | `md-expansion-panel` | `@aphrody/material-web/expansion/expansion-panel.js` | 1 |
| `MdFab` | `md-fab` | `@aphrody/material-web/fab/fab.js` | — |
| `MdFabMenu` | `md-fab-menu` | `@aphrody/material-web/fabmenu/fab-menu.js` | 2 |
| `MdFabMenuItem` | `md-fab-menu-item` | `@aphrody/material-web/fabmenu/fab-menu-item.js` | 1 |
| `MdFilledButton` | `md-filled-button` | `@aphrody/material-web/button/filled-button.js` | — |
| `MdFilledCard` | `md-filled-card` | `@aphrody/material-web/labs/card/filled-card.js` | — |
| `MdFilledField` | `md-filled-field` | `@aphrody/material-web/field/filled-field.js` | — |
| `MdFilledIconButton` | `md-filled-icon-button` | `@aphrody/material-web/iconbutton/filled-icon-button.js` | — |
| `MdFilledSelect` | `md-filled-select` | `@aphrody/material-web/select/filled-select.js` | 6 |
| `MdFilledTextField` | `md-filled-text-field` | `@aphrody/material-web/textfield/filled-text-field.js` | 2 |
| `MdFilledTonalButton` | `md-filled-tonal-button` | `@aphrody/material-web/button/filled-tonal-button.js` | — |
| `MdFilledTonalIconButton` | `md-filled-tonal-icon-button` | `@aphrody/material-web/iconbutton/filled-tonal-icon-button.js` | — |
| `MdFilterChip` | `md-filter-chip` | `@aphrody/material-web/chips/filter-chip.js` | — |
| `MdFocusRing` | `md-focus-ring` | `@aphrody/material-web/focus/md-focus-ring.js` | — |
| `MdGauge` | `md-gauge` | `@aphrody/material-web/charts/gauge.js` | — |
| `MdGridList` | `md-grid-list` | `@aphrody/material-web/gridlist/grid-list.js` | — |
| `MdGridTile` | `md-grid-tile` | `@aphrody/material-web/gridlist/grid-tile.js` | — |
| `MdIcon` | `md-icon` | `@aphrody/material-web/icon/icon.js` | — |
| `MdIconButton` | `md-icon-button` | `@aphrody/material-web/iconbutton/icon-button.js` | — |
| `MdInputChip` | `md-input-chip` | `@aphrody/material-web/chips/input-chip.js` | 2 |
| `MdItem` | `md-item` | `@aphrody/material-web/labs/item/item.js` | — |
| `MdLinearProgress` | `md-linear-progress` | `@aphrody/material-web/progress/linear-progress.js` | — |
| `MdLineChart` | `md-line-chart` | `@aphrody/material-web/charts/line-chart.js` | — |
| `MdLink` | `md-link` | `@aphrody/material-web/link/link.js` | — |
| `MdList` | `md-list` | `@aphrody/material-web/list/list.js` | — |
| `MdListDetail` | `md-list-detail` | `@aphrody/material-web/layout/md-list-detail.js` | — |
| `MdListItem` | `md-list-item` | `@aphrody/material-web/list/list-item.js` | — |
| `MdLoadingIndicator` | `md-loading-indicator` | `@aphrody/material-web/loadingindicator/loading-indicator.js` | — |
| `MdMenu` | `md-menu` | `@aphrody/material-web/menu/menu.js` | 4 |
| `MdMenuGroup` | `md-menu-group` | `@aphrody/material-web/labs/gb/components/menu/md-menu-group.js` | — |
| `MdMenuItem` | `md-menu-item` | `@aphrody/material-web/menu/menu-item.js` | 1 |
| `MdMobileStepper` | `md-mobile-stepper` | `@aphrody/material-web/mobilestepper/mobile-stepper.js` | — |
| `MdNavigationBar` | `md-navigation-bar` | `@aphrody/material-web/labs/navigationbar/navigation-bar.js` | 1 |
| `MdNavigationDrawer` | `md-navigation-drawer` | `@aphrody/material-web/labs/navigationdrawer/navigation-drawer.js` | 1 |
| `MdNavigationDrawerModal` | `md-navigation-drawer-modal` | `@aphrody/material-web/labs/navigationdrawer/navigation-drawer-modal.js` | 1 |
| `MdNavigationRail` | `md-navigation-rail` | `@aphrody/material-web/navigationrail/navigation-rail.js` | 2 |
| `MdNavigationRailItem` | `md-navigation-rail-item` | `@aphrody/material-web/navigationrail/navigation-rail-item.js` | — |
| `MdNavigationTab` | `md-navigation-tab` | `@aphrody/material-web/labs/navigationtab/navigation-tab.js` | — |
| `MdOutlinedButton` | `md-outlined-button` | `@aphrody/material-web/button/outlined-button.js` | — |
| `MdOutlinedCard` | `md-outlined-card` | `@aphrody/material-web/labs/card/outlined-card.js` | — |
| `MdOutlinedField` | `md-outlined-field` | `@aphrody/material-web/field/outlined-field.js` | — |
| `MdOutlinedIconButton` | `md-outlined-icon-button` | `@aphrody/material-web/iconbutton/outlined-icon-button.js` | — |
| `MdOutlinedSegmentedButton` | `md-outlined-segmented-button` | `@aphrody/material-web/labs/segmentedbutton/outlined-segmented-button.js` | — |
| `MdOutlinedSegmentedButtonSet` | `md-outlined-segmented-button-set` | `@aphrody/material-web/labs/segmentedbuttonset/outlined-segmented-button-set.js` | — |
| `MdOutlinedSelect` | `md-outlined-select` | `@aphrody/material-web/select/outlined-select.js` | 6 |
| `MdOutlinedTextField` | `md-outlined-text-field` | `@aphrody/material-web/textfield/outlined-text-field.js` | 2 |
| `MdPaginator` | `md-paginator` | `@aphrody/material-web/paginator/paginator.js` | 1 |
| `MdPane` | `md-pane` | `@aphrody/material-web/layout/md-pane.js` | — |
| `MdPieChart` | `md-pie-chart` | `@aphrody/material-web/charts/pie-chart.js` | — |
| `MdPopover` | `md-popover` | `@aphrody/material-web/popover/popover.js` | 2 |
| `MdPrimaryTab` | `md-primary-tab` | `@aphrody/material-web/tabs/primary-tab.js` | — |
| `MdRadarChart` | `md-radar-chart` | `@aphrody/material-web/charts/radar-chart.js` | — |
| `MdRadio` | `md-radio` | `@aphrody/material-web/radio/radio.js` | 2 |
| `MdRating` | `md-rating` | `@aphrody/material-web/rating/rating.js` | 2 |
| `MdRipple` | `md-ripple` | `@aphrody/material-web/ripple/ripple.js` | — |
| `MdScaffold` | `md-scaffold` | `@aphrody/material-web/layout/md-scaffold.js` | — |
| `MdScatterChart` | `md-scatter-chart` | `@aphrody/material-web/charts/scatter-chart.js` | — |
| `MdScheduler` | `md-scheduler` | `@aphrody/material-web/scheduler/scheduler.js` | 3 |
| `MdSearchBar` | `md-search-bar` | `@aphrody/material-web/search/search-bar.js` | 4 |
| `MdSecondaryTab` | `md-secondary-tab` | `@aphrody/material-web/tabs/secondary-tab.js` | — |
| `MdSelectOption` | `md-select-option` | `@aphrody/material-web/select/select-option.js` | — |
| `MdSideSheet` | `md-side-sheet` | `@aphrody/material-web/sheet/side-sheet.js` | 4 |
| `MdSkeleton` | `md-skeleton` | `@aphrody/material-web/skeleton/skeleton.js` | — |
| `MdSlider` | `md-slider` | `@aphrody/material-web/slider/slider.js` | 2 |
| `MdSnackbar` | `md-snackbar` | `@aphrody/material-web/snackbar/snackbar.js` | 4 |
| `MdSparkline` | `md-sparkline` | `@aphrody/material-web/charts/sparkline.js` | — |
| `MdSplitButton` | `md-split-button` | `@aphrody/material-web/labs/gb/components/splitbutton/md-split-button.js` | — |
| `MdStep` | `md-step` | `@aphrody/material-web/stepper/step.js` | — |
| `MdStepper` | `md-stepper` | `@aphrody/material-web/stepper/stepper.js` | 1 |
| `MdSubMenu` | `md-sub-menu` | `@aphrody/material-web/menu/sub-menu.js` | 4 |
| `MdSuggestionChip` | `md-suggestion-chip` | `@aphrody/material-web/chips/suggestion-chip.js` | — |
| `MdSupportingPane` | `md-supporting-pane` | `@aphrody/material-web/layout/md-supporting-pane.js` | — |
| `MdSurface` | `md-surface` | `@aphrody/material-web/surface/surface.js` | — |
| `MdSwitch` | `md-switch` | `@aphrody/material-web/switch/switch.js` | 2 |
| `MdTable` | `md-table` | `@aphrody/material-web/table/table.js` | 2 |
| `MdTabs` | `md-tabs` | `@aphrody/material-web/tabs/tabs.js` | 1 |
| `MdTextButton` | `md-text-button` | `@aphrody/material-web/button/text-button.js` | — |
| `MdTimePicker` | `md-time-picker` | `@aphrody/material-web/timepicker/time-picker.js` | 1 |
| `MdToolbar` | `md-toolbar` | `@aphrody/material-web/toolbar/md-toolbar.js` | — |
| `MdTooltip` | `md-tooltip` | `@aphrody/material-web/tooltip/tooltip.js` | 2 |
| `MdTopAppBar` | `md-top-app-bar` | `@aphrody/material-web/appbar/top-app-bar.js` | — |
| `MdTree` | `md-tree` | `@aphrody/material-web/tree/tree.js` | 1 |
| `MdTreeItem` | `md-tree-item` | `@aphrody/material-web/tree/tree-item.js` | 2 |
| `MdType` | `md-type` | `@aphrody/material-web/typography/md-type.js` | — |
| `MdVirtualScroller` | `md-virtual-scroller` | `@aphrody/material-web/virtualscroll/virtual-scroller.js` | 1 |
| `MdWebgpuCanvas` | `md-webgpu-canvas` | `@aphrody/material-web/effects/webgpu-canvas.js` | — |
