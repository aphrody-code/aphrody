<!-- Livrable 06 du kit de migration MUI → material-web. Voir 00-CONVENTIONS.md §6 (contrat). -->

# Intégration native Tailwind ⇄ material-web

Ce document décrit comment faire cohabiter **Tailwind CSS v4** (`tailwindcss/` — `tailwindcss@4.3.0`) avec **`@material/web`** (fork aphrody, `material-web/` — `@material/web@2.4.1`, web components Lit) de façon _native_ : Tailwind prend en charge le **layout / host / composants non-`md`**, les composants `md-*` restent thémés par les tokens `--md-sys-*`, et on n'utilise **qu'une seule source de vérité couleurs** partagée entre les deux mondes. Tout le code ci-dessous est vérifié sur les deux repos locaux et sur la doc Tailwind v4.

---

## 1. Le mur du Shadow DOM (fait fondateur)

### 1.1 Pourquoi les classes Tailwind ne pénètrent pas dans un `md-*`

Chaque composant `@material/web` est un custom element Lit qui rend son template dans un **shadow root encapsulé**. La stylesheet d'une feuille de styles globale (donc tout l'output Tailwind, qui vit dans le _light DOM_) **ne franchit pas** la frontière du shadow DOM — c'est la garantie d'encapsulation du standard. Les styles internes des composants sont injectés _dans_ le shadow root via les `static styles = css\`…\`` de Lit.

On le voit directement dans le code source : les composants déclarent un shadow root et rendent leur structure interne dedans. Exemple sur le bouton :

```ts
// material-web/button/internal/button.ts:108-114
return html`
  <button …>
    <md-focus-ring part="focus-ring" for=${buttonId}></md-focus-ring>
    <md-ripple
      part="ripple"
      …
```

Ces `<md-focus-ring>`, `<md-ripple>`, le `<button>` natif et la `<slot>` vivent **dans le shadow DOM** du `md-*`. Une classe utilitaire posée sur le host (`<md-filled-button class="bg-red-500">`) ne peut ni atteindre ce `<button>` ni repeindre le ripple.

> Important : Tailwind v4 cible bien `:host` dans son reset (`tailwindcss/packages/tailwindcss/preflight.css` — `html, :host { … }`), **mais** un custom element importé en tant que dépendance définit son propre shadow root via Lit ; la feuille Tailwind du document hôte n'y est jamais injectée. Le `:host` du preflight ne s'applique qu'à un shadow root **que vous créeriez vous-même** dans le même document, pas à celui, déjà fermé sur ses styles, d'un `md-*`.

### 1.2 Démonstration

```html
<!-- ❌ NE FAIT RIEN sur l'intérieur du bouton.
     `bg-red-500`/`text-white`/`rounded-full` n'atteignent PAS le <button> interne. -->
<md-filled-button class="bg-red-500 text-white rounded-full"> Envoyer </md-filled-button>
```

```css
/* Ce que Tailwind génère réellement (light DOM) — bloqué par le shadow boundary : */
.bg-red-500 {
  background-color: var(--color-red-500);
} /* posé sur le host, jamais sur le <button> interne */
```

```html
<!-- ✅ FONCTIONNE : Tailwind sur le host = layout/box-model du host, pas le styling interne. -->
<md-filled-button class="w-full mt-4">Envoyer</md-filled-button>
<!-- ✅ Le styling INTERNE passe par les tokens, en CSS classique : -->
<style>
  md-filled-button {
    --md-sys-color-primary: #b3261e; /* repeint réellement le fond du bouton */
    --md-filled-button-container-shape: 9999px; /* arrondi via token composant */
  }
