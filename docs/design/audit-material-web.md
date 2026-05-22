# Audit d'architecture — `packages/material-web`

Audit en lecture seule du fork de `material-components/material-web` (Material Design 3 web) vivant dans le monorepo aphrody. Objectif : éclairer la fusion d'un système UI unifié réunissant Tailwind CSS (`packages/tailwindcss`), shadcn/ui (`packages/ui`) et Material Design 3 web (`packages/material-web`). Le fork n'est pas modifié.

## 1. Identité et version

- Paquet : `@material/web`, version `2.4.1` (cf. `packages/material-web/package.json`).
- Origine : fork upstream de `github.com/material-components/material-web` (MWC), implémentation officielle Google de Material Design 3 (M3) en web components.
- Licence : `Apache-2.0` (compatible aphrody, pas de contamination GPL).
- Module type : ESM pur (`"type": "module"`). Distribution livrée en `.js` + `.js.map` + `.d.ts` + `.scss` + `.css`, sources `.ts`/`test`/`catalog`/`scripts` exclues du publish (`files` du package.json).

## 2. Stack et langage

- Langage source : TypeScript `5.9.2`.
- Runtime des composants : Lit (`lit` `^2.8.0 || ^3.0.0`), via `LitElement`, décorateurs (`@customElement`, `@property`), templates `html`, et styles `CSSResult`.
- Dépendances runtime : `lit`, `@lit/context` (`^1.1.6`, propagation de contexte ex. champs/formulaires), `tslib`.
- Styling : SCSS (Sass `^1.93.2`) compilé en CSS, puis converti en modules TS (`*.cssresult.ts`, `*.css.ts`) consommés comme `static styles` des éléments Lit.
- Pas de dépendance React, pas de `@lit-labs/ssr` ni de wrappers `@lit/react` livrés dans le fork (vérifié : aucune occurrence dans `package.json`, `README.md`, `docs/quick-start.md`). MWC est purement « custom elements natifs ».
- Cibles : navigateurs modernes (custom elements v1, ElementInternals ; `element-internals-polyfill` en devDependency pour les tests).

## 3. Build system

- Orchestrateur : `wireit` (`^0.14.12`) — toutes les entrées `scripts` du package.json délèguent à wireit, qui gère le graphe de tâches incrémental et le cache.
- Pipeline (`wireit` config dans `package.json`) :
  - `build:sass` : compile les `.scss` en CSS.
  - `build:css-to-ts` : transforme le CSS en modules `*.cssresult.ts` / `*.css.ts` (styles importables par Lit).
  - `build:ts` → `build:ts:main` (`tsc --pretty` sur `tsconfig.json`) + `build:ts:labs-gb` (`tsc -p labs/gb/tsconfig.json`).
  - `build:catalog`, `build:scripts`, `update-docs`, `update-size` : tooling de site/doc/poids (hors distribution).
- Tests : `@web/test-runner` + Playwright + Jasmine (`web-test-runner.config.js`).
- Bundling de prototypage : Rollup (`rollup ^2`, plugins node-resolve / terser / multi-entry) — utilisé pour les bundles de démo, pas pour la consommation par composant.
- Important pour aphrody : ce build est intégralement JS/TS/Sass/Node, ce qui entre en tension directe avec la politique §2 de CLAUDE.md (« JS/TS/Node/Bun BANNIS »). Le fork est donc un *artefact source de référence design*, pas un maillon de la chaîne de build Rust du repo.

## 4. Modèle de design tokens / theming M3

Hiérarchie M3 à trois niveaux (cf. `docs/theming/README.md`), tous matérialisés en CSS custom properties scopables par sélecteur CSS :

