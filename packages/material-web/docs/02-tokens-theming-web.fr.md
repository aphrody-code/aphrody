---
nav_exclude: true
search_exclude: true
---

# Jetons & thématiques Material Design 3 sur le Web (2026)

> Portée : ce document explique **concrètement** comment les design tokens Material Design 3 (M3) deviennent des CSS custom properties, comment les générer (`material-color-utilities`, Material Theme Builder), comment faire du dynamic color et du dark mode côté web, et comment les appliquer en CSS vanilla, Tailwind et `material-web`. Toutes les références `material-web/...` pointent vers le repo local `/home/ubuntu/md3/material-web/`. État vérifié mai 2026.

---

## 1. Du token au CSS : la hiérarchie `ref → sys → comp`

M3 organise les tokens en **trois niveaux**, et sur le web **chaque token est une CSS custom property** que l'on peut scoper avec n'importe quel sélecteur CSS.

| Niveau        | Préfixe CSS                                  | Rôle                                                                              | Exemple                                                                                         |
| ------------- | -------------------------------------------- | --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **Référence** | `--md-ref-*`                                 | Valeurs primitives concrètes (un hex, une famille de police, une taille)          | `--md-ref-palette-primary40`, `--md-ref-typeface-plain`                                         |
| **Système**   | `--md-sys-*`                                 | Rôles/décisions du design system (couleur, typo, forme, élévation, motion, state) | `--md-sys-color-primary`, `--md-sys-typescale-body-medium-size`, `--md-sys-shape-corner-medium` |
| **Composant** | `--md-<composant>-*` (PAS de préfixe `comp`) | Attribut d'un composant donné, qui retombe (fallback) sur un token système        | `--md-filled-button-container-color`, `--md-checkbox-outline-color`                             |

Point d'attention sur le nommage : contrairement à `--md-ref-*` et `--md-sys-*`, **les component tokens ne portent PAS le préfixe `comp`** dans leur custom property CSS finale. Le `md-comp-*` n'existe qu'au niveau des fichiers de tokens source (Sass/JSON) — voir les fichiers `material-web/tokens/_md-comp-*.scss`. Le rendu CSS d'un component token est `--md-<nom-du-composant>-<propriété>`.

### Familles `--md-sys-*` principales

```css
:root {
  /* Couleur — rôles dynamiques */
  --md-sys-color-primary: #006a6a;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-primary-container: #6ff7f6;
  --md-sys-color-on-primary-container: #002020;
  --md-sys-color-surface: #f4fbfa;
  --md-sys-color-on-surface: #161d1d;

  /* Typographie — par scale/size/propriété */
  --md-sys-typescale-body-medium-font: "Roboto", system-ui;
  --md-sys-typescale-body-medium-size: 0.875rem;
  --md-sys-typescale-body-medium-line-height: 1.25rem;
  --md-sys-typescale-body-medium-weight: 400;

  /* Forme — rayons de coin */
  --md-sys-shape-corner-none: 0px;
  --md-sys-shape-corner-small: 8px;
  --md-sys-shape-corner-medium: 12px;
  --md-sys-shape-corner-large: 16px;
  --md-sys-shape-corner-full: 9999px;
}
```

**Typographie de la convention** : `--md-sys-typescale-<scale>-<size>-<property>`.

- échelles : `affichage`, `titre`, `titre`, `corps`, `étiquette`
- tailles : `petit`, `moyen`, `grand`
- propriétés : `font`, `size`, `line-height`, `weight`
- (la liste exhaustive vérifiable dans `material-web/tokens/_md-sys-typescale.scss`, ex. `body-large-font`, `display-large-weight`…)

**Mécanisme de fallback / cascade** : un component token référence un sys token via `var()`. Modifier un sys token au `:root` se propage donc à tous les composants. On peut aussi surcharger le component token directement pour un cas isolé :

```css
:root {
  --md-filled-button-container-shape: 0px; /* tous les boutons remplis */
}

md-filled-button.error {
  /* cas particulier : un bouton "error" */
  --md-filled-button-container-color: var(--md-sys-color-error);
  --md-filled-button-label-text-color: var(--md-sys-color-on-error);
}
```