</style>
```

### 1.3 Conséquence — partage net des responsabilités

| Couche                                                     | Outil                                      | Exemple                                 |
| ---------------------------------------------------------- | ------------------------------------------ | --------------------------------------- |
| Layout autour des composants (flex/grid/gap/margin/width…) | **Tailwind** (host + `<div>`)              | `class="flex flex-col gap-4 p-6"`       |
| Box-model du _host_ d'un `md-*`                            | **Tailwind** (sur le host)                 | `<md-filled-button class="w-full">`     |
| Composants non-`md` (cartes maison, hero, sidebar…)        | **Tailwind**                               | `<div class="rounded-xl bg-surface …">` |
| Styling **interne** d'un `md-*` (couleur, forme, état)     | **Tokens `--md-sys-*`** / tokens composant | `--md-sys-color-primary: …`             |
| Retouche ciblée d'une sous-partie exposée d'un `md-*`      | **`::part()`** (CSS classique)             | `md-tabs::part(divider) { … }`          |

Règle d'or (reprise du contrat §5/§6) : **Tailwind ne thème jamais l'intérieur d'un `md-*`. Le theming interne passe exclusivement par `--md-sys-*`.**

---

## 2. Source de vérité unique des couleurs (`@theme inline`)

Objectif : que `bg-primary`, `text-on-surface`, `border-outline-variant` côté Tailwind correspondent **exactement** aux couleurs rendues par les composants `md-*`. La seule façon propre est de faire **dériver le `@theme` de Tailwind des tokens `--md-sys-*`** — les tokens M3 sont la source, Tailwind n'est qu'un consommateur.

### 2.1 Pourquoi `@theme inline` est obligatoire ici

Tailwind v4.3 supporte `@theme` et son option `inline` (vérifié : `tailwindcss/packages/tailwindcss/src/index.ts:93` — `else if (option === 'inline')`). La différence est décisive quand on référence une _autre_ variable CSS :

- `@theme { --color-primary: var(--md-sys-color-primary); }` → l'utilitaire génère `background-color: var(--color-primary)`. Comme `--color-primary` vaut `var(--md-sys-color-primary)`, la résolution se fait **au scope où `--color-primary` est défini** (`:root`), pas au scope de l'élément. Si vous redéfinissez `--md-sys-color-primary` localement (ex. sur une sous-arbre en thème sombre), `bg-primary` **ne suit pas**.
- `@theme inline { --color-primary: var(--md-sys-color-primary); }` → l'utilitaire **inline la valeur** : `background-color: var(--md-sys-color-primary)`. La résolution se fait au scope de **l'élément stylé** → `bg-primary` suit toute redéfinition locale de `--md-sys-color-primary`, exactement comme les `md-*`.

> Doc Tailwind v4 : « Using the `inline` option, the utility class will use the theme variable _value_ instead of referencing the actual theme variable. » C'est précisément le comportement voulu pour rester synchrone avec les overrides de tokens M3 (thème par sous-arbre, dark mode scoped, theming par composant).

### 2.2 Le bloc `@theme inline` complet (couleurs M3)

Noms de tokens couleur vérifiés dans `material-web/tokens/_md-sys-color.scss:15-66` (`$supported-tokens` = **47 rôles**, incluant les 12 `*-fixed` et `surface-tint`). Mapping 1:1 vers le namespace `--color-*` de Tailwind (vérifié : `tailwindcss/packages/tailwindcss/src/utilities.ts:2243` — `themeKeys: ['--color']`, qui alimente `bg-*`, `text-*`, `border-*`, `fill-*`, `stroke-*`, `ring-*`…). Contrairement aux autres familles (typescale-tracking, shape, motion, elevation, state), **les `--md-sys-color-*` SONT de vraies vars CSS émises au runtime par la lib** — c'est la seule famille en dérivation native (les composants en consomment 39 ; les 47 restent mappables et inertes si non définis).

```css
/* app.css — chargé APRÈS @import "tailwindcss" */
@theme inline {
  /* ----- M3 color roles → Tailwind color namespace -----
     Source de vérité = --md-sys-color-* (définis en §5.3).
     `inline` => bg-primary etc. résolvent au scope de l'élément, comme les md-*. */
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
  /* ----- fixed (accent stables clair/sombre) ----- */
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
  /* ----- background / surfaces ----- */
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
  --color-surface-tint: var(--md-sys-color-surface-tint);
  --color-inverse-surface: var(--md-sys-color-inverse-surface);
  --color-inverse-on-surface: var(--md-sys-color-inverse-on-surface);
  --color-inverse-primary: var(--md-sys-color-inverse-primary);
  --color-outline: var(--md-sys-color-outline);
  --color-outline-variant: var(--md-sys-color-outline-variant);
  --color-scrim: var(--md-sys-color-scrim);
  --color-shadow: var(--md-sys-color-shadow);
}
```

Résultat : `bg-primary text-on-primary`, `bg-surface-container text-on-surface`, `border-outline-variant`, `bg-error text-on-error`, `text-on-surface-variant`, etc. sont tous disponibles et **strictement alignés** sur les composants `md-*`.

> Reset utile : par défaut Tailwind v4 conserve sa palette complète (`--color-red-500`…, cf. `tailwindcss/packages/tailwindcss/theme.css`). Pour forcer une palette 100 % M3 et éviter `bg-blue-500`, ajoutez `@theme { --color-*: initial; }` **avant** le bloc M3 (efface le namespace couleur, puis vous le re-remplissez).

### 2.3 Typescale M3 (`--text-*`, `--leading-*`, `--tracking-*`, `--font-weight-*`)

Noms vérifiés dans `material-web/tokens/_md-sys-typescale.scss` ; valeurs réelles dans `material-web/tokens/versions/v0_192/_md-sys-typescale.scss`. Namespaces Tailwind vérifiés : `--text-*` (taille, `utilities.ts:5268`), `--leading-*` (`utilities.ts:4931`), `--tracking-*` (`utilities.ts:4950`), `--font-weight-*` (`utilities.ts:3974`).

> ⚠️ **Couverture partielle — le `tracking` (letter-spacing) N'EST PAS émis par la lib.** `_md-sys-typescale.scss` range tous les `*-tracking` dans `$unsupported-tokens` (lignes `:82-114`), filtrés à l'émission (`validate.values(... $unsupported-tokens)`, `:144-148`). Seuls `*-size`, `*-line-height`, `*-weight` (et `*-font`) sortent comme `var(--md-sys-typescale-*)` au runtime. Donc `var(--md-sys-typescale-body-large-tracking)` **ne résout vers RIEN** par défaut. Deux remèdes : (1) importer `@aphrody/m3-tokens/m3-tokens.css` qui déclare ces 15 vars sur `:root`, ou (2) inliner la valeur réelle en dur dans le `@theme`.

Un nom de taille Tailwind (`--text-body-large`) peut embarquer ses `line-height`/`letter-spacing`/`font-weight` par défaut via la syntaxe à valeurs liées de v4 :

```css
@theme inline {
  /* size --text-<role> ; defaults: --line-height + --letter-spacing + --font-weight liés.
     size/line-height/weight = vars runtime natives de la lib. */
  --text-display-large: var(--md-sys-typescale-display-large-size); /* 3.5625rem */
  --text-display-large--line-height: var(--md-sys-typescale-display-large-line-height);
  --text-display-large--font-weight: var(--md-sys-typescale-display-large-weight);

  --text-headline-small: var(--md-sys-typescale-headline-small-size); /* 1.5rem */
  --text-headline-small--line-height: var(--md-sys-typescale-headline-small-line-height);

  --text-title-medium: var(--md-sys-typescale-title-medium-size); /* 1rem */
  --text-title-medium--line-height: var(--md-sys-typescale-title-medium-line-height); /* 1.5rem */
  /* letter-spacing : --md-sys-typescale-*-tracking N'EST PAS émis par la lib.
     Résout uniquement si @aphrody/m3-tokens/m3-tokens.css est importé, sinon inliner : */
  --text-title-medium--letter-spacing: var(
    --md-sys-typescale-title-medium-tracking
  ); /* 0.009375rem — via m3-tokens.css */

  --text-body-large: var(--md-sys-typescale-body-large-size); /* 1rem */
  --text-body-large--line-height: var(--md-sys-typescale-body-large-line-height); /* 1.5rem */
  --text-body-large--letter-spacing: 0.03125rem; /* inliné en dur (valeur v0_192), PAS une var de la lib */

  --text-label-large: var(--md-sys-typescale-label-large-size); /* 0.875rem */
  --text-label-large--line-height: var(--md-sys-typescale-label-large-line-height); /* 1.25rem */
  --text-label-large--font-weight: var(--md-sys-typescale-label-large-weight); /* 500 */
}
```

Usage : `<h1 class="text-display-large">`, `<p class="text-body-large">`, `<span class="text-label-large">`. (Les rôles complets — `*-small|medium|large` × `display|headline|title|body|label` — se déclinent sur le même patron.)

### 2.4 Shape → radius (`--radius-*`)

Noms vérifiés dans `material-web/tokens/_md-sys-shape.scss:15-23` ; valeurs dans `material-web/tokens/versions/v0_192/_md-sys-shape.scss` (`none 0px`, `extra-small 4px`, `small 8px`, `medium 12px`, `large 16px`, `extra-large 28px`, `full 9999px`). Namespace `--radius-*` → utilitaires `rounded-*` (vérifié : `tailwindcss/packages/tailwindcss/src/utilities.ts:2339` — `themeKeys: ['--radius']`).

> ⚠️ **`--md-sys-shape-corner-*` N'EST PAS émis au runtime par la lib.** La famille shape est résolue au compile-time Sass : les composants consomment des tokens composant (`--md-comp-<x>-container-shape`), pas de `--md-sys-shape-*` dans le DOM (0 occurrence dans le CSS compilé). Donc `var(--md-sys-shape-corner-medium)` ci-dessous **ne résout vers rien** tant que l'app ne déclare pas ces vars elle-même. Deux options : (1) importer `@aphrody/m3-tokens/m3-tokens.css` (déclare les 7 corners sur `:root`), ou (2) inliner directement les px. Ce n'est PAS une dérivation native de la lib, c'est un jeu de vars parallèle.

```css
@theme inline {
  /* Résout uniquement si @aphrody/m3-tokens/m3-tokens.css est importé.
     Sinon, remplacer chaque var() par sa valeur px en commentaire. */
  --radius-none: var(--md-sys-shape-corner-none); /* 0px */
  --radius-extra-small: var(--md-sys-shape-corner-extra-small); /* 4px */
  --radius-small: var(--md-sys-shape-corner-small); /* 8px */
  --radius-medium: var(--md-sys-shape-corner-medium); /* 12px */
  --radius-large: var(--md-sys-shape-corner-large); /* 16px */
  --radius-extra-large: var(--md-sys-shape-corner-extra-large); /* 28px */
  --radius-full: var(--md-sys-shape-corner-full); /* 9999px */
}
```

Usage : `rounded-large`, `rounded-extra-large`, `rounded-full`. Pour `Paper` MUI (voir §4) → `<div class="bg-surface-container rounded-medium">`. Les coins **partiels** (`corner-large-top`, `corner-large-start/end`…) sont `$unsupported-tokens` côté M3 et n'ont pas d'équivalent `--radius-*` unique → composer au cas par cas avec `rounded-t-*`/`rounded-s-*`.

### 2.5 Motion, elevation, state — familles SANS var runtime (`@aphrody/m3-tokens/m3-tokens.css`)

Trois familles M3 supplémentaires (motion, elevation, state) ne sont **émises par AUCUNE `--md-sys-*` au runtime** : comme shape et typescale-tracking, elles sont résolues au compile-time Sass dans les composants (motion = passthrough, elevation = calculée par `<md-elevation>`, state = vars locales `--_*` dans le shadow DOM). Pour les rendre mappables en `@theme`, le package fork fournit un **asset CSS prêt à importer** qui les déclare comme vraies vars sur `:root`, avec les valeurs réelles de `versions/v0_192/_md-sys-*.scss` :

```css
/* app.css — APRÈS @import "tailwindcss" */
@import "@aphrody/m3-tokens/m3-tokens.css"; /* déclare shape(7) + tracking(15) + motion-duration(16) + motion-easing(10) + elevation(6) + state(4) sur :root */
```

Une fois importé, le `@theme` peut consommer ces familles. Le fichier embarque un bloc `@theme inline` d'exemple complet en commentaire.

**Motion — easings → `ease-*` (mappable), durations → arbitraire.**

```css
@theme inline {
  --ease-standard: var(--md-sys-motion-easing-standard); /* cubic-bezier(0.2,0,0,1) */
  --ease-emphasized: var(--md-sys-motion-easing-emphasized); /* cubic-bezier(0.2,0,0,1) */
  --ease-emphasized-decelerate: var(
    --md-sys-motion-easing-emphasized-decelerate
  ); /* cubic-bezier(0.05,0.7,0.1,1) */
  --ease-emphasized-accelerate: var(
    --md-sys-motion-easing-emphasized-accelerate
  ); /* cubic-bezier(0.3,0,0.8,0.15) */
}
```

Les **durations** n'ont **pas** de namespace `--duration-*` thématique en Tailwind v4 (pilotage par `--tw-duration`). À consommer en arbitraire : `class="duration-[var(--md-sys-motion-duration-medium2)]"` (300ms).

**Elevation — `<md-elevation>` reste la voie fidèle ; `shadow-*` = approximation.** La courbe d'ombre M3 (double box-shadow key-light `.3` + ambient `.15` interpolée par `clamp`, cf. `elevation/internal/elevation-styles.cssresult.ts`) n'est pas reproductible par un `shadow-*` simple. `m3-tokens.css` expose une **reconstitution statique** (`--md-sys-elevation-level1..5`, valeurs des 6 niveaux évaluées), utilisable en utilitaire si on accepte de perdre la transition dynamique :

```css
@theme inline {
  --shadow-md3-1: var(--md-sys-elevation-level1); /* 0 1px 2px 0 /.3 , 0 1px 3px 1px /.15 */
  --shadow-md3-2: var(--md-sys-elevation-level2);
  --shadow-md3-3: var(--md-sys-elevation-level3);
  --shadow-md3-4: var(--md-sys-elevation-level4);
  --shadow-md3-5: var(--md-sys-elevation-level5);
}
```

Pour la fidélité 100 % (transition, focus-elevation), garder `<md-elevation style="--md-elevation-level: 1">` dans un conteneur `relative` (cf. §4 Paper).

**State — hors portée des md-\* ; utile seulement pour surfaces maison.** Les opacités (`hover 0.08`, `focus 0.12`, `pressed 0.12`, `dragged 0.16`) vivent dans le shadow DOM des `md-*` → Tailwind ne peut ni les lire ni les peindre. `m3-tokens.css` les expose uniquement pour styliser des state layers de surfaces **maison** (light DOM, ex. hover d'une carte `<div>`) ; ça **n'affecte pas** l'intérieur des `md-*` (override via `--md-ripple-*` / `--md-<comp>-*-state-layer-*` en CSS classique).

> **À retenir** : `m3-tokens.css` ne « débloque » pas une dérivation native — il **redéclare un jeu parallèle** de vars (valeurs copiées de la lib) pour que `@theme` ait quelque chose à mapper. Seule `color` (et typescale size/line-height/weight) est une vraie dérivation runtime de `@material/web`.

---

## 3. Styling ciblé optionnel du shadow DOM via `::part()`

Le shadow DOM est franchissable **uniquement** aux endroits explicitement exposés par un `part="…"`. C'est le seul vecteur de styling externe des internes d'un `md-*`, et il reste limité (on stylise l'élément porteur du part, pas une arborescence interne arbitraire).

### 3.1 `part` réellement exposés (recensement `grep -rn "part=" material-web/`)

49 fichiers `.ts` exposent un `part`. Noms uniques relevés (light + labs/gb), avec leurs composants principaux :

| `part`                                                                                                                                                                                                                                                                 | Exposé par (exemples vérifiés)                                                                       |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `focus-ring`                                                                                                                                                                                                                                                           | button, radio, switch, tabs/tab, menu-item, segmented-button, fab, icon-button, chips, select-option |
| `ripple`                                                                                                                                                                                                                                                               | button, radio, menu-item, select-option                                                              |
| `elevation`                                                                                                                                                                                                                                                            | elevated/tonal/filled-button, tab, menu, card (labs), navigation-bar/drawer (labs), fab, chips       |
| `divider`                                                                                                                                                                                                                                                              | tabs (`tabs.ts:175`), divider (labs/gb)                                                              |
| `field`                                                                                                                                                                                                                                                                | select (`select.ts:399`)                                                                             |
| `menu`                                                                                                                                                                                                                                                                 | select (`select.ts:478`), menu (labs/gb)                                                             |
| `input`                                                                                                                                                                                                                                                                | search-bar (`search-bar.ts:83`), autocomplete (`autocomplete.ts:118`)                                |
| `view`                                                                                                                                                                                                                                                                 | search-bar (`search-bar.ts:107`)                                                                     |
| `bar`                                                                                                                                                                                                                                                                  | bottom-app-bar (`bottom-app-bar.ts:22`), top-app-bar (`top-app-bar.ts:105`)                          |
| `rail`                                                                                                                                                                                                                                                                 | navigation-rail (`navigation-rail.ts:94`)                                                            |
| `pane`, `container`, `list`, `detail`                                                                                                                                                                                                                                  | layout (`pane.ts:69`, `list-detail.ts:128-132`)                                                      |
| `canvas`                                                                                                                                                                                                                                                               | effects/webgpu-canvas                                                                                |
| `trailing-focus-ring`                                                                                                                                                                                                                                                  | chips/trailing-icons                                                                                 |
| `btn`, `icon-btn`, `card`, `card-btn`, `checkbox`, `radio`, `switch`, `fab`, `list`, `list-item`, `menu`, `menu-item`, `divider`, `split-btn`, `leading-btn`, `trailing-btn`, `label`, `body`, `supporting`, `top-bar`, `bottom-bar`, `navigation`, `main`, `scaffold` | composants du fork **`labs/gb/`** (variante "gb") + layout                                           |

> À retenir : sur les composants **upstream** courants, l'exposition est volontairement minimale — surtout `focus-ring`, `ripple`, `elevation`, plus `field`/`menu` (select), `input`/`view` (search), `bar`/`rail` (app-bar, rail), `divider` (tabs). Il n'y a **pas** de `part` pour la majorité des surfaces internes (texte de label de bouton, container coloré, etc.) : pour celles-là, **seuls les tokens** agissent. Toujours vérifier le `part` réel avant d'en cibler un (`grep -rn 'part=' material-web/<composant>/`).

### 3.2 Cibler un `part` — CSS classique (recommandé)

```css
/* Repeindre le ripple d'un bouton (part="ripple" — button.ts:113) */
md-filled-button::part(ripple) {
  --md-ripple-pressed-color: var(--md-sys-color-tertiary);
}