1. Tokens de référence (`--md-ref-*`) — valeurs concrètes. MWC expose surtout `--md-ref-typeface-brand` / `--md-ref-typeface-plain` (familles/poids). Note importante : `--md-ref-palette` n'est PAS supporté à l'exécution par MWC (« MWC does not currently support `--md-ref-palette` tokens »).
2. Tokens de système (`--md-sys-*`) — rôles :
   - Couleur : `--md-sys-color-*` (`primary`, `on-primary`, `primary-container`, `surface`, `surface-container-*`, `outline`, `error`, `inverse-*`, `*-fixed`, etc.). Chaque rôle de surface possède son `on-` à contraste accessible.
   - Typographie : `--md-sys-typescale-*` (ex. `body-medium-size`, `body-medium-line-height`, plus `font`/`weight`/`tracking`).
   - Forme : `--md-sys-shape-corner-*` (`small`, `medium`, `large`, …).
   - Élévation : `--md-sys-elevation-*`.
   - Motion : `--md-sys-motion-*` NON supporté à l'exécution par MWC.
3. Tokens de composant (`--md-<component>-*`, ex. `--md-filled-button-container-shape`, `--md-filled-button-container-color`) — non préfixés `comp`. Par défaut ils référencent des tokens système ; surchargeables par instance/sélecteur.

Mécanique interne (cf. `button/internal/_filled-button.scss`, `_shared.scss`, `tokens/_md-comp-filled-button.scss`) :
- Le fichier de tokens du composant (`md-comp-filled-button-values()`) agrège les valeurs système (`md-sys-color.values-light()`, `md-sys-elevation.values()`, `md-sys-shape.values()`, `md-sys-typescale.values()`).
- Le mixin `styles()` réémet chaque token de composant en variable privée `--_<token>` sur `:host`, puis le CSS du shadow DOM consomme `var(--_label-text-font)`, `var(--_container-shape-start-start)`, etc. Ce niveau d'indirection (`--md-…` public → `--_…` privé) est le point d'extension du theming par composant.

Génération de palette / HCT :
- MWC ne génère PAS de palette tonale à l'exécution dans son cœur. Il recommande soit le plugin Figma « Material Theme Builder », soit la librairie `@material/material-color-utilities` (HCT, palettes tonales) côté consommateur.
- Mixins Sass d'aide : `color.theme()`, `color.light-theme()`, `color.dark-theme()` (cf. `color/_color.scss`) valident et émettent les `--md-sys-color-*` ; `typography/_typescale.scss` fournit `typescale.theme()` et un générateur de classes utilitaires `.md-typescale-*`.

Recoupement avec le crate Rust `m3-tokens` (`crates/m3-tokens/`) :
- Ce crate `no_std` fournit en Rust les mêmes familles : `color` (rôles ARGB `u32`, seed baseline `#6750A4`), `hct`, `dynamic` (algorithme `material-color-utilities` : seed HCT → palette 13 tons via `dynamic::seed_to_palette`), `tonal`, `typography` (type scale de 15 styles), `elevation`, `shape`, `motion`, `state`, plus du brand (`gemini_brand`, `google_sans_*`).
- Surtout : `m3_tokens::color::export_css(&ColorRoles)` émet un bloc `:root { --md-sys-color-*: …; }` au format EXACTEMENT identique à celui attendu par MWC (`--md-sys-color-primary`, `-on-primary`, `-surface-container-*`, `-inverse-*`, etc.), et `export_aphrody_brand_css` produit le brand. C'est le pont naturel : un générateur de tokens Rust côté aphrody, un consommateur MWC côté CSS, parlant le même vocabulaire `--md-sys-*`.

## 5. Modèle de composants

- Forme : custom elements natifs préfixés `md-*` (ex. `md-filled-button`, `md-checkbox`, `md-dialog`, `md-list`, `md-menu`, `md-text-field`, `md-tabs`, `md-fab`, `md-chip-set`). ~82 classes `@customElement` dans le fork (hors tests).
- Architecture par composant (motif récurrent, ex. `button/`) :
  - Fichier d'export public `filled-button.ts` : `@customElement('md-filled-button')`, étend une classe `internal/`, agrège des `static styles` (`sharedStyles`, `…-styles.cssresult.js`), déclare l'augmentation `HTMLElementTagNameMap`.
  - Logique dans `internal/` (classe Lit non enregistrée) + styles SCSS `internal/_*.scss`.
