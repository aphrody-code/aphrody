# Audit d'architecture — `packages/ui` (fork shadcn-ui/ui)

Audit en lecture seule. Aucune modification du fork. Objectif : éclairer la fusion de trois
systèmes UI du monorepo aphrody — `packages/tailwindcss` (moteur CSS), `packages/ui`
(shadcn/ui, ce document) et `packages/material-web` (Material Design 3 web components).

> Note de gouvernance : la racine du repo (`CLAUDE.md`) bannit JS/TS et déclare que
> `packages/` n'existe plus. Le présent dossier `packages/ui` est néanmoins présent sur
> disque sous forme de fork JS/TS importé tel quel. Cet audit le décrit en l'état ; il ne
> préjuge pas de sa réintégration au workspace Rust.

## 1. Nature, version, structure du monorepo

- Fork de `shadcn-ui/ui` (repo upstream `https://github.com/shadcn-ui/ui.git`). Licence MIT,
  auteur `shadcn`.
- Racine `packages/ui/package.json` : `name: "ui"`, `version: "0.0.1"`, `private: true`,
  `type: "module"`, `packageManager: "pnpm@9.0.6"`.
- Le paquet publiable est `packages/shadcn` (le CLI), `name: "shadcn"`, `version: "4.7.0"`.
  C'est la version de référence du système (génération « v4 », Tailwind v4, support Base UI).
- Monorepo pnpm + Turborepo. Workspaces déclarés dans `pnpm-workspace.yaml` et le champ
  `workspaces` de `package.json` : `apps/*` et `packages/*` (avec exclusions
  `!**/test/**`, `!**/fixtures/**`, `!**/temp/**`, `!packages/tests/temp/**`).

Structure des membres :

- `apps/v4` — application Next.js (le site `ui.shadcn.com`), qui héberge **le registry**
  (`registry.json`, `app/api`, `public/r`), la doc MDX (`content`), et l'ensemble des
  composants source dans `registry/new-york-v4/`.
- `packages/shadcn` — le CLI `shadcn` (binaire `bin: ./dist/index.js`) + serveur MCP +
  schémas + builder de registry.
- `packages/tests` — tests d'intégration (exclu des sorties via `pnpm-workspace.yaml`).
- `templates/*` — 10 templates de projet de démarrage : `astro-app`, `astro-monorepo`,
  `next-app`, `next-monorepo`, `react-router-app`, `react-router-monorepo`, `start-app`
  (TanStack Start), `start-monorepo`, `vite-app`, `vite-monorepo`.
- `scripts/sync-templates.sh` — synchronisation des templates.

## 2. Stack et langage

- **Langage** : TypeScript (TSX) intégral. `tsconfig.json` partagé, build CLI via `tsup`.
- **UI runtime** : React 18/19 (`pnpm.overrides` force `@types/react` et `@types/react-dom`
  en `19.2.2`).
- **Primitives accessibles** : `radix-ui` (paquet unifié ; ex. `import { Slot } from "radix-ui"`)
  et **Base UI** (`base-ui`, mentionné dans les keywords du CLI et le type de registry
  `registry:base`). Le fork est donc en transition Radix → Base UI.
- **Styling** : Tailwind CSS. La racine épingle `tailwindcss: ^3.4.18` (outillage/lint), mais
  `apps/v4` cible **Tailwind v4** (CSS-first : `@import "tailwindcss"`, `@theme inline`,
  `@custom-variant`, PostCSS via `postcss.config.mjs`). Plugin d'animation `tw-animate-css`.
- **Variants** : `class-variance-authority` (`cva`, `VariantProps`).
- **Fusion de classes** : helper `cn` = `twMerge(clsx(...))` (`clsx` + `tailwind-merge`),
  dans `registry/new-york-v4/lib/utils.ts`.
