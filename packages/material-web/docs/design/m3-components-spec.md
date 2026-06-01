# Audit de couverture M3 Components × material-web

Source spec : `https://m3.material.io/components/<slug>` (fetch via `mcp__aphrody__universal_web_fetch`, mai 2026). Synthèses par composant dans `var/m3-spec/components/<slug>.md`.
Couverture code : custom elements `md-*` du package `packages/material-web/` (inventaire `@customElement` réel ; `labs/` = non-stable, `labs/gb/` = génération M3 Expressive en cours).

Légende statut :

- **présent** = custom element exporté dans une famille stable (hors `labs/`).
- **partiel** = présent uniquement sous `labs/` (preview, non publié stable) ou couverture incomplète des variantes.
- **manquant** = aucun `md-*` correspondant dans material-web.

## Tableau des 35 composants

| #   | Composant M3           | Catégorie         | Élément(s) `md-*`                                                                                          | Statut   | Variantes clés                                                          |
| --- | ---------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------- | -------- | ----------------------------------------------------------------------- |
| 1   | button-groups          | action            | (`md-button` labs/gb)                                                                                      | manquant | standard, connected ; XS-XL ; single/multi/required select              |
| 2   | buttons                | action            | `md-elevated-button`, `md-filled-button`, `md-filled-tonal-button`, `md-outlined-button`, `md-text-button` | présent  | 5 couleurs, 5 tailles (Expressive), round/square, default/toggle        |
| 3   | extended-fab           | action            | `md-fab` (variant extended), `md-branded-fab`                                                              | partiel  | small/medium/large (Expressive non couvert web)                         |
| 4   | fab-menu               | action            | —                                                                                                          | manquant | 2-6 actions ; primary/secondary/tertiary                                |
| 5   | floating-action-button | action            | `md-fab`, `md-branded-fab`                                                                                 | présent  | FAB/medium/large ; tone + container colors (Expressive non couvert)     |
| 6   | icon-buttons           | action            | `md-icon-button`, `md-filled-icon-button`, `md-filled-tonal-icon-button`, `md-outlined-icon-button`        | présent  | default/toggle ; 5 tailles, 2 shapes, 3 widths (Expressive non couvert) |
| 7   | segmented-buttons      | action/selection  | `md-outlined-segmented-button`, `md-outlined-segmented-button-set` (labs)                                  | partiel  | single/multi-select (déprécié Expressive → connected button group)      |
| 8   | split-button           | action            | `md-split-button` (labs/gb)                                                                                | partiel  | XS-XL ; elevated/filled/tonal/outlined                                  |
| 9   | date-pickers           | selection         | —                                                                                                          | manquant | docked, modal, modal input                                              |
| 10  | time-pickers           | selection         | —                                                                                                          | manquant | dial, input                                                             |
| 11  | loading-indicator      | communication     | —                                                                                                          | manquant | loading / contained (Expressive, < 5s, pull-to-refresh)                 |
| 12  | progress-indicators    | communication     | `md-linear-progress`, `md-circular-progress`                                                               | présent  | linear/circular (wavy Expressive non couvert)                           |
| 13  | navigation-bar         | navigation        | `md-navigation-bar`, `md-navigation-tab` (labs)                                                            | partiel  | 3-5 destinations ; flexible bar Expressive                              |
| 14  | navigation-drawer      | navigation        | `md-navigation-drawer`, `md-navigation-drawer-modal` (labs)                                                | partiel  | standard, modal (déprécié Expressive → expanded rail)                   |
| 15  | navigation-rail        | navigation        | —                                                                                                          | manquant | collapsed/expanded ; 3-7 destinations + FAB                             |
| 16  | bottom-sheets          | containment       | —                                                                                                          | manquant | standard, modal (radius 28dp, drag handle)                              |
| 17  | side-sheets            | containment       | —                                                                                                          | manquant | standard, modal (radius 16dp, RTL)                                      |
| 18  | app-bars               | navigation        | —                                                                                                          | manquant | search/small/medium flexible/large flexible                             |
| 19  | badges                 | communication     | `md-badge` (labs)                                                                                          | partiel  | small (dot), large (texte/compte, max 4 char)                           |
| 20  | cards                  | containment       | `md-elevated-card`, `md-filled-card`, `md-outlined-card` (labs)                                            | partiel  | elevated, filled, outlined                                              |
| 21  | carousel               | containment       | —                                                                                                          | manquant | 6 layouts (multi-browse, hero, full-screen, ...)                        |
| 22  | checkbox               | selection         | `md-checkbox`                                                                                              | présent  | unselected/selected/indeterminate + error states                        |
| 23  | chips                  | selection         | `md-assist-chip`, `md-filter-chip`, `md-input-chip`, `md-suggestion-chip`, `md-chip-set`                   | présent  | assist, filter, input, suggestion                                       |
| 24  | dialogs                | communication     | `md-dialog`                                                                                                | présent  | basic, full-screen                                                      |
| 25  | divider                | containment       | `md-divider`                                                                                               | présent  | horizontal, vertical                                                    |
| 26  | lists                  | containment       | `md-list`, `md-list-item`, `md-item`                                                                       | présent  | standard, segmented (Expressive non couvert)                            |
| 27  | menus                  | containment       | `md-menu`, `md-menu-item`, `md-sub-menu`, `md-menu-group` (labs/gb)                                        | présent  | dropdown/context ; vertical menus Expressive (non couvert)              |
| 28  | radio-button           | selection         | `md-radio`                                                                                                 | présent  | single-select                                                           |
| 29  | search                 | navigation        | —                                                                                                          | manquant | search bar + search view ; contained/divided                            |
| 30  | sliders                | selection         | `md-slider`                                                                                                | présent  | standard, centered, range (orientation/sizes Expressive non couverts)   |
| 31  | snackbar               | communication     | —                                                                                                          | manquant | dismissive, non-dismissive                                              |
| 32  | switch                 | selection         | `md-switch`                                                                                                | présent  | on/off, icône optionnelle dans le handle                                |
| 33  | tabs                   | navigation        | `md-tabs`, `md-primary-tab`, `md-secondary-tab`                                                            | présent  | primary, secondary                                                      |
| 34  | text-fields            | text-input        | `md-filled-text-field`, `md-outlined-text-field`                                                           | présent  | filled, outlined                                                        |
| 35  | toolbars               | navigation/action | —                                                                                                          | manquant | docked toolbar, floating toolbar                                        |