/* Styliser le divider d'un md-tabs (part="divider" — tabs.ts:175) */
md-tabs::part(divider) {
  border-color: var(--md-sys-color-outline-variant);
}

/* Cibler le champ interne d'un md-outlined-select (part="field" — select.ts:399) */
md-outlined-select::part(field) {
  min-width: 240px;
}
```

> Limite du standard : `::part()` ne style que l'élément exposé ; on ne peut **pas** descendre dans ses enfants non re-exposés (pas de `::part(a) .child`). Et `::part()` ne combine qu'avec des pseudo-classes/pseudo-éléments (`::part(x):hover`), pas avec des sélecteurs de descendance internes.

### 3.3 Côté Tailwind — la limite et les contournements

Tailwind v4 **n'a pas** de variant `::part()` natif. Trois options, par ordre de propreté :

**(a) Arbitrary variant inline** (one-shot, fonctionne tel quel, syntaxe `[&::part(x)]:`) :

```html
<md-tabs class="[&::part(divider)]:border-outline-variant">…</md-tabs>
```

Attention : `border-outline-variant` reste une déclaration light-DOM appliquée _via_ `::part`, donc valide ; mais ça reste verbeux et peu lisible. À réserver aux cas isolés.

**(b) `@custom-variant`** (réutilisable ; `@custom-variant`/`@variant` supportés — vérifié `tailwindcss/packages/tailwindcss/src/index.ts:311,352`) :

```css
/* app.css */
@custom-variant part-ripple (&::part(ripple));
@custom-variant part-divider (&::part(divider));
@custom-variant part-field (&::part(field));
```

```html
<md-tabs class="part-divider:border-outline-variant">…</md-tabs>
<md-filled-button class="part-ripple:opacity-50">…</md-filled-button>
```

**(c) CSS classique dans `@layer`** (le plus maintenable pour des règles non triviales) — voir §3.2, à placer dans une couche dédiée (§5.4).

**Recommandation** : pour `::part()`, privilégier le **CSS classique** (§3.2) ou un `@custom-variant` réutilisable (b). Éviter les arbitrary variants en masse — peu lisibles, et Tailwind ne valide pas le nom de part.

---

## 4. Layout des ex-composants MUI en utilitaires Tailwind

Conformément au contrat (§3 « Layout MUI → PAS d'élément md »), `Box / Stack / Grid / Container / Paper` deviennent des `<div>` + utilitaires Tailwind autour des composants `md-*`. Avant/après :

### `Box` → `<div>` + utilitaires

```jsx
// AVANT (MUI)
<Box sx={{ display: 'flex', alignItems: 'center', gap: 2, p: 3 }}>…</Box>
// APRÈS (Tailwind + md-*)
<div className="flex items-center gap-2 p-6">…</div>
```

### `Stack` → flex direction + gap

```jsx
// AVANT
<Stack direction="column" spacing={2}>
  <Button variant="contained">A</Button>
  <Button variant="outlined">B</Button>
