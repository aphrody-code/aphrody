# Audit d'architecture — `packages/tailwindcss`

Audit en lecture seule du fork vendoré dans le monorepo aphrody, en vue de la fusion de trois systèmes UI : Tailwind CSS (`packages/tailwindcss`), shadcn/ui (`packages/ui`) et Material Design 3 web (`packages/material-web`).

Chemin audité : `C:\src\aphrody\packages\tailwindcss`. Aucune modification apportée au fork.

## 1. Identité et version

- Racine du workspace : `package.json` → `"name": "@tailwindcss/root"`, `"private": true`, `license: MIT`.
- Le paquet publié central est `packages/tailwindcss/package.json` → `"name": "tailwindcss"`, **`"version": "4.3.0"`**.
- Description : « A utility-first CSS framework for rapidly building custom user interfaces ».
- Il s'agit donc du **monorepo amont complet de Tailwind CSS v4** (tailwindlabs/tailwindcss), pas d'un simple paquet de distribution. Il embarque l'ensemble des sous-paquets d'écosystème : `@tailwindcss/cli`, `@tailwindcss/node`, `@tailwindcss/postcss`, `@tailwindcss/vite`, `@tailwindcss/webpack`, `@tailwindcss/browser`, `@tailwindcss/standalone`, `@tailwindcss/upgrade`, plus `internal-example-plugin`.

Note de cohérence repo : ce fork est du TypeScript/Rust amont. Il contredit la politique §2 du `CLAUDE.md` (« JS/TS BANNIS », « 100% Rust »). Le présent audit le documente tel quel, sans le faire évoluer.

## 2. Stack et langage