Composants connexes hors-catalogue présents dans material-web (helpers, non comptés dans les 35) : `md-elevation`, `md-focus-ring`, `md-ripple`, `md-icon`, `md-field` (`md-filled-field`/`md-outlined-field`), `md-select` (`md-filled-select`/`md-outlined-select`/`md-select-option`). À noter : `select` (dropdown menu exposé) couvre une partie de la surface « menus » M3.

## Bilan par catégorie (35 composants)

| Catégorie     | Total | présent                                    | partiel                                           | manquant                                        |
| ------------- | ----- | ------------------------------------------ | ------------------------------------------------- | ----------------------------------------------- |
| action        | 8     | 3 (buttons, fab, icon-buttons)             | 3 (extended-fab, segmented-buttons, split-button) | 2 (button-groups, fab-menu)                     |
| containment   | 6     | 3 (divider, lists, menus)                  | 2 (cards, carousel\*)                             | 1 (bottom-sheets/side-sheets/carousel)          |
| communication | 5     | 2 (progress, dialogs)                      | 1 (badges)                                        | 2 (loading-indicator, snackbar)                 |
| navigation    | 7     | 1 (tabs)                                   | 2 (navigation-bar, navigation-drawer)             | 4 (navigation-rail, app-bars, search, toolbars) |
| selection     | 8     | 5 (checkbox, chips, radio, slider, switch) | 1 (segmented-buttons)                             | 2 (date-pickers, time-pickers)                  |
| text-input    | 1     | 1 (text-fields)                            | 0                                                 | 0                                               |

