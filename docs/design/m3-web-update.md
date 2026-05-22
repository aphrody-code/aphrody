<!-- SPDX-License-Identifier: Apache-2.0 -->
# Mise à jour « M3 Web » (material-web) depuis le scrape récursif du glossaire

> **MAJ 2026-05-22 — catalogue complété.** Les 12 composants manquants sont
> désormais livrés dans `packages/material-web` en Lit auto-suffisant (css
> inline + tokens `--md-sys-*`, sans pipeline SASS) : `md-snackbar`,
> `md-top-app-bar`/`md-bottom-app-bar`, `md-navigation-rail`(+item),
> `md-search-bar`, `md-toolbar`, `md-bottom-sheet`, `md-side-sheet`,
> `md-carousel`(+item), `md-loading-indicator`, `md-button-group`,
> `md-fab-menu`(+item), `md-date-picker`, `md-time-picker`. Ajoutés aussi : la
> famille **layout** adaptative (`md-scaffold`, `md-pane`, `md-list-detail`,
> `md-supporting-pane`), la typo **`md-type`** (axes Google Sans Flex,
> flexibilité max + animation) et les effets **`md-webgpu-canvas`**
> (spectrum-shift / sparkle / glimmer, WGSL + fallback CSS). Les composants
> `labs/` stables (badge, cards, navigation bar/drawer/tab, segmented button)
> sont promus via `aphrody-labs.ts`. Tout est branché dans `all.ts` (via
> `aphrody-components.ts`) + wrappers React dans `apps/m3-react`. Détail :
> [`packages/material-web/APHRODY-M3.md`](../../packages/material-web/APHRODY-M3.md).
> Validation : `tsc --noEmit` exit 0 sur les 24 custom elements (flags stricts
> du projet : noUnusedLocals, noImplicitOverride, noImplicitReturns,
> noPropertyAccessFromIndexSignature).

Consolide le **scrape récursif** de Material Design 3 — index glossaire +
toutes les pages composants — en un plan d'update concret pour `packages/material-web`
(le « M3 web »). Référence : [`m3-glossary.md`](m3-glossary.md),
[`m3-components-spec.md`](m3-components-spec.md), [`DESIGN-PACKAGE.md`](DESIGN-PACKAGE.md).

## Méthode (et statut bxc)

- Cible demandée : `bxc --help` puis scrape récursif de
  <https://m3.material.io/foundations/glossary>.
- **bxc inutilisable sur cet hôte** : `bxc.exe` v0.3.0 (101 Mo) **segfault au
  démarrage** (bug Bun standalone `--smol`, Windows) ; la variante Linux via WSL
  manque `libbxc_rust_bridge.so` (chemin dev hardcodé) et `bxc install` échoue
  hors-ligne. Documenté dans [`gemini/README.md`](gemini/README.md) et la mémoire.
- **Fallback fonctionnel** : outil Rust `universal_web_fetch` (rend les SPA).
- **Scrape récursif réalisé** : glossaire (Material A–Z) + **35 pages composants**
  + pages Styles/Foundations → `var/m3-spec/{components,foundations,styles}/*.md`
  (40 fichiers). Le glossaire liste chaque composant avec un lien « Learn more » ;
  ces 35 pages cibles ont toutes été scrapées (= récursif sur les composants).

## État de couverture M3 web (`packages/material-web`)

35 composants M3 × custom elements `md-*` (détail dans `m3-components-spec.md`) :
- **15 présents** (stables) : buttons, icon-buttons, FAB, checkbox, radio, switch,
  slider, chips, text-fields, dialogs, divider, lists, menus, progress, tabs.
- **8 partiels** : variantes Expressive non couvertes en stable, segmented-buttons,
  navigation-bar, badges, cards (labs/gb non publié)…
- **12 manquants** : snackbar, app-bars, navigation-rail, search, side-sheets,
  bottom-sheets, date-pickers, time-pickers, toolbars, carousel, loading-indicator,
  button-groups / fab-menu / split-button.

Constat clé du scrape : **material-web n'expose aucune variante M3 Expressive en
stable** (« Web: Expressive Unavailable » sur toutes les pages) ; la nouvelle
génération vit dans `labs/gb/` (non publié).

## Plan d'update concret

### P0 — composants manquants à fort usage (nouveaux web components Lit)
`snackbar`, `app-bars` (top/bottom), `navigation-rail`, `search`.

### P1 — wrappers / promotion depuis labs
`cards`, `badges`, `navigation-bar` (stabiliser), `segmented-buttons`.

### Voie de consommation (déjà en place, côté JS/TS)
- Les `md-*` existants sont déjà exposés en React via **`apps/m3-react`**
  (`@lit/react createComponent`, ~32 wrappers) → consommables par shadcn.
- Au fur et à mesure que les P0/P1 sont ajoutés à material-web, ajouter le wrapper
  correspondant dans `apps/m3-react/src/index.ts`.
- Thématisation : tokens M3 (`--md-sys-color-*`) via `aphrody design tokens`
  (fusion shadcn/tailwind) ; cf. [`FUSION-PLAN.md`](FUSION-PLAN.md).

## Modernisation lit / material-web (focus JS/TS)

- **lit** (`packages/lit`) : forké (`aphrody-code/lit`), config oxc/bun synchronisée,
  **bunisé via `n2b --fix`** (125 fichiers : `node:` prefixes, `Bun.file`…).
- **material-web** (`packages/material-web`) : **bunisé via `n2b --fix`** (21 fichiers),
  config oxc/bun synchronisée.
- Sync toolchain des 5 forks : `just sync-packages` (oxlint + oxfmt + n2b report).
- Bridge React moderne : `apps/m3-react` (TS6, `tsc --noEmit` + oxlint verts).