Architecture hybride **TypeScript (moteur de compilation CSS) + Rust (moteur d'analyse/scan « Oxide »)**.

### Couche TypeScript — moteur CSS
- Tout le cœur de compilation est en TS sous `packages/tailwindcss/src/` : parseur CSS (`css-parser.ts`), AST (`ast.ts`), parseur de valeurs (`value-parser.ts`), parseur de sélecteurs (`selector-parser.ts`), génération d'utilitaires (`utilities.ts`), variantes (`variants.ts`), tri (`sort.ts`, `property-order.ts`), `@apply` (`apply.ts`), `@import` (`at-import.ts`), design system (`design-system.ts`), theme (`theme.ts`), compile (`compile.ts`), IntelliSense (`intellisense.ts`).
- Couche de compatibilité v3 sous `src/compat/` (ex. `colors.ts`, `default-theme.ts`, `flatten-color-palette.ts`).
- TypeScript `^5.9.3`, build via `tsup`/`tsup-node`, tests via `vitest ^4.1.7` (et benchmarks `vitest bench`).

### Couche Rust — Oxide (`crates/`)
Workspace Cargo (`Cargo.toml`, `resolver = "2"`, `members = ["crates/*"]`, `lto = true` en release) :
- `crates/oxide` — paquet `tailwindcss-oxide` (v0.1.0, edition 2021). Rôle = **scanner de classes** haute performance : parcourt le code source des projets pour en extraire les candidats de classes utilitaires. Modules : `scanner`, `extractor`, `glob`, `paths`, `fast_skip`, `cursor`, `throughput`. Dépendances clés : `rayon` (parallélisme), `bstr`, `globwalk`/`fast-glob`/`walkdir`, `rustc-hash` (fxhash), `regex`, `bexpand`, `tracing`.
- `crates/classification-macros` — macros de classification de caractères (fast-skip).
- `crates/ignore` — fork local du crate `ignore` (gestion `.gitignore`).
- `crates/node` — binding NAPI (Rust → Node) exposant Oxide au moteur TS ; publié comme `@tailwindcss/oxide` (workspace `crates/node/npm/*`).

Séparation des responsabilités : **Rust scanne et détecte** les classes utilisées dans les fichiers ; **TypeScript compile** ces classes en CSS à partir du thème et des définitions d'utilitaires. C'est l'« oxide engine » mentionné dans le brief.

Autre composant Rust de l'écosystème : `lightningcss@1.32.0` (catalogué, patché via `patches/lightningcss@1.32.0.patch`) sert au minify/transform CSS final côté distribution.

## 3. Gestionnaire de paquets et système de build

- **Gestionnaire** : pnpm, épinglé `"packageManager": "pnpm@9.6.0"`. Workspace défini dans `pnpm-workspace.yaml` (membres : `crates/node`, `crates/node/npm/*`, `packages/*`, `playgrounds/*`, `integrations`).
- **Catalogue pnpm** (`catalog:`) verrouille les versions partagées : `@types/node 22.19.19`, `lightningcss 1.32.0` (+ toutes ses variantes de plateforme), `postcss 8.5.15`, `prettier 3.8.3`, `vite 8.0.14`, `webpack 5.107.0`.
- **Orchestrateur** : Turborepo (`turbo ^2.9.14`, `turbo.json`). Scripts racine : `build` = `turbo build --filter=!./playgrounds/*`, `dev` = `turbo dev …`, `lint` = `prettier --check . && turbo lint`.
- **Tests** : `test` = `cargo test && vitest run` (les deux toolchains), `test:integrations` (vitest sur `integrations/`), `test:ui` (Playwright `^1.60.0`).
- **Rust** : toolchain pinné via `rust-toolchain.toml` propre au fork ; `Cargo.lock` présent. Pas de NASM/aws-lc requis (scanner pur).
- **Patches** : `patches/@parcel__watcher@2.5.1.patch` et `patches/lightningcss@1.32.0.patch` (déclarés sous `pnpm.patchedDependencies`).

## 4. Modèle de design tokens / theming

C'est le point central pour la fusion. Tailwind v4 a **abandonné le fichier de config JS `tailwind.config.js`** au profit d'une définition **CSS-native via la règle `@theme`**, qui matérialise chaque token en **CSS custom property**.

### Mécanisme `@theme`
- Le thème par défaut vit dans `packages/tailwindcss/theme.css`, ouvert par `@theme default { … }`.
- `index.css` (point d'entrée du paquet) déclare l'ordre des couches et importe le thème :
  ```css
  @layer theme, base, components, utilities;
  @import './theme.css' layer(theme);
  @import './preflight.css' layer(base);
  @import './utilities.css' layer(utilities);
  ```
- Chaque entrée `@theme` est une variable CSS dont **le préfixe de namespace pilote la génération d'utilitaires** : `--color-*` → utilitaires de couleur (`bg-*`, `text-*`, `border-*`…), `--spacing` → échelle d'espacement, `--breakpoint-*` → variantes responsive, `--text-*` → tailles de police, `--font-*` → familles, `--radius-*`, `--shadow-*`, `--leading-*`, `--tracking-*`, `--container-*`, etc.
- La classe `Theme` (`src/theme.ts`) gère ces valeurs avec des `ThemeOptions` (bitflags) : `INLINE`, `REFERENCE`, `DEFAULT`, `STATIC`, `USED`. Elle gère aussi les namespaces et leurs exclusions (`ignoredThemeKeyMap` : ex. `--font` n'absorbe pas `--font-weight`/`--font-size`).
- Effacement/override : `--namespace-*: initial;` vide un namespace ; `--*: initial;` vide tout le thème (utile pour repartir d'une base propre).

### Tokens concrets (extraits de `theme.css`)
- **Couleurs** : palette complète en **espace `oklch()`** (red/orange/amber/yellow/lime/green/emerald/teal/cyan/sky… ×11 nuances 50→950). Ex. `--color-red-500: oklch(63.7% 0.237 25.331);`. C'est exactement l'espace colorimétrique de Material 3 et de la skill `color-expert` du repo.
- **Typographie** : `--font-sans/serif/mono` ; échelle `--text-xs … --text-9xl` avec **line-height appairé** via la convention `--text-<size>--line-height` (ex. `--text-base--line-height: calc(1.5 / 1)`). `--tracking-*` (letter-spacing) et `--leading-*` (line-height nommés).
- **Espacement** : token de base unique `--spacing: 0.25rem;` ; toutes les classes (`p-4`, `gap-2`, `m-8`…) sont dérivées par multiplication dynamique de cette base (échelle fluide, plus de map figée).
- **Breakpoints** : `--breakpoint-sm: 40rem` / `md: 48rem` / `lg: 64rem` / `xl: 80rem` / `2xl: 96rem`. `--container-*` (3xs→7xl) pour les container queries.
- **Rayons** : `--radius-xs … --radius-4xl` (+ `--radius` legacy). **Ombres** : `--shadow-2xs … --shadow-2xl`, `--shadow-inner`, et `--text-shadow-*`.

### Conséquence clé
Tous les tokens sont **émis comme variables CSS dans `:root`**, donc lisibles/surchageables à l'exécution sans recompiler. Le thème devient un contrat CSS public, pas un objet JS interne.

## 5. Approche de styling

- **Utility-first** : génération à la demande de classes atomiques (une propriété par classe), tri déterministe (`sort.ts`, `property-order.ts`).
- **Pipeline** : Oxide (Rust) scanne les sources → liste de candidats → le moteur TS (`compile.ts`/`utilities.ts`) ne génère que le CSS des classes réellement employées, résolu contre le thème.
- **Cascade layers** natives (`@layer theme, base, components, utilities`) pour un ordre de spécificité prévisible et facilement surchargeable.
- **Variantes** : système de variantes (`variants.ts`) + variantes custom utilisateur via `@custom-variant`.
- Pas d'étape PurgeCSS séparée : le tree-shaking est intrinsèque (génération basée sur l'usage détecté).

## 6. Surface d'API / configuration

Configuration **principalement CSS-native** (v4), avec une couche de compat JS optionnelle. At-rules détectées/traitées dans `src/index.ts` :
- `@theme` — définition des tokens (voir §4).
- `@utility <name>` — déclaration d'utilitaires custom (statiques ou fonctionnels avec suffixe `-*`). Validation stricte du nom (alphanumérique, minuscule, `-*` final unique).
- `@custom-variant <name>` / `@variant` — variantes custom (sélecteur ou corps avec `@slot`).
- `@source "…"` — déclaration explicite des chemins à scanner (chemins obligatoirement quotés).
- `@apply` — composition de classes utilitaires dans des règles CSS.
- `@plugin` / `@config` — pont de compatibilité vers les plugins/config JS de l'écosystème v3 (`src/plugin.ts`, `src/compat/`).
- `@import` (`at-import.ts`) — y compris `@reference "…"` pour importer un thème sans dupliquer le CSS.
- Exports du paquet (`package.json`) : `./theme.css`, `./preflight.css`, `./utilities.css`, `./index.css`, plus `./plugin`, `./defaultTheme`, `./colors`, `./lib/util/flattenColorPalette` (chemins de compat v3).

Surface JS encore disponible (legacy) : `tailwindcss/plugin`, `tailwindcss/defaultTheme`, `tailwindcss/colors` — utile comme passerelle de migration.

## 7. Points d'intégration et conflits potentiels

Voisins dans le monorepo (confirmés par inspection) :
- `packages/ui` (shadcn/ui) — workspace pnpm propre, paquet `packages/shadcn`. **Déjà aligné Tailwind v4** : `packages/ui/packages/shadcn/src/tailwind.css` utilise `@theme inline { … }`, `@custom-variant data-open/closed/checked/…`, `@utility no-scrollbar`, `@keyframes`. shadcn consomme donc nativement la surface d'API de ce fork.
- `packages/material-web` (MD3 web) — composants Lit + tokens **SCSS** (`tokens/_md-comp-*.scss`, `color/_color.scss`) émettant des custom properties `--md-sys-color-*`, `--md-sys-typescale-*`, `--md-sys-*`. Modèle de tokens parallèle à celui de Tailwind, mais dans un autre namespace et un autre outillage (Sass vs `@theme`).

Conflits potentiels :
1. **Deux espaces de noms de tokens** : Tailwind émet `--color-*`, `--spacing`, `--text-*`, `--radius-*` ; Material 3 émet `--md-sys-color-*`, `--md-sys-typescale-*`. Sans pont, on duplique sémantiquement (primary, surface, etc.).
2. **Outillage hétérogène** : Tailwind = `@theme` CSS + scanner Rust ; MD3 = compilation Sass. Pipelines de build distincts (Turbo/pnpm côté TW, npm/Sass côté material-web — `package-lock.json` présent, donc gestionnaire différent du pnpm racine).
3. **Modèles de styling opposés** : utility-first (classes atomiques globales) vs Web Components encapsulés (Shadow DOM) — les classes utilitaires Tailwind **ne traversent pas le Shadow DOM** des composants MD3 ; seules les custom properties héritées passent la frontière du shadow.
4. **Preflight/reset** : `preflight.css` (layer `base`) de Tailwind peut entrer en tension avec les styles internes des composants MD3 ; à confiner via cascade layers.
5. **Politique repo** : fork TS/Rust amont vs exigence « 100% Rust / WASM » du `CLAUDE.md` — décision d'architecture à trancher (vendoriser tel quel vs réimplémenter le moteur en Rust).

## Points de fusion

Stratégie de convergence des trois systèmes, en exploitant que **les trois reposent in fine sur des CSS custom properties**.

1. **Tailwind comme couche utilitaire transverse, MD3 comme couche composants, shadcn comme couche primitives.** Tailwind fournit les utilitaires de mise en page/espacement/typo ; MD3 fournit les composants riches (Web Components) ; shadcn fournit les primitives React/headless. Les trois lisent le même contrat de variables CSS.

2. **Un seul jeu de tokens canonique exposé via `@theme`.** Faire de `@theme` le point d'entrée unique des tokens et y **mapper les tokens MD3** : déclarer dans le bloc `@theme` les `--color-primary`, `--color-surface`, etc. en pointant vers/dérivant des `--md-sys-color-*`. Tailwind générera alors `bg-primary`, `text-on-surface`… directement adossés aux tokens Material 3. L'alignement est facilité par le **fait que les deux systèmes utilisent déjà `oklch()`** (palette Tailwind en oklch ; M3 HCT/oklch).

3. **`@theme inline` pour le pont sémantique (pattern déjà éprouvé par shadcn).** shadcn fait exactement cela dans `src/tailwind.css` : il mappe ses variables sémantiques (`--background`, `--primary`…) en tokens Tailwind via `@theme inline`. Réutiliser ce pattern pour brancher les `--md-sys-*` de Material 3 : `@theme inline { --color-primary: var(--md-sys-color-primary); … }`. C'est le mécanisme de fusion le plus direct, sans toucher le moteur.

4. **Source de tokens unique en amont.** Générer les trois familles de variables (`--color-*` Tailwind, `--md-sys-*` MD3, `--background`/`--primary`/… shadcn) à partir d'**une seule source de vérité** (le seed M3 du repo, cf. `docs/` design tokens et `crates/m3-tokens`). Tailwind/oklch et MD3 partageant l'espace oklch, un même couple teinte/chroma alimente les deux sorties.

5. **Isolation par cascade layers + Shadow DOM piercing.** Confiner `preflight` au layer `base` et placer les overrides MD3 dans un layer dédié pour éviter les collisions. Pour styliser les composants MD3 (Shadow DOM) depuis Tailwind, ne compter que sur l'héritage des custom properties (qui traversent le shadow) — pas sur les classes utilitaires, qui restent côté light DOM (primitives shadcn, layout).

6. **Custom variants partagées.** Étendre les `@custom-variant` de shadcn (`data-open`, `data-checked`…) pour couvrir aussi les états/attributs des composants MD3, afin d'avoir un même vocabulaire de variantes sur les trois couches.

Synthèse : Tailwind v4 est le **liant** idéal car son thème EST déjà du CSS custom-property pur — la fusion se fait par mapping de tokens dans `@theme`/`@theme inline` (pattern shadcn existant) plutôt que par réécriture, MD3 conservant ses Web Components et son émission `--md-sys-*` comme source des valeurs.