</Stack>
// APRÈS
<div className="flex flex-col gap-2">
  <md-filled-button>A</md-filled-button>
  <md-outlined-button>B</md-outlined-button>
</div>
```

### `Grid` → CSS grid utilitaires

```jsx
// AVANT
<Grid container spacing={2}>
  <Grid item xs={12} md={6}>…</Grid>
  <Grid item xs={12} md={6}>…</Grid>
</Grid>
// APRÈS
<div className="grid grid-cols-1 md:grid-cols-2 gap-2">
  <div>…</div>
  <div>…</div>
</div>
```

### `Container` → max-width + centrage + padding

```jsx
// AVANT
<Container maxWidth="md">…</Container>
// APRÈS
<div className="mx-auto w-full max-w-3xl px-4">…</div>
```

### `Paper` → surface tokenisée + radius M3 (+ `md-elevation` si élévation voulue)

```jsx
// AVANT
<Paper elevation={1} sx={{ p: 2 }}>Contenu</Paper>
// APRÈS — surface M3 via le namespace couleur partagé (§2.2) + radius M3 (§2.4)
<div className="relative bg-surface-container text-on-surface rounded-medium p-4">
  <md-elevation style={{ '--md-elevation-level': 1 }}></md-elevation>
  Contenu
</div>
```

> `bg-surface-container` et `text-on-surface` proviennent du `@theme inline` (§2.2) → la carte « maison » est **du même bleu/gris que les composants `md-*`**. `md-elevation` (token `--md-elevation-level`) reproduit l'ombre M3 ; un simple `shadow-md` Tailwind ne respecte pas la courbe d'ombre M3.

### Écran complet (avant/après condensé)

```jsx
// APRÈS — layout 100 % Tailwind, composants 100 % md-*, couleurs partagées
<div className="mx-auto max-w-3xl px-4 py-8">
  <h1 className="text-headline-small text-on-surface mb-6">Profil</h1>
  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
    <div className="bg-surface-container rounded-large p-4 flex flex-col gap-3">
      <md-outlined-text-field label="Nom" class="w-full"></md-outlined-text-field>
      <md-outlined-text-field label="Email" class="w-full"></md-outlined-text-field>
    </div>
    <div className="bg-surface-container rounded-large p-4 flex items-end justify-end gap-2">
      <md-text-button>Annuler</md-text-button>
      <md-filled-button>Enregistrer</md-filled-button>
    </div>
  </div>