Sources : [Design tokens – Material Design 3](https://m3.material.io/foundations/design-tokens), [Material Web – Theming](https://material-web.dev/theming/material-theming/), [Material Web – Couleur](https://material-web.dev/theming/color/).

---

## 2. `material-color-utilities` : la lib officielle

**Package : `@material/material-color-utilities`** (publié par Material Foundation). C'est LA bibliothèque JS/TS officielle qui implémente l'algorithme de couleur M3.

### Statut / maintenance en 2026

- Dernière version stable : **0.4.0** (publiée fin 2025).
- ~45 000 téléchargements npm/semaine, classée « popular ».
- **Maintenue** : l'issue tracker reste actif en 2026 (issues ouvertes en février/mars 2026).
- ⚠️ **N'accepte PAS de contributions externes de code** — seulement bug reports et feature requests.
- ⚠️ **Méfiance sur les forks** : `@usa-reddragon/...` et `@importantimport/...` sont abandonnés. Pour le spec couleur 2025 (DynamicScheme avec `SpecVersion.SPEC_2025`), un fork communautaire `@materialx/material-color-utilities` existe mais **n'est pas officiel**. Pour la version maintenue par Google : utiliser **`@material/material-color-utilities`**.

```bash
npm install @material/material-color-utilities
```

### HCT : l'espace couleur fondateur

M3 raisonne en **HCT** (Hue, Chroma, Tone) — un espace perceptuel où `Tone` est garant du contraste/accessibilité.

```ts
import { Hct } from "@material/material-color-utilities";

const color = Hct.fromInt(0xff4285f4); // ARGB
console.log(color.hue); // teinte 0–360
console.log(color.chroma); // saturation perceptuelle
console.log(color.tone); // luminosité 0–100 (pilote le contraste)
```

### `themeFromSourceColor` + `applyTheme`

À partir d'une couleur source, la lib construit 5+1 `TonalPalette` (primary, secondary, tertiary, neutral, neutral-variant, error) puis des schemes light/dark.

```ts
import { argbFromHex, themeFromSourceColor, applyTheme } from "@material/material-color-utilities";

// 1. Générer le thème depuis une couleur source (+ couleurs custom optionnelles)
const theme = themeFromSourceColor(argbFromHex("#f82506"), [
  { name: "success", value: argbFromHex("#0b8043"), blend: true },
]);

// 2. Détecter le dark mode système
const systemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;

// 3. Appliquer : pose les --md-sys-color-* sur la cible
applyTheme(theme, { target: document.body, dark: systemDark });
```

Signatures clés (cf. `typescript/utils/theme_utils.ts` du repo officiel) :

```ts
function themeFromSourceColor(source: number, customColors?: CustomColor[]): Theme;

async function themeFromImage(
  image: HTMLImageElement,
  customColors?: CustomColor[],
): Promise<Theme>;

function applyTheme(
  theme: Theme,
  options?: {
    dark?: boolean; // applique le scheme dark
    target?: HTMLElement; // élément cible (def: document.body via style)
    brightnessSuffix?: boolean; // génère aussi -light / -dark suffixés
    paletteTones?: number[]; // expose --md-ref-palette-<famille>-<tone>
  },
): void;

interface CustomColor {
  value: number;
  name: string;
  blend: boolean;
}
interface Theme {
  source: number;
  schemes: { light: Scheme; dark: Scheme };
  palettes: { primary; secondary; tertiary; neutral; neutralVariant; error };
  customColors: CustomColorGroup[];
}
```

**Ce que fait `applyTheme` en interne** : il itère sur les entrées du scheme et pose des custom properties nommées `--md-sys-color-{token}{suffix}` sur l'élément cible, en convertissant chaque valeur ARGB en hex via `hexFromArgb()`. C'est exactement le même nom que les tokens Sass de `material-web` — l'interop est directe.

### Quantize + score depuis une image

Pour extraire une couleur source depuis un wallpaper/image (cf. `typescript/utils/image_utils.ts` + `quantize/` + `score/`) :

```ts
import {
  sourceColorFromImage, // pipeline tout-en-un
  QuantizerCelebi,
  Score,
} from "@material/material-color-utilities";

// API haut niveau
const img = document.querySelector("img")!;
const sourceArgb = await sourceColorFromImage(img);
const theme = themeFromSourceColor(sourceArgb);
applyTheme(theme, { target: document.body });

// — ou pipeline bas niveau —
// 1. pixels (Uint8 RGBA) -> 2. quantize -> 3. score (couleurs "désirables")
const result = QuantizerCelebi.quantize(pixels, 128); // Map<argb, count>
const ranked = Score.score(result); // couleurs triées par pertinence UI
const best = ranked[0];
```

`Score.score()` ne renvoie pas la couleur la plus fréquente mais la plus « adaptée à un thème » (chroma suffisant, etc.).

### Contrast levels intégrés

La lib embarque 4 niveaux de contraste via des `ContrastCurve` : reduced (-1.0), standard (0.0), medium (0.5), high (1.0). Le niveau standard garantit WCAG AA, le niveau high vise AAA — sans calcul manuel côté appelant.

Sources : [@material/material-color-utilities (npm)](https://www.npmjs.com/package/@material/material-color-utilities), [GitHub materials-foundation/material-color-utilities](https://github.com/material-foundation/material-color-utilities).

---

## 3. Générateur de thèmes matériels

Plugin Figma officiel (Material Foundation) + version web. Permet de générer un thème M3 complet et de l'exporter pour le code.

- **Génération** : ouvrir le plugin → _Create Theme_. Visualise le dynamic color (light/dark) et crée les tokens M3 comme styles/variables Figma.
- **Surfaces** : la mise à jour récente introduit des _Surfaces basées sur les tones_ (plus des overlays read-only). Les Surfaces 1–5 restent pour compat, mais Material recommande de migrer vers les nouveaux surface tokens (`surface-container`, `surface-container-high`, etc. — visibles dans `material-web/tokens/_md-sys-color.scss`).
- **Export** : bouton _Export_. Le format historique **DSP n'est plus supporté en v2**. On exporte en **Material Theme (JSON)**. Le JSON est ré-importable (move de thème entre fichiers et entre web/Figma).
- **CSS** : le plugin officiel produit des variables/styles Figma. Pour sortir directement des **CSS custom properties** conformes au spec M3, on passe par des plugins compagnons :
  - **m3-variable-exporter** — exporte les variables Figma (groupes _Schemes_, _Palettes_, _Extended Colors_) en variables CSS conformes au spec token M3.
  - **Export variable – CSS & JSON** — exportez des multi-collections en CSS ou JSON.

Le CSS exporté est directement consommable : il pose les `--md-sys-color-*` (et palettes `--md-ref-palette-*`).

Sources : [Material Theme Builder (Figma)](https://www.figma.com/community/plugin/1034969338659738588/material-theme-builder), [GitHub materials-theme-builder](https://github.com/material-foundation/material-theme-builder), [m3-variable-exporter](https://www.figma.com/community/plugin/1357242754722607687/m3-variable-exporter).

---

## 4. Dynamic color sur le web : faisabilité & fallbacks

Le « vrai » dynamic color OS (couleur dérivée du wallpaper système, façon Android 12+) **n'est pas accessible** depuis un navigateur : pas d'API web pour lire le wallpaper. Sur le web, le dynamic color se fait donc **au runtime avec `material-color-utilities`**, à partir d'une source choisie :

- une **couleur de marque** fixe (`themeFromSourceColor`) ;
- une **image fournie par l'utilisateur** (upload, avatar, cover) via `sourceColorFromImage` / `themeFromImage` ;
- une **couleur stockée** (préférence user persistée).

```ts
// Theming réactif depuis une image choisie par l'utilisateur
async function applyThemeFromUserImage(img: HTMLImageElement) {
  const source = await sourceColorFromImage(img);
  const theme = themeFromSourceColor(source);
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  applyTheme(theme, { target: document.documentElement, dark });
}
```

**Fallbacks recommandés** :

1. Toujours définir un **thème statique par défaut** dans le CSS (`:root { --md-sys-color-* }`) issu de Material Theme Builder. Le runtime ne fait que **surcharger** ces valeurs — si le JS échoue/est désactivé, l'UI reste correcte.
2. Calculer le thème **avant le first paint** (script inline `<head>`, ou SSR avec sérialisation du scheme en CSS) pour éviter le FOUC.
3. Persister la couleur source (localStorage) pour réappliquer instantanément au reload.

---

## 5. Mode sombre et niveaux de contraste et variables CSS

### Mode sombre

Sur `material-web`, le dark mode est **opt-in** (pas automatique) : le coût CSS d'embarquer light+dark par défaut serait élevé et moins flexible. Deux approches.

**(a) Via `prefers-color-scheme` (deux jeux de tokens) :**

```css
:root {
  color-scheme: light dark; /* indispensable pour les form controls natifs */

  /* Scheme LIGHT (généré par MTB / MCU) */
  --md-sys-color-primary: #006a6a;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-surface: #f4fbfa;
  --md-sys-color-on-surface: #161d1d;
}

@media (prefers-color-scheme: dark) {
  :root {
    /* Scheme DARK */
    --md-sys-color-primary: #4cdada;
    --md-sys-color-on-primary: #003737;
    --md-sys-color-surface: #0e1514;
    --md-sys-color-on-surface: #dde4e3;
  }
}
```

**(b) Via classe / attribut (toggle manuel)** — utile pour un switch in-app :

```css
:root,
[data-theme="light"] {
  --md-sys-color-surface: #f4fbfa; /* ... */
}
[data-theme="dark"] {
  --md-sys-color-surface: #0e1514; /* ... */
}
```

**(c) Au runtime avec MCU** : `applyTheme(theme, { dark: true })` repose simplement le même jeu de `--md-sys-color-*` avec les valeurs du scheme dark.

### Niveaux de contraste

Les 4 niveaux (`standard`, `medium`, `high`, `reduced`) ne sont pas un token CSS standard : on génère un **jeu de `--md-sys-color-*` distinct par niveau** via MCU (le `contrastLevel` du `DynamicScheme`) et on switch par sélecteur :

```ts
import { argbFromHex, Hct, SchemeTonalSpot, hexFromArgb } from "@material/material-color-utilities";

function schemeVars(sourceHex: string, dark: boolean, contrastLevel: number) {
  const scheme = new SchemeTonalSpot(Hct.fromInt(argbFromHex(sourceHex)), dark, contrastLevel);
  // -> itérer sur les color roles et produire des --md-sys-color-* ;
  //    contrastLevel: 0 (AA) | 0.5 (medium) | 1.0 (AAA, high)
  return scheme;
}
```

```css
[data-contrast="high"] {
  /* jeu de --md-sys-color-* régénéré avec contrastLevel: 1.0 */
}
```

> Note spec 2025 : avec `SpecVersion.SPEC_2025`, les contrastLevel négatifs (reduced) ne sont plus possibles ; il faut `SPEC_2021` (non recommandé) pour un thème low-contrast.

---

## 6. Intégration pratique

### (a) CSS vanille

On pose les tokens au `:root` et on les consomme dans ses propres composants :

```css
:root {
  --md-sys-color-primary: #006a6a;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-surface: #f4fbfa;
  --md-sys-color-on-surface: #161d1d;
  --md-sys-shape-corner-medium: 12px;
  --md-sys-typescale-label-large-size: 0.875rem;
  --md-sys-typescale-label-large-weight: 500;
}

.card {
  background: var(--md-sys-color-surface);
  color: var(--md-sys-color-on-surface);
  border-radius: var(--md-sys-shape-corner-medium);
}
.button-primary {
  background: var(--md-sys-color-primary);
  color: var(--md-sys-color-on-primary);
  font-size: var(--md-sys-typescale-label-large-size);
  font-weight: var(--md-sys-typescale-label-large-weight);
}
```

### (b) Tailwind (jetons de cartographie → thème)

L'idée : **les tokens M3 restent la source de vérité en CSS variables**, Tailwind ne fait que les exposer en utilitaires.

**Tailwind v4 (`@theme`, approche 2026)** — on mappe directement les custom properties M3 :

```css
@import "tailwindcss";

/* Tokens M3 (générés par MTB / MCU) */
:root {
  --md-sys-color-primary: #006a6a;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-surface: #f4fbfa;
  --md-sys-color-on-surface: #161d1d;
}
@media (prefers-color-scheme: dark) {
  :root {
    --md-sys-color-primary: #4cdada;
    --md-sys-color-on-primary: #003737;
    --md-sys-color-surface: #0e1514;
    --md-sys-color-on-surface: #dde4e3;
  }
}

/* On expose les tokens M3 comme couleurs Tailwind */
@theme inline {
  --color-primary: var(--md-sys-color-primary);
  --color-on-primary: var(--md-sys-color-on-primary);
  --color-surface: var(--md-sys-color-surface);
  --color-on-surface: var(--md-sys-color-on-surface);

  --radius-m3-medium: var(--md-sys-shape-corner-medium);
}
```

```html
<!-- le dark mode suit automatiquement les tokens M3 -->
<button class="bg-primary text-on-primary rounded-[--radius-m3-medium]">OK</button>
<div class="bg-surface text-on-surface">Carte</div>
```

`@theme inline` est important : il évite que Tailwind « fige » la valeur et laisse la cascade des `--md-sys-*` (dark mode / runtime MCU) reprendre la main.

**Tailwind v3 (`tailwind.config.js`)** — même principe via `theme.extend` :

```js
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      colors: {
        primary: "var(--md-sys-color-primary)",
        "on-primary": "var(--md-sys-color-on-primary)",
        surface: "var(--md-sys-color-surface)",
        "on-surface": "var(--md-sys-color-on-surface)",
      },
      borderRadius: {
        "m3-medium": "var(--md-sys-shape-corner-medium)",
      },
    },
  },
};
```

### (c) Avec `material-web` (`<md-*>`)

Les web components `<md-*>` lisent **nativement** les `--md-sys-color-*`. Deux voies.

**Voie CSS pure** (pas de Sass) — poser le scheme et c'est tout :

```css
:root {
  --md-sys-color-primary: #006a6a;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-primary-container: #6ff7f6;
  --md-sys-color-on-primary-container: #002020;
  /* ... le scheme complet généré par MTB / MCU ... */
}
```

```html
<md-filled-button>Click</md-filled-button>
<!-- prend la couleur du token -->
```

**Voie Sass** (API fournie par le repo, cf. `material-web/color/_color.scss`) :

```scss
@use "@material/web/color/color";
@use "@material/web/typography/typescale";

:root {
  // Émet les --md-sys-color-* depuis une map de tokens
  @include color.theme(
    (
      "primary": #006a6a,
      "on-primary": #ffffff,
      "primary-container": #6ff7f6,
      "surface": #f4fbfa,
    )
  );

  // ou depuis une palette de référence -> génère tout le scheme
  @include color.light-theme;

  @include typescale.theme(
    (
      "body-medium-size": 1rem,
      "body-medium-line-height": 1.5rem,
    )
  );
}

@media (prefers-color-scheme: dark) {
  :root {
    @include color.dark-theme;
  }
}
```

Surcharge ciblée d'un component token (sans préfixe `comp`) :

```css
.square-buttons {
  --md-filled-button-container-shape: 0px; /* tous les md-filled-button du scope */
}
```

> MWC ne supporte pas (encore) `--md-ref-palette-*` ni `--md-sys-motion-*` comme custom properties ; les palettes passent par les mixins Sass `light-theme`/`dark-theme`.

---

## 7. Comment `material-web` applique réellement les tokens (repo local)

Le repo `/home/ubuntu/md3/material-web/` est la source canonique du mapping token → CSS. Points clés :

### Organisation des fichiers de tokens — `material-web/tokens/`

- `material-web/tokens/_index.scss` : ré-exporte toutes les familles via `@forward ... as md-sys-color-*`, `md-ref-palette-*`, `md-comp-<composant>-*`. C'est l'agrégateur.
- `material-web/tokens/_md-sys-color.scss` : liste blanche ($supported-tokens`) des color role M3 — on y voit les **surface tokens récents** : `surface-bright`, `surface-dim`, `surface-container`, `surface-container-low/lowest/high/highest`, plus les `_-fixed`/`_-fixed-dim`et`inverse-\*`.
- `material-web/tokens/_md-sys-typescale.scss` : jetons `--md-sys-typescale-<scale>-<size>-<property>` (ex. `body-large-font`, `display-large-weight`).
- `material-web/tokens/_md-sys-shape.scss` + `material-web/tokens/versions/v0_192/_md-sys-shape.scss` : valeurs des coins — `corner-none : 0px`, `corner-small : 8px`, `corner-medium : 12px`, `corner-large : 16px`, `corner-extra-large : 28px`, `corner-full : 9999px`, plus variantes directionnelles (`corner-large-top`, `corner-large-start`…).
- `material-web/tokens/_md-ref-palette.scss` : palette de référence (tones par famille). Les valeurs concrètes versionnées sont dans `material-web/tokens/versions/v0_192/` (fichiers marqués _AUTOMATICALLY GENERATED — Design system version: v0.192, Platform: Web, Scheme: Dynamic_).
- `material-web/tokens/_md-comp-*.scss` (≈ 60 fichiers, ex. `_md-comp-filled-button.scss`, `_md-comp-checkbox.scss`) : présente les **component tokens**, qui retombent sur les sys tokens.

### Le mapping sys-color est dérivé de la palette ref

Dans `material-web/tokens/versions/v0_192/_md-sys-color.scss`, chaque rôle système pointe vers un tone de la palette de référence — c'est le cœur du système. Extrait (scheme dark) :

```scss
@function values-dark($deps: $_default-dark) {
  @return (
    "background": map.get($deps, "md-ref-palette", "neutral6"),
    "error": map.get($deps, "md-ref-palette", "error80"),
    "on-primary": map.get($deps, "md-ref-palette", "primary20"),
    "on-primary-container": map.get($deps, "md-ref-palette", "primary90"),
    "inverse-primary": map.get($deps, "md-ref-palette", "primary40") /* ... */
  );
}
```

### Mixins qui émettent du CSS — `material-web/color/_color.scss`

C'est ce fichier qui transforme une map de tokens en custom properties. Le mixin `theme()` est littéralement :

```scss
@mixin theme($tokens) {
  @each $token, $value in $tokens {
    @if list.index(tokens.$md-sys-color-supported-tokens, $token) == null {
      @error 'md-sys-color `#{$token}` is not a supported token.';
    }
    @if $value {
      --md-sys-color-#{$token}: #{$value};
    }
  }
}
```

`light-theme()` / `dark-theme()` prennent une `md-ref-palette` et appellent `md-sys-color-values-light/dark()` (le mapping ci-dessus) puis `theme()`. Résultat : un `:root { --md-sys-color-* }` complet.

### Typographie — `material-web/typography/_typescale.scss`

- Le mixin `typescale.theme()` émet `--md-sys-typescale-<token>` (même logique, validation par `$md-sys-typescale-supported-tokens`).
- Le mixin `typescale.styles()` génère en plus des **classes utilitaires** `.md-typescale-display-large`, `.md-typescale-body-medium`, etc. (shorthand `font:`), encapsulées dans un `@layer` pour spécificité basse. Appelé dans `material-web/typography/md-typescale-styles.scss` :

```scss
@include typescale.styles(tokens.md-sys-typescale-values());
```

### Synthèse du flux dans material-web

```
ref palette (tones)  -->  md-sys-color-values-light/dark (mapping role->tone)
        |                                  |
        v                                  v
  color.light-theme()  --->  color.theme()  --->  :root { --md-sys-color-* }
                                                          |
                                          var() fallback  v
                              component tokens  --->  --md-<composant>-* (consommé par <md-*>)
```

C'est exactement le pipeline que reproduisent Material Theme Builder (côté Figma/export CSS) et `material-color-utilities` (`applyTheme`, côté runtime) : tous trois convergent vers le **même contrat de nommage `--md-sys-color-*`**.

---

## Sources

- [Jetons de conception – Material Design 3](https://m3.material.io/foundations/design-tokens)
- [Web matériel – Thématique](https://material-web.dev/theming/material-theming/) · [Web matériel – Couleur](https://material-web.dev/theming/color/)
- [GitHub – Material-foundation/material-color-utilities](https://github.com/material-foundation/material-color-utilities) · [npm @material/material-color-utilities](https://www.npmjs.com/package/@material/material-color-utilities)
- [Constructeur de thèmes matériels (Figma)](https://www.figma.com/community/plugin/1034969338659738588/material-theme-builder) · [GitHub Material-theme-builder](https://github.com/material-foundation/material-theme-builder) · [m3-variable-exporter](https://www.figma.com/community/plugin/1357242754722607687/m3-variable-exporter)
- Dépôt local : `material-web/tokens/`, `material-web/color/_color.scss`, `material-web/typography/_typescale.scss`, `material-web/tokens/versions/v0_192/`
