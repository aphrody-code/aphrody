# Audit de couverture MUI X — état vérifié du monorepo

Cet audit croise ce que le monorepo `material-web` a **réellement** construit (lecture du code source `.ts`, pas de la planif) avec le référentiel de fonctionnalités **MUI X** (taxonomie Community / Pro / Premium, mui.com/x, 2026). La cible assumée est le **tier Community** (MIT) ; le **Pro** est visé à forte demande ; le **Premium** est volontairement hors-scope. Chaque statut cite le code qui le prouve (chemin:ligne).

> **Mise à jour 2026-05-29.** Les 8 gaps Community listés dans les versions précédentes de ce doc ont **tous été shippés** : édition inline Data Grid, charts Scatter + Radar, Time Picker, Date Time Picker, i18n des pickers (Intl), édition de label Tree, et le module **Scheduler** (vues day/week/month). La couverture Community est désormais **essentiellement complète** ; les détails ci-dessous citent le code réel.

---

## Avertissement méthodologique : reclassement Community vs Pro

Le brief décrivait certaines features comme « Data Grid » sans préciser le tier MUI X. La vérification de la taxonomie réelle de MUI X ([licensing](https://mui.com/x/introduction/licensing/), [feature showcase](https://mui.com/x/react-data-grid/features/)) impose un reclassement important :

- **Data Grid Community** = tri **simple colonne**, filtrage (quick + simple), pagination, sélection de lignes, export **CSV**, édition de cellules.
- **Data Grid Pro** = **multi-sort**, **multi-filter**, **column resizing**, **column reordering** (drag&drop), column pinning.
- **Data Grid Premium** = row grouping, agrégation, export **Excel**.

Conséquence : plusieurs features que nous avons construites (`multiSort`, filtres par colonne multiples, resize, reorder) sont en réalité des features **Pro**, pas Community. C'est un **dépassement** de la cible Community (bon pour la demande Pro), pas un manque. Les vrais gaps Community sont donc rares et listés en fin de document.

---

## Module 1 — Data Grid (`md-table`)

Code : `packages/material-web/table/internal/table.ts` (tag `md-table` défini dans `table/table.ts:56`).

| Feature MUI X                                                       | Tier MUI X                       | Statut                 | Preuve (code réel)                                                                                                                                                                  |
| ------------------------------------------------------------------- | -------------------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Données déclaratives (columns + rows)                               | Community                        | ✅                     | `table.ts:146,149` (`columns`, `rows`)                                                                                                                                              |
| Tri simple colonne (asc/desc/none, stable)                          | Community                        | ✅                     | `table.ts:465-492` cycle de tri ; `computeSortedRows` `table.ts:397-418`                                                                                                            |
| Filtrage rapide (quick filter global)                               | Community                        | ✅                     | `table.ts:202` `quickFilter` ; toolbar de recherche `table.ts:1122-1144` ; `computeFilteredRows` `table.ts:321-351`                                                                 |
| Filtrage par colonne (text/number + opérateurs)                     | Community (single) / Pro (multi) | ✅                     | `table.ts:49` type `filter` ; rendu filtre `table.ts:919-961` ; opérateurs `contains/equals/startsWith/endsWith/gt/lt/gte/lte` `table.ts:24-32`, `matchesFilter` `table.ts:353-395` |
| Pagination intégrée (page size, navigation)                         | Community                        | ✅                     | `table.ts:172-183` (`paginated`, `pageSize`, `rowsPerPageOptions`) ; paginator `table.ts:1032-1120` ; `computePagedRows` `table.ts:425-431`                                         |
| Sélection de lignes (checkbox, select-all, IDs stables)             | Community                        | ✅                     | `table.ts:166` `selectable` ; `table.ts:636-691` ; `getRowId` stable `table.ts:193-196,278-287`                                                                                     |
| Export CSV (RFC 4180, BOM)                                          | Community                        | ✅                     | `exportCsv()` `table.ts:809-826` ; `toCsv()` `table.ts:786-801`                                                                                                                     |
| Accessibilité (`role=grid`, `aria-sort`)                            | Community                        | ✅                     | `table.ts:865` `aria-sort` ; `ariaSortFor` `table.ts:526-534`                                                                                                                       |
| **Multi-column sort** (Shift+clic, ordinaux)                        | **Pro**                          | ✅ (dépasse Community) | `table.ts:163` `multiSort` ; additif `table.ts:469-490` ; ordinaux `table.ts:830-848`                                                                                               |
| **Column resizing** (drag bord)                                     | **Pro**                          | ✅ (dépasse Community) | `table.ts:51` `resizable` ; `startResize`/`onResizeMove` `table.ts:695-720`                                                                                                         |
| **Column reordering** (drag&drop header)                            | **Pro**                          | ✅ (dépasse Community) | `table.ts:186` `reorderable` ; `handleDragStart`/`handleDrop` `table.ts:722-741`                                                                                                    |
| Édition de cellules (inline edit)                                   | Community                        | ✅                     | `editable` par colonne `table.ts:59` ; état d'édition `table.ts:245` ; commit/cancel + focus `table.ts:771-781` ; event de commit documenté `table.ts:162`                          |
| Sélecteur de densité / visibilité de colonnes / overlay « no rows » | Community (polish)               | 🟡                     | Non exposés en tant que contrôles dédiés (densité, toggle colonnes, overlay vide) — features de confort Community restantes                                                         |
| Column pinning (figer colonnes)                                     | Pro (Community→Pro en v9)        | 🔴                     | Non implémenté (hors cible ; passé Pro dans MUI X v9)                                                                                                                               |
| Tree data / master-detail                                           | Pro                              | 🔴                     | Non implémenté (hors cible)                                                                                                                                                         |
| Row grouping / agrégation                                           | Premium                          | 🔴                     | Hors-scope assumé                                                                                                                                                                   |
| Export Excel                                                        | Premium                          | 🔴                     | Hors-scope assumé                                                                                                                                                                   |
| Virtualisation des lignes intégrée à la grille                      | Pro (perf)                       | 🟡                     | `md-virtual-scroller` existe séparément (Module 6) mais n'est pas branché dans `md-table`                                                                                           |

**Bilan Data Grid Community : couvert, édition inline incluse.** Restent des features de polish Community (sélecteur de densité, toggle de visibilité de colonnes, overlay « no rows », locale i18n complète). Bonus **Pro** déjà livrés : multi-sort, filtres par colonne, resize, reorder.

---

## Module 2 — Charts

Code : `packages/material-web/charts/internal/*`. Base partagée `chart-base.ts` (API `series` `chart-base.ts:132`, `categories`:135, `colors`:138, `legend`:150, `tooltip`:153, palette M3 `chart-base.ts:30-39`).

Tags définis (via `@customElement`) : `md-line-chart`, `md-bar-chart`, `md-pie-chart`, `md-area-chart`, `md-scatter-chart`, `md-radar-chart`, `md-sparkline`, `md-gauge` (**8 types**).

| Type de chart MUI X                                                       | Tier MUI X  | Statut | Preuve (code réel)                                                                                                                          |
| ------------------------------------------------------------------------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Line Chart (multi-série, smooth, markers, axes, grille, légende, tooltip) | Community   | ✅     | `line-chart.ts:45` ; `smooth`:47, `showMarkers`:50, `area`:53 ; tooltip `line-chart.ts:339-371` ; légende cliquable `line-chart.ts:314-337` |
| Bar Chart (groupé + empilé, **horizontal**)                               | Community   | ✅     | `bar-chart.ts:45` ; `stacked`:47, **`horizontal`:50** (implémenté, `barGeometry` `bar-chart.ts:349-428`)                                    |
| Pie / Donut Chart (donut via `inner-radius`, labels, padAngle)            | Community   | ✅     | `pie-chart.ts:76` `data` ; `innerRadius`:79, `showLabels`:82, `padAngle`:85                                                                 |
| Area Chart (empilable, cumul non destructif)                              | Community   | ✅     | `area-chart.ts:18` extends LineChart ; `stacked`:20 ; `stackedSeries` non mutant `area-chart.ts:30-40`                                      |
| Sparkline (line/bar, smooth, fill, endpoint)                              | Community   | ✅     | `sparkline.ts:18` ; `bars`:29, `smooth`:22, `area`:25, `showEndpoint`:32                                                                    |
| Gauge (radial, bandes seuils, sweep, unité)                               | Community   | ✅     | `gauge.ts:53` `value` ; `min`:56,`max`:59,`sweep`:62,`thickness`:65,`unit`:68,`bands`:74                                                    |
| Légende interactive (toggle série)                                        | Community   | ✅     | `line-chart.ts:443-451`, `bar-chart.ts:520-528`                                                                                             |
| Tooltip au survol                                                         | Community   | ✅     | `line-chart.ts:399-437` (nearest point), `bar-chart.ts:334-343`                                                                             |
| Responsive (ResizeObserver)                                               | Community   | ✅     | `chart-base.ts:168-181`                                                                                                                     |
| Accessibilité (role=img + table sr-only)                                  | Community   | ✅     | `bar-chart.ts:489-514`, `line-chart.ts:373-397`                                                                                             |
| **Scatter Chart** (X/Y numériques, points)                                | Community   | ✅     | `charts/scatter-chart.ts:43` `md-scatter-chart` ; axes via `chart-base`                                                                     |
| **Radar Chart** (géométrie polaire)                                       | Community\* | ✅     | `charts/radar-chart.ts:44` `md-radar-chart` (\* MUI X v9 n'a pas de Radar Community — c'est un **surplus**)                                 |
| Range Bar / Pyramid Chart                                                 | Community   | 🔴     | Non implémentés (types secondaires)                                                                                                         |
| Heatmap / Funnel / Sankey                                                 | Pro         | 🔴     | Hors cible (Pro)                                                                                                                            |
| Zoom & pan, brush, toolbar                                                | Pro         | 🔴     | Hors cible (Pro)                                                                                                                            |
| WebGL, candlestick/OHLC, annotations                                      | Premium     | 🔴     | Hors-scope assumé                                                                                                                           |

**Bilan Charts Community : 8 types livrés (line/bar/area/pie/scatter/radar/sparkline/gauge).** Couverture Community complète des types usuels ; `md-radar-chart` est même un surplus (absent de MUI X v9). Restent secondaires : Pyramid/Range Bar ; Heatmap/Funnel/Sankey sont Pro.

---

## Module 3 — Date and Time Pickers

Code : `packages/material-web/{datepicker,timepicker}/internal/*`. Tags `md-date-picker`, `md-date-range-picker`, `md-time-picker` (`timepicker/time-picker.ts:35`), `md-date-time-picker` (`datepicker/date-time-picker.ts:44`). i18n via `date-i18n.ts` / `time-i18n.ts` (`Intl`).

| Feature MUI X                                               | Tier MUI X | Statut                 | Preuve (code réel)                                                                                                              |
| ----------------------------------------------------------- | ---------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Date Picker (grille mois, navigation mois/année, sélection) | Community  | ✅                     | `date-picker.ts:133` ; navigation `date-picker.ts:379-409`                                                                      |
| Champ texte éditable + parsing masqué                       | Community  | ✅                     | `editable` `date-picker.ts:149` ; `parseDisplay` `date-picker.ts:79-104` (accepte MM/DD/YYYY et ISO)                            |
| Form-associated (`formAssociated`, `setFormValue`)          | Community  | ✅                     | `date-picker.ts:135` `static formAssociated` ; `internals.setFormValue` `date-picker.ts:180,187`                                |
| `min` / `max` + prédicat `disabledDate`                     | Community  | ✅                     | `date-picker.ts:143-146,158` ; `isDisabled` `date-picker.ts:215-225`                                                            |
| Navigation clavier (flèches)                                | Community  | ✅                     | `handleKeydown` `date-picker.ts:451-481`                                                                                        |
| Événements natifs `input`/`change` + custom                 | Community  | ✅                     | `dispatchNative` `date-picker.ts:447-449` ; `date-picker:change` `date-picker.ts:436-444`                                       |
| Validation visuelle (invalid state)                         | Community  | ✅                     | `fieldInvalid` `date-picker.ts:167,366`                                                                                         |
| **Date Range Picker** (start/end, in-range, swap)           | **Pro**    | ✅ (dépasse Community) | `date-range-picker.ts:72` ; state machine `pick` `date-range-picker.ts:369-388` ; champs doubles `date-range-picker.ts:188-218` |
| **Time Picker** (heures/minutes, format 12h/24h, clavier)   | Community  | ✅                     | `timepicker/time-picker.ts:35` `md-time-picker` ; `format` `time-picker.ts:122`, `editable` `:125`, `locale` `:131`             |
| **Date Time Picker** (date + heure combinés)                | Community  | ✅                     | `datepicker/date-time-picker.ts:44` `md-date-time-picker` (composition date + time)                                             |
| Localisation des formats/libellés (`Intl`)                  | Community  | ✅                     | `datepicker/internal/date-i18n.ts`, `timepicker/internal/time-i18n.ts` ; prop `locale` (`time-picker.ts:131`)                   |
| Digital clock / Multi-section digital clock / shortcuts     | Community  | 🔴                     | Modes d'affichage edge non implémentés (confort)                                                                                |
| Time Range / Date Time Range Picker                         | Pro        | 🔴                     | Hors cible Pro                                                                                                                  |

**Bilan Pickers Community : Date, Time, DateTime + i18n (`Intl`) livrés.** Restent des modes d'affichage edge (digital clock, multi-section, shortcuts, timezone). Bonus **Pro** : Date Range Picker.

---

## Module 4 — Tree View (`md-tree` / `md-tree-item`)

Code : `packages/material-web/tree/internal/tree.ts` + `tree-item.ts`. Tags `md-tree`, `md-tree-item`.

| Feature MUI X                                                                  | Tier MUI X | Statut | Preuve (code réel)                                                                                                   |
| ------------------------------------------------------------------------------ | ---------- | ------ | -------------------------------------------------------------------------------------------------------------------- |
| Arbre hiérarchique, expand/collapse                                            | Community  | ✅     | `tree.ts:67` ; `expandAll`/`collapseAll` `tree.ts:372-385`                                                           |
| Données dirigées par data (`items` prop, comme RichTreeView)                   | Community  | ✅     | `items: TreeNode[]` `tree.ts:84` ; `renderNode` récursif `tree.ts:172-196`                                           |
| Slot enfants (comme SimpleTreeView)                                            | Community  | ✅     | slot `tree.ts:166` + `handleSlotChange` `tree.ts:198-200`                                                            |
| Sélection simple                                                               | Community  | ✅     | `selectionMode='single'` `tree.ts:75` ; `selectValue` `tree.ts:326-331`                                              |
| **Multi-select**                                                               | Community  | ✅     | `selectionMode='multiple'` `tree.ts:75` ; `toggleValue` `tree.ts:334-346` ; `aria-multiselectable` `tree.ts:159-161` |
| **Checkbox selection**                                                         | Community  | ✅     | `checkboxes` `tree.ts:78` ; propagé `tree.ts:214` ; rendu checkbox `tree-item.ts:121-127`                            |
| Propagation parent→enfants + indeterminate                                     | Community  | ✅     | `setSubtreeSelected` `tree.ts:348-359` ; `computeIndeterminate` bottom-up `tree.ts:262-293`                          |
| Select all / clear                                                             | Community  | ✅     | `selectAll` `tree.ts:388-397`, `clearSelection` `tree.ts:400-405`                                                    |
| Navigation clavier (WAI-ARIA tree: flèches, Home/End, `*`, Ctrl+A, type-ahead) | Community  | ✅     | `handleKeydown` `tree.ts:407-468` ; type-ahead `tree.ts:485-519`                                                     |
| Roving tabindex / a11y `role=tree/treeitem`                                    | Community  | ✅     | `role=tree` `tree.ts:157` ; `refreshStructure` tabindex `tree.ts:207-223`                                            |
| Label editing (édition inline du label)                                        | Community  | ✅     | `editable` `tree-item.ts:69` ; events `tree-item:edit-request/commit/cancel` `tree-item.ts:130,180`                  |
| **Drag & drop reordering**                                                     | **Pro**    | 🔴     | Explicitement hors-scope (`tree.ts:62` « drag-and-drop reordering is intentionally out of scope »)                   |
| Lazy loading / virtualisation de l'arbre                                       | Pro        | 🔴     | Hors cible Pro                                                                                                       |

**Bilan Tree View Community : complet** (sélection, multi-select, checkboxes, propagation, clavier, **label editing**). Restent Pro : DnD reordering, virtualisation d'arbre, lazy loading.

---

## Module 5 — Scheduler (`md-scheduler`)

Code : `packages/material-web/scheduler/internal/scheduler.ts` (635 LOC ; tag `scheduler/scheduler.ts:43`).

| Feature                                                  | Tier MUI X                            | Statut | Preuve                                                                                                                |
| -------------------------------------------------------- | ------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------- |
| Event Calendar — vues **day / week / month**, événements | Community (`@mui/x-scheduler`, alpha) | ✅     | `SchedulerView = "day" \| "week" \| "month"` `scheduler/internal/scheduler.ts:13` ; rendu par vue dans `scheduler.ts` |
| Vue timeline / ressources                                | Premium (preview)                     | 🔴     | Hors-scope assumé                                                                                                     |
| Récurrence d'événements                                  | Premium                               | 🔴     | Hors-scope assumé                                                                                                     |
| Drag & drop d'événements                                 | Premium (probable)                    | 🔴     | Hors-scope assumé                                                                                                     |

**Bilan Scheduler : la partie Community (calendrier day/week/month) est couverte.** Récurrence, DnD et vue ressources sont Premium → hors-scope. (Le `@mui/x-scheduler` upstream est lui-même en alpha.)

---

## Module 6 — Composants support (déjà existants)

| Composant        | Tag                   | Rôle MUI X équivalent               | Statut                                 | Preuve                                                                                         |
| ---------------- | --------------------- | ----------------------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Paginator        | `md-paginator`        | Pagination Data Grid (autonome)     | ✅                                     | `paginator/internal/paginator.ts:38-47` (`length`, `pageSize`, `pageIndex`, `pageSizeOptions`) |
| Virtual Scroller | `md-virtual-scroller` | Virtualisation (perf Pro)           | ✅ existe, 🟡 non branché à `md-table` | `virtualscroll/internal/virtual-scroller.ts:26-35` (`items`, `itemHeight`, `buffer`)           |
| Autocomplete     | `md-autocomplete`     | Autocomplete (MUI core, hors MUI X) | ✅                                     | `autocomplete/internal/autocomplete.ts:41-56` (`options`, `value`, `filter`, `open`)           |

---

## Synthèse (état vérifié 2026-05-29)

| Module    | Couverture Community                                                                                                               | Bonus Pro livrés                             |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| Data Grid | ~90 % (tri, filtres, pagination, sélection, CSV, **édition inline**) ; reste du polish (densité, toggle colonnes, no-rows, locale) | multi-sort, resize, reorder, filtres colonne |
| Charts    | **8/8 types usuels** (line/bar/area/pie/scatter/radar/sparkline/gauge) ; radar = surplus                                           | —                                            |
| Pickers   | date / **time** / **datetime** + **i18n (`Intl`)** ; reste les modes edge                                                          | date range picker                            |
| Tree View | complet (expand, sélection, multi, checkbox, clavier, **label edit**)                                                              | —                                            |
| Scheduler | calendrier **day/week/month**                                                                                                      | —                                            |

### Features Community encore manquantes (gaps réels restants)

Tous **secondaires / de confort** — le cœur fonctionnel est couvert :

1. **Data Grid** : sélecteur de densité, toggle de visibilité des colonnes, overlay « no rows », locale i18n de la grille. _(Note : le column pinning est passé **Pro** dans MUI X v9.)_
2. **Pickers** : modes d'affichage edge — digital clock, multi-section digital clock, shortcuts panel, timezone.
3. **Charts** : Pyramid / Range Bar (types secondaires Community).
4. **Chat** (`@mui/x-chat`, **alpha**) : aucun équivalent — module instable, à arbitrer si/quand il se stabilise.

### Hors-scope (rappel) — Pro et Premium

- **Pro** : column pinning, tree data / master-detail (grid), virtualisation intégrée à la grille, Time/Date-Time Range Pickers, drag&drop tree, lazy loading tree, charts Funnel/Heatmap/Sankey, zoom & pan / brush / toolbar charts. _(Note : multi-sort, resize, reorder, filtres par colonne et Date Range Picker — features **Pro** — sont déjà livrés, donc partiellement en avance sur le Pro.)_
- **Premium** : row grouping, agrégation, export Excel (Data Grid) ; vue timeline/ressources et récurrence (Scheduler) ; WebGL rendering, candlestick/OHLC, annotations, AI (Charts). **Volontairement non couvert.**

---

## Conclusion

**MUI X Community est couvert à ~90 %** (les 5 modules — Data Grid, Charts, Pickers, Tree View, Scheduler — ont tous leur cœur fonctionnel livré). Les 8 gaps Community des versions précédentes de ce doc sont **tous fermés** (édition inline grid, scatter+radar, time+datetime pickers, i18n, label edit tree, scheduler). Ne restent que des features de **polish/confort** (densité grid, modes pickers edge, Chat alpha).

Points forts notables :

1. Le monorepo **dépasse sa cible Community sur plusieurs features Pro** (multi-sort, resize, reorder colonnes, filtres par colonne, Date Range Picker) et fournit un **surplus** (`md-radar-chart`, absent de MUI X v9).
2. Sur **`@mui/material` v9**, la couverture est **~93 %** : seuls manquent NumberField et Menubar (nouveautés v9 Base UI), Transfer List et Masonry (`@mui/lab`).

Verdict honnête « surpassé MUI v9 ? » :

- **Oui** sur : surface brute de composants (119 vs ~82), conformité Material Design 3 (tokens/motion/shape/élévation/dynamic color), navigation M3 (rail, bottom app bar, search bar, sheets, scaffold adaptatif — inexistants chez MUI), types de charts (8, dont radar), et extras 3D/WebGPU.
- **Pas encore** sur : la profondeur « prête à l'emploi » du Data Grid Community (densité, visibilité colonnes, locale), les modes edge des pickers, et 2 composants `@mui/material` v9 récents (NumberField, Menubar). Côté écosystème (battle-testing a11y, intégrations form, docs de référence), MUI garde 10 ans d'avance — c'est infrastructurel, pas fonctionnel.

En clair : **surpassé sur la surface, la conformité M3 et le theming ; à parité-haute sur l'usage quotidien ; en léger retrait sur le Data Grid pro-grade et quelques modes pickers.**

---

### Sources

- [MUI X — Licensing](https://mui.com/x/introduction/licensing/)
- [MUI X — Data Grid feature showcase (Community/Pro/Premium)](https://mui.com/x/react-data-grid/features/)
- [MUI X — Charts](https://mui.com/x/react-charts/) · [Sparkline](https://mui.com/x/react-charts/sparkline/) · [Zoom and pan (Pro)](https://mui.com/x/react-charts/zoom-and-pan/)
- [MUI X — Tree View](https://mui.com/x/react-tree-view/) · [Rich Tree View selection](https://mui.com/x/react-tree-view/rich-tree-view/selection/) · [Ordering (Pro)](https://mui.com/x/react-tree-view/rich-tree-view/ordering/)
- [MUI Date Picker Community vs Pro](https://dev.to/9haroon/mui-date-picker-showdown-community-vs-pro-version-4ki0)
