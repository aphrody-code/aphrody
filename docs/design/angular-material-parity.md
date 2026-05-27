<!-- SPDX-License-Identifier: Apache-2.0 -->
# Parité Angular Material ↔ aphrody material-web

Vérification d'équivalence (2026-05-23) de notre fork `packages/material-web`
(base `@material/web` + extensions `aphrody-components.ts` + labs promus) face à
**Angular Material** (`angular/components`, `src/material/*` + `src/cdk/*`,
clone `C:\src\_refs\angular-components`).

## Composants `src/material/*` (35 hors core/schematics/testing/prebuilt)

| Angular Material | Notre équivalent `md-*` | Statut |
|---|---|---|
| autocomplete | `md-autocomplete` | porté (parité) |
| badge | `md-badge` (labs promu) | ✅ |
| bottom-sheet | `md-bottom-sheet` | ✅ |
| button | `md-{elevated,filled,filled-tonal,outlined,text}-button` | ✅ |
| button-toggle | `md-button-group` / `md-outlined-segmented-button(-set)` | ✅ équivalent |
| card | `md-{elevated,filled,outlined}-card` (labs promu) | ✅ |
| checkbox | `md-checkbox` | ✅ |
| chips | `md-{assist,filter,input,suggestion}-chip` + `md-chip-set` | ✅ |
| core (ripple/option/theming) | `md-ripple`, `md-elevation`, `md-focus-ring`, tokens m3 | ✅ |
| datepicker | `md-date-picker` | ✅ |
| dialog | `md-dialog` | ✅ |
| divider | `md-divider` | ✅ |
| expansion | `md-expansion-panel` + `md-accordion` | porté (parité) |
| form-field | `md-{filled,outlined}-field` | ✅ |
| grid-list | `md-grid-list` + `md-grid-tile` | porté (parité) |
| icon | `md-icon` | ✅ |
| input | `md-{filled,outlined}-text-field` | ✅ |
| list | `md-list`, `md-list-item`, `md-item` | ✅ |
| menu | `md-menu`, `md-menu-item`, `md-sub-menu` | ✅ |
| paginator | `md-paginator` | porté (parité) |
| progress-bar | `md-linear-progress` | ✅ |
| progress-spinner | `md-circular-progress` | ✅ |
| radio | `md-radio` | ✅ |
| select | `md-{filled,outlined}-select` + `md-select-option` | ✅ |
| sidenav | `md-navigation-drawer(-modal)` + `md-scaffold` | ✅ équivalent M3 |
| slide-toggle | `md-switch` | ✅ équivalent M3 |
| slider | `md-slider` | ✅ |
| snack-bar | `md-snackbar` (Popover top-layer) | ✅ |
| sort | tri intégré à `md-table` (`aria-sort`) | porté (parité) |
| stepper | `md-stepper` + `md-step` | porté (parité) |
| table | `md-table` (data-driven, tri, sélection) | porté (parité) |
| tabs | `md-tabs`, `md-primary-tab`, `md-secondary-tab` | ✅ |
| timepicker | `md-time-picker` | ✅ |
| toolbar | `md-toolbar` | ✅ |
| tooltip | `md-tooltip` (plain + rich, anchor positioning) | porté (parité) |
| tree | `md-tree` + `md-tree-item` | porté (parité) |

**Bilan composants : parité complète** (35/35) une fois les 9 portages validés
(tooltip, expansion, grid-list, table+sort, paginator, stepper, autocomplete,
tree). Plus, aphrody ajoute des composants/surfaces qu'Angular Material n'a
pas : `md-navigation-rail`, `md-top/bottom-app-bar`, `md-fab-menu`,
`md-loading-indicator`, `md-carousel`, `md-search-bar`, layout adaptatif
(`md-scaffold`/`md-pane`/`md-list-detail`/`md-supporting-pane`), typo
(`md-type`) et effets (`md-webgpu-canvas`).

## Primitives `src/cdk/*`

Le CDK est une couche de primitives Angular ; en monde web-components, la
plupart sont assurées par la **plateforme** ou nos composants :

| CDK | Couverture aphrody |
|---|---|
| overlay, portal | **Popover API** + top layer (cf. snackbar/menus) |
| a11y (focus trap, live announcer, focus monitor) | natif : dialog/popover focus-trap, `aria-live` (snackbar), `:focus-visible` |
| layout (BreakpointObserver) | `layout/internal/breakpoint-controller` + `ResizeObserver` |
| bidi | propriétés logiques CSS (`inset-inline`, RTL) |
| observers | `ResizeObserver` / `IntersectionObserver` natifs |
| platform | `CSS.supports` / feature detection |
| clipboard | `navigator.clipboard` |
| keycodes, coercion, collections, private | utilitaires internes — N/A en WC |
| menu, listbox | `md-menu` / `role=listbox` (`md-autocomplete`, `md-list`) |
| dialog | `md-dialog` |
| accordion, stepper, table, tree | portés en composants `md-*` ci-dessus |
| text-field (autosize, autofill) | text-field M3 ; **autosize textarea = candidat** |
| drag-drop | drag du `md-bottom-sheet` ; **drag-drop générique = candidat** |
| scrolling (**virtual scroll**) | ✅ `md-virtual-scroller` (fixed-size, data-driven `.items`/`.renderItem`) |

**Candidats primitives restants** (non bloquants) : drag-drop générique,
textarea autosize.

## Source de vérité
Clone de référence : `C:\src\_refs\angular-components` (sparse :
`src/material`, `src/cdk`, `src/material-experimental`). Re-cloner pour
rafraîchir. Notre bundle : [`packages/material-web/APHRODY-M3.md`](../../packages/material-web/APHRODY-M3.md).
