# Migration du thème MUI (Material 2) vers les tokens M3 `--md-sys-*`

> Portée : ce document décrit **comment convertir un thème MUI (`createTheme`, palette Material 2) en design tokens Material Design 3** (`--md-sys-color-*`, `--md-sys-typescale-*`, `--md-sys-shape-corner-*`) consommés nativement par `@material/web` (fork aphrody, `material-web/`). Il complète le contrat `migration/00-CONVENTIONS.md` (§5) et la doc de référence `docs/02-tokens-theming-web.md`. Il fournit le mapping étendu (couleur, typo, shape, dark mode), explique la génération des rôles M3 absents de MUI via **material-color-utilities**, documente les pertes de fidélité M2→M3, et accompagne le script exécutable `migration/scripts/theme-to-tokens.ts` (sortie réelle vérifiée plus bas).

---

## 1. Le problème de fond : deux systèmes de couleur incompatibles

MUI v9 (`material-ui/packages/mui-material/`, `@mui/material@9.0.1`) utilise une **palette Material 2** : un petit nombre de couleurs (`primary`, `secondary`, `error`, `warning`, `info`, `success`), chacune avec `main` / `light` / `dark` / `contrastText`, plus `background`, `text`, `divider`, `action`. C'est un modèle **à intentions explicites**, choisies à la main par le designer.

`@material/web` consomme un **scheme Material 3** : ~50 _color roles_ (`primary`, `on-primary`, `primary-container`, `on-primary-container`, `secondary*`, `tertiary*`, `surface*`, `surface-variant`, `outline`, `inverse-*`…) **dérivés algorithmiquement** d'une couleur source via HCT (Hue/Chroma/Tone). C'est un modèle **génératif**.

Conséquence directe : **la majorité des rôles M3 n'ont aucune source dans un thème MUI**. On ne peut pas faire un mapping 1:1. La stratégie retenue :

