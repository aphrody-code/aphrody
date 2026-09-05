# Le framework Material Design 3 ultime d'aphrody

Framework UI unifié : **Material Design 3** comme système de design canonique,
piloté par Rust, exposé à tout l'écosystème (web components, React/shadcn,
Tailwind). Capstone des forks `packages/*` et de la chaîne de tokens.

Référence terminologique : [m3-glossary.md](m3-glossary.md) (scrape de
m3.material.io/foundations/glossary). Plan de fusion : [FUSION-PLAN.md](FUSION-PLAN.md).

## Architecture en une image

```
                 crates/m3-tokens (Rust, cible #1)         ← SOURCE DE VÉRITÉ
   seed → HCT (hue/chroma/tone) → palettes tonales (13 tons) → ColorRoles
                          │
        export_css / export_fusion_css / export_shadcn_aliases / export_tailwind_theme
                          │  (CLI: `aphrody design tokens [--fusion|--dark|--format shadcn-registry]`)
                          ▼
        tokens.css : :root { --md-sys-color-* ; --md-sys-typescale-* ; --md-sys-shape-* }
                          │
   ┌──────────────────────┼───────────────────────────────────────────┐
   ▼                      ▼                                             ▼
 packages/material-web   packages/ui (shadcn)                    packages/tailwindcss
 web components md-*     theme-aphrody.json (registry:theme)      @theme inline
 (Lit) héritent direct   --primary: var(--md-sys-color-primary)   --color-*: var(--md-sys-color-*)
   │                      │
   └──── packages/lit : @lit/react `createComponent` ────┐
         + labs/gen-wrapper-react + labs/ssr-react/nextjs ▼
                          composants md-* consommables en React/Next.js (shadcn)
```

## Les piliers (forks `packages/*`, tous synchronisés)

| Pilier | Fork | Rôle dans le framework |
|---|---|---|
| **Tokens** | `crates/m3-tokens` (Rust, in-tree) | Source de vérité : HCT, palettes tonales, `--md-sys-*`, export CSS + fusion. |
| **Composants** | `packages/material-web` | ~82 web components `md-*` (Lit), héritent les `--md-sys-color-*`. Bunisé (n2b). |
| **Runtime WC** | `packages/lit` | `@lit/react` (wrap md-* → React), `labs/gen-wrapper-react`, `labs/ssr-react`/`nextjs`. Bunisé. |
| **React UI** | `packages/ui` (shadcn) | Couche composants React ; thème via `theme-aphrody.json` (`registry:theme`). |
| **Utilitaires CSS** | `packages/tailwindcss` | Tailwind v4 ; `@theme inline` mappé sur les tokens M3. |
| **Style guard** | `packages/gts` | Référence config oxc Google-style (TS6 + bun). |

Dénominateur commun = **CSS custom properties** (seules à traverser le Shadow DOM des `md-*`). Toolchain commune : **oxlint + oxfmt + bun + n2b** (`just sync-packages`).

## Ce qui est livré (étapes 1-2)

1. **Générateur `m3-tokens`** : `export_css`, `export_shadcn_aliases`, `export_tailwind_theme`, `export_fusion_css`, `color_vars`, `FUSION_ALIAS_MAP` (public). 81 tests + 9 doctests, clippy clean.
2. **CLI** : `aphrody design tokens [--fusion] [--dark] [--format css|shadcn-registry] [-o f]` (100% Rust, headless).
3. **shadcn** : `theme-aphrody.json` (`registry:theme`, schéma-valide, 19 alias + 36 couleurs light/dark) dans `packages/ui/apps/v4/public/r/styles/{new-york,default}/`.
4. **Glossaire M3** : `m3-glossary.md` (référence canonique, index composants ↔ `md-*`).
5. **Forks synchronisés** (5) sur oxc/bun + bunisés (n2b).

## Reste (étapes 3-5, dérivables)

- **Tailwind** : importer `@theme inline` (sortie `aphrody design tokens --fusion`) dans les apps.
- **material-web** : héritage direct de `tokens.css` (aucun mapping).
- **Wrappers React** : FAIT — `apps/m3-react` (`@aphrody/m3-react`) wrappe ~32 composants `md-*` en React via `@lit/react createComponent`, thémés par `theme.css` (sortie `aphrody design tokens --fusion`). `tsc --noEmit` + oxlint verts. Consommable par shadcn/React. (Alternative codegen : `labs/gen-wrapper-react`.)
- **SSR** : `labs/ssr-react` + `labs/nextjs` pour rendre les `md-*` dans l'app Next.js de `packages/ui`.
- **Typographie/forme/élévation** : étendre la fusion au-delà de la couleur (`--md-sys-typescale-*`, `--md-sys-shape-*`, `--md-sys-elevation-*`) — `m3-tokens` a déjà `typography.rs`, `shape.rs`, `elevation.rs`.

## Couverture composants

Les 32 composants/concepts du glossaire sont cartographiés vers leurs `md-*` dans
[m3-glossary.md](m3-glossary.md). Les manquants côté material-web (Card, Banner,
Bottom sheet, Data table, Date/Time picker, Navigation rail, Side sheet, Snackbar,
Toolbar) sont les candidats prioritaires pour des composants custom Lit dans la
suite du framework.

## Adaptive layout natif (Rust) — `m3-tokens::adaptive`

Implémentation 100% Rust de l'adaptive design M3 (cible #1, headless, no_std-compatible) — refs [glossary](m3-glossary.md), m3.material.io/foundations/layout.

- **`Breakpoint`** (window size class) : `Compact <600` · `Medium 600–839` · `Expanded 840–1199` · `Large 1200–1599` · `ExtraLarge ≥1600` (dp). `from_width_dp()`, `min/max_width_dp()`.
- **Recommandations par breakpoint** : `recommended_panes()`/`max_panes()` (1→3), `navigation()` (`Bar`→`RailCollapsed`→`RailExpanded`), `action_surface()` (`BottomSheet`→`Menu`), `dialog()` (`FullScreenOrBasic`→`Basic`).
- **Parts of layout** : `ScaffoldRegion` (Bars/Rails/Panes), `PaneKind` (Fixed/Flexible), `Containment` (Implicit/Explicit), `pane_layout()` (logique leading→trailing, RTL-aware), `grid_columns()` (4/12), `margin_dp()` (16/24), `PANE_SPACER_DP=24`.
- **Export CSS** : `export_breakpoints_css()` → `:root { --md-sys-breakpoint-* }` + `@theme { --breakpoint-* }` (Tailwind v4) ⇒ s'intègre à la fusion tokens.
- **Tests** : 7 unit + 3 doctests (boundaries, contiguïté des plages, swaps navigation/action/dialog, panes, parts of layout, export CSS).
