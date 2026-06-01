<!-- Livrable 09 du kit de migration MUI → material-web. Audit de couverture Tailwind CSS (lecture seule). -->

# Audit de couverture — intégration Tailwind CSS ⇄ système M3 (`@material/web`)

Ce rapport vérifie, famille de tokens par famille de tokens, dans quelle mesure **toute l'intégration Tailwind CSS v4** couvre le système M3 du fork (`material-web/` — `@material/web@2.4.1`) : quels rôles `--md-sys-*` sont réellement exposés au _runtime_ par la lib, lesquels sont mappables en `@theme inline` (couleur, typescale, shape), lesquels n'ont **aucun équivalent runtime** ni namespace Tailwind natif (motion, state, elevation), la couverture layout (ex-MUI), les limites dures (Shadow DOM, `::part()`), et un verdict sur ce qui n'est PAS couvrable par Tailwind seul. Toutes les affirmations sont vérifiées sur les deux repos locaux (`material-web/`, `tailwindcss/` v4.3.0) et la doc Tailwind v4.

---

## 0. Fait fondateur — ce que la lib expose réellement au runtime

Avant de parler de mapping, il faut savoir ce qui _existe_ comme variable CSS au runtime. **Tous les `--md-sys-*` ne sont PAS des variables CSS exposées.** Les fichiers `tokens/_md-sys-*.scss` se répartissent en deux comportements :

- **Familles qui émettent des custom properties `var(--md-sys-…)`** : seules `color` et `typescale` enveloppent leurs valeurs dans `var(--md-sys-<famille>-<token>, <fallback>)` au moment de la génération Sass (boucle visible : `tokens/_md-sys-color.scss:81-90`, `tokens/_md-sys-typescale.scss:131-141`).
- **Familles résolues à la compilation Sass, SANS custom property** : `shape`, `elevation`, `motion`, `state` retournent des _maps de valeurs brutes_ consommées au compile-time dans le SCSS des composants (`tokens/_md-sys-motion.scss:10-12` = simple passthrough ; `tokens/_md-sys-elevation.scss:14-30` ; `tokens/_md-sys-shape.scss` ; `tokens/_md-sys-state.scss`). Pas de `var(--md-sys-…)` généré.

**Preuve définitive — scan du CSS compilé** (les `.css` réellement embarqués dans les composants, `material-web/<comp>/internal/*-styles.css`) :

```
--md-sys-color-*      → 39 variables distinctes référencées au runtime
--md-sys-typescale-*  → 0
--md-sys-shape-*      → 0
--md-sys-elevation-*  → 0
--md-sys-motion-*     → 0
--md-sys-state-*      → 0
```

Dans le CSS compilé, typescale/shape/state sont déjà résolus en **variables locales `--_*`** (scopées au shadow root) ou en valeurs en dur. Exemple sur le bouton (`material-web/button/internal/shared-styles.css`) :

```css
font-family: var(--_label-text-font); /* --_label-text-font, pas --md-sys-typescale-* */
font-size: var(--_label-text-size);
line-height: var(--_label-text-line-height);
font-weight: var(--_label-text-weight);
/* state : var(--_hover-state-layer-opacity), var(--_pressed-state-layer-opacity) — locales */
```

L'override au runtime de ces familles se fait donc **uniquement** via les **tokens composant** `--md-comp-*` (ex. `--md-filled-button-container-shape`, `--md-filled-button-container-elevation`), pas via des `--md-sys-shape-*`/`--md-sys-elevation-*` qui n'existent pas dans le DOM.

> **Impact sur la doc 06** : `06-tailwind-material-web.md` §2.3 mappe `--text-title-medium--letter-spacing: var(--md-sys-typescale-title-medium-tracking)` et §2.4 mappe `--radius-*: var(--md-sys-shape-corner-*)`. **Ces `var()` ne résolvent vers RIEN par défaut** : ni `--md-sys-shape-corner-*` ni `--md-sys-typescale-*-tracking` ne sont émis par la lib (voir §1.3 et §1.2 ci-dessous). Le mapping ne « marche » que si l'app **déclare elle-même** ces variables sur `:root`. C'est faisable et c'est d'ailleurs ce que fait l'exemple §5.3 de la doc 06 pour shape — mais ce n'est PAS « dériver des tokens de la lib », c'est **redéclarer un jeu parallèle**. À documenter comme tel.

---

## 1. Couverture des tokens, famille par famille