</div>
```

---

## 5. Pipeline d'intégration concret

### 5.1 Installation (bun uniquement — contrat §7.1)

```bash
bun add -d tailwindcss @tailwindcss/vite   # v4 — moteur du repo local tailwindcss/ (4.3.0)
bun add @material/web
```

### 5.2 Brancher Tailwind v4 — `@tailwindcss/vite` (recommandé) ou PostCSS

**Vite** (paquet `@tailwindcss/vite@4.3.0`, vérifié `tailwindcss/packages/@tailwindcss-vite/package.json`) :

```ts
// vite.config.ts
import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss()],
});
```

**Alternative PostCSS** (si pas de Vite — paquet `@tailwindcss/postcss`) :

```js
// postcss.config.mjs
export default { plugins: { "@tailwindcss/postcss": {} } };
```

### 5.3 La feuille CSS — ordre de chargement et couches

Le point névralgique. Tailwind v4 déclare ses couches dans cet ordre (vérifié `tailwindcss/packages/tailwindcss/index.css`) :

```css
@layer theme, base, components, utilities;
@import "./theme.css" layer(theme);
@import "./preflight.css" layer(base);
@import "./utilities.css" layer(utilities);
```

`@import "tailwindcss"` ré-émet exactement ce fichier. On charge donc nos tokens **et** notre `@theme inline` après l'import :

```css
/* app.css — point d'entrée unique */
@import "tailwindcss";