(\* carousel comptée manquante en containment ; cards partiel via labs.)

Synthèse globale : **15 présents stables**, **8 partiels** (labs / labs.gb / variantes Expressive non couvertes), **12 manquants**.
Note transversale : material-web n'expose **aucune** variante M3 Expressive (mai 2025) en stable ; le statut « Web: Expressive Unavailable » est confirmé sur toutes les pages spec. La nouvelle génération apparaît seulement dans `labs/gb/` (button, card, fab, split-button, menu, list, switch, checkbox, radio, divider, icon-button) — non publiée.

## Composants manquants prioritaires et reco d'implémentation

Priorisation selon usage probable dans une UI aphrody (terminal LLM, dashboards, chat, design suite). Politique repo : UI = WASM Rust natif (`wasm-bindgen`) avec wrappers de Material Web Components 3 ; pour les composants absents de material-web, deux voies — (A) **wrapper** d'un MWC labs (rapide, dette = preview API), (B) **web component Lit custom** (Lit déjà dépendance transitive de material-web ; alignement tokens via `@material/web/tokens`).

P0 — bloquants UI courants, aucun MWC stable :

- **snackbar** (communication) : feedback async omniprésent (chat, deploy, image gen). Reco : **Lit custom** léger (pas de labs upstream), s'appuyer sur `md-elevation` + tokens ; supporter dismissive/non-dismissive + slot action.
- **app-bars** (navigation) : ossature de toute vue. Reco : **Lit custom** (small variant d'abord), scroll-fill via `md-elevation` ; reporter medium/large flexible (Expressive).
- **navigation-rail** (navigation) : cible medium/expanded (desktop dashboards). Reco : **Lit custom** collapsed d'abord ; réutiliser `md-focus-ring`/`md-ripple` pour les items, active pill via tokens.
- **menus** est présent (`md-menu`) mais **search** (navigation) manque : entrée globale fréquente. Reco : **Lit custom** search bar + search view, résultats en `md-list`.

P1 — fréquents, composants labs disponibles (wrapper vs hardening) :

- **cards** : promouvoir le **wrapper** `labs/card` (`md-elevated/filled/outlined-card`) en surface stable interne ; risque API faible (3 variants figés).
- **badges** : **wrapper** `labs/badge` (`md-badge`) ; ancrage upper-trailing à gérer côté host.
- **navigation-bar** : **wrapper** `labs/navigationbar` (compact) ; documenter le statut preview.
- **segmented-buttons** : **wrapper** `labs/segmentedbutton(set)` mais marquer déprécié Expressive — préférer à terme un connected **button-group** custom.

P2 — spécialisés, effort élevé, demande conditionnelle :

- **date-pickers** / **time-pickers** (selection) : surfaces calendrier/dial complexes. Reco : **Lit custom** seulement si un formulaire date l'exige ; sinon `<input type=date/time>` natif stylé via tokens en intérim.
- **carousel** (containment) : 6 layouts + parallax. Reco : **Lit custom** layout `hero`/`multi-browse` d'abord, snap via CSS scroll-snap.
- **bottom-sheets** / **side-sheets** (containment) : Reco : **Lit custom** au-dessus de `md-elevation`, drag handle 48dp ; side-sheet d'abord (desktop).
- **toolbars** (navigation/action) : docked/floating Expressive. Reco : **Lit custom**, après stabilisation app-bars.
- **fab-menu** (action) : **Lit custom** s'ouvrant depuis `md-fab` (overlay + 2-6 items).
- **button-groups** (action) : **Lit custom** conteneur orchestrant des `md-*-button`, shape morph via tokens — remplace segmented-buttons à terme.

Recommandation transversale : centraliser les nouveaux web components custom dans une famille dédiée (p. ex. `packages/material-web-ext/` ou un crate de wrappers `wasm-bindgen`), consommer `@material/web/tokens` pour rester aligné dynamic color, et ne pas dépendre des API `labs/gb/` Expressive tant qu'elles ne sont pas publiées par Google.
