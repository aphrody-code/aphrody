---
title: "Tailwind & MD3"
nav_order: 7
---

# 05 — L'écosystème Tailwind face à Material Design 3

> Portée : ce document explore en profondeur le repo `material-tailwind/` (Creative Tim), puis le situe dans l'écosystème Tailwind pour bâtir un design system Material Design 3 (MD3), en comparant trois approches — `material-tailwind` (composants « Material-inspired »), Tailwind CSS nu (moteur utility/CSS) et `shadcn/ui` (modèle registry + tokens CSS) — et en évaluant la faisabilité réelle d'un vrai MD3 en Tailwind.

---

## 1. material-tailwind : exploration du repo

### 1.1 Nature du repo cloné

Le repo `material-tailwind/` (origine `creativetimofficial/material-tailwind`) est en réalité **deux choses dans un même dépôt** :

1. **Le site de documentation/marketing** à la racine — une app **Next.js 13** (`next: 13.1.1` dans `material-tailwind/package.json`), avec `pages/`, `docs-content/`, `documentation/`, `widgets/`. Le `package.json` racine a `"version": "0.0.0"` et n'est pas publié.
2. **Les packages réellement distribués sur npm**, dans `material-tailwind/packages/` (monorepo pnpm, cf. `material-tailwind/pnpm-workspace.yaml` → `packages/*`).

Trois packages :

| Package                    | Chemin                                                 | Version    | Rôle                                   |
| -------------------------- | ------------------------------------------------------ | ---------- | -------------------------------------- |
| `@material-tailwind/react` | `material-tailwind/packages/material-tailwind-react/`  | **2.1.10** | Composants React + ThemeProvider       |
| `@material-tailwind/html`  | `material-tailwind/packages/material-tailwind-html/`   | **2.3.2**  | Plugin Tailwind + CSS/JS pour HTML pur |
| `create-material-tailwind` | `material-tailwind/packages/create-material-tailwind/` | —          | Scaffolder CLI                         |

Le clone est sur `main` au voisinage du tag `2.3.2-html` (`git describe` → `2.3.2-html-104-g1a343ad1`), dernier commit daté du 2026-04-28 (commits de docs/optimisation, pas de la lib).

### 1.2 Ce que fournit la librairie

Deux livrables complémentaires, **un seul système de thème partagé** :

- **`@material-tailwind/react`** : composants React (`.tsx`) qui consomment un thème objet et émettent des **chaînes de classes Tailwind**. Dépendances clés (`material-tailwind/packages/material-tailwind-react/package.json`) : `@floating-ui/react` (positionnement popover/menu/tooltip), `framer-motion@6` (animations), `material-ripple-effects` (ondulation Material au clic), `tailwind-merge` + `classnames` + `deepmerge` (composition de classes et merge de thème). `react`/`react-dom` en `peerDependencies` (`^16 || ^17 || ^18`).
- **`@material-tailwind/html`** : pas de runtime React. Il expose un **plugin Tailwind** via la fonction `withMT()` (`material-tailwind/packages/material-tailwind-html/utils/withMT.js`) + un CSS compilé et un `ripple.js`. C'est ce package qui porte les tokens de base.

Ce n'est donc **pas** un simple plugin Tailwind façon « ajoute des classes » : c'est une **librairie de composants** dont le styling est exprimé en utilities Tailwind, paramétrée par un thème injecté dans la config Tailwind par `withMT()`.

#### Le mécanisme `withMT()`

`material-tailwind/packages/material-tailwind-html/utils/withMT.js` fait un `deepmerge` de la config Tailwind de l'utilisateur avec un bloc Material :

```js
const materialTailwindConfig = {
  content: [],
  theme: {
    colors, // theme/base/colors.js
    fontFamily: typography,
    boxShadow: shadows,
    screens: breakpoints,
  },
  safelist: ["hidden"],
  plugins: [],
};

function withMT(tailwindConfig) {
  return merge(materialTailwindConfig, { ...tailwindConfig });
}
```