/* (0) Familles M3 SANS var runtime (shape/tracking/motion/elevation/state).
   Déclare ces vars sur :root avec les valeurs réelles v0_192 (cf. §2.4/§2.5).
   color + typescale size/line-height/weight sont déjà émis par la lib → pas ici. */
@import "@aphrody/m3-tokens/m3-tokens.css";

/* (1) Tokens COULEUR M3 = SOURCE DE VÉRITÉ (seule famille en var runtime native). light + dark scoped. */
:root {
  /* Généré par Material Theme Builder ou material-color-utilities (cf. 02-theme-token-migration.md) */
  --md-sys-color-primary: #006a6a;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-primary-container: #6ff7f6;
  --md-sys-color-on-primary-container: #002020;
  --md-sys-color-surface: #f4fbfa;
  --md-sys-color-on-surface: #161d1d;
  --md-sys-color-surface-container: #e8efee;
  --md-sys-color-on-surface-variant: #3f4948;
  --md-sys-color-outline-variant: #bec9c8;
  /* … les 47 rôles de _md-sys-color.scss (§2.2) … */
  /* shape/tracking/motion/elevation/state : NE PAS les redéclarer ici, ils
     viennent de m3-tokens.css (override possible en les redéfinissant après). */
}

/* (2) Dérivation Tailwind ← tokens M3 (§2.2/§2.3/§2.4/§2.5). inline OBLIGATOIRE. */
@theme inline {
  --color-primary: var(--md-sys-color-primary);
  --color-on-primary: var(--md-sys-color-on-primary);
  --color-surface: var(--md-sys-color-surface);
  --color-on-surface: var(--md-sys-color-on-surface);
  --color-surface-container: var(--md-sys-color-surface-container);
  --color-on-surface-variant: var(--md-sys-color-on-surface-variant);
  --color-outline-variant: var(--md-sys-color-outline-variant);
  /* … (cf. blocs §2) … */
  --radius-medium: var(--md-sys-shape-corner-medium); /* via m3-tokens.css */
  --radius-large: var(--md-sys-shape-corner-large); /* via m3-tokens.css */
  --text-body-large: var(--md-sys-typescale-body-large-size); /* var runtime native */
  --text-body-large--letter-spacing: var(
    --md-sys-typescale-body-large-tracking
  ); /* via m3-tokens.css */
  --ease-emphasized: var(--md-sys-motion-easing-emphasized); /* via m3-tokens.css */
}
```

JS — enregistrer les composants (effet de bord) :

```ts
// main.ts
import "./app.css";
import "@material/web/button/filled-button.js";
import "@material/web/textfield/outlined-text-field.js";
// (via wrappers React : voir migration/wrappers/ — contrat §2)
```

### 5.4 Preflight / reset vs `md-*` — ne pas casser les composants

Bonne nouvelle : le preflight de Tailwind v4 cible le **light DOM** ; il **ne franchit pas** le shadow DOM des `md-*` (même raison qu'au §1) → il ne peut pas casser leurs internes. Les risques réels sont **sur le host** et **avant définition** :

1. **`box-sizing`, `margin`, `border` réinitialisés sur le host.** Le preflight applique `*, ::after, ::before { box-sizing: border-box; margin: 0; border: 0 solid; }` (`tailwindcss/packages/tailwindcss/preflight.css`). Sur un host `md-*`, c'est bénin (les composants ne dépendent pas de marges externes) mais à connaître. Si un composant tiers en souffrait, on peut exclure le host via `@layer`/sélecteur.

2. **FOUC / `:not(:defined)`.** Avant l'exécution du JS qui enregistre l'élément, `<md-filled-button>` est un élément inconnu sans shadow DOM → il « flashe » son contenu brut. Standard material-web : masquer jusqu'à définition. À mettre en CSS classique (pas un utilitaire) :

   ```css
   /* app.css — hors @theme, dans le light DOM */
   md-filled-button:not(:defined),
   md-outlined-text-field:not(:defined),
   [class^="md-"]:not(:defined) {
     /* ou lister les tags utilisés */
     visibility: hidden;
   }
   ```

   (Pattern présent dans le repo : `material-web/catalog/src/components/top-app-bar.ts` utilise `:not(:defined)`.)

3. **Ordre des couches.** Garder le `@layer theme, base, components, utilities;` de Tailwind. Vos règles `::part()` / overrides de tokens composant doivent vivre dans une couche **postérieure à `base`** pour ne pas être écrasées par le preflight, et **idéalement hors `utilities`** pour ne pas entrer en guerre de spécificité avec les classes :
   ```css
   @layer components {
     md-tabs::part(divider) {
       border-color: var(--md-sys-color-outline-variant);
     }
   }
   ```

> Pas besoin de désactiver le preflight (contrairement à l'inquiétude habituelle). Il n'atteint pas les internes md-\*. Le seul ajout indispensable est la règle `:not(:defined)`.

---

## 6. Tailwind v3 vs v4 pour ce cas — et `material-tailwind`

| Critère                            | Tailwind v3 (JS config)                                                                                                             | **Tailwind v4 (CSS-first)** ✅                                                   |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Définition du thème                | `tailwind.config.js` (objet JS)                                                                                                     | `@theme` / `@theme inline` en CSS                                                |
| Référencer `var(--md-sys-color-*)` | possible mais via `colors: { primary: 'var(--md-sys-color-primary)' }` — résolution non scopée, pas d'équivalent propre de `inline` | **`@theme inline`** : valeur inlinée, suit les overrides de tokens scopés (§2.1) |
| Source de vérité partagée          | indirecte (config JS pointant des vars)                                                                                             | **directe** : le CSS des tokens M3 _est_ la source, Tailwind dérive              |
| Variants custom (`::part`)         | plugin JS                                                                                                                           | `@custom-variant` en CSS (§3.3)                                                  |
| Moteur dispo localement            | non                                                                                                                                 | **oui** (`tailwindcss/` = v4.3.0)                                                |

→ **Tailwind v4 CSS-first est la voie recommandée** : `@theme inline` est exactement le mécanisme qui rend possible la source de vérité unique scopée. v3 fonctionnerait pour les couleurs statiques mais perd la synchronisation sur les overrides de tokens (dark mode scoped, theming par sous-arbre).

⚠️ **`material-tailwind` n'est PAS la voie.** C'est une bibliothèque de composants React qui _réimplémente_ Material Design (et reste sur **Material 2 / Tailwind v3**), sans rapport avec `@material/web`. L'utiliser réintroduirait un 2ᵉ jeu de composants concurrent des `md-*`, en MD2, et casserait l'objectif de source de vérité M3 unique. À écarter.

---

## 7. Recommandation finale — architecture la plus propre

1. **Moteur** : Tailwind **v4.3.0** (repo local `tailwindcss/`) via **`@tailwindcss/vite`**. `@import "tailwindcss"` comme unique entrée.
2. **Source de vérité COULEUR = tokens `--md-sys-color-*`** posés sur `:root` (générés par `material-color-utilities` — cf. `02-theme-token-migration.md`). C'est la **seule famille réellement émise au runtime** par la lib ; les `md-*` la consomment nativement. Pour shape/typescale-tracking/motion/elevation/state (résolus au compile-time, donc absents du DOM), importer **`@aphrody/m3-tokens/m3-tokens.css`** qui les déclare sur `:root` avec les valeurs réelles v0_192 (§2.4/§2.5) — jeu de vars parallèle, pas une dérivation.
3. **Tailwind dérive ses tokens des `--md-sys-*` via `@theme inline`** (couleurs §2.2 = dérivation native ; typescale size/line-height/weight §2.3 = native ; tracking §2.3 + radius §2.4 + ease §2.5 = via m3-tokens.css) → `bg-primary`/`text-on-surface`/`rounded-large`/`ease-emphasized` alignés sur M3 et **réactifs aux overrides scopés** (pour la couleur). Optionnel : `--color-*: initial` pour purger la palette par défaut. **Non couvrables** : élévation fidèle (→ `<md-elevation>`), durations (arbitraire `duration-[var(…)]`), state des `md-*` (shadow DOM).
4. **Répartition stricte** : Tailwind = layout + host + composants non-`md` (`Box/Stack/Grid/Container/Paper` → `<div>` + utilitaires, §4) ; **theming interne des `md-*` = tokens uniquement** ; **`::part()`** (CSS classique ou `@custom-variant`, §3) pour les rares retouches ciblées des sous-parties exposées.
5. **Robustesse** : conserver le preflight (il n'atteint pas le shadow DOM des `md-*`) ; ajouter la règle `:not(:defined)` anti-FOUC ; mettre les overrides `::part`/tokens composant dans `@layer components`.
6. **Ne jamais** styliser l'intérieur d'un `md-*` avec une classe utilitaire (mur du shadow DOM, §1) ni introduire `material-tailwind` (MD2, §6).

---

### Sources

- `material-web/button/internal/button.ts:108-114` (shadow DOM, `part="focus-ring"`/`part="ripple"`)
- `material-web/tokens/_md-sys-color.scss:15-66` (47 rôles couleur M3, dont `*-fixed`/`surface-tint`) ; `material-web/tokens/_md-sys-shape.scss:15-23` + `versions/v0_192/_md-sys-shape.scss` (corner values) ; `material-web/tokens/_md-sys-typescale.scss` (`*-tracking` dans `$unsupported-tokens`) + `versions/v0_192/_md-sys-typescale.scss` (size/line-height/weight/tracking)
- `material-web/tokens/versions/v0_192/_md-sys-motion.scss` (16 durations + 10 easings) ; `_md-sys-elevation.scss` (level0-5 = 0/1/3/6/8/12) ; `material-web/elevation/internal/elevation-styles.cssresult.ts` (courbe double box-shadow `.3`/`.15`) ; `versions/v0_192/_md-sys-state.scss` (opacités 0.16/0.12/0.08/0.12)
- `packages/m3-tokens/src/m3-tokens.css` (asset runtime : shape/tracking/motion/elevation/state sur `:root`) ; `packages/m3-tokens/package.json` (export `./m3-tokens.css`)
- `material-web/docs/theming/color.md` (`--md-sys-color-*`, `:root`, material-color-utilities)
- `material-web/{tabs,select,appbar,search,navigationrail,layout}/internal/*.ts` (`part=` exposés — recensement §3.1)
- `tailwindcss/packages/tailwindcss/index.css` (ordre des couches) ; `preflight.css` (reset, `:host`) ; `theme.css` (palette par défaut)
- `tailwindcss/packages/tailwindcss/src/index.ts:93` (`@theme inline`), `:311,352` (`@custom-variant`/`@variant`)
- `tailwindcss/packages/tailwindcss/src/utilities.ts:2243` (`--color`), `:2339` (`--radius`), `:5268` (`--text`), `:3974` (`--font-weight`), `:4931` (`--leading`), `:4950` (`--tracking`)
- `tailwindcss/packages/@tailwindcss-vite/package.json` (`@tailwindcss/vite@4.3.0`)
- Tailwind v4 — Theme variables : https://tailwindcss.com/docs/theme (syntaxe `@theme` / `@theme inline`, namespaces)
- Tailwind v4 — Vite : https://tailwindcss.com/docs/installation/using-vite
- Material Web — theming : https://github.com/material-components/material-web/blob/main/docs/theming/color.md