- Variantes M3 explicites en éléments distincts plutôt qu'en props : `md-elevated-button`, `md-filled-button`, `md-filled-tonal-button`, `md-outlined-button`, `md-text-button`.
- Composition : Shadow DOM + `<slot>` (slots par défaut et nommés ex. `icon`).
- Accessibilité : ARIA géré en interne, `ElementInternals` (form-associated custom elements pour checkbox/radio/textfield/select), focus ring dédié (`focus/`), ripple (`ripple/`).
- Bundle de commodité `all.ts` : importe tous les composants (prototypage seulement ; en prod on importe par composant, ex. `import '@material/web/button/filled-button.js'`).
- `labs/` : composants pré-stables (`badge`, `card`, `item`, `navigationbar`, `navigationdrawer`, `segmentedbutton`, `gb`…) — API non figée.

## 6. Surface d'API

- Import des composants : effet de bord par fichier, `import '@material/web/<dir>/<element>.js'` (enregistre le custom element). Usage HTML : `<md-filled-button>Save</md-filled-button>`.
- Import des styles globaux : `@material/web/typography/md-typescale-styles.js` (classes `.md-typescale-*`).
- Theming applicatif (deux voies) :
  - CSS pur : poser les `--md-sys-color-*` / `--md-sys-typescale-*` / `--md-sys-shape-*` sur `:root` (ou tout sélecteur) — voie privilégiée pour l'intégration aphrody.
  - Sass : `@use '@material/web/color/color'` puis `color.theme(...)` / `light-theme()` / `dark-theme()` ; idem typographie/shape.
- Theming par composant : surcharge des tokens `--md-<component>-*` sur `:root`, par classe ou par instance.
- Docs internes : `docs/theming/{README,color,typography,shape}.md`, `docs/components/*`, `docs/quick-start.md`.

## 7. Points d'intégration et conflits avec Tailwind et shadcn/ui

Inventaire des trois systèmes :
- `packages/tailwindcss` : `@tailwindcss/root` v1 — moteur Tailwind (CSS utilitaire, modèle `@theme` v4, déjà partiellement en crates Rust côté Oxide).
- `packages/ui` : `ui` v0.0.1, monorepo shadcn/ui (React + Tailwind ; build `turbo`/`pnpm`). Composants React copiés-dans-le-code, stylés par classes utilitaires Tailwind, tokens via CSS vars (`--background`, `--primary`, modèle HSL/OKLCH shadcn).
- `packages/material-web` : `@material/web` v2.4.1 — web components Lit, Shadow DOM, tokens `--md-sys-*`.

Conflits structurels :
1. Paradigme de styling : Tailwind/shadcn = classes utilitaires sur le DOM clair (light DOM). MWC = CSS encapsulé en Shadow DOM. Les utilitaires Tailwind (`bg-primary`, `p-4`) NE TRAVERSENT PAS la frontière du Shadow DOM ; on ne peut pas styler l'intérieur d'un `md-*` avec des classes Tailwind. Seuls les tokens CSS (custom properties) héritent à travers le Shadow DOM.
2. Modèle de composant : shadcn = composants React (JSX, hooks, refs). MWC = custom elements impératifs (attributs/propriétés/événements DOM). React < 19 gère mal les props non-string et les events custom des web components ; nécessite des wrappers (`@lit/react createComponent`) — non fournis par le fork.
3. Vocabulaire de tokens divergent : MWC = `--md-sys-color-primary` / `--md-sys-color-on-primary` / paire surface+on-surface. shadcn = `--primary` / `--primary-foreground` / `--background`. Tailwind v4 = mapping `@theme { --color-primary: … }`. Trois espaces de noms à réconcilier.
4. Génération de palette : MWC/M3 = HCT + palettes tonales (rôles `container`, `surface-container-*`, `inverse-*`). shadcn = paires foreground simples (HSL/OKLCH). M3 est strictement plus riche.
5. Chaîne d'outils : MWC = Sass + wireit + tsc ; shadcn = turbo/pnpm + React ; Tailwind = PostCSS/Oxide. Plus la politique §2 d'aphrody bannit JS/TS/Node — donc aucune de ces chaînes ne doit subsister telle quelle dans la distribution Rust.