Usage côté projet (cf. `material-tailwind/tailwind.config.js`) :

```js
const withMT = require("@material-tailwind/react/utils/withMT");
module.exports = withMT({ content: [...], theme: { extend: {} } });
```

C'est une approche **Tailwind v3** (config JS, `tailwind.config.js`, directives `@tailwind base/components/utilities` dans `material-tailwind/packages/material-tailwind-html/styles/tailwind.css`). `tailwindcss` est en devDependency `3.3.6` (racine) / `^3.2.4` (html).

### 1.3 Le système de thème (tokens)

Les tokens de base vivent dans `material-tailwind/packages/material-tailwind-react/src/theme/base/` :

- `colors.js` — palette de couleurs
- `typography.js` — `fontFamily`
- `shadows.js` — `boxShadow`
- `breakpoints.js` — `screens`

**Point décisif sur Material Design :** `colors.js` est la **palette Material Design 2 (2014)** — les familles `blue-gray`, `brown`, `deep-orange`, `gray` avec échelles `50→900` et les hex historiques de Material (`#607d8b`, `#795548`, `#ff5722`…). Aucune notion de rôles M3 (`primary`/`on-primary`/`surface-container`…), aucun token `--md-sys-*`, aucune palette tonale 0–100.

Les **styles par composant** sont des objets de classes Tailwind, ex. `material-tailwind/packages/material-tailwind-react/src/theme/components/button/` :

```
buttonFilled.ts  buttonGradient.ts  buttonOutlined.ts  buttonText.ts  index.ts
```

`buttonFilled.ts` mappe chaque couleur vers des classes :

```js
gray: {
  background: "bg-gray-900",
  color: "text-white",
  shadow: "shadow-md shadow-gray-900/10",
  hover: "hover:shadow-lg hover:shadow-gray-900/20",
  focus: "focus:opacity-[0.85] focus:shadow-none",
  active: "active:opacity-[0.85] active:shadow-none",
}
```

Les variantes de Button sont **`filled | gradient | outlined | text`** (`button/index.ts`). À noter : `gradient` n'existe pas dans M3, et les variantes M3 manquantes sont **`elevated`** et **`tonal`**. Le `defaultProps.color` est `gray` (neutre), pas une couleur primaire dérivée d'un seed. Le ThemeProvider permet d'override ce thème par `deepmerge`.

### 1.4 Inventaire des composants

**34 composants React** (`material-tailwind/packages/material-tailwind-react/src/components/`), avec un thème miroir dans `.../src/theme/components/` :

```
Accordion  Alert  Avatar  Badge  Breadcrumbs  Button  ButtonGroup  Card
Carousel  Checkbox  Chip  Collapse  Dialog  Drawer  IconButton  Input
List  Menu  Navbar  Popover  Progress  Radio  Rating  Select  Slider
SpeedDial  Spinner  Stepper  Switch  Tabs  Textarea  Timeline  Tooltip  Typography
```

Le package HTML (`material-tailwind/packages/material-tailwind-html/theme/base/`) couvre la même base de tokens (sans le runtime React).

### 1.5 Rapport à Material Design : M2, pas M3

Le positionnement officiel est « **inspired by Material Design** » (`material-tailwind/package.json` : _"easy-to-use components library for Tailwind CSS inspired by Material Design"_). La preuve interne la plus nette est dans les release notes du repo :

`material-tailwind/docs-content/react/releases.ts:921` et `docs-content/html/releases.ts:423` :

> "Started project from Tailwind CSS and **Material Design 2**"
> "Added design from **Material Design 2** using Tailwind CSS"

Un `grep` exhaustif du repo (hors `node_modules`) **ne renvoie aucune** occurrence de `Material Design 3`, `MD3`, `md-sys`, `Material You` ou `dynamic color`. Conclusion : **material-tailwind est du Material Design 2 « inspired »**, transposé en utilities Tailwind. Ce n'est pas une implémentation MD3, ni token-complete vis-à-vis de la spec M3.

### 1.6 Licence et statut de maintenance (2026)