- **Icônes** : `lucide` par défaut (champ `iconLibrary`), alternative `radix` supportée.
- Outillage : ESLint 9 + `eslint-plugin-tailwindcss`, Prettier (+ tri d'imports), Changesets,
  Vitest, Puppeteer (capture de registry), `motion` pour les animations de démo.

## 3. Gestionnaire de paquets et build

- **pnpm 9** + **Turborepo 1.x**. `turbo.json` (clé `pipeline`, ancienne syntaxe Turbo 1)
  définit `build` (dépend de `^build`, sorties `dist/**` et `.next/**`), `dev`, `lint`,
  `typecheck`, `test`, `registry:build`, etc.
- Scripts racine notables : `pnpm registry:build` (filtre `v4`), `pnpm test`
  (build registry puis `start-server-and-test` sur `http://localhost:4000`),
  `pnpm shadcn` (lance le CLI buildé contre un registry local).
- CLI `packages/shadcn` : `build` via `tsup` (multi-entrypoints, voir `exports`), publication
  via Changesets (`pub:beta`, `pub:next`, `pub:release`).

## 4. Modèle de design tokens / theming

Le theming shadcn est **100% variables CSS Tailwind v4**, sans composant runtime de thème.

- **Tokens sémantiques** (et non bruts) : `--background`, `--foreground`, `--card`,
  `--popover`, `--primary`, `--secondary`, `--muted`, `--accent`, `--destructive`,
  `--border`, `--input`, `--ring`, `--chart-1..5`, `--sidebar*`, `--surface`, `--code*`,
  `--selection`, plus `--radius`. Définis sur `:root` (clair) et `.dark` (sombre) dans
  `apps/v4/app/globals.css`.
- **Espace colorimétrique** : **OKLCH** systématique (ex. `--primary: oklch(0.205 0 0)`).
  C'est le point d'ancrage clé pour la fusion M3 (HCT/OKLCH proches).
- **Pont token → utilitaire Tailwind** : le bloc `@theme inline { --color-primary: var(--primary); ... }`
  mappe chaque token sémantique vers la couleur Tailwind v4, ce qui génère les utilitaires
  `bg-primary`, `text-foreground`, etc. Les rayons dérivent par calcul :
  `--radius-sm: calc(var(--radius) * 0.6)`, etc.
- **Palette brute** : `packages/shadcn/src/colors.ts` embarque la palette Tailwind complète en
  OKLCH (`TAILWIND_COLORS`), familles `TAILWIND_COLOR_FAMILIES` (incluant les familles custom
  du fork : `mauve`, `olive`, `mist`, `taupe`) × échelles `50..950`. Fonctions
  `findTailwindColorFamily`, `normalizeColorValue` (normalisation OKLCH) servent au CLI pour
  reconnaître/mapper une couleur.
- **Base colors** : sélectionnables à l'init via `baseColor` de `components.json`
  (`neutral`, `stone`, `zinc`, `mauve`, `olive`, `mist`, `taupe` — cf.
  `apps/v4/registry/base-colors.ts`).
- **Styles / « bases »** : le fork introduit des familles de style nommées
  (`vega`, `nova`, `lyra`, `maia`, `mira`, `luma`, `sera`) déclinées en `base-*` et `radix-*`
  (cf. `apps/v4/styles/` et `registry/styles/style-*.css`). Chaque `style-<nom>.css` est importé
  en `layer(base)` et activé via `@custom-variant style-<nom>` + classe racine `.style-<nom>`.
  Les styles ciblent des classes sémantiques `cn-*` (ex. `.cn-accordion-trigger`,
  `.cn-alert-variant-destructive`) via `@apply`, découplant l'apparence du composant React.
- Le schéma de registry porte aussi le theming par item : `cssVars` (`theme`/`light`/`dark`),
  `css` (CSS arbitraire récursif), `tailwind.config.theme`. Un item `registry:theme` ou
  `registry:base` peut donc livrer un thème complet.

## 5. Modèle de composants

- **Pas de paquet npm de composants** : le modèle shadcn est « copier le code source dans le
  projet consommateur ». Le code canonique vit dans `apps/v4/registry/new-york-v4/`
  (sous-dossiers `ui/`, `blocks/`, `charts/`, `hooks/`, `lib/`, `examples/`, `internal/`).
- **57 composants UI** dans `registry/new-york-v4/ui/` (accordion, alert, button, calendar,
  carousel, chart, command, dialog, drawer, form, sidebar, … ). Le style par défaut est
  `new-york` (l'ancien `default` est legacy).
- **Patron de composant** (ex. `button.tsx`) :
  - `cva(base, { variants: { variant, size }, defaultVariants })` pour les classes.
  - Composant fonctionnel typé `React.ComponentProps<"button"> & VariantProps<...>`.
  - `asChild` via `Slot.Root` (de `radix-ui`) pour la composition polymorphe.
  - Attributs `data-slot` / `data-variant` / `data-size` (ciblés par les styles `cn-*` et le
    CSS des « bases »).
  - Classes 100% Tailwind référant les tokens sémantiques (`bg-primary`,
    `text-primary-foreground`, `focus-visible:ring-ring/50`, variantes `dark:`).
- **Dépendances inter-items** : champ `registryDependencies` (un `block` tire ses `ui`),
  `dependencies` (npm), `registryDependencies` peut référencer d'autres registries via `@nom`.

## 6. Surface d'API

- **CLI `shadcn`** (`src/index.ts`, commander) — commandes :
  `init`, `apply`, `add`, `diff`, `docs`, `view`, `search`, `migrate`, `info`, `build`,
  `mcp`, `preset`, `registry`.
- **`components.json`** (schéma `src/registry/schema.ts` → `rawConfigSchema`) : champs `style`,
  `rsc`, `tsx`, `tailwind` (`config`, `css`, `baseColor`, `cssVariables`, `prefix`),
  `iconLibrary`, `rtl`, `menuColor`, `menuAccent`, `aliases`
  (`components`/`utils`/`ui`/`lib`/`hooks`), et `registries` (registries tiers, clés en `@`).
  `configSchema` étend avec `resolvedPaths`.
- **Schéma de registry item** (`registryItemSchema`, union discriminée sur `type`) :
  - Types : `registry:lib`, `registry:block`, `registry:component`, `registry:ui`,
    `registry:hook`, `registry:page`, `registry:file`, `registry:theme`, `registry:style`,
    `registry:item`, `registry:base`, `registry:font` (+ internes `registry:example`,
    `registry:internal`).
  - Champs communs : `name`, `title`, `author`, `description`, `dependencies`,
    `devDependencies`, `registryDependencies`, `files[]`, `tailwind`, `cssVars`, `css`,
    `envVars`, `meta`, `docs`, `categories`, `extends`.
  - JSON Schema public exposé à `apps/v4/public/schema/registry-item.json` (doit rester
    synchronisé avec le Zod, cf. commentaire en tête de `schema.ts`).
- **Registries tiers** : `registryConfigSchema` impose un nom en `@` et une URL contenant le
  placeholder `{name}`, avec `params`/`headers` (auth). Résolution dans `src/registry/`
  (`resolver`, `fetcher`, `namespaces`, `api`).
- **Serveur MCP** (`src/mcp/index.ts`, `@modelcontextprotocol/sdk`) — outils dont
  `get_project_registries`, `list_items_in_registries`, … (recherche/installation de composants
  pilotée par agent). Lancé via `shadcn mcp`.
- Sous-exports du paquet (`exports`) : `.`, `./registry`, `./schema`, `./mcp`, `./utils`,
  `./icons`, `./preset`, `./tailwind.css` — réutilisables comme bibliothèque.

## 7. Intégration et conflits avec Tailwind et Material 3

**Tailwind** — c'est déjà la fondation, pas de conflit :
- shadcn n'invente aucun moteur de style ; il génère des classes Tailwind et un set de
  variables CSS. `apps/v4` est sur Tailwind v4 (CSS-first), tandis que l'outillage racine
  épingle `tailwindcss ^3.4.18` (incohérence à surveiller lors d'une fusion :
  v3 `tailwind.config` vs v4 `@theme`). Le `prefix` de `components.json` permet d'éviter les
  collisions d'utilitaires.
- Le moteur `packages/tailwindcss` (crates Rust + paquet) du monorepo est l'implémentation
  bas niveau ; shadcn en est purement consommateur via PostCSS/`@import "tailwindcss"`.

**Material 3** — conflit de paradigme, mais réconciliable par les tokens :
- shadcn = composants React copiés + classes Tailwind ; `packages/material-web` = Web
  Components (`<md-*>`, Lit) avec son propre système de tokens M3 (`--md-sys-color-*`,
  élévation, typographie). Deux runtimes (React vs custom elements) et deux nomenclatures de
  tokens.
- Convergence : les deux utilisent un espace perceptuel proche (shadcn = OKLCH ; M3 = HCT).
  Le pont naturel est la **table de variables CSS**, déjà le point d'extension de shadcn
  (`cssVars`, `@theme inline`).

## Points de fusion

1. **Tailwind comme socle commun (déjà acquis)**. shadcn repose nativement sur Tailwind ;
   unifier sur Tailwind v4 (CSS-first `@theme`) et abandonner le `tailwind.config` v3 pour que
   les trois systèmes partagent le même pipeline PostCSS et le même registre d'utilitaires.
   Utiliser `prefix` (`components.json`) si M3 introduit des utilitaires concurrents.

2. **Mapper les tokens M3 → variables CSS shadcn**. Générer, à partir d'un thème M3 source
   (HCT / Material Theme Builder, déjà présent dans `crates/m3-tokens` et `docs/` du repo),
   les variables sémantiques shadcn :
   - `--md-sys-color-primary` → `--primary` ; `--md-sys-color-on-primary` →
     `--primary-foreground` ; `surface`/`surface-variant` → `--card`/`--muted`/`--surface` ;
     `outline` → `--border`/`--input` ; `error` → `--destructive` ; etc.
   - Conversion HCT → OKLCH (les deux sont perceptuels) pour rester homogène avec
     `colors.ts`/`globals.css`. Émettre l'ensemble comme un item `registry:theme` (ou
     `registry:base`) avec `cssVars.light` / `cssVars.dark`, ce qui rend le thème M3
     installable via `shadcn add`.

3. **Wrapper les Web Components M3 en composants shadcn**. Pour les primitives où M3 apporte
   une valeur (motion, ripple, élévation), publier des items `registry:ui` qui encapsulent les
   `<md-*>` dans des composants React (`data-slot`, `cva` pour les variantes, props mappées sur
   les attributs M3). Les styles `cn-*` et les « bases » (`vega`/`nova`/...) du fork sont le
   bon emplacement pour réconcilier l'apparence (élévation/forme M3 exprimées en `@apply`).

4. **Distribuer la fusion via le registry**. Créer un registry aphrody (nom `@aphrody`,
   schéma `registry-item.json`) servi par l'app `v4` ou un endpoint dédié, livrant :
   le thème M3→shadcn (point 2), les wrappers M3 (point 3), et la palette OKLCH étendue. Les
   trois systèmes deviennent alors consommables par une seule commande `shadcn add @aphrody/...`
   et pilotables par agent via le serveur MCP.

5. **Source unique de tokens**. Faire de `crates/m3-tokens` (Rust) le générateur canonique :
   sortie 1 = variables CSS `@theme`/`cssVars` pour shadcn ; sortie 2 = tokens `--md-sys-*`
   pour `packages/material-web`. Une seule définition de marque alimente les deux runtimes,
   éliminant la divergence de nomenclature.