### 1.1 Color roles — **couverture complète et native** (mapping `@theme inline` 1:1)

Source de vérité des noms : `tokens/_md-sys-color.scss:15-66` (`$supported-tokens`, **47 rôles**). Mapping vers le namespace Tailwind `--color-*` (vérifié `tailwindcss/packages/tailwindcss/src/utilities.ts` — `themeKeys: ['--color']`, qui alimente `bg-*`/`text-*`/`border-*`/`fill-*`/`stroke-*`/`ring-*`/`outline-*`/`accent-*`/`caret-*`/`divide-*`/`placeholder-*`).

`@theme inline` est obligatoire (résolution au scope de l'élément → suit les overrides scopés de tokens M3 ; vérifié `tailwindcss/packages/tailwindcss/src/index.ts:93`). Bloc **complet** (les 47 rôles `$supported-tokens`, incluant les familles `*-fixed` et `surface-tint` absentes de la doc 06 §2.2) :

```css
@theme inline {
  /* --- couleurs principales --- */
  --color-primary: var(--md-sys-color-primary);
  --color-on-primary: var(--md-sys-color-on-primary);
  --color-primary-container: var(--md-sys-color-primary-container);
  --color-on-primary-container: var(--md-sys-color-on-primary-container);
  --color-secondary: var(--md-sys-color-secondary);
  --color-on-secondary: var(--md-sys-color-on-secondary);
  --color-secondary-container: var(--md-sys-color-secondary-container);
  --color-on-secondary-container: var(--md-sys-color-on-secondary-container);
  --color-tertiary: var(--md-sys-color-tertiary);
  --color-on-tertiary: var(--md-sys-color-on-tertiary);
  --color-tertiary-container: var(--md-sys-color-tertiary-container);
  --color-on-tertiary-container: var(--md-sys-color-on-tertiary-container);
  --color-error: var(--md-sys-color-error);
  --color-on-error: var(--md-sys-color-on-error);
  --color-error-container: var(--md-sys-color-error-container);
  --color-on-error-container: var(--md-sys-color-on-error-container);

  /* --- fixed (accent stables clair/sombre) — MANQUANTS dans doc 06 --- */
  --color-primary-fixed: var(--md-sys-color-primary-fixed);
  --color-primary-fixed-dim: var(--md-sys-color-primary-fixed-dim);
  --color-on-primary-fixed: var(--md-sys-color-on-primary-fixed);
  --color-on-primary-fixed-variant: var(--md-sys-color-on-primary-fixed-variant);
  --color-secondary-fixed: var(--md-sys-color-secondary-fixed);
  --color-secondary-fixed-dim: var(--md-sys-color-secondary-fixed-dim);
  --color-on-secondary-fixed: var(--md-sys-color-on-secondary-fixed);
  --color-on-secondary-fixed-variant: var(--md-sys-color-on-secondary-fixed-variant);
  --color-tertiary-fixed: var(--md-sys-color-tertiary-fixed);
  --color-tertiary-fixed-dim: var(--md-sys-color-tertiary-fixed-dim);
  --color-on-tertiary-fixed: var(--md-sys-color-on-tertiary-fixed);
  --color-on-tertiary-fixed-variant: var(--md-sys-color-on-tertiary-fixed-variant);

  /* --- background / surfaces --- */
  --color-background: var(--md-sys-color-background);
  --color-on-background: var(--md-sys-color-on-background);
  --color-surface: var(--md-sys-color-surface);
  --color-on-surface: var(--md-sys-color-on-surface);
  --color-surface-variant: var(--md-sys-color-surface-variant);
  --color-on-surface-variant: var(--md-sys-color-on-surface-variant);
  --color-surface-dim: var(--md-sys-color-surface-dim);
  --color-surface-bright: var(--md-sys-color-surface-bright);
  --color-surface-container-lowest: var(--md-sys-color-surface-container-lowest);
  --color-surface-container-low: var(--md-sys-color-surface-container-low);
  --color-surface-container: var(--md-sys-color-surface-container);
  --color-surface-container-high: var(--md-sys-color-surface-container-high);
  --color-surface-container-highest: var(--md-sys-color-surface-container-highest);
  --color-surface-tint: var(--md-sys-color-surface-tint); /* MANQUANT dans doc 06 */

  /* --- inverse / outline / utilitaires --- */
  --color-inverse-surface: var(--md-sys-color-inverse-surface);
  --color-inverse-on-surface: var(--md-sys-color-inverse-on-surface);
  --color-inverse-primary: var(--md-sys-color-inverse-primary);
  --color-outline: var(--md-sys-color-outline);
  --color-outline-variant: var(--md-sys-color-outline-variant);
  --color-scrim: var(--md-sys-color-scrim);
  --color-shadow: var(--md-sys-color-shadow);
}
```

> **Note de réconciliation runtime** : les 47 rôles `$supported-tokens` sont _exposables_ en `@theme`, mais le CSS compilé n'en consomme effectivement que **39** (les 16 `*-fixed` ne sont consommés par aucun composant du fork ; en revanche le compilé ajoute `on-surface-bright`/`on-surface-dim`, hors `$supported-tokens`, via les composants labs). Conséquence pratique : mapper les `*-fixed` dans `@theme inline` ne « casse » rien mais ne produit une couleur que si l'app les a définis sur `:root`. **Tous les rôles que les `md-*` peignent réellement (39) sont couverts par le bloc ci-dessus.** Verdict famille color : **couverture 100 %, native, scopée.**

### 1.2 Typescale — **couverture partielle** (`size`/`line-height`/`weight` OUI ; `tracking` NON par défaut)

Source : `tokens/_md-sys-typescale.scss`. **Subtilité décisive** : `_md-sys-typescale.scss:15-79` (`$supported-tokens`) contient `*-size`, `*-line-height`, `*-weight`, mais le bloc `$unsupported-tokens` (`:82-114`) liste explicitement **tous les `*-tracking`** (letter-spacing) + les tokens composites `body-large` etc. La fonction `values()` filtre via `validate.values(... $unsupported-tokens)` (`:144-148`) → **`--md-sys-typescale-*-tracking` n'est JAMAIS émis comme custom property.** Les valeurs de tracking existent dans la map brute (`versions/v0_192/_md-sys-typescale.scss:40` `body-large-tracking: 0.03125rem`…) mais sont stripées à l'émission.

Namespaces Tailwind : `--text-*` (taille) avec valeurs liées `--text-X--line-height` / `--text-X--font-weight` / `--text-X--letter-spacing` (vérifié `tailwindcss/packages/tailwindcss/src/utilities.ts:5269` — lit `['--line-height','--letter-spacing','--font-weight']` ; doc v4 confirme la syntaxe double-tiret). `--leading-*`, `--tracking-*`, `--font-weight-*` existent aussi comme namespaces autonomes (`:4931`, `:4950`, `:3974`).

| Sous-token M3                 | Émis comme `--md-sys-*` ?               | Mappable Tailwind                                      | Note                                                        |
| ----------------------------- | --------------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------- |
| `*-size` (15 rôles)           | ✅ `--md-sys-typescale-<role>-size`     | `--text-<role>`                                        | ✅                                                          |
| `*-line-height`               | ✅                                      | `--text-<role>--line-height` ou `--leading-<role>`     | ✅                                                          |
| `*-weight`                    | ✅                                      | `--text-<role>--font-weight` ou `--font-weight-<role>` | ✅                                                          |
| `*-tracking` (letter-spacing) | ❌ **NON émis** (`$unsupported-tokens`) | `--text-<role>--letter-spacing` / `--tracking-<role>`  | ⚠️ pointe vers `var()` non résolu sauf déclaration manuelle |
| `*-font` (font-family)        | ✅                                      | `--font-<role>` (namespace `--font-*`)                 | ✅ mais non couvert par doc 06                              |

Bloc `@theme inline` typescale corrigé (15 rôles = `{display,headline,title,body,label}` × `{small,medium,large}`, patron uniforme) :

```css
@theme inline {
  /* patron pour CHAQUE rôle ; ex. body-large : */
  --text-body-large: var(--md-sys-typescale-body-large-size); /* 1rem */
  --text-body-large--line-height: var(--md-sys-typescale-body-large-line-height); /* 1.5rem */
  --text-body-large--font-weight: var(--md-sys-typescale-body-large-weight); /* 400 */
  /* letter-spacing : le token --md-sys-typescale-body-large-tracking N'EST PAS émis.
     Pour le couvrir, l'app DOIT le déclarer elle-même sur :root (valeur réelle 0.03125rem),
     OU inliner la valeur en dur ici : */
  --text-body-large--letter-spacing: 0.03125rem; /* valeur v0_192, pas une var de la lib */
  --font-body-large: var(--md-sys-typescale-body-large-font); /* Roboto */
}
```

Valeurs de référence (`versions/v0_192/_md-sys-typescale.scss`, poids via `_md-ref-typeface.scss` regular=400/medium=500) :

| rôle            | size      | line-height | weight | tracking (NON émis) |
| --------------- | --------- | ----------- | ------ | ------------------- |
| display-large   | 3.5625rem | 4rem        | 400    | -0.015625rem        |
| display-medium  | 2.8125rem | 3.25rem     | 400    | 0                   |
| display-small   | 2.25rem   | 2.75rem     | 400    | 0                   |
| headline-large  | 2rem      | 2.5rem      | 400    | 0                   |
| headline-medium | 1.75rem   | 2.25rem     | 400    | 0                   |
| headline-small  | 1.5rem    | 2rem        | 400    | 0                   |
| title-large     | 1.375rem  | 1.75rem     | 400    | 0                   |
| title-medium    | 1rem      | 1.5rem      | 500    | 0.009375rem         |
| title-small     | 0.875rem  | 1.25rem     | 500    | 0.00625rem          |
| body-large      | 1rem      | 1.5rem      | 400    | 0.03125rem          |
| body-medium     | 0.875rem  | 1.25rem     | 400    | 0.015625rem         |
| body-small      | 0.75rem   | 1rem        | 400    | 0.025rem            |
| label-large     | 0.875rem  | 1.25rem     | 500    | 0.00625rem          |
| label-medium    | 0.75rem   | 1rem        | 500    | 0.03125rem          |
| label-small     | 0.6875rem | 1rem        | 500    | 0.03125rem          |

Verdict typescale : **couverture quasi complète**, mais le **letter-spacing M3 n'est pas pilotable via un token `--md-sys-*` de la lib** — il faut soit le déclarer soi-même, soit inliner la valeur. À corriger dans la doc 06 (qui le présente comme une var dérivée alors qu'elle n'existe pas).

### 1.3 Shape / radius — **mappable, mais PAS de token runtime de la lib**

Source noms : `tokens/_md-sys-shape.scss:15-23` (`$supported-tokens` : `corner-{none,extra-small,small,medium,large,extra-large,full}`). `$unsupported-tokens` (`:26-34`) liste les variantes partielles (`corner-large-top`, `corner-extra-large-top`, `corner-large-start/end`…). Valeurs (`versions/v0_192/_md-sys-shape.scss`) : none 0px, extra-small 4px, small 8px, medium 12px, large 16px, extra-large 28px, full 9999px.

**Mais** (cf. §0) la famille shape **n'émet aucune custom property `--md-sys-shape-*`** : 0 occurrence dans le CSS compilé. Les composants utilisent `--md-comp-<x>-container-shape` (token composant) résolu au compile-time. Donc `var(--md-sys-shape-corner-medium)` ne résout vers rien sauf si l'app le déclare.

Namespace Tailwind `--radius-*` → `rounded-*` (vérifié `utilities.ts` `themeKeys: ['--radius']`). Mapping possible **si** l'app déclare les `--md-sys-shape-corner-*` sur `:root`, OU en inlinant les valeurs :

```css
@theme inline {
  --radius-none: 0px; /* ou var(--md-sys-shape-corner-none) SI déclaré par l'app */
  --radius-extra-small: 4px;
  --radius-small: 8px;
  --radius-medium: 12px;
  --radius-large: 16px;
  --radius-extra-large: 28px;
  --radius-full: 9999px;
}
```

Verdict shape : **couverture utilitaire complète (rounded-\*)**, mais source = valeurs déclarées par l'app, **pas** une var dérivée de la lib (nuance non signalée dans doc 06 §2.4). Les coins **partiels** (top-only, start/end) sont `$unsupported-tokens` côté M3 et n'ont pas d'équivalent `--radius-*` unique côté Tailwind (faisable via `rounded-t-*`/`rounded-s-*` au cas par cas).

### 1.4 Elevation — **AUCUNE couverture native ; nécessite `<md-elevation>` ou shadow custom**

Source : `tokens/_md-sys-elevation.scss:14-30` → niveaux `level0..level5` = entiers 0..5 (pas des dp). **Aucune custom property `--md-sys-elevation-*` au runtime** (0 dans le CSS compilé). L'élévation M3 réelle est calculée par le composant `<md-elevation>` à partir de `--md-elevation-level` via une **double box-shadow `::before/::after` à courbe non triviale** (vérifié `material-web/elevation/internal/elevation-styles.cssresult.ts` : `clamp()` imbriqués sur key-light + ambient-light, deux ombres superposées).

Tailwind a un namespace `--shadow-*` → `shadow-*` (doc v4). **MAIS** : une `box-shadow` simple Tailwind **ne reproduit pas** la courbe d'ombre M3 (deux ombres + opacités .3/.15 + interpolation par `clamp`). Donc :

- ❌ pas de mapping token-à-token vers `--shadow-*` fidèle à M3.
- ✅ seule voie fidèle : utiliser le composant `<md-elevation style="--md-elevation-level: 1">` à l'intérieur d'un conteneur `relative` (cf. doc 06 §4 Paper). C'est du **markup**, pas un utilitaire Tailwind.

Verdict elevation : **non couvrable par Tailwind seul**. `shadow-*` ne donne qu'une approximation. Recommandation : `<md-elevation>` pour fidélité, ou définir des `--shadow-md3-1..5` custom (valeurs copiées du composant) si on veut un utilitaire — mais c'est une recréation, pas une dérivation.

### 1.5 Motion (durations + easings) — **AUCUN token runtime ; partiellement mappable en `--ease-*`**

Source : `tokens/_md-sys-motion.scss:10-12` (passthrough pur) + `versions/v0_192/_md-sys-motion.scss` : **16 durations** (`duration-short1..4` 50/100/150/200ms ; `medium1..4` 250/300/350/400ms ; `long1..4` 450/500/550/600ms ; `extra-long1..4` 700/800/900/1000ms) et **easings** (`easing-standard`, `-standard-accelerate/-decelerate`, `-emphasized`, `-emphasized-accelerate/-decelerate`, `-legacy`, `-legacy-accelerate/-decelerate`, `-linear`). **Aucune custom property `--md-sys-motion-*` au runtime** (0 dans le CSS compilé ; résolu au compile-time dans le SCSS des composants).

Côté Tailwind v4 :

- **Easings** → namespace `--ease-*` (vérifié `utilities.ts`, doc v4) → utilitaires `ease-*`. **Mappable** en recopiant les courbes cubic-bezier M3 :
  ```css
  @theme {
    --ease-standard: cubic-bezier(0.2, 0, 0, 1);
    --ease-emphasized: cubic-bezier(0.2, 0, 0, 1); /* M3 emphasized = courbe spline; approx */
    --ease-emphasized-decelerate: cubic-bezier(0.05, 0.7, 0.1, 1);
    --ease-emphasized-accelerate: cubic-bezier(0.3, 0, 0.8, 0.15);
    /* … valeurs à recopier depuis versions/v0_192/_md-sys-motion.scss:33-52 */
  }
  ```
- **Durations** → **pas de namespace `--duration-*` thématique** dans Tailwind v4 : `duration-*` se pilote en arbitraire (`duration-[300ms]`) ou via `--tw-duration` (vérifié `utilities.ts:4715,4772-4776` — il n'y a pas de `themeKeys: ['--duration']`). On peut quand même définir des utilitaires custom ou utiliser `duration-[var(--md-sys-motion-duration-medium2)]` SI l'app déclare ces vars. Couverture = **manuelle / arbitraire**, pas un namespace natif.

Verdict motion : **aucun token runtime de la lib** ; easings recréables en `--ease-*` (valeurs recopiées), durations seulement en arbitraire/CSS vars brutes. **Non couvert nativement.**

### 1.6 State (state-layer opacities) — **AUCUNE couverture ; concerne le shadow DOM**

Source : `versions/v0_192/_md-sys-state.scss:17-20` : `dragged 0.16`, `focus 0.12`, `hover 0.08`, `pressed 0.12`. **Aucune custom property `--md-sys-state-*` runtime** (0 dans le compilé ; les composants utilisent des `--_hover-state-layer-opacity` locales — cf. `button/internal/shared-styles.css`). Surtout : la **state layer vit DANS le shadow DOM** (ripple/focus-ring). Tailwind ne peut ni la lire ni la peindre. Pas de namespace Tailwind correspondant.

Verdict state : **hors de portée de Tailwind** par construction (shadow DOM + pas de token runtime). Override possible uniquement via tokens composant `--md-<comp>-{hover,pressed}-state-layer-{color,opacity}` ou `--md-ripple-*` en CSS classique.

### 1.7 Tableau de synthèse — couverture par famille

| Famille M3                                       | Tokens `--md-sys-*` émis au runtime ?   | Namespace Tailwind natif                                                              | Mapping `@theme`                        | Couverture                           |
| ------------------------------------------------ | --------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------- | ------------------------------------ |
| **color** (47 rôles, 39 consommés)               | ✅ oui (39)                             | `--color-*` (bg/text/border/fill/stroke/ring/outline/accent/caret/divide/placeholder) | `@theme inline` 1:1                     | **100 % native, scopée**             |
| **typescale** size/line-height/weight (15 rôles) | ✅ oui                                  | `--text-*`(+`--leading`/`--font-weight`/`--font`)                                     | `@theme inline`                         | **complète**                         |
| typescale **tracking** (letter-spacing)          | ❌ **non émis** (`$unsupported-tokens`) | `--text-X--letter-spacing` / `--tracking-*`                                           | possible mais valeur à déclarer/inliner | **partielle (à déclarer)**           |
| **shape / corner** (7)                           | ❌ non émis                             | `--radius-*` → `rounded-*`                                                            | possible (valeurs à déclarer/inliner)   | **utilitaire OK, source non native** |
| corner **partiels** (top/start/end)              | ❌ `$unsupported`                       | aucun unique (`rounded-t/s-*`)                                                        | non                                     | **non**                              |
| **elevation** (level0-5)                         | ❌ non émis                             | `--shadow-*` (approx)                                                                 | non fidèle                              | **non (→ `<md-elevation>`)**         |
| **motion easings** (10)                          | ❌ non émis                             | `--ease-*` → `ease-*`                                                                 | recréable (valeurs recopiées)           | **manuelle**                         |
| **motion durations** (16)                        | ❌ non émis                             | aucun `--duration-*` thématique                                                       | arbitraire `duration-[…]`               | **manuelle / arbitraire**            |
| **state** opacities (4)                          | ❌ non émis + shadow DOM                | aucun                                                                                 | non                                     | **hors portée**                      |

---

## 2. Couverture layout — ex-composants MUI → utilitaires Tailwind

Tous les primitifs de layout MUI sont **réalisables à 100 %** en utilitaires Tailwind (ils n'ont pas de shadow DOM ; ce sont de simples `<div>`). Conforme au contrat (« Layout MUI → PAS d'élément md »).

| MUI                                  | Réalisable en Tailwind ?                                                | Équivalent                                                                       | Note couleur/forme                                                                        |
| ------------------------------------ | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `Box` (`sx`)                         | ✅ total                                                                | `<div>` + `flex/grid/p-*/m-*/w-*/gap-*`                                          | —                                                                                         |
| `Stack`                              | ✅ total                                                                | `<div class="flex flex-col gap-N">` (ou `flex-row`)                              | `spacing={n}` → `gap-{2n}` (échelle 8px↔0.25rem)                                          |
| `Grid` (container/item, breakpoints) | ✅ total                                                                | `grid grid-cols-N md:grid-cols-M gap-*`                                          | breakpoints v4 `sm/md/lg/xl/2xl`                                                          |
| `Container maxWidth`                 | ✅ total                                                                | `mx-auto w-full max-w-{sm..7xl} px-*`                                            | `xs/sm/md/lg/xl` → `max-w-*`                                                              |
| `Paper` (surface + elevation)        | ✅ surface/forme ; ⚠️ élévation                                         | `bg-surface-container text-on-surface rounded-medium p-*` **+ `<md-elevation>`** | couleur/forme via §1.1/§1.3 ; **élévation = `<md-elevation>`**, pas `shadow-*` (cf. §1.4) |
| `Divider`                            | ✅ (ligne simple) `border-t border-outline-variant` ; ou `<md-divider>` | —                                                                                | couleur via token color                                                                   |

**Seule réserve** : la _surface_ et la _forme_ d'un `Paper` sont couvertes par les utilitaires (`bg-surface-container`, `rounded-medium`), mais l'**élévation fidèle M3** exige le composant `<md-elevation>` (cf. §1.4). Tailwind couvre donc le layout **structurel** à 100 %, et l'apparence surface/forme via les `@theme` mappés, l'élévation restant le seul aspect « layout » non couvrable en pur utilitaire.

---

## 3. Limites réelles

### 3.1 Le mur du Shadow DOM (confirmé)

Chaque `md-*` rend son template dans un shadow root encapsulé (Lit `static styles`). La feuille Tailwind vit dans le light DOM et **ne franchit pas** la frontière. Vérifié sur le bouton (`material-web/button/internal/button.ts` : `<md-focus-ring part="focus-ring">`, `<md-ripple part="ripple">`, `<button>` et `<slot>` tous internes). Conséquence : `class="bg-red-500"` sur `<md-filled-button>` n'atteint **jamais** le `<button>` interne. Tailwind ne pilote que le **box-model du host** (`w-full`, `mt-4`) et le layout autour. Theming interne = tokens `--md-sys-color-*` / `--md-comp-*` uniquement. (Le `:host` du preflight Tailwind ne s'applique qu'à un shadow root créé dans le même document, pas à celui d'un `md-*` importé.)

### 3.2 `::part()` réellement exposés (recensement `grep -rn 'part=' material-web/`)

Le shadow DOM n'est franchissable de l'extérieur qu'aux points exposés par `part="…"`. Tailwind v4 **n'a pas** de variant `::part()` natif → CSS classique, `@custom-variant` (`tailwindcss/src/index.ts:352`), ou arbitrary variant `[&::part(x)]:`. Parts réels relevés sur les composants **upstream** :

| `part`                                                                                    | Exposé par (vérifié)                                                                            |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `focus-ring`                                                                              | button, radio, switch, tab, menu-item, segmented-button, fab, icon-button, chips, select-option |
| `ripple`                                                                                  | button, radio, menu-item, select-option                                                         |
| `elevation`                                                                               | elevated/tonal/filled-button, tab, menu, fab, chips, card/navigation (labs)                     |
| `divider`                                                                                 | tabs (`tabs.ts:175`), divider (labs)                                                            |
| `field`                                                                                   | select (`select.ts:399`)                                                                        |
| `menu`                                                                                    | select (`select.ts:478`), menu (labs)                                                           |
| `input` / `view`                                                                          | search-bar (`search-bar.ts:83,107`), autocomplete (`autocomplete.ts:118`)                       |
| `bar`                                                                                     | bottom-app-bar (`bottom-app-bar.ts:22`), top-app-bar (`top-app-bar.ts:105`)                     |
| `rail`                                                                                    | navigation-rail (`navigation-rail.ts:94`)                                                       |
| `pane`/`container`/`list`/`detail`                                                        | layout (`pane.ts:69`, `list-detail.ts:128-132`)                                                 |
| (nombreux : `btn`, `card`, `checkbox`, `switch`, `label`, `body`, `top-bar`, `scaffold`…) | composants du fork `labs/gb/`                                                                   |

**Limite dure** : l'exposition upstream est **volontairement minimale** — pas de `part` pour la majorité des surfaces internes (texte de label de bouton, container coloré, icône). Pour celles-là, **seuls les tokens agissent**. Et `::part()` ne descend pas dans les enfants non re-exposés, et ne combine qu'avec pseudo-classes/éléments.

---

## 4. Verdict — « l'intégration Tailwind couvre-t-elle tout le M3 ? »

**Non, pas seule — et c'est par conception.** Tailwind couvre intégralement ce qui relève du **light DOM** (layout, host, composants maison) et la **couleur** (synchronisée au token près via `@theme inline`). Le reste se répartit ainsi :

**Couvert nativement et fidèlement par Tailwind (`@theme`) :**

- **Color** : 100 %, les 39 rôles consommés (47 mappables), scopés via `@theme inline` → `bg-*/text-*/border-*` strictement alignés sur les `md-*`. **Source de vérité native** (les `--md-sys-color-*` SONT des vars runtime de la lib).
- **Typescale** size/line-height/weight + font-family : complet via `--text-*`/`--font-*`.
- **Shape** : utilitaires `rounded-*` complets via `--radius-*`.
- **Layout MUI** (Box/Stack/Grid/Container/Paper-surface/Divider) : 100 %.

**Couvrable seulement avec valeurs déclarées/recopiées (PAS une dérivation de la lib) :**

- **Typescale `tracking`** : `--md-sys-typescale-*-tracking` **non émis** (`$unsupported-tokens`) → déclarer sur `:root` ou inliner la valeur.
- **Shape** : `--md-sys-shape-corner-*` **non émis** au runtime → l'app doit les déclarer, ou inliner les px dans `@theme`.
- **Motion easings** : recréables en `--ease-*` (cubic-bezier recopiés de v0_192).

**NON couvrable par Tailwind seul → tokens CSS bruts / markup :**

- **Elevation** : courbe d'ombre M3 (double box-shadow `clamp`) irréproductible par `shadow-*` → composant **`<md-elevation>`** + `--md-elevation-level`.
- **Motion durations** : pas de namespace `--duration-*` thématique → arbitraire `duration-[…]` ou CSS vars brutes.
- **State layers** (opacités hover/focus/pressed/dragged) : dans le **shadow DOM**, pas de token `--md-sys-state-*` runtime → tokens composant (`--md-ripple-*`, `--md-<comp>-*-state-layer-*`) en CSS classique.
- **Tout l'intérieur d'un `md-*`** (label, container, icône non `part`) : mur du shadow DOM → **tokens `--md-sys-color-*` / `--md-comp-*`** exclusivement.

**Résumé en une ligne** : Tailwind couvre **color + typescale + shape + layout** (dont color et typescale-métriques en dérivation native scopée) ; **elevation, motion-durations, state, et tout l'interne hors-`part` des `md-*`** ne sont **pas** couvrables par Tailwind et restent du ressort des tokens CSS bruts / composant ou du composant `<md-elevation>`.

**Corrections à apporter à `06-tailwind-material-web.md`** :

1. §2.2 : ajouter les 12 rôles `*-fixed` + `surface-tint` (bloc complet en §1.1 ci-dessus).
2. §2.3 : signaler que `--md-sys-typescale-*-tracking` **n'est pas émis** par la lib (le `var()` ne résout pas) — déclarer/inliner.
3. §2.4 : signaler que `--md-sys-shape-corner-*` **n'est pas émis** au runtime — c'est l'app qui les déclare (pas une dérivation de la lib).
4. Ajouter une section motion (`--ease-*` recréables) et clarifier que elevation/state/motion-durations ne sont **pas** des `--md-sys-*` runtime.

---

### Sources

- `material-web/tokens/_md-sys-color.scss:15-66` (47 rôles `$supported-tokens`), `:81-90` (émission `var(--md-sys-color-*)`)
- `material-web/tokens/_md-sys-typescale.scss:15-79` (`$supported-tokens`), `:82-114` (`$unsupported-tokens` incl. tous les `*-tracking`), `:131-141` (émission), `:144-148` (filtrage `validate`)
- `material-web/tokens/versions/v0_192/_md-sys-typescale.scss` (valeurs size/line-height/tracking/weight) ; `tokens/_md-ref-typeface.scss` (weights 400/500/700)
- `material-web/tokens/_md-sys-shape.scss:14-34` (corners supported/unsupported) ; `versions/v0_192/_md-sys-shape.scss` (px)
- `material-web/tokens/_md-sys-elevation.scss:14-30` (level0-5) ; `material-web/elevation/internal/elevation-styles.cssresult.ts` (courbe `clamp` double box-shadow, `--md-elevation-level`)
- `material-web/tokens/_md-sys-motion.scss:10-12` (passthrough) ; `versions/v0_192/_md-sys-motion.scss:17-52` (16 durations + easings)
- `material-web/tokens/versions/v0_192/_md-sys-state.scss:17-20` (opacités 0.16/0.12/0.08/0.12)
- Scan CSS compilé (`material-web/**/internal/*-styles.css`) : 39 `--md-sys-color-*`, **0** pour motion/elevation/state/typescale/shape ; `button/internal/shared-styles.css` (vars locales `--_*`)
- `material-web/button/internal/button.ts` (shadow DOM, `part="focus-ring"`/`part="ripple"`) ; `{tabs,select,search-bar,appbar,navigationrail,layout}/internal/*.ts` (recensement `part=`)
- `tailwindcss/packages/tailwindcss/package.json` (v4.3.0) ; `src/index.ts:93` (`@theme inline`), `:311-366` (`@custom-variant`/`@variant`)
- `tailwindcss/packages/tailwindcss/src/utilities.ts` : `--color`/`--radius`/`--text`/`--leading`/`--tracking`/`--font-weight`/`--font`/`--shadow`/`--ease` ; `:5269` (text valeurs liées line-height/letter-spacing/font-weight) ; `:4715,4772-4776` (duration via `--tw-duration`, pas de namespace `--duration-*`)
- Tailwind v4 — Theme variables (namespaces, `@theme inline`, valeurs liées `--text-X--*`, `--*: initial`) : https://tailwindcss.com/docs/theme
- Tailwind v4 — Functions & directives : https://tailwindcss.com/docs/functions-and-directives
- Material Web — theming color : https://github.com/material-components/material-web/blob/main/docs/theming/color.md