- **Licence : MIT** (`material-tailwind/LICENSE.md`, © 2021-2023 Material Tailwind / Creative Tim). Réutilisation libre.
- **Maintenance : ralentie / quasi-gelée.** Le stable npm `@material-tailwind/react@2.1.10` n'a pas de release depuis ~2 ans. Une v3 existe mais reste **bloquée en beta** (`3.0.0-beta.6` côté React, `3.0.0-beta.7` côté HTML) depuis début 2025, sans stable. Backlog d'issues ouvertes important (~180+). Creative Tim a en partie réorienté l'effort vers d'autres produits (« David AI »).
- **Tailwind v4 : non supporté proprement.** L'architecture repose sur `withMT()` (config JS Tailwind v3), en conflit frontal avec le modèle **CSS-first** de Tailwind v4 (plus de `tailwind.config.js` requis, `@theme` en CSS). C'est un point de douleur communautaire ouvert, sans réponse officielle propre.

---

## 2. Tailwind CSS (v4) : le moteur, pas le design system

Repo `tailwindcss/` = **Tailwind CSS v4.3.0** (`tailwindcss/packages/tailwindcss/package.json`). Monorepo (engine Rust dans `crates/`, packages JS dans `packages/`). Tailwind **n'est pas un design system** : c'est un moteur d'**utilities** qui génère des classes à partir de **tokens de thème**. Aucune opinion esthétique Material — juste des primitives (espacement, couleur, typo, radius, ombres).

### 2.1 Le mécanisme `@theme` (v4)

En v4, les tokens sont déclarés en **CSS** via `@theme`, et chaque variable génère automatiquement les utilities correspondantes. `tailwindcss/packages/tailwindcss/index.css` :

```css
@layer theme, base, components, utilities;
@import "./theme.css" layer(theme);
@import "./preflight.css" layer(base);
@import "./utilities.css" layer(utilities);
```

`tailwindcss/packages/tailwindcss/theme.css` (extrait) — les couleurs par défaut sont en **OKLCH** :

```css
@theme default {
  --font-sans: ui-sans-serif, system-ui, sans-serif, ...;
  --color-red-500: oklch(63.7% 0.237 25.331);
  --color-orange-500: oklch(70.5% 0.213 47.604);
  /* ... */
}
```

Règle clé : `--color-primary: <valeur>` dans `@theme` **génère** `bg-primary`, `text-primary`, `border-primary`, etc. C'est exactement le point d'ancrage pour brancher des **tokens M3**.

### 2.2 Comment y brancher des tokens M3

On pose les rôles M3 comme custom properties, puis on les expose à Tailwind via `@theme inline` (qui référence des variables existantes sans les recopier) :

```css
:root {
  --md-sys-color-primary: oklch(48% 0.18 265);
  --md-sys-color-on-primary: oklch(100% 0 0);
  --md-sys-color-surface: oklch(98% 0.01 265);
  --md-sys-color-surface-container: oklch(94% 0.012 265);
  /* ...les ~26 rôles de couleur M3... */
}

@theme inline {
  --color-primary: var(--md-sys-color-primary);
  --color-on-primary: var(--md-sys-color-on-primary);
  --color-surface: var(--md-sys-color-surface);
  --color-surface-container: var(--md-sys-color-surface-container);
}
```

→ on obtient `bg-primary text-on-primary`, `bg-surface-container`, etc. Le dark mode = re-déclarer les `--md-sys-color-*` sous un sélecteur `.dark`. C'est le **socle technique** d'un MD3 en Tailwind v4 : Tailwind fournit le pipeline tokens→utilities, à nous d'apporter les _valeurs_ et la _sémantique_ M3.

---

## 3. shadcn/ui : pas Material, mais le bon **modèle**

Repo `shadcn-ui/` = monorepo de `shadcn/ui` (CLI `packages/shadcn/`, app de doc/registry `apps/v4/`).

### 3.1 Ce n'est PAS Material Design

