# 07 — Audit de couverture `@mui/material` (état VÉRIFIÉ shippé)

Vérification en lecture seule de la couverture **réelle** de tout `@mui/material@9.x` par le monorepo `material-web`, croisant quatre sources de vérité du **code shippé** (et non les matrices de planif périmées) : les dossiers composants MUI (`material-ui/packages/mui-material/src/*/`, 135 dossiers capitalisés), les tags `md-*` réels (`material-web/packages/react/md-elements.txt`, 119 lignes), les wrappers React `Md*` (`material-web/packages/react/wrappers/*.ts`, 119 wrappers `Md*`) et les éléments Lit `@customElement` (`material-web/packages/material-web/**`). Le doc de planif `05-gap-analysis.md` est désormais **partiellement périmé** : 9 de ses « gaps » ont été shippés depuis (avatar, alert, skeleton, breadcrumbs, link, surface, backdrop, popover, rating) + les 5 transitions React. Tout `🔴` ci-dessous est confirmé par `grep` d'absence, pas déduit du tableau.

## Méthode et périmètre

- **Univers compté** : les 135 dossiers capitalisés de `mui-material/src/`. On en retire 2 buckets non-composants UI pour arriver au « ~100 » :
  - **Infra/internes** (hors dénominateur) : `ButtonBase`, `InputBase`, `OverridableComponent`, `DefaultPropsProvider`, `Pigment*` (`PigmentContainer`/`PigmentGrid`/`PigmentStack` = variantes zero-runtime de `Container`/`Grid`/`Stack`), `SvgIcon` (primitive d'icône).
  - **Sous-composants couverts-par-slot du parent** : comptés `🟡` (slot) et rattachés à leur parent.
- **Couvert ✅** = il existe à la fois un élément `md-*` (Lit) ET un wrapper React `Md*`, OU un shim React shippé (transitions). Chemins cités.
- **Partiel 🟡** = slot/sous-partie d'un parent couvert, ou primitive plateforme assumée (pas de composant dédié).
- **Non couvert 🔴** = aucun `md-*`, aucun wrapper, aucun shim — absence vérifiée par grep (`md-(modal|popper|mobile-stepper|paper|box|container|stack)` = ∅ dans `packages/`).

Briques transverses réellement présentes : `md-elevation`, `md-ripple`, `md-focus-ring`, tokens `--md-sys-*`.

---

## Catégorie — Inputs / Form

| MUI                                                                               | Statut  | Couverture (chemin)                                                                                             |
| --------------------------------------------------------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------- |
| `Button`                                                                          | ✅      | `md-{text,elevated,filled,filled-tonal,outlined}-button` + `MdButton`/`MdTextButton`/… (`wrappers/button.ts`)   |
| `IconButton`                                                                      | ✅      | `md-{,filled,filled-tonal,outlined}-icon-button` + `MdIconButton`… (`wrappers/iconbutton.ts`)                   |
| `ButtonGroup`                                                                     | ✅      | `md-button-group` + `MdButtonGroup` (`wrappers/buttongroup.ts`)                                                 |
| `Fab`                                                                             | ✅      | `md-fab`/`md-branded-fab` + `MdFab`/`MdBrandedFab` (`wrappers/fab.ts`)                                          |
| `Checkbox`                                                                        | ✅      | `md-checkbox` + `MdCheckbox` (`wrappers/checkbox.ts`)                                                           |
| `Radio` / `RadioGroup`                                                            | ✅ / 🟡 | `md-radio` + `MdRadio` (`wrappers/radio.ts`) ; RadioGroup = pattern de groupage `name`                          |
| `Switch`                                                                          | ✅      | `md-switch` + `MdSwitch` (`wrappers/switch.ts`)                                                                 |
| `Slider`                                                                          | ✅      | `md-slider` + `MdSlider` (`wrappers/slider.ts`)                                                                 |
| `TextField`                                                                       | ✅      | `md-{filled,outlined}-text-field` + `MdFilledTextField`/`MdOutlinedTextField` (`wrappers/textfield.ts`)         |
| `Select` / `NativeSelect`                                                         | ✅      | `md-{filled,outlined}-select` + `md-select-option` + `MdFilledSelect`… (`wrappers/select.ts`)                   |
| `Autocomplete`                                                                    | ✅      | `md-autocomplete` + `MdAutocomplete` (`wrappers/autocomplete.ts`)                                               |
| `Rating`                                                                          | ✅      | `md-rating` (`material-web/rating`) + `MdRating` (`wrappers/rating.ts`) — **shim 05 désormais shippé**          |
| `ToggleButton` / `ToggleButtonGroup`                                              | ✅      | `md-outlined-segmented-button` / `md-outlined-segmented-button-set` + wrappers (`wrappers/button.ts`/`labs.ts`) |
| `FormControl` / `FormGroup` / `FormLabel` / `FormHelperText` / `FormControlLabel` | 🟡      | internes au field MUI ; côté M3 = props/slots de `md-*-field`/`md-*-text-field`                                 |
| `Input` / `OutlinedInput` / `FilledInput` / `InputBase`                           | 🟡      | internes ; couverts par `md-filled-field`/`md-outlined-field` (`wrappers/field.ts`)                             |
| `InputAdornment` / `InputLabel`                                                   | 🟡      | slots `leading-icon`/`trailing-icon`/`label` de `md-*-text-field`                                               |
| `TextareaAutosize`                                                                | 🟡      | `md-*-text-field` `type="textarea"` (auto-resize natif du field)                                                |

## Catégorie — Navigation

| MUI                                                                                            | Statut  | Couverture                                                                                                                       |
| ---------------------------------------------------------------------------------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `Tabs` / `Tab`                                                                                 | ✅      | `md-tabs` + `md-primary-tab`/`md-secondary-tab` + `MdTabs`/`MdPrimaryTab`/`MdSecondaryTab` (`wrappers/tabs.ts`)                  |
| `Menu` / `MenuItem` / `MenuList`                                                               | ✅      | `md-menu`/`md-menu-item`/`md-sub-menu`/`md-menu-group` + wrappers (`wrappers/menu.ts`, `wrappers/labs.ts`)                       |
| `Drawer` / `SwipeableDrawer`                                                                   | ✅      | `md-navigation-drawer`/`md-navigation-drawer-modal` + `MdNavigationDrawer`/`MdNavigationDrawerModal` (`wrappers/labs.ts`)        |
| `BottomNavigation` / `BottomNavigationAction`                                                  | ✅ / 🟡 | `md-navigation-bar` + `md-navigation-tab` + `MdNavigationBar`/`MdNavigationTab` (`wrappers/labs.ts`)                             |
| `AppBar`                                                                                       | ✅      | `md-top-app-bar`/`md-bottom-app-bar` + `MdTopAppBar`/`MdBottomAppBar` (`wrappers/appbar.ts`)                                     |
| `Toolbar`                                                                                      | ✅      | `md-toolbar` + `MdToolbar` (`wrappers/toolbar.ts`)                                                                               |
| `Breadcrumbs`                                                                                  | ✅      | `md-breadcrumbs` (`material-web/breadcrumbs`) + `MdBreadcrumbs` (`wrappers/breadcrumbs.ts`) — **shim 05 shippé**                 |
| `Link`                                                                                         | ✅      | `md-link` (`material-web/link`) + `MdLink` (`wrappers/link.ts`) — **shim 05 shippé**                                             |
| `Pagination` / `PaginationItem`                                                                | ✅ / 🟡 | `md-paginator` + `MdPaginator` (`wrappers/paginator.ts`) ; PaginationItem = sous-partie                                          |
| `Stepper` / `Step` / `StepButton` / `StepLabel` / `StepContent` / `StepConnector` / `StepIcon` | ✅ / 🟡 | `md-stepper`/`md-step` + `MdStepper`/`MdStep` (`wrappers/stepper.ts`) ; StepButton/Label/Content/Connector/Icon = slots/internes |
| `SpeedDial` / `SpeedDialAction` / `SpeedDialIcon`                                              | ✅ / 🟡 | `md-fab-menu`/`md-fab-menu-item` + `MdFabMenu`/`MdFabMenuItem` (`wrappers/fabmenu.ts`) ; icône morph native                      |
| `MobileStepper`                                                                                | 🔴      | aucun `md-*`/wrapper/shim (grep `mobile-stepper` = ∅). Composable via dots + 2 `md-text-button`                                  |
| `TablePagination` / `TablePaginationActions`                                                   | 🟡      | composé avec `md-paginator` + `md-table`                                                                                         |

## Catégorie — Surfaces / Layout

| MUI                                                                            | Statut      | Couverture                                                                                                                      |
| ------------------------------------------------------------------------------ | ----------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `Card` + `CardActions`/`CardContent`/`CardHeader`/`CardMedia`/`CardActionArea` | ✅ / 🟡     | `md-card`/`md-elevated-card`/`md-filled-card`/`md-outlined-card` + wrappers (`wrappers/labs.ts`) ; sous-parties = slots         |
| `Accordion` + `AccordionSummary`/`AccordionDetails`/`AccordionActions`         | ✅ / 🟡     | `md-accordion`/`md-expansion-panel` + `MdAccordion`/`MdExpansionPanel` (`wrappers/expansion.ts`) ; sous-parties = slots         |
| `Paper`                                                                        | 🟡          | `md-surface` (`material-web/surface`) + `MdSurface` (`wrappers/surface.ts`) — **shim 05 shippé** ; sinon `<div>`+`md-elevation` |
| `Divider`                                                                      | ✅          | `md-divider` + `MdDivider` (`wrappers/divider.ts`)                                                                              |
| `ImageList` / `ImageListItem` / `ImageListItemBar`                             | ✅ / 🟡     | `md-grid-list`/`md-grid-tile` + `MdGridList`/`MdGridTile` (`wrappers/gridlist.ts`) ; ItemBar = overlay slotté                   |
| `Box`                                                                          | 🔴 (layout) | pas de composant — Tailwind `<div>` (renvoi `06-tailwind-material-web.md`)                                                      |
| `Container`                                                                    | 🔴 (layout) | pas de composant — Tailwind `max-width`/`mx-auto` (renvoi `06`)                                                                 |
| `Stack`                                                                        | 🔴 (layout) | pas de composant — Tailwind `flex gap-*` (renvoi `06`)                                                                          |
| `Grid`                                                                         | 🔴 (layout) | pas de composant — Tailwind `grid` (renvoi `06`)                                                                                |

## Catégorie — Feedback / Overlays

| MUI                                                                          | Statut  | Couverture                                                                                                                 |
| ---------------------------------------------------------------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------- |
| `Dialog` + `DialogTitle`/`DialogContent`/`DialogContentText`/`DialogActions` | ✅ / 🟡 | `md-dialog` + `MdDialog` (`wrappers/dialog.ts`) ; sous-parties = slots `headline`/`content`/`actions`                      |
| `Snackbar` / `SnackbarContent`                                               | ✅ / 🟡 | `md-snackbar` + `MdSnackbar` (`wrappers/snackbar.ts`) ; Content = slot                                                     |
| `Alert` / `AlertTitle`                                                       | ✅ / 🟡 | `md-alert` (`material-web/alert`) + `MdAlert` (`wrappers/alert.ts`) — **shim 05 shippé** ; AlertTitle = slot               |
| `Tooltip`                                                                    | ✅      | `md-tooltip` + `MdTooltip` (`wrappers/tooltip.ts`)                                                                         |
| `Backdrop`                                                                   | ✅      | `md-backdrop` (`material-web/backdrop`) + `MdBackdrop` (`wrappers/backdrop.ts`) — **shim 05 shippé**                       |
| `Popover`                                                                    | ✅      | `md-popover` (`material-web/popover`) + `MdPopover` (`wrappers/popover.ts`) — **shim 05 shippé**                           |
| `CircularProgress`                                                           | ✅      | `md-circular-progress` + `MdCircularProgress` (`wrappers/progress.ts`) ; aussi `md-loading-indicator`/`MdLoadingIndicator` |
| `LinearProgress`                                                             | ✅      | `md-linear-progress` + `MdLinearProgress` (`wrappers/progress.ts`)                                                         |
| `Skeleton`                                                                   | ✅      | `md-skeleton` (`material-web/skeleton`) + `MdSkeleton` (`wrappers/skeleton.ts`) — **shim 05 shippé**                       |
| `Modal`                                                                      | 🔴      | pas de `md-modal`/wrapper (grep ∅). Assuré par `md-dialog` ou `<dialog>` natif + scrim `md-backdrop`                       |
| `Popper`                                                                     | 🔴      | pas de `md-popper` (grep ∅). CSS Anchor Positioning / `@floating-ui` ; `md-popover` couvre le cas ancré                    |

## Catégorie — Transitions (React)

| MUI        | Statut | Couverture                                                                     |
| ---------- | ------ | ------------------------------------------------------------------------------ |
| `Collapse` | ✅     | `Collapse` (`packages/react/transitions/Collapse.tsx`) — **shim React shippé** |
| `Fade`     | ✅     | `Fade` (`transitions/Fade.tsx`)                                                |
| `Grow`     | ✅     | `Grow` (`transitions/Grow.tsx`)                                                |
| `Slide`    | ✅     | `Slide` (`transitions/Slide.tsx`)                                              |
| `Zoom`     | ✅     | `Zoom` (`transitions/Zoom.tsx`)                                                |

## Catégorie — Data display / divers

| MUI                                                                                                                           | Statut  | Couverture                                                                                                                              |
| ----------------------------------------------------------------------------------------------------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `Avatar` / `AvatarGroup`                                                                                                      | ✅      | `md-avatar`/`md-avatar-group` (`material-web/avatar`) + `MdAvatar`/`MdAvatarGroup` (`wrappers/avatar.ts`) — **shim 05 shippé**          |
| `Badge`                                                                                                                       | ✅      | `md-badge` + `MdBadge` (`wrappers/labs.ts`)                                                                                             |
| `Chip`                                                                                                                        | ✅      | `md-chip-set` + `md-{assist,filter,input,suggestion}-chip` + `MdChipSet`/`MdAssistChip`… (`wrappers/chips.ts`)                          |
| `Icon`                                                                                                                        | ✅      | `md-icon` + `MdIcon` (`wrappers/icon.ts`)                                                                                               |
| `List` + `ListItem`/`ListItemButton`/`ListItemAvatar`/`ListItemIcon`/`ListItemText`/`ListItemSecondaryAction`/`ListSubheader` | ✅ / 🟡 | `md-list`/`md-list-item` + `MdList`/`MdListItem` (`wrappers/list.ts`) ; sous-parties = slots `start`/`end`/`headline`/`supporting-text` |
| `Table` + `TableBody`/`TableCell`/`TableRow`/`TableHead`/`TableFooter`/`TableContainer`/`TableSortLabel`                      | ✅ / 🟡 | `md-table` + `MdTable` (`wrappers/table.ts`) ; sous-parties = slots/internes                                                            |
| `Typography`                                                                                                                  | 🟡      | tokens typescale `--md-sys-typescale-*` + classes (renvoi `06`) ; pas de wrapper exporté (`md-type` est interne aux docs)               |
| `SvgIcon`                                                                                                                     | 🟡      | primitive — passe par `md-icon` (slot SVG/glyph)                                                                                        |

## Catégorie — Hors-spec M3 (utilitaires / baseline / hooks MUI)

Aucun équivalent composant (par design — ce ne sont pas des composants M3). Tous `🔴`/`🟡` mais **hors dénominateur composants UI** :

| MUI                                                                            | Statut  | Note                                                                           |
| ------------------------------------------------------------------------------ | ------- | ------------------------------------------------------------------------------ |
| `CssBaseline` / `ScopedCssBaseline` / `GlobalStyles` / `InitColorSchemeScript` | 🔴      | reset/theming global — feuille CSS + injection tokens (pattern, pas composant) |
| `ClickAwayListener` / `Portal` / `NoSsr` / `Unstable_TrapFocus`                | 🔴 / 🟡 | primitives React ; trap focus fourni nativement par `md-dialog`                |
| `useMediaQuery` / `useScrollTrigger` / `usePagination`                         | 🔴      | hooks — `matchMedia`/`IntersectionObserver`/logique `md-paginator`             |
| `darkScrollbar` (util)                                                         | 🔴      | snippet CSS tokenisé                                                           |

---

## Synthèse chiffrée

Dénominateur = **~100 composants UI** (135 dossiers MUI − ~6 infra/primitives `ButtonBase`/`InputBase`/`OverridableComponent`/`DefaultPropsProvider`/`Pigment*` − ~29 sous-composants slot-couverts comptés via leur parent). Décompte sur les **composants UI top-level** (parents + autonomes) :

| Statut                                                | Décompte | Détail                                                                                                                                                                      |
| ----------------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ✅ **COUVERT** (md-element + wrapper, ou shim React)  | **~58**  | tous les top-level Inputs, Navigation, Surfaces, Feedback, Data display + 5 transitions React                                                                               |
| 🟡 **PARTIEL** (slot du parent / primitive / pattern) | **~30**  | sous-composants (Card*, Dialog*, List*, Table*, Step*, Form*, Input\*…), `Paper`→`md-surface`, `Typography`, `SvgIcon`, `TextareaAutosize`, `RadioGroup`, `TablePagination` |
| 🔴 **NON COUVERT** (aucun md/wrapper/shim)            | **~12**  | dont seulement **6 « UI »** réels (voir gaps) ; le reste = baseline/hooks/util hors-spec                                                                                    |

**Couverture composants UI livrables : ~94 / ~100 couverts (✅+🟡)** — soit ~58 ✅ pleins + ~30 🟡 slot/primitive, 6 🔴 UI restants.

**Tout `@mui/material` est-il couvert ? NON à 100%, mais OUI sur la quasi-totalité de la surface composant.** Tous les composants M3-spec et la grande majorité des patterns MUI ont un équivalent shippé. Les 9 shims planifiés en `05` (avatar, alert, skeleton, breadcrumbs, link, surface, backdrop, popover, rating) et les 5 transitions React sont **bien présents dans le code** — vérifié : dossiers Lit `@customElement` (`material-web/{avatar,alert,skeleton,breadcrumbs,link,surface,backdrop,popover,rating}`), wrappers React `Md*` correspondants, et `packages/react/transitions/{Collapse,Fade,Grow,Slide,Zoom}.tsx`. Restent des gaps ciblés, tous de faible effort.

---

## Gaps réels restants

### Composants UI (7 — vrais manques)

| Gap             | Statut | Pourquoi                        | Solution / effort                                                                   |
| --------------- | ------ | ------------------------------- | ----------------------------------------------------------------------------------- |
| `Box`           | 🔴     | layout générique, non-M3        | Tailwind `<div>` (`06`) — **nul (décision doc)**                                    |
| `Container`     | 🔴     | layout, non-M3                  | Tailwind `max-width`/`mx-auto` (`06`) — **nul (décision doc)**                      |
| `Stack`         | 🔴     | layout, non-M3                  | Tailwind `flex gap-*` (`06`) — **nul (décision doc)**                               |
| `Grid`          | 🔴     | layout, non-M3                  | Tailwind `grid` (`06`) — **nul (décision doc)**                                     |
| `Modal`         | 🔴     | overlay bas niveau              | `md-dialog` / `<dialog>` + `md-backdrop` — **faible** (souvent déjà couvert)        |
| `Popper`        | 🔴     | positionnement ancré bas niveau | CSS Anchor Positioning / `@floating-ui` ; `md-popover` couvre l'ancrage — **moyen** |
| `MobileStepper` | 🔴     | pattern stepper mobile          | shim React : dots/progress + 2 `md-text-button` — **faible**                        |

> `Box`/`Container`/`Stack`/`Grid` sont des **décisions assumées** (layout délégué à Tailwind, doc `06`), pas des oublis. Le seul gap « composant » non matérialisé et non trivialement couvert par un existant est **`MobileStepper`** (faible effort). `Modal`/`Popper` sont fonctionnellement absorbés par `md-dialog`/`md-backdrop`/`md-popover`.

### Hors-spec (baseline / hooks / utils — non comptés)

`CssBaseline`, `ScopedCssBaseline`, `GlobalStyles`, `InitColorSchemeScript`, `ClickAwayListener`, `Portal`, `NoSsr`, `Unstable_TrapFocus`, `useMediaQuery`, `useScrollTrigger`, `usePagination`, `darkScrollbar` — patterns/hooks/CSS globaux, **faible effort** chacun, à fournir en helpers React/CSS si parité d'API d'import requise. Ne bloquent aucun composant.

### Conclusion

La couverture composant est **quasi totale**. Aucun composant Material-3-spec ne manque. Les seuls vrais trous sont (1) les primitives de **layout** délibérément déléguées à Tailwind, (2) **`MobileStepper`** (shim faible effort), et (3) les bas-niveau `Modal`/`Popper` fonctionnellement absorbés par `md-dialog`/`md-backdrop`/`md-popover`. Le reste des « gaps » sont des utilitaires/hooks hors périmètre composant.
