# Playbook stratégique de migration MUI → material-web

> **Portée.** Ce document est le **plan de bataille** d'une migration de production d'une codebase React/MUI (`@mui/material@9.0.1`, Emotion, palette Material 2 — `material-ui/packages/mui-material/`) vers `@material/web@2.4.1` (fork **aphrody-code/material-web**, web components Lit consommant les tokens `--md-sys-*`). Il fixe la **stratégie d'ensemble, les phases, l'outillage, les tests et les pièges** ; il ne refait pas le travail des autres livrables du kit auxquels il renvoie systématiquement. Lire d'abord le contrat partagé `migration/00-CONVENTIONS.md`. bun uniquement.

---

## 1. Stratégie d'ensemble : strangler fig vs big-bang

### 1.1 Le choix recommandé : incrémental (strangler fig)

La migration **doit être incrémentale** selon le pattern _strangler fig_ (Martin Fowler) : on fait pousser le nouveau système (`md-*` via wrappers React) **autour** de l'ancien (MUI), on bascule écran par écran / composant par composant, et MUI dépérit jusqu'à son retrait final. À aucun moment l'application n'est cassée ; chaque PR est livrable.

| Approche                                                 | Avantages                                                                                                                                                            | Inconvénients                                                                                                                                            | Verdict                                   |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| **Big-bang** (tout réécrire d'un coup, branche longue)   | conceptuellement simple, pas de période de coexistence à gérer                                                                                                       | branche qui diverge des semaines, merge hell, zéro livraison entre-temps, régressions massives découvertes tard, rollback = tout ou rien, équipe bloquée | ❌ rejeté pour une vraie codebase de prod |
| **Strangler fig** (incrémental, coexistence MUI + md-\*) | chaque étape livrable et testable, risque borné par étape, rollback granulaire, l'équipe continue à livrer des features, apprentissage progressif des web components | il faut **gérer la coexistence** (double theming, double bundle) le temps de la transition ; discipline de découpage                                     | ✅ **recommandé**                         |

**Pourquoi l'incrémental gagne ici, concrètement :**

1. **Changement de paradigme, pas juste d'API.** On ne remplace pas une lib React par une autre lib React : on passe de composants React/Emotion à des **custom elements Lit avec Shadow DOM**. La signature des events change (`onChange(e, value)` MUI → events natifs `input`/`change` avec `e.target.value`, cf. `00-CONVENTIONS.md §4` et `03-react-integration.md`), le styling change (Emotion `sx`/`styled` → tokens `--md-sys-*` + Tailwind sur le host). Un big-bang concentre **tous** ces risques au même instant.
2. **Couverture incomplète.** Il existe des **gaps** sans équivalent `md-*` (`Avatar`, `Alert`, `Breadcrumbs`, `Rating`, `Skeleton`, `Backdrop`, transitions `Collapse/Fade/...`, cf. `05-gap-analysis.md`). Une migration incrémentale permet de **garder ces composants MUI** jusqu'à ce qu'un shim soit prêt, au lieu de bloquer toute la migration.
3. **SSR/Next.** L'hydratation des custom elements dans React 18/19 + Next App Router demande des précautions (cf. `03-react-integration.md`). Mieux vaut les éprouver sur un écran pilote que sur toute l'app.
4. **Fork gelé en upstream.** On dépend du fork aphrody (cf. §9). Migrer progressivement = on découvre tôt les éléments du fork instables, sans avoir tout misé dessus.

### 1.2 Principe directeur

> **À chaque étape, l'app compile, les tests passent, l'écran est visuellement cohérent.** On ne migre jamais « à moitié » un écran livré sans flag. La coexistence est **assumée et outillée**, pas subie.

---

## 2. Coexistence MUI + material-web pendant la transition

C'est le cœur de la faisabilité du strangler fig. Trois axes : **isolation des styles**, **double theming**, **bundle size**.

### 2.1 Isolation des styles — peu de collisions, mais des pièges

| Système          | Mécanisme de style                                                                                                                                   | Portée                  |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| **MUI**          | Emotion : classes hashées injectées dans `<head>` (`css-xxxx`)                                                                                       | global (light DOM)      |
| **material-web** | CSS dans le **Shadow DOM** de chaque élément + tokens `--md-sys-*` hérités via les custom properties (qui, elles, **traversent** le shadow boundary) | encapsulé par composant |

**Bonne nouvelle :** le Shadow DOM des `md-*` **isole nativement** leur CSS interne. Les classes Emotion de MUI ne fuient pas dedans, et le CSS interne des `md-*` ne fuit pas dehors. Les **collisions directes de sélecteurs sont donc rares**.

**Pièges réels à gérer :**

1. **`CssBaseline` / reset global.** MUI applique un reset global (`box-sizing: border-box`, marges `body`, typographie de base) via `<CssBaseline />`. material-web **suppose** un reset minimal et pose sa propre typographie via tokens. Pendant la coexistence :
   - **Conserver** un reset global (celui de MUI `CssBaseline`, ou la base Tailwind `@layer base` une fois Tailwind en place — cf. `06-tailwind-material-web.md`). Il s'applique au **light DOM** ; il **ne traverse pas** le Shadow DOM des `md-*`, donc il ne perturbe pas leur rendu interne. Aucun conflit.
   - `CssBaseline`/`ScopedCssBaseline` **n'ont pas d'équivalent `md-*`** (gap, cf. `05-gap-analysis.md`) : on les **garde** jusqu'à la fin, puis on les remplace par le reset Tailwind + un layer typographie basé sur `--md-sys-typescale-*`.
2. **`font-family` héritée.** Les `md-*` lisent `--md-sys-typescale-*` (qui inclut la famille de police). Si MUI impose Roboto via `CssBaseline` et que les tokens M3 pointent ailleurs, on aura **deux polices** à l'écran. → Aligner la `font-family` des tokens M3 sur celle du thème MUI dès la phase 1 (cf. `02-theme-token-migration.md`).
3. **`z-index` / overlays.** MUI gère ses overlays (`Modal`, `Popover`, `Snackbar`) avec une échelle de `z-index` (1200–1500) et des portails dans `document.body`. `md-dialog`/`md-menu` utilisent le **top-layer** natif (`::backdrop`, Popover API), qui passe **au-dessus** de tout z-index. → Pendant la coexistence, un `md-dialog` ouvert recouvrira un overlay MUI. Acceptable si on ne mélange pas deux overlays simultanés ; à surveiller sur les écrans qui empilent dialogs MUI + menus md.
4. **Box-sizing.** Les `md-*` posent leur propre `box-sizing` en interne ; le `* { box-sizing: border-box }` global n'affecte que le host. Pas de conflit, mais vérifier les wrappers de layout.

### 2.2 Double theming — une seule palette, deux consommateurs

Pendant la transition, **MUI et material-web coexistent à l'écran** : ils doivent partager **la même identité visuelle**. La règle :

> **Une seule source de couleur**, projetée sur **deux cibles** : le thème MUI (`createTheme`, objet JS + CSS vars MUI) **et** les tokens `--md-sys-*` (CSS custom properties). Les deux sont dérivés de **la même palette source** (idéalement `palette.primary.main` du thème MUI existant).

```
                 couleur source (palette.primary.main, etc.)
                          │
        ┌─────────────────┴──────────────────┐
        ▼                                     ▼
  createTheme (MUI / M2)              material-color-utilities
  → ThemeProvider                    → tokens --md-sys-* (M3)
  → composants MUI                   → composants md-* (héritent via :root)
```

- **MUI (M2)** reste piloté par `createTheme` + `<ThemeProvider>` comme aujourd'hui.
- **material-web (M3)** est piloté par les tokens `--md-sys-*` posés sur `:root` (ou un conteneur), **traversant** le Shadow DOM via héritage des custom properties.
- Le **mapping M2→M3** (best-effort, avec pertes documentées) et le **script de génération** des rôles M3 manquants (`tertiary`, `*-container`, `surface-variant`…) via `material-color-utilities` sont détaillés dans **`02-theme-token-migration.md`** (cf. aussi le tableau de mapping dans `00-CONVENTIONS.md §5` et `docs/02-tokens-theming-web.md`).
- **Objectif visuel :** qu'un `Button` MUI `contained` et un `md-filled-button` posés côte à côte aient **la même couleur primaire, le même rayon de coin, la même typo**. C'est ce qui rend la coexistence invisible pour l'utilisateur final.

**Dark mode :** maintenir les deux jeux en parallèle — `palette.mode` côté MUI, et un second jeu de `--md-sys-color-*` sous `@media (prefers-color-scheme: dark)` ou une classe `.dark` (cf. `02-theme-token-migration.md`). Les deux doivent basculer **ensemble**.

### 2.3 Bundle size pendant la coexistence

Pendant la transition, **les deux librairies sont chargées**. C'est le coût temporaire assumé du strangler fig.

| Levier                        | Détail                                                                                                                                                                                                                                                                                                                                       |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Tree-shaking MUI**          | imports nommés / par chemin (`@mui/material/Button`) — déjà la norme MUI. Au fur et à mesure de la migration, les imports MUI disparaissent et le tree-shaking réduit MUI mécaniquement.                                                                                                                                                     |
| **Tree-shaking md-\***        | **import par effet de bord** : `import '@material/web/button/filled-button.js'` enregistre l'élément. **N'importer QUE les éléments utilisés** — jamais `import '@material/web/all.js'` en prod (il enregistre les 93 éléments). Les wrappers (`migration/wrappers/`) importent chacun leur définition précise (cf. `00-CONVENTIONS.md §2`). |
| **Roboto / Material Symbols** | ne charger les Material Symbols qu'une fois (icônes `md-icon`), éviter de charger en plus les `@mui/icons-material` pour les écrans déjà migrés.                                                                                                                                                                                             |
| **Suivi**                     | mesurer le bundle à chaque phase (`bun build` + analyse). Le pic de bundle est **pendant** la coexistence ; il **redescend** à la phase 6 (retrait MUI). Documenter la courbe pour rassurer les stakeholders.                                                                                                                                |

> **Mesure avant affirmation.** Ne jamais affirmer « le bundle a baissé » sans une mesure (`bun build`, taille des chunks). Le gain réel vient de la **phase 6**.

---

## 3. Phases ordonnées

### 3.1 Diagramme de phases (texte)

```
┌──────────────────────────────────────────────────────────────────────────┐
│  PHASE 0 — Prérequis & outillage                                           │
│  bun, @lit/react, wrappers scaffold, codemods scaffold, CI tests/visuels   │
│  Critère sortie: 1 wrapper (MdFilledButton) rendu+testé dans l'app         │
└───────────────┬──────────────────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  PHASE 1 — Tokens & theming AVANT tout composant                           │
│  palette MUI → --md-sys-* (02-theme-token-migration.md), double theming    │
│  Critère sortie: md-* posé en démo == cohérent visuellement avec MUI       │
└───────────────┬──────────────────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  PHASE 2 — Composants feuilles (atomiques)                                 │
│  Button, IconButton, Checkbox, Radio, Switch, Slider, TextField, Chip…     │
│  Critère sortie: tous les atomiques d'un écran migrables sans dépendance   │
└───────────────┬──────────────────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  PHASE 3 — Composants composés                                             │
│  Dialog, Menu/Select, Card, Tabs, List, Snackbar, Tooltip…                 │
│  Critère sortie: overlays/slots/focus OK, pas de régression a11y           │
└───────────────┬──────────────────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  PHASE 4 — Layout (Box/Stack/Grid/Paper → Tailwind)                        │
│  06-tailwind-material-web.md : utilitaires sur le host, single source token│
│  Critère sortie: layout d'1 écran sans MUI layout, parité visuelle         │
└───────────────┬──────────────────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  PHASE 5 — Navigation & écrans                                             │
│  AppBar/Drawer/BottomNav → md-top-app-bar/navigation-*, migration écran-par│
│  Critère sortie: chaque écran 100% md-* (sauf gaps), flag retirable        │
└───────────────┬──────────────────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  PHASE 6 — Retrait de MUI                                                  │
│  désinstaller @mui/*, supprimer ThemeProvider/CssBaseline, nettoyer bundle │
│  Critère sortie: 0 import @mui/*, bundle réduit (mesuré), tests verts       │
└──────────────────────────────────────────────────────────────────────────┘
```

**Règle de dépendance :** chaque phase suppose la précédente **terminée et stable**. On peut paralléliser **à l'intérieur** d'une phase (plusieurs composants atomiques en même temps), jamais entre phases sur le même périmètre.

### 3.2 Détail par phase

#### Phase 0 — Prérequis & outillage

| Item         | Action                                                                                                                                                    |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Runtime      | bun uniquement (`bun install`, `bun build`, `bun test`). Jamais npm/pnpm (`00-CONVENTIONS.md §7.1`).                                                      |
| Dépendances  | `bun add @lit/react` (non installé — requis par les wrappers, `00-CONVENTIONS.md §0`), `@material/web@2.4.1` (fork aphrody).                              |
| Wrappers     | scaffold de `migration/wrappers/` (package logique `@aphrody/m3-react`) — 1 wrapper React par `md-*` via `createComponent` (`00-CONVENTIONS.md §2`). |
| Codemods     | scaffold de `migration/codemods/` (jscodeshift + ast-grep).                                                                                               |
| CI           | brancher la pipeline de tests (unitaires custom elements, snapshots Playwright, axe — cf. §5) **avant** de migrer quoi que ce soit.                       |
| Feature flag | mécanisme pour basculer un écran/composant entre MUI et md-\* (env var, flag par route, ou simple swap d'import).                                         |

- **Ordre :** runtime → deps → wrappers scaffold → codemods scaffold → CI.
- **Critère de sortie :** `MdFilledButton` (wrapper) s'affiche dans l'app réelle, réagit au clic, et un test unitaire + un snapshot Playwright passent.
- **Risques :** `@lit/react` mal configuré (events non mappés) ; SSR Next qui plante sur `customElements` côté serveur (cf. `03-react-integration.md`).

#### Phase 1 — Tokens & theming **d'abord**

- **Pourquoi en premier :** sans tokens cohérents, **chaque `md-*` posé sera visuellement faux** (couleurs par défaut M3 baseline violet/lavande). Le theming est la fondation de la coexistence (§2.2).
- **Ordre :** extraire la palette MUI → mapper M2→M3 → générer les rôles manquants (`material-color-utilities`) → poser `--md-sys-*` sur `:root` → vérifier light + dark. Tout est dans **`02-theme-token-migration.md`**.
- **Critère de sortie :** un `md-filled-button` de démo et un `Button` MUI `contained` ont **la même** couleur/rayon/typo, en light et dark.
- **Risques :** pertes de mapping M2→M3 (tons intermédiaires) à documenter ; police désalignée (§2.1 piège 2).

#### Phase 2 — Composants feuilles (atomiques)

- **Périmètre :** `Button` (variant-dépendant → `md-filled/outlined/text/elevated/filled-tonal-button`), `IconButton`, `Fab`, `Checkbox`, `Radio`, `Switch`, `Slider`, `TextField` (`md-filled/outlined-text-field`), `Chip` (`md-assist/filter/input/suggestion-chip` + `md-chip-set`), `Icon`, `Typography` (`md-type` ou classes typescale), `Divider`, `LinearProgress`/`CircularProgress` (cf. tableau `00-CONVENTIONS.md §3` et `01-component-mapping.md`).
- **Ordre interne :** d'abord les sans-état (`Divider`, `Icon`, progress), puis les controlled simples (`Checkbox`, `Switch`, `Radio`), puis `TextField`/`Select` (formulaires, le plus délicat). Parallélisable.
- **Critère de sortie :** tous les composants atomiques d'un écran cible peuvent être remplacés sans dépendre d'un composé.
- **Risques :** **controlled components** — la signature d'event change (`onChange(e, value)` → `e.target.value`, `00-CONVENTIONS.md §4`) ; vérifier le **nom réel des props** sur l'élément md (ex. `checked` vs `selected`) avant mapping — ne jamais inventer (`§7.2`).

#### Phase 3 — Composants composés

- **Périmètre :** `Dialog` (slots `headline`/`content`/`actions`), `Menu`/`MenuItem` (+ `md-sub-menu`), `Select` (`md-*-select` + `md-select-option`), `Card` (`md-elevated/filled/outlined-card`), `Tabs` (`md-tabs` + `md-primary/secondary-tab`), `List`, `Snackbar` (fork), `Tooltip` (fork), `Accordion` (fork), `Table`/`Stepper`/`Paginator` (fork).
- **Ordre :** d'abord ceux **100% upstream/stables** (Dialog, Menu, Card, Tabs, List), ensuite les composants **du fork** (Snackbar, Tooltip, Accordion, Table, Stepper — plus risqués, cf. §9).
- **Critère de sortie :** overlays positionnés correctement (top-layer, §2.1 piège 3), slots remplis, focus trap et a11y vérifiés (axe).
- **Risques :** mapping des sous-composants MUI (`DialogTitle`, `CardHeader`…) → **slots** (`00-CONVENTIONS.md §4`) ; gestion `open`/controlled des dialogs/menus ; collisions z-index avec overlays MUI restants.

#### Phase 4 — Layout (Box/Stack/Grid/Paper)

- **Périmètre :** `Box`/`Container`/`Stack`/`Grid` → `<div>` + utilitaires **Tailwind** ; `Paper` → `<div>` surface + `md-elevation`. **Aucun équivalent `md-*`** pour le layout (`00-CONVENTIONS.md §3`).
- **Ordre :** introduire Tailwind v4 (`@theme` dérivant ses couleurs des tokens `--md-sys-*` = single source, cf. `06-tailwind-material-web.md`), puis convertir les conteneurs de layout écran par écran.
- **Rappel dur :** les utilitaires Tailwind **ne franchissent pas** le Shadow DOM des `md-*` — ils stylent le **host** et le **layout autour**, pas l'intérieur des composants (`00-CONVENTIONS.md §6`). Tout est dans **`06-tailwind-material-web.md`**.
- **Critère de sortie :** le layout d'un écran complet sans aucun composant de layout MUI, parité visuelle.
- **Risques :** `sx` de layout (espacements, breakpoints) à traduire en classes Tailwind ; breakpoints MUI ≠ breakpoints Tailwind (recaler).

#### Phase 5 — Navigation & écrans

- **Périmètre :** `AppBar`/`Toolbar` → `md-top-app-bar`/`md-bottom-app-bar`/`md-toolbar` (fork) ; `Drawer` → `md-navigation-drawer`(`-modal`) (fork) ; `BottomNavigation` → `md-navigation-bar` (fork). Puis migration **écran par écran** : chaque route bascule en 100% md-\* (sauf gaps assumés).
- **Ordre :** shell de navigation d'abord, puis écrans du moins au plus critique.
- **Critère de sortie :** chaque écran migré ne dépend plus de MUI (hors gaps documentés) ; le feature flag de l'écran peut être retiré.
- **Risques :** composants de nav **tous issus du fork** (§9) ; comportement responsive du drawer ; intégration routing (Next App Router, `03-react-integration.md`).

#### Phase 6 — Retrait de MUI

- **Périmètre :** `bun remove @mui/material @mui/icons-material @emotion/react @emotion/styled` (et `@mui/*` restants) ; supprimer `<ThemeProvider>`/`createTheme` MUI ; remplacer `CssBaseline` par le reset Tailwind + layer typographie M3 ; nettoyer les imports morts.
- **Pré-requis :** **zéro import `@mui/*`** restant (sauf shims de gaps explicitement conservés, cf. `05-gap-analysis.md` — auquel cas MUI peut rester partiellement, ou le gap est remplacé par un shim maison).
- **Critère de sortie :** `bun run` OK, tests verts, snapshots stables, **bundle mesuré en baisse** (la coexistence est finie).
- **Risques :** un import MUI oublié dans un coin (chercher `from '@mui/'` sur tout le repo) ; un gap encore dépendant de MUI qui force à garder une partie de la lib.

---

## 4. Outillage : codemods, wrappers — automatique vs manuel

| Couche                   | Emplacement                                      | Rôle                                                                                                                                                                                                                       | Phase                          |
| ------------------------ | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| **Wrappers React**       | `migration/wrappers/` (`@aphrody/m3-react`) | 1 wrapper par `md-*` via `@lit/react` `createComponent`, mappe les events natifs en `onInput`/`onChange` (`00-CONVENTIONS.md §2`). C'est la **couche cible** des codemods.                                                 | 0 (scaffold) → utilisé partout |
| **Codemods jscodeshift** | `migration/codemods/`                            | transforment les **imports** (`@mui/material/Button` → `@aphrody/m3-react`), renomment les composants (variant-dépendant : `Button variant="outlined"` → `MdOutlinedButton`), réécrivent les **props** mappables 1:1. | 2–5                            |
| **Règles ast-grep**      | `migration/codemods/`                            | détection/lint des patterns **non automatisables** (signale `sx=`, `styled(`, `onChange={(e,v)=>}`) pour traitement manuel.                                                                                                | 2–5                            |

### Ce qui s'automatise (codemods)

- Remplacement des **imports** MUI → wrappers `@aphrody/m3-react`.
- **Renommage** de composant, y compris la résolution **variant-dépendante** (`Button`/`IconButton`/`TextField`/`Chip` — cf. `00-CONVENTIONS.md §3`) quand le variant est un littéral statique.
- Props **1:1** (`disabled`, `value`, `label`…) — après vérification du nom réel sur l'élément md.
- `startIcon`/`endIcon` → `<md-icon slot="icon">` / `slot="start"`/`end` quand l'icône est statique.

### Ce qui reste manuel (signalé par ast-grep, pas transformé)

- **`sx` / `styled`** : pas d'équivalent ; conversion vers `style`/Tailwind (host) + tokens `--md-sys-*` (interne) — décision humaine (cf. §7).
- **Controlled handlers** : `onChange={(e, value) => …}` → `onChange={(e) => e.target.value}` (sémantique différente, `00-CONVENTIONS.md §4`).
- **Variant dynamique** (`variant={cond ? 'outlined' : 'text'}`) : le codemod ne peut pas trancher le tag → flag manuel.
- **Sous-composants → slots** (`DialogTitle`→`slot="headline"`…) : restructuration de l'arbre JSX, manuel.
- **Gaps** (`Avatar`, `Alert`…) : aucune transformation ; suivre `05-gap-analysis.md`.

> Règle : un codemod **ne devine jamais** un tag/prop/slot inexistant. En cas de doute → laisse le code intact + commentaire/flag ast-grep pour revue humaine (`00-CONVENTIONS.md §7.2`).

---

## 5. Tests & non-régression

### 5.1 Stratégie en trois couches

| Couche                   | Outils                                                                                                                | Cible                                                                                                                               |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| **Unitaire / composant** | `@open-wc/testing-helpers` (fixture custom elements) **ou** Testing Library avec support custom elements + `bun test` | rendu d'un wrapper, events (`input`/`change` → `onInput`/`onChange`), state controlled, props                                       |
| **Snapshot visuel**      | **Playwright** (screenshots par composant + par écran, light/dark)                                                    | non-régression visuelle pendant la coexistence (un `md-*` doit matcher la maquette / l'ancien MUI tant que le design ne change pas) |
| **Accessibilité**        | `axe` (axe-core, via Playwright ou unitaire)                                                                          | rôles ARIA, contraste (tokens M3), focus, labels — par composant et par écran                                                       |

- Mettre la pipeline en place **en Phase 0**, avant toute migration : c'est le filet qui sécurise chaque PR du strangler fig.
- Snapshot **baseline figé avant migration** d'un écran → comparer après → diff = régression à justifier.

### 5.2 Pièges de test du Shadow DOM

1. **Le contenu est dans le shadow root.** Les sélecteurs classiques (`getByText`, `querySelector` light DOM) **ne voient pas** l'intérieur des `md-*`. Il faut `element.shadowRoot.querySelector(...)` ou les helpers `@open-wc` qui pénètrent le shadow.
2. **Timing d'upgrade.** Un custom element n'est « upgradé » qu'après `customElements.whenDefined(...)` / `await element.updateComplete` (Lit). Tester **après** ce point, sinon on teste un élément non rendu.
3. **Playwright** traverse le Shadow DOM **par défaut** pour les sélecteurs de texte/rôle → utiliser les **locators Playwright** (ils piercent le shadow), pas du `evaluate` manuel.
4. **axe** sait analyser le Shadow DOM (axe-core ≥ 3.3) — vérifier qu'il est lancé sur l'élément complet, pas seulement le host.
5. **Events** : tester que `onInput`/`onChange` du wrapper reçoivent bien `e.target.value` (et non la signature MUI à 2 args) — c'est le bug de régression n°1 des formulaires.

---

## 6. Pièges & anti-patterns

| Piège                               | Symptôme                                                                        | Parade                                                                                                                                                                                                                       |
| ----------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`sx` / `styled` sans équivalent** | styles perdus après codemod                                                     | Tailwind sur le host pour layout/espacement ; tokens `--md-sys-*` pour l'interne ; `::part()` pour cibler des parties exposées (cf. `06-tailwind-material-web.md`). Jamais d'`!important` qui tente de forcer le Shadow DOM. |
| **Controlled props**                | input qui ne se met plus à jour / handler reçoit `undefined`                    | adapter la signature : `e.target.value` (`00-CONVENTIONS.md §4`, détail `03-react-integration.md`). Vérifier `value`/`checked`/`selected` réel sur l'élément.                                                                |
| **Formulaires**                     | `md-*` pas captés par un `<form>` / lib de form (RHF, Formik)                   | les `md-*` form-associated exposent `name`/`value` ; pour RHF utiliser `Controller` et lire `e.target.value`. Voir `03-react-integration.md`.                                                                                |
| **SSR / Next**                      | `ReferenceError: HTMLElement is not defined` côté serveur, FOUC à l'hydratation | imports d'éléments côté client uniquement, garde `customElements`, stratégie d'hydratation — tout dans `03-react-integration.md`.                                                                                            |
| **Tree-shaking**                    | bundle énorme                                                                   | jamais `@material/web/all.js` en prod ; import par chemin précis dans chaque wrapper (§2.3).                                                                                                                                 |
| **Perfs**                           | jank au mount de listes de `md-*`                                               | les custom elements ont un coût d'upgrade ; virtualiser les grandes listes, éviter de monter des centaines de `md-*` d'un coup.                                                                                              |
| **Inventer un élément/prop**        | `md-foo` inexistant, attribut ignoré                                            | interdit (`00-CONVENTIONS.md §7.2`) : vérifier dans `material-web/`, sinon → gap (`05-gap-analysis.md`).                                                                                                                     |
| **Tailwind dans le Shadow DOM**     | classe Tailwind sans effet sur l'intérieur du composant                         | normal : Tailwind = host/layout seulement (`00-CONVENTIONS.md §6`, `06-tailwind-material-web.md`).                                                                                                                           |
| **Double overlay MUI+md**           | z-index incohérent                                                              | éviter d'empiler un overlay MUI et un `md-dialog`/`md-menu` (top-layer) sur le même écran pendant la transition (§2.1).                                                                                                      |

---

## 7. Checklist de migration **par composant** (template réutilisable)

À copier pour chaque composant migré.

```
### Composant : <NomMUI>  →  <md-élément> (<MdWrapper>)

PRÉ-ANALYSE
- [ ] Élément md cible identifié dans 01-component-mapping.md / 00-CONVENTIONS.md §3
- [ ] Élément vérifié existant dans material-web/ (PAS inventé) — fork ou upstream ?
- [ ] Variant-dépendant ? (Button/IconButton/TextField/Chip…) → tags listés
- [ ] Gap éventuel noté (05-gap-analysis.md) ?

WRAPPER (migration/wrappers/)
- [ ] Wrapper @lit/react créé (createComponent), nom PascalCase du tag
- [ ] Import par chemin précis de la définition (effet de bord d'enregistrement)
- [ ] Events natifs mappés (onInput/onChange/…) — 00-CONVENTIONS.md §2

PROPS / EVENTS
- [ ] Props 1:1 mappées (noms réels vérifiés sur l'élément, pas supposés)
- [ ] Controlled : signature event adaptée (e.target.value, pas (e,value))
- [ ] Slots : sous-composants/children → slots md (headline/content/icon/start/end)
- [ ] Icônes startIcon/endIcon → <md-icon slot="...">

STYLING
- [ ] sx/styled traité : layout→Tailwind(host), interne→tokens --md-sys-*, ::part() si besoin
- [ ] Cohérence visuelle avec le thème (tokens posés en phase 1)

CODEMOD
- [ ] Import + renommage automatisés (jscodeshift) si statique
- [ ] Cas dynamiques flaggés (ast-grep) pour revue manuelle

TESTS
- [ ] Unitaire (shadowRoot/updateComplete) : rendu + events + controlled
- [ ] Snapshot Playwright light + dark, avant/après
- [ ] axe : rôles, contraste, focus, label OK

SORTIE
- [ ] App compile, tests verts, parité visuelle, pas de fuite z-index
- [ ] PR livrable indépendamment (feature flag si nécessaire)
```

---

## 8. Gestion du fork (point de risque structurel)

**Fait terrain :** `material-web` upstream (`material-components/material-web`) est **gelé en maintenance** (Google a stoppé le développement actif). Le kit dépend donc du **fork `aphrody-code/material-web`** (`origin` du repo local `material-web/`, `@material/web@2.4.1`), qui ajoute les **composants du fork** indispensables à la migration : `md-autocomplete`, `md-snackbar`, `md-tooltip`, `md-badge`, `md-navigation-bar`, `md-navigation-drawer`, `md-top-app-bar`, `md-stepper`, `md-table`, `md-paginator`, `md-accordion`, `md-grid-list`, `md-type`, etc. (cf. `00-CONVENTIONS.md §3`, liste réelle `migration/scripts/md-elements.txt` — 93 éléments).

### Implications maintenance

| Implication                              | Conséquence                                                                                 | Mitigation                                                                                                                                          |
| ---------------------------------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Plus de patchs upstream**              | bugs/sécurité non corrigés par Google                                                       | s'appuyer sur le fork ; contribuer/patcher le fork si besoin (on en est owner-side via aphrody-code).                                               |
| **Composants critiques = fork only**     | navigation, snackbar, tooltip, table, stepper, autocomplete n'existent **que** dans le fork | les migrer **en dernier** dans chaque phase (3 et 5), après les éléments upstream stables ; tests renforcés dessus.                                 |
| **API du fork peut diverger**            | un `md-*` du fork peut changer de prop/slot entre versions                                  | **pinner** la version (`@material/web@2.4.1`), vérifier l'API réelle dans `material-web/` avant chaque mapping, jamais supposer la parité upstream. |
| **Pas de communauté large**              | moins de Stack Overflow / issues publiques                                                  | documenter en interne ; le kit (`01`→`06`) **est** la doc de référence.                                                                             |
| **M3 Expressive (2025) absent côté web** | pas de nouvelles formes/motion expressifs                                                   | accepté ; `md-type` porte le type scale M3 classique (`00-CONVENTIONS.md §0`).                                                                      |
| **Mises à jour du fork**                 | suivre `aphrody-code/material-web`                                                          | tester chaque bump de version contre la suite Playwright/axe avant adoption ; ne pas bumper en aveugle.                                             |

> **Règle d'or fork :** avant de mapper ou wrapper un composant du fork, **ouvrir sa source dans `material-web/`** pour confirmer tag, props (reactive properties Lit), slots et events réels. Tout ce qui manque → gap explicite dans `05-gap-analysis.md`, jamais une invention (`00-CONVENTIONS.md §7.2`).

---

## 9. Récapitulatif — l'ordre canonique

1. **Phase 0** : bun + `@lit/react` + wrappers/codemods scaffold + CI tests.
2. **Phase 1** : tokens & theming (`02-theme-token-migration.md`) — la fondation.
3. **Phase 2** : feuilles atomiques (Button, Checkbox, Switch, TextField…).
4. **Phase 3** : composés (Dialog, Menu, Card, Tabs…), upstream avant fork.
5. **Phase 4** : layout → Tailwind (`06-tailwind-material-web.md`).
6. **Phase 5** : navigation (fork) & migration écran par écran.
7. **Phase 6** : retrait de MUI, bundle mesuré en baisse.

Le tout en **strangler fig** : chaque étape livrable, testée (unitaire + Playwright + axe), avec coexistence MUI/md-\* maîtrisée (double theming sur palette unique, isolation Shadow DOM, `CssBaseline`/reset conservé jusqu'au bout). Voir le mapping exhaustif dans `01-component-mapping.md`, les gaps dans `05-gap-analysis.md`, l'intégration React/SSR dans `03-react-integration.md`.