shadcn/ui est **neutre par construction** : esthétique sobre, sans surcouche Material. Le primary par défaut est une nuance quasi-noire neutre (`shadcn-ui/apps/v4/app/globals.css`) :

```css
:root {
  --primary: oklch(0.205 0 0); /* near-black, chroma 0 */
  --background: oklch(1 0 0);
}
```

Pas d'élévation Material, pas de ripple, pas de palette tonale, pas de rôles `on-*`/`surface-container`. Donc **shadcn ≠ MD3**. Mais son **modèle de distribution et de tokens est exactement ce qu'il faut** pour bâtir un MD3 custom.

### 3.2 Modèle de distribution : registry copy-paste

shadcn/ui n'est **pas une dépendance npm** de composants. C'est un **registry** : la CLI (`shadcn-ui/packages/shadcn/`) lit un `registry.json` (`shadcn-ui/apps/v4/registry.json`) et **copie le code source** des composants dans ton projet (`apps/v4/registry/new-york-v4/ui/`, **56 composants `.tsx`**). Tu **possèdes** le code → tu peux le réécrire pour MD3 (élévation, states layers, formes M3) sans te battre contre une lib verrouillée.

Les composants s'appuient sur des **primitives accessibles** : Radix UI (`import { Dialog as DialogPrimitive } from "radix-ui"` dans `apps/v4/registry/new-york-v4/ui/dialog.tsx`) et `@base-ui/react`, plus `lucide-react` pour les icônes. L'a11y (focus, ARIA, clavier) vient des primitives, pas du styling — donc réutilisable tel quel pour un thème Material.

### 3.3 Système de tokens : CSS variables sémantiques + `@theme inline`

C'est le cœur transférable. Dans `shadcn-ui/apps/v4/app/globals.css` :

```css
@theme inline {
  --color-background: var(--background);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-muted: var(--muted);
  --color-accent: var(--accent);
  /* ... */
}
:root {
  --primary: oklch(0.205 0 0);
  --primary-foreground: oklch(0.985 0 0);
  /* ... */
}
.dark {
  /* re-déclaration des mêmes variables */
}
```

Deux niveaux : (1) des **variables sémantiques** (`--primary`, `--background`, paires `*/*-foreground`) en OKLCH dans `:root`/`.dark` ; (2) `@theme inline` qui les **promeut en utilities Tailwind**. C'est **structurellement isomorphe** au modèle M3 (rôles sémantiques + paires couleur/on-couleur). Construire MD3 = remplacer ces valeurs neutres par les rôles M3 et étendre le set de tokens.

---

## 4. Comparaison des 3 approches face à la conformité MD3

