<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — Package design complet

Point d'entrée canonique du système de design aphrody. Assemble la philosophie,
la typographie, la couleur/les tokens, les composants, l'adaptatif et les
références distillées de Google Design, avec les artefacts d'implémentation.

Base : **Material Design 3** ([`M3-FRAMEWORK.md`](M3-FRAMEWORK.md),
[`m3-glossary.md`](m3-glossary.md)). Référence M3 détaillée (paraphrasée) :
[`m3-styles.md`](m3-styles.md), [`m3-foundations.md`](m3-foundations.md),
[`m3-layout.md`](m3-layout.md), [`m3-components.md`](m3-components.md). Source de
vérité des tokens : `crates/m3-tokens` (Rust). Surface JS/TS : `apps/m3-react` +
forks `packages/{material-web,ui,tailwindcss,lit}`.

## 1. Philosophie

D'après [Reinfurt — « vrai > nouveau »](references/reinfurt-true-not-new.md) :
privilégier la **justesse et la cohérence** (une source de tokens unique) à la
nouveauté gratuite ; assumer l'**intuition** et une **dissonance** maîtrisée
(accents de marque sur base neutre) plutôt qu'un design « invisible ».

## 2. Typographie

[Famille Google Sans](references/google-sans-family.md) :
- **Google Sans Flex** (variable, open-source 2025) = police d'UI/display.
- **Google Sans Code** (open-source 2025, Universal Thirst) = code (la police du code dans Gemini).
- Axe **optical size** pour la lisibilité (contreformes, espacement) — cf. [Glimmer](references/transparent-screens-glimmer.md).
- Tokens : `--gem-sys-typography-*` (Gemini), échelle de type M3 (`m3-tokens/typography.rs`, 15 styles).
- **Natif (mui-rs)** : `mui-rs-renderer::text` centralise `FONT_UI` (Google Sans
  Text → Google Sans → Roboto → system), `FONT_DISPLAY`, `FONT_CODE` (Google
  Sans Code) + `TextStyle::ui/display/code` ; tous les composants partagent
  `FONT_UI`. Rendu réel parley → vello `draw_glyphs`.

## 3. Couleur & tokens

- **Source de vérité** : `m3-tokens` (Rust) → HCT → palettes tonales → `--md-sys-color-*`. CLI : `aphrody design tokens [--fusion|--dark|--format shadcn-registry]`.
- **Fusion** : `--md-sys-color-*` → alias shadcn (`--primary: var(...)`) + Tailwind `@theme inline` (`docs/design/FUSION-PLAN.md`).
- **Thème Gemini** (importé au pixel près) : [`gemini/theme.css`](gemini/theme.css) + [`gemini/README.md`](gemini/README.md) — palette de marque `--gem-sys-color--brand-*`, surfaces dark, gradients directionnels.
- **Langage visuel Gemini** : [gradients/cercle/motion](references/gemini-visual-language.md).
  Natif : `mui-rs-renderer::gradient` (`brand_linear` bleu→violet→cyan,
  `directional` lead-opaque→transparent) + couleurs de marque.
- **Contraste** : règle M3 (Δ40 ⇒ ≥3.0, Δ50 ⇒ ≥4.5) ; pour l'ambient/additif, ratio de contraste additif (cf. Glimmer).

## 4. Forme, élévation, motion, états

`m3-tokens` : `shape.rs` (none 0 → full), `elevation.rs` (0/1/3/6/8/12 dp),
`motion.rs` (easing + durations), `state.rs` (opacités state layers). Cercle =
forme fondatrice (Gemini). Motion directionnelle à début/fin définis.

## 5. Adaptatif

[`m3-tokens/adaptive.rs`](../../crates/m3-tokens/src/adaptive.rs) (natif) :
breakpoints (Compact/Medium/Expanded/Large/ExtraLarge), panes (1→3),
navigation (bar→rail collapsed→rail expanded), grille (4/12 colonnes, marges
16/24 dp), parts of layout (bars/rails/panes). Ambient/XR : principes
[Glimmer](references/transparent-screens-glimmer.md) (surfaces sombres + contenu
clair, ~1 m de profondeur, ~0,6° min, notifications ~2 s).

## 6. Composants

- **Spec M3 & couverture web** : [`m3-components-spec.md`](m3-components-spec.md)
  + [`m3-web-update.md`](m3-web-update.md) (35 composants M3 × statut
  material-web : 15 présents / 8 partiels / 12 manquants + roadmap P0/P1).
- **Couverture native (mui-rs)** : [`m3-components.md`](m3-components.md) — le set
  M3 **complet** est implémenté en Rust (`crates/mui-rs-components`), chaque
  composant rendant formes + glyphes réels (zéro stub). C'est la voie native ;
  material-web est la voie web/JS.
- **Implémentation web** : `packages/material-web` (`md-*`, Lit) ; **wrappers
  React** : `apps/m3-react` (~32) consommables par shadcn (`packages/ui`).
- **Manquants web prioritaires** (P0) : snackbar, app-bars, navigation-rail,
  search (déjà présents côté natif mui-rs).

## 7. Références (distillées, attribuées)

| Note | Source Google Design |
|------|----------------------|
| [Famille Google Sans](references/google-sans-family.md) | google-sans-flex-font |
| [Langage visuel Gemini](references/gemini-visual-language.md) | gemini-ai-visual-design |
| [Écrans transparents / Glimmer](references/transparent-screens-glimmer.md) | transparent-screens |
| [Reinfurt — vrai > nouveau](references/reinfurt-true-not-new.md) | david-reinfurt-teaches-design |

> Les notes sont des distillations factuelles (nos mots) avec attribution et
> liens ; le texte des articles Google Design (copyright) n'est pas reproduit
> verbatim.

## 8. Artefacts d'implémentation

| Domaine | Emplacement |
|---|---|
| Tokens (source de vérité) | `crates/m3-tokens` (Rust) |
| CLI génération | `aphrody design tokens` |
| Fusion shadcn (registry:theme) | `packages/ui/.../public/r/styles/*/theme-aphrody.json` |
| Wrappers React | `apps/m3-react` |
| Thème Gemini importé | `docs/design/gemini/theme.css` |
| Toolchain forks (oxlint/oxfmt/bun/n2b) | `just sync-packages` |