Points de compatibilité exploitables :
- Tout MWC, tout Tailwind v4 et tout shadcn convergent sur le MÊME mécanisme bas niveau : les CSS custom properties scopées par sélecteur, qui héritent (y compris dans le Shadow DOM). C'est le dénominateur commun de la fusion.
- Le crate `m3-tokens` produit déjà des `--md-sys-*` au format MWC et calcule les palettes tonales/HCT en Rust — il peut servir de générateur unique de tokens, conforme à la politique « 100% Rust ».

## Points de fusion

Architecture cible recommandée : tokens M3 comme source de vérité unique, en Rust, diffusés en CSS custom properties que les trois systèmes consomment.

1. Source de vérité = `crates/m3-tokens` (Rust). Le seed de marque (brand) passe par `dynamic::seed_to_palette` (HCT → 13 tons), produit `ColorRoles` clair + sombre, puis `export_css` / `export_aphrody_brand_css` émet le bloc `:root { --md-sys-* }`. Ceci respecte §2 (zéro JS dans la chaîne) et remplace le plugin Figma + `material-color-utilities` JS par du Rust. C'est la pièce maîtresse de la fusion.

2. M3 → couche d'alias de tokens (CSS pur, généré). Émettre, à côté des `--md-sys-*` canoniques, une feuille de pont qui aliase vers les autres vocabulaires :
   - shadcn : `--primary: var(--md-sys-color-primary); --primary-foreground: var(--md-sys-color-on-primary); --background: var(--md-sys-color-surface); --border: var(--md-sys-color-outline-variant); …`
   - Tailwind v4 `@theme` : `@theme { --color-primary: var(--md-sys-color-primary); --radius-md: var(--md-sys-shape-corner-medium); --text-body-md: var(--md-sys-typescale-body-medium-size); … }` — ce qui expose ensuite `bg-primary`, `rounded-md`, etc. comme utilitaires Tailwind adossés aux rôles M3.
   Cette feuille d'alias est idéalement générée par `m3-tokens` (un nouvel exporteur `export_tailwind_theme` / `export_shadcn_vars`) pour rester en Rust.

3. Composants : trois zones distinctes, un seul thème.
   - Light DOM React/shadcn + utilitaires Tailwind : stylés par les alias ci-dessus (donc indirectement par M3). Aucun changement de runtime, seulement la couche de tokens.
   - Web components MWC `md-*` : consommés tels quels ; ils héritent automatiquement des `--md-sys-*` posés sur `:root`. Pour les utiliser DANS React/shadcn, générer des wrappers React fins (via `@lit/react createComponent` ou wrappers maison) — ou, conformément à §2 (Web = WASM Rust natif), wrapper les Material Web Components en composants Rust/`wasm-bindgen` exposant attributs/events, plutôt qu'en React.
   - Frontière Shadow DOM : ne JAMAIS compter sur les classes Tailwind pour pénétrer un `md-*` ; toute personnalisation interne passe par les tokens de composant `--md-<component>-*`, eux-mêmes adossés aux `--md-sys-*`.

4. Discipline de nommage : figer `--md-sys-*` comme noms canoniques internes (ils sont les plus expressifs : surfaces, containers, paires on-*, inverse-*). Les noms shadcn (`--primary`) et Tailwind (`--color-*`) deviennent des vues dérivées, jamais des sources. Mode sombre : un seul switch (`@media (prefers-color-scheme: dark)` ou attribut `data-theme`) qui réémet les `--md-sys-color-*` ; les alias suivent sans duplication.

5. Politique repo : conserver `packages/material-web` en référence design (specs SCSS, tables de tokens, docs theming) mais ne pas l'inclure dans le build de distribution Rust ; produire les CSS finales via `m3-tokens` + un éventuel pipeline Sass one-shot toléré. shadcn/ui et Tailwind suivent la même règle : code de référence, tokens pilotés par Rust.

En résumé : `m3-tokens` (Rust) calcule la palette tonale HCT → exporte `--md-sys-*` (format MWC natif) → une feuille d'alias générée mappe ces rôles vers les vars shadcn et le `@theme` Tailwind → les web components MWC héritent directement, React/shadcn et les utilitaires Tailwind héritent via alias, le tout sous un unique switch de thème.