1. Mapper directement ce qui existe des deux côtés (cf. §2) ;
2. **Générer** les rôles M3 manquants avec `material-color-utilities` à partir de `palette.primary.main` ;
3. **Ré-imposer** les couleurs MUI explicites par-dessus le scheme généré (fidélité au design d'origine) ;
4. Documenter et assumer les pertes (cf. §6).

---

## 2. Mapping canonique étendu MUI (M2) → M3

### 2.1 Couleur

Extension du tableau §5 du contrat. La colonne « source » indique d'où vient la valeur.

| MUI theme                        | → token M3 (`--md-sys-color-*`)                                                         | Source                               |
| -------------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------ |
| `palette.primary.main`           | `primary`                                                                               | direct                               |
| `palette.primary.contrastText`   | `on-primary`                                                                            | direct (sinon calculé par luminance) |
| `palette.secondary.main`         | `secondary`                                                                             | direct                               |
| `palette.secondary.contrastText` | `on-secondary`                                                                          | direct/calculé                       |
| `palette.error.main`             | `error`                                                                                 | direct                               |
| `palette.error.contrastText`     | `on-error`                                                                              | direct/calculé                       |
| `palette.background.default`     | `background` + `surface`                                                                | direct                               |
| `palette.background.paper`       | `surface-container`                                                                     | approximation (surface élevée MUI)   |
| `palette.text.primary`           | `on-background` + `on-surface`                                                          | direct                               |
| `palette.text.secondary`         | `on-surface-variant`                                                                    | approximation                        |
| `palette.divider`                | `outline-variant`                                                                       | direct                               |
| _(aucune)_                       | `tertiary`, `*-container`, `surface-variant`, `outline`, `inverse-*`, `scrim`, `shadow` | **généré par MCU** (cf. §3)          |

> Les rôles "container" (`primary-container`, `secondary-container`…) sont l'innovation centrale de M3 et n'ont **strictement aucun équivalent M2**. Ils sont toujours générés.

### 2.2 Typographie : `typography.*` → `--md-sys-typescale-*`

M3 nomme ses tokens `--md-sys-typescale-<scale>-<size>-<property>` (cf. `material-web/tokens/_md-sys-typescale.scss`) :

- scales : `display`, `headline`, `title`, `body`, `label`
- sizes : `small`, `medium`, `large`
- properties : `font`, `size`, `line-height`, `weight`, `tracking`

MUI a 13 variants typographiques. Correspondance retenue (la plus proche sémantiquement) :

| MUI variant | → typescale M3    |
| ----------- | ----------------- |
| `h1`        | `display-large`   |
| `h2`        | `display-medium`  |
| `h3`        | `display-small`   |
| `h4`        | `headline-large`  |
| `h5`        | `headline-medium` |
| `h6`        | `headline-small`  |
| `subtitle1` | `title-medium`    |
| `subtitle2` | `title-small`     |
| `body1`     | `body-large`      |
| `body2`     | `body-medium`     |
| `button`    | `label-large`     |
| `caption`   | `body-small`      |
| `overline`  | `label-small`     |

Conversions de valeurs (faites par le script) :

- `typography.fontFamily` (global) propagé en `<scale>-<size>-font` sur chaque token (M3 n'a pas de famille globale unique : c'est par token).
- `fontSize` number (px MUI) → `<n>px`.
- `lineHeight` ratio MUI (ex. `1.5`) → **longueur** M3 (`1.5 * fontSize` px) — M3 attend une longueur, pas un ratio.
- `fontWeight` → `<scale>-<size>-weight`.
- `letterSpacing` → `<scale>-<size>-tracking`.

### 2.3 Forme : `shape.borderRadius` → `--md-sys-shape-corner-*`

Piège important : **`shape.borderRadius` (défaut MUI = `4`) est une _unité de base_, PAS l'équivalent du coin `medium` M3** (qui vaut `12px`, cf. `material-web/tokens/versions/v0_192/_md-sys-shape.scss`). Mapper bêtement `4 → corner-medium` écraserait toute l'échelle (`medium: 4px`, `small: 3px`, `large: 5px`).

Le script calcule donc un **ratio = `borderRadius / 4`** et l'applique à toute la famille de coins M3, préservant les proportions :

| corner M3            | base (px) | émis (ratio `r/4`) |
| -------------------- | --------- | ------------------ |
| `corner-none`        | 0         | `0 * r/4`          |
| `corner-extra-small` | 4         | `4 * r/4`          |
| `corner-small`       | 8         | `8 * r/4`          |
| `corner-medium`      | 12        | `12 * r/4`         |
| `corner-large`       | 16        | `16 * r/4`         |
| `corner-extra-large` | 28        | `28 * r/4`         |
| `corner-full`        | 9999      | inchangé (pilule)  |

Ainsi `borderRadius: 4` (défaut) → échelle M3 nominale ; `borderRadius: 8` → tout doublé.

### 2.4 Dark mode

MUI gère le dark de deux façons :

- **historique** : un 2e `createTheme({ palette: { mode: 'dark', … } })` distinct ;
- **MUI v6+** : `colorSchemes: { light, dark }` dans un seul thème.

M3, lui, dérive light **et** dark d'une **même couleur source**. Le script accepte les deux : si un thème/palette dark est fourni (option `darkTheme` ou `theme.colorSchemes.dark`), il alimente le bloc `@media (prefers-color-scheme: dark)` avec **sa propre source** (les couleurs dark MUI sont en général éclaircies — `#90caf9` au lieu de `#1976d2`). Sinon, le dark est dérivé de la source light.

---

## 3. Générer les rôles M3 manquants avec `material-color-utilities`

### 3.1 API réelle vérifiée (repo local)

La lib est présente dans `material-web/node_modules/@material/material-color-utilities` — **version `0.2.7`** (et non `0.4.0` comme parfois indiqué ; à vérifier au moment de l'install). `package.json` : `"type": "module"`, `"main": "index.js"`. Exports pertinents vérifiés au runtime (`bun`) :

```
DynamicScheme, Hct, Scheme, SchemeContent, SchemeExpressive, SchemeFidelity,
SchemeMonochrome, SchemeNeutral, SchemeTonalSpot, SchemeVibrant, TonalPalette,
argbFromHex, hexFromArgb, themeFromImage, themeFromSourceColor, applyTheme, …
```

`themeFromSourceColor(argb)` renvoie un `Theme` dont `theme.schemes.light` / `.dark` exposent **`.toJSON()`**. En v0.2.7, ce JSON contient **exactement 29 rôles** (camelCase) :

```
primary, onPrimary, primaryContainer, onPrimaryContainer,
secondary, onSecondary, secondaryContainer, onSecondaryContainer,
tertiary, onTertiary, tertiaryContainer, onTertiaryContainer,
error, onError, errorContainer, onErrorContainer,
background, onBackground, surface, onSurface,
surfaceVariant, onSurfaceVariant, outline, outlineVariant,
shadow, scrim, inverseSurface, inverseOnSurface, inversePrimary
```

Chaque valeur est un ARGB int → `hexFromArgb(int)` donne le `#rrggbb`. Le script mappe ces clés camelCase vers les tokens kebab `--md-sys-color-*`.

### 3.2 Limite de fidélité du scheme MCU 0.2.7

Le `Scheme` classique de MCU 0.2.7 **ne contient PAS** les surface tokens tonaux récents que `material-web` supporte pourtant (`surface-bright`, `surface-dim`, `surface-container-low/lowest/high/highest`, `*-fixed`, `*-fixed-dim`, `surface-tint`) — liste vérifiée dans `material-web/tokens/_md-sys-color.scss`. Pour les obtenir, il faudrait passer par les `DynamicScheme` modernes (`SchemeTonalSpot` + `MaterialDynamicColors`), absents/incomplets en 0.2.7. Conséquence : ces tokens ne sont **pas émis** → les composants `<md-*>` retombent sur leurs **fallbacks internes** (chaque component token a un `var(--md-sys-…, <fallback>)`), donc l'UI reste fonctionnelle mais sans les nuances de surface M3 les plus fines. Pour un thème complet, générer le scheme avec **Material Theme Builder** (cf. `docs/02-tokens-theming-web.md` §3) reste l'option la plus fidèle.

### 3.3 Pipeline (≈ ce que fait `applyTheme`, mais en émettant du CSS texte)

`applyTheme(theme, { target, dark })` de MCU pose les `--md-sys-color-<token>` **directement sur un élément DOM** (cf. `material-web/node_modules/@material/material-color-utilities/utils/theme_utils.js`). Notre script fait la **même chose en mode statique** : il sérialise le scheme en bloc CSS (`:root { … }`), utilisable en SSR / build sans DOM, ce qui évite le FOUC (cf. `docs/02-tokens-theming-web.md` §4).

---

## 4. Injection du résultat & consommation par material-web

### 4.1 Où poser les tokens

Le CSS généré se pose au `:root` (light) + `@media (prefers-color-scheme: dark)` (dark) :

```css
:root {
  color-scheme: light dark; /* indispensable aux form controls natifs */
  --md-sys-color-primary: #1976d2;
  /* … tout le scheme light … */
  --md-sys-typescale-body-large-size: 16px;
  --md-sys-shape-corner-medium: 12px;
}
@media (prefers-color-scheme: dark) {
  :root {
    --md-sys-color-primary: #90caf9; /* … scheme dark … */
  }
}
```

Pour un **toggle manuel** plutôt que la media query, remplacer le sélecteur dark par `[data-theme='dark']` (cf. `docs/02-tokens-theming-web.md` §5b).

### 4.2 Comment `<md-*>` consomme ces tokens

Les composants du fork (`material-web/`) sont _self-contained_ : leurs **component tokens** (`--md-filled-button-container-color`, etc.) retombent par `var(--md-sys-color-…, <fallback>)` sur les sys tokens. Poser un `--md-sys-color-primary` au `:root` se propage donc à **tous** les `<md-*>` du scope sans autre câblage (cf. `material-web/tokens/` et `docs/02-tokens-theming-web.md` §1 et §7). Aucune compilation Sass requise : la **voie CSS pure** suffit.

```html
<!-- prend automatiquement --md-sys-color-primary / on-primary -->
<md-filled-button>OK</md-filled-button>
```

Surcharge ciblée d'un component token (sans préfixe `comp`, cf. `docs/02-tokens-theming-web.md` §1) :

```css
.square-buttons {
  --md-filled-button-container-shape: 0px;
}
```

### 4.3 Pont vers Tailwind

Les tokens émis servent aussi de **source de vérité unique** pour Tailwind v4 via `@theme inline` (cf. `docs/02-tokens-theming-web.md` §6b et le livrable `migration/06-tailwind-material-web.md`). On ne duplique pas les couleurs : Tailwind ne fait que ré-exposer les `--md-sys-*`.

---

## 5. Le script `theme-to-tokens.ts`

Fichier : `migration/scripts/theme-to-tokens.ts` (Node ESM, exécuté avec **bun**).

### 5.1 Install de la dépendance

MCU est déjà présent via `material-web/node_modules` (le script le résout automatiquement en fallback). Pour l'avoir au niveau du kit :

```bash
bun add @material/material-color-utilities
```

Le script tente, dans l'ordre : (a) résolution standard du package ; (b) le copy de `material-web/node_modules`. Si **rien** n'est trouvé → `mcu = null` → **fallback de mapping direct** : il n'émet que `primary`/`on-primary` (+ les overrides MUI), les rôles M3 dérivés restent absents (les composants `<md-*>` utilisent leurs fallbacks internes). Le commentaire `/* MCU: … */` en tête de sortie indique le mode actif.

### 5.2 API

```js
import { muiThemeToTokens } from "./migration/scripts/theme-to-tokens.ts";

const { css, mcuAvailable } = await muiThemeToTokens(muiLightTheme, {
  darkTheme: muiDarkTheme, // optionnel : palette ou thème MUI dark
});
console.log(css);
```

`muiThemeToTokens(lightTheme, { darkTheme })` retourne `{ css, mcuAvailable, lightRoles, darkRoles, typescale, shape }`. La fonction est `async` (chargement dynamique de MCU).

### 5.3 Exécution & sortie réelle (vérifiée)

```bash
bun migration/scripts/theme-to-tokens.ts
# stderr -> [theme-to-tokens] MCU disponible : true
```

Le bloc en bas du fichier (`if (import.meta.main)`) convertit un thème MUI réaliste (primary `#1976d2`, secondary `#9c27b0`, dark séparé avec `#90caf9`). **Sortie réellement produite** (extrait — exécuté le 2026-05-26, exit 0) :

```css
/* Généré par migration/scripts/theme-to-tokens.ts */
/* MCU: disponible (API ok) */

:root {
  color-scheme: light dark;
}

:root {
  --md-sys-color-primary: #1976d2;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-primary-container: #d4e3ff; /* généré par MCU */
  --md-sys-color-on-primary-container: #001c3a; /* généré par MCU */
  --md-sys-color-secondary: #9c27b0;
  --md-sys-color-on-secondary: #ffffff;
  --md-sys-color-tertiary: #6e5676; /* généré (aucune source MUI) */
  --md-sys-color-tertiary-container: #f7d8ff; /* généré */
  --md-sys-color-error: #d32f2f;
  --md-sys-color-on-error: #ffffff;
  --md-sys-color-background: #fafafa;
  --md-sys-color-surface: #fafafa;
  --md-sys-color-on-surface: #212121;
  --md-sys-color-surface-variant: #e0e2ec; /* généré */
  --md-sys-color-on-surface-variant: #757575; /* = text.secondary MUI */
  --md-sys-color-outline: #74777f; /* généré */
  --md-sys-color-outline-variant: #e0e0e0; /* = divider MUI */
  --md-sys-color-inverse-surface: #2f3033; /* généré */
  --md-sys-color-surface-container: #ffffff; /* = background.paper MUI */
  /* … 29 rôles au total … */
}

:root {
  --md-sys-typescale-display-large-font: "Roboto", "Helvetica", "Arial", sans-serif;
  --md-sys-typescale-display-large-size: 96px;
  --md-sys-typescale-display-large-weight: 300;
  --md-sys-typescale-display-large-tracking: -1.5px;
  --md-sys-typescale-headline-small-size: 20px;
  --md-sys-typescale-headline-small-line-height: 32px; /* 1.6 * 20 */
  --md-sys-typescale-body-large-size: 16px;
  --md-sys-typescale-body-large-line-height: 24px; /* 1.5 * 16 */
  --md-sys-typescale-label-large-size: 14px;
  --md-sys-typescale-label-large-weight: 500;
  --md-sys-shape-corner-none: 0px;
  --md-sys-shape-corner-extra-small: 4px;
  --md-sys-shape-corner-small: 8px;
  --md-sys-shape-corner-medium: 12px; /* borderRadius:4 -> ratio 1.0 */
  --md-sys-shape-corner-large: 16px;
  --md-sys-shape-corner-extra-large: 28px;
}

@media (prefers-color-scheme: dark) {
  :root {
    --md-sys-color-primary: #90caf9;
    --md-sys-color-on-primary: #000000;
    --md-sys-color-primary-container: #004b71; /* généré (source dark) */
    --md-sys-color-secondary: #ce93d8;
    --md-sys-color-tertiary: #d0c0e8;
    --md-sys-color-error: #f44336;
    --md-sys-color-background: #121212;
    --md-sys-color-surface: #121212;
    --md-sys-color-on-surface: #ffffff;
    --md-sys-color-surface-container: #1e1e1e;
    /* … 29 rôles … */
  }
}
```

---

## 6. Pertes de fidélité M2 → M3 (à assumer / documenter)

| #   | Perte                                                        | Détail                                                                                                                                                                                                                                     | Mitigation                                                                  |
| --- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------- |
| 1   | **Rôles inventés**                                           | `tertiary*`, tous les `*-container`, `surface-variant`, `outline`, `inverse-*`, `scrim` n'ont **aucune source MUI** : MCU les génère depuis `primary.main`. Ce ne sont **pas** des couleurs choisies par le designer.                      | Revue visuelle ; au besoin, surcharge manuelle des `--md-sys-color-*` clés. |
| 2   | **Dérive de la teinte primary**                              | MCU recale `primary.main` sur une `TonalPalette` HCT : le `primary` émis peut différer du `main` MUI. **Le script ré-impose `primary.main` exact** par-dessus → fidèle, mais les containers dérivés restent calés sur la teinte _recalée_. | Override déjà appliqué pour primary/secondary/error.                        |
| 3   | **`on-*` recalculés**                                        | Si `contrastText` absent, le script choisit noir/blanc par luminance (≈ WCAG) — peut différer du choix MUI.                                                                                                                                | Fournir `contrastText` dans le thème MUI.                                   |
| 4   | **`background.paper` → `surface-container`**                 | M2 n'a que `default`/`paper` ; M3 a 6 niveaux de surface tonale. On ne mappe qu'une approximation.                                                                                                                                         | Voir #6.                                                                    |
| 5   | **Surfaces tonales absentes**                                | MCU 0.2.7 (`Scheme`) n'émet pas `surface-container-low/high/highest`, `surface-bright/dim`, `*-fixed`, `surface-tint`.                                                                                                                     | Fallbacks `<md-*>` internes ; sinon Material Theme Builder.                 |
| 6   | **Typo : 13 variants MUI ≠ 15 typescales M3**                | `title-large` et `body-small`/`label-medium` n'ont pas toujours de source MUI 1:1 ; `lineHeight` ratio → longueur fixe (perd l'adaptativité).                                                                                              | Compléter manuellement les typescales manquants.                            |
| 7   | **Shape : 1 valeur MUI → échelle M3**                        | MUI n'a qu'un `borderRadius` ; le script l'étend par ratio, mais c'est une heuristique (M3 a des coins per-composant distincts).                                                                                                           | Ajuster les `--md-sys-shape-corner-*` au cas par cas.                       |
| 8   | **`palette.light/dark`, `action.*`, `warning/info/success`** | Non mappés : M3 n'a pas `warning/info/success` (utiliser `tertiary` ou des couleurs custom MCU `CustomColor`). `action.hover/selected` → state layers M3 (`--md-sys-state-*`), hors scope script.                                          | Couleurs custom via `themeFromSourceColor(src, [{name,value,blend}])`.      |

---

## 7. Exemple avant / après concret

### Avant — thème MUI (React + Emotion)

```jsx
import { createTheme, ThemeProvider } from "@mui/material/styles";

const theme = createTheme({
  palette: {
    primary: { main: "#1976d2", contrastText: "#ffffff" },
    secondary: { main: "#9c27b0" },
    error: { main: "#d32f2f" },
    background: { default: "#fafafa", paper: "#ffffff" },
    divider: "#e0e0e0",
  },
  typography: { fontFamily: '"Roboto", sans-serif', body1: { fontSize: 16, lineHeight: 1.5 } },
  shape: { borderRadius: 4 },
});

<ThemeProvider theme={theme}>
  <Button variant="contained">OK</Button>
</ThemeProvider>;
```

### Après — tokens M3 + material-web (CSS pur, pas de provider)

`tokens.generated.css` (produit par le script) :

```css
:root {
  color-scheme: light dark;
  --md-sys-color-primary: #1976d2;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-primary-container: #d4e3ff;
  --md-sys-color-secondary: #9c27b0;
  --md-sys-color-error: #d32f2f;
  --md-sys-color-surface: #fafafa;
  --md-sys-color-on-surface: #212121;
  --md-sys-color-outline-variant: #e0e0e0;
  --md-sys-shape-corner-medium: 12px;
  --md-sys-typescale-body-large-size: 16px;
  --md-sys-typescale-body-large-line-height: 24px;
}
@media (prefers-color-scheme: dark) {
  :root {
    /* scheme dark */
  }
}
```

Markup (aucun `ThemeProvider` ; le token cascade vers `<md-*>`) :

```html
<link rel="stylesheet" href="tokens.generated.css" />
<script type="module">
  import "@material/web/button/filled-button.js";
</script>

<md-filled-button>OK</md-filled-button>
```

Génération dans un build :

```bash
bun migration/scripts/theme-to-tokens.ts > src/styles/tokens.generated.css
```

---

## Sources

- Contrat : `migration/00-CONVENTIONS.md` (§5 mapping tokens)
- Doc de référence : `docs/02-tokens-theming-web.md` (hiérarchie ref/sys/comp, MCU `themeFromSourceColor`/`applyTheme`, dark mode, Tailwind)
- Repo local M3 : `material-web/tokens/_md-sys-color.scss`, `material-web/tokens/_md-sys-typescale.scss`, `material-web/tokens/versions/v0_192/_md-sys-shape.scss`
- MCU local : `material-web/node_modules/@material/material-color-utilities/` (v0.2.7), `utils/theme_utils.js` (`applyTheme`), `utils/string_utils.js` (`argbFromHex`/`hexFromArgb`)
- MUI : `material-ui/packages/mui-material/` (`@mui/material@9.0.1`, palette M2, `createTheme`)
- [Design tokens – Material Design 3](https://m3.material.io/foundations/design-tokens) · [Material Web – Color](https://material-web.dev/theming/color/) · [npm @material/material-color-utilities](https://www.npmjs.com/package/@material/material-color-utilities)
- Script : `migration/scripts/theme-to-tokens.ts`