| Critère                                             | material-tailwind                | Tailwind CSS nu (v4)                     | shadcn/ui                                 |
| --------------------------------------------------- | -------------------------------- | ---------------------------------------- | ----------------------------------------- |
| **Nature**                                          | Lib de composants (React + HTML) | Moteur utility/CSS                       | Registry de composants (copy-paste)       |
| **Version (clone)**                                 | `react@2.1.10`, `html@2.3.2`     | `4.3.0`                                  | monorepo (apps/v4)                        |
| **Base Tailwind**                                   | v3 (`withMT()`, config JS)       | v4 (`@theme`, CSS-first)                 | v4 (`@theme inline`)                      |
| **Référence Material**                              | **MD2 "inspired"**               | Aucune (neutre)                          | Aucune (neutre)                           |
| **Rôles M3 (`primary`/`on-*`/`surface-container`)** | Non                              | Non (à fournir)                          | Non, mais structure de tokens compatible  |
| **Palette tonale 0–100 / HCT**                      | Non (palette MD2 50–900)         | Non                                      | Non                                       |
| **Dynamic color (seed → schéma)**                   | Non                              | Non                                      | Non                                       |
| **Élévation Material**                              | Partiel (ombres + ripple)        | Non                                      | Non (ombres neutres)                      |
| **A11y des primitives**                             | Maison (`@floating-ui`)          | N/A                                      | **Radix / Base UI** (fort)                |
| **Propriété du code / customisation**               | Faible (lib npm)                 | Totale                                   | **Totale** (code copié)                   |
| **Maintenance 2026**                                | Faible / v3 gelée en beta        | Très active                              | Très active                               |
| **Conformité MD3 réelle**                           | **Faible** (c'est du MD2)        | **Nulle telle quelle**, mais socle idéal | **Nulle telle quelle**, mais modèle idéal |
| **Licence**                                         | MIT                              | MIT                                      | MIT                                       |

**Lecture :** aucune des trois n'est MD3 « out of the box ». material-tailwind est le plus proche _visuellement_ de Material (ripple, ombres, vocabulaire) mais c'est **MD2**, coincé en v3/beta. Tailwind nu et shadcn ne sont pas Material du tout, mais offrent **le bon pipeline** (Tailwind v4 `@theme`) et **le bon modèle** (shadcn : tokens sémantiques + code possédé + a11y Radix) pour _construire_ MD3.

---

## 5. Faisabilité d'un vrai MD3 en Tailwind

Oui, c'est faisable — en **assemblant Tailwind v4 (moteur) + le modèle shadcn (tokens/registry/a11y) + les valeurs et la sémantique M3**. Détail.

### 5.1 Mapping des tokens : `--md-sys-*` → `@theme`

La spec M3 expose ~26 rôles de couleur, des tokens de typo (`display/headline/title/body/label`), de forme (`corner-*`) et d'élévation (`level0–5`). On les pose en `:root` puis on les promeut :

```css
:root {
  /* couleur — rôles M3 */
  --md-sys-color-primary: oklch(48% 0.18 265);
  --md-sys-color-on-primary: oklch(100% 0 0);
  --md-sys-color-primary-container: oklch(90% 0.05 265);
  --md-sys-color-on-primary-container: oklch(20% 0.07 265);
  --md-sys-color-surface: oklch(98% 0.005 265);
  --md-sys-color-surface-container-high: oklch(92% 0.01 265);
  --md-sys-color-outline: oklch(55% 0.02 265);
  /* secondary, tertiary, error, *-container, on-*, surface-*… */

  /* forme */
  --md-sys-shape-corner-small: 8px;
  --md-sys-shape-corner-medium: 12px;
  --md-sys-shape-corner-large: 16px;

  /* élévation */
  --md-sys-elevation-level1: 0 1px 2px 0 rgb(0 0 0 / 0.3), 0 1px 3px 1px rgb(0 0 0 / 0.15);
}

@theme inline {
  --color-primary: var(--md-sys-color-primary);
  --color-on-primary: var(--md-sys-color-on-primary);
  --color-primary-container: var(--md-sys-color-primary-container);
  --color-surface: var(--md-sys-color-surface);
  --color-surface-container-high: var(--md-sys-color-surface-container-high);
  --color-outline: var(--md-sys-color-outline);

  --radius-md3-sm: var(--md-sys-shape-corner-small);
  --radius-md3-md: var(--md-sys-shape-corner-medium);
  --radius-md3-lg: var(--md-sys-shape-corner-large);

  --shadow-md3-1: var(--md-sys-elevation-level1);
}
```

On code ensuite les composants (en partant des `.tsx` copiés du registry shadcn) avec `bg-primary text-on-primary rounded-md3-lg shadow-md3-1`, etc. Le dark scheme = re-déclarer les `--md-sys-color-*` sous `.dark`.

### 5.2 Ce qui manque dans Tailwind (et qu'il faut ajouter)

| Manque côté Tailwind/shadcn                                         | Pourquoi c'est M3-spécifique                                                             | Solution à ajouter                                                                                                             |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **HCT (Hue-Chroma-Tone)**                                           | M3 définit ses couleurs dans l'espace HCT (perception-uniforme Google), pas en OKLCH/HSL | Pré-calculer hors-build (lib `@material/material-color-utilities`) et **émettre du OKLCH/hex** dans les `--md-sys-color-*`     |
| **Dynamic color (seed → schéma complet)**                           | M3 génère ~26 rôles light+dark depuis 1 couleur source via les palettes tonales          | Script de build qui prend un seed, appelle `material-color-utilities`, écrit le CSS des tokens. Tailwind ne fait pas ce calcul |
| **Palettes tonales 0–100**                                          | Base de la dérivation M3 (containers, surfaces)                                          | Générées par le même script ; non fournies par Tailwind (qui a des échelles 50–950 non-tonales)                                |
| **State layers** (hover/focus/press = overlay on-color à 8/12/16 %) | Modèle d'interaction M3 standardisé                                                      | Utilities/plugin custom : pseudo-élément overlay `color-mix(in oklch, var(--md-sys-color-on-surface) 8%, transparent)`         |
| **Élévation tonale + ombre**                                        | M3 combine ombre _et_ teinte de surface selon le niveau                                  | Tokens `--md-sys-elevation-*` + variation de `surface-container-*`                                                             |
| **Typo scale M3** (rôles display/headline/title/body/label × 3)     | Échelle nommée, pas une simple `text-sm/lg`                                              | Mapper en `--text-*` custom dans `@theme`                                                                                      |
| **Ripple / motion M3**                                              | Animation Material (easing/durée standardisés)                                           | JS (cf. `material-ripple-effects`) + tokens d'easing/durée                                                                     |

En résumé : Tailwind v4 + shadcn fournissent **le système de distribution des tokens et l'a11y** ; il faut ajouter **la couche couleur M3 (HCT + dynamic color + palettes tonales)**, **les state layers**, **l'élévation tonale** et **la motion**. Aucun de ces éléments n'est livré par les trois repos — ils sont l'apport propre d'un design system MD3 custom.

### 5.3 Architecture recommandée (chemin pragmatique)

1. **Moteur** : Tailwind CSS v4 (`@theme` / `@theme inline`) — cf. `tailwindcss/packages/tailwindcss/theme.css`.
2. **Génération des tokens** : script build `@material/material-color-utilities` (seed → schéma M3) → écrit `--md-sys-color-*` (light/.dark) + palettes tonales en OKLCH.
3. **Exposition** : `@theme inline` mappe `--md-sys-*` → utilities (`--color-primary`, `--radius-md3-*`, `--shadow-md3-*`).
4. **Composants + a11y** : partir des `.tsx` du registry shadcn (`shadcn-ui/apps/v4/registry/new-york-v4/ui/`), Radix/Base UI pour le comportement, re-styler en classes M3.
5. **State layers + ripple + motion** : plugin/utilities custom + `color-mix()`.

---

## 6. Quand choisir la voie Tailwind pour MD3 ?

**Choisir Tailwind v4 + modèle shadcn (DS MD3 custom)** quand :

- On veut **MD3 véritable** (rôles sémantiques, dynamic color, dark scheme cohérent) **tout en gardant le pipeline utility Tailwind** et la pleine propriété du code.
- On a besoin d'a11y solide _sans_ dépendre du styling d'une lib (Radix/Base UI découplent comportement et apparence).
- On accepte d'**investir** dans la couche couleur M3 (HCT, génération de tokens, state layers) — non fournie par l'écosystème.

**Choisir `material-tailwind`** seulement quand :

- On vit déjà en **Tailwind v3**, on veut un look « Material » rapide (ripple/ombres) **sans exigence de conformité M3**, et on accepte une lib **MD2** en maintenance ralentie. À éviter pour tout nouveau projet ciblant Tailwind v4 ou MD3 strict.

**Éviter Tailwind entièrement** et préférer un DS Material natif (`material-web` Web Components, ou `@mui/material` côté React) quand :

- La **conformité MD3 stricte (HCT, dynamic color, motion, états) est non-négociable** et qu'on ne veut pas la réimplémenter. Ces librairies portent nativement les tokens `--md-sys-*` et la dynamic color, là où Tailwind exige de tout assembler soi-même.

---

## 7. Moteur de compilation Rust (Tailwind Oxide) & CLI

Tailwind CSS v4 introduit un moteur d'extraction de classes et de variables écrit en Rust (`tailwind-oxide`), remplaçant les expressions régulières JavaScript lourdes par des automates finis performants et parallélisés.

### Innovations de Tailwind Oxide

- **State-Machine Extractor (FSM) :** Analyse du flux de caractères au niveau des octets (`&[u8]`) via un curseur léger. Le typestate Rust (`ArbitraryPropertyMachine<State>`) est utilisé pour coder les étapes d'analyse dans les types du compilateur, ce qui permet des sauts rapides et une optimisation fine sans variables d'état globales.
- **Classificateurs d'octets branchless :** Un macro de dérivation génère des tables de recherche statiques directes d'octets (0..255) vers des enums, éliminant les branches conditionnelles dans les boucles de traitement critique.
- **Saut SIMD des blancs :** Skipping vectorisé par blocs de 16 octets (`[u8; 16]`) pour sauter instantanément l'indentation et les espaces des fichiers HTML, PHP ou TSX.
- **Parcours parallèle sans verrou (Lock-free) :** Collecte des fichiers modifiés dans des tampons locaux de thread (capacité 256) avant de fusionner via un unique verrou Mutex global, évitant la contention de thread lors des scans.
- **Tree-shaking dynamique de variables CSS :** Le compilateur analyse les dépendances des variables CSS générées et supprime automatiquement de l'AST final toutes les variables ou animations non utilisées, réduisant la taille du CSS final.

### Utilisation du CLI Tailwind

Le CLI Tailwind v4 est configuré et installé dans ce monorepo en tant que dépendance de développement. Pour lancer la compilation :

```bash
# Compilation unitaire minifiée
bunx tailwindcss -i src/theme.css -o dist/theme.css --minify

# Mode écoute pour le développement incrémental
bunx tailwindcss -i src/theme.css -o dist/theme.css --watch
```

---

## Annexe — Chemins repo cités

- `material-tailwind/package.json`, `material-tailwind/LICENSE.md`, `material-tailwind/tailwind.config.js`, `material-tailwind/pnpm-workspace.yaml`
- `material-tailwind/packages/material-tailwind-react/package.json` · `.../src/components/` (34 composants) · `.../src/theme/base/{colors,typography,shadows,breakpoints}.js` · `.../src/theme/components/button/{buttonFilled,buttonGradient,buttonOutlined,buttonText,index}.ts`
- `material-tailwind/packages/material-tailwind-html/package.json` · `.../utils/withMT.js` · `.../styles/tailwind.css` · `.../theme/base/`
- `material-tailwind/docs-content/react/releases.ts:921` · `material-tailwind/docs-content/html/releases.ts:423` (« Material Design 2 »)
- `tailwindcss/packages/tailwindcss/package.json` (v4.3.0) · `.../index.css` · `.../theme.css` (`@theme default`, OKLCH)
- `shadcn-ui/apps/v4/app/globals.css` (`@theme inline` + `:root`/`.dark`) · `shadcn-ui/apps/v4/registry/new-york-v4/ui/` (56 composants, `dialog.tsx` → `radix-ui`) · `shadcn-ui/apps/v4/registry.json` · `shadcn-ui/packages/shadcn/` (CLI)

## Annexe — Sources web (statut 2026)

- [@material-tailwind/react — npm](https://www.npmjs.com/package/@material-tailwind/react)
- [@material-tailwind/html — npm](https://www.npmjs.com/package/@material-tailwind/html)
- [Releases — creativetimofficial/material-tailwind](https://github.com/creativetimofficial/material-tailwind/releases)
- [Material Tailwind v3](https://www.material-tailwind.com/v3)
- [Install Material Tailwind — tailwindlabs/tailwindcss Discussion #15958 (conflit Tailwind v4)](https://github.com/tailwindlabs/tailwindcss/discussions/15958)
- [Tailwind CSS v4.0 (blog officiel)](https://tailwindcss.com/blog/tailwindcss-v4)
- [tailwind-material-3 (alternative communautaire MD3)](https://github.com/rinturaj/tailwind-material-3)
