---
nav_exclude: true
search_exclude: true
---

# 06 — Panorama Material Design 3 sur le web (2026) & recommandations d'architecture

> Document chapeau. Il synthétise l'état réel de l'écosystème MD3 web en mai 2026, compare les options viables, explique le constat structurant (pas d'implémentation web first-party complète et maintenue), puis propose des scénarios d'architecture concrets et une recommandation finale par profil de projet. Il s'appuie sur les fiches détaillées du dossier : `01-md3-spec-foundations.md` (fondations spec : tokens, HCT, dynamic color, typo, shape, elevation, motion), `02-material-web-deep-dive.md`, `03-mui-react-md3.md`, `04-shadcn-registry-tokens.md`, `05-tailwind-ecosystem-md3.md`.

---

## 1. Panorama 2026 : les vraies options pour faire du MD3 sur le web

Material Design 3 (« M3 » / « Material You ») repose sur quelques piliers qui définissent la _conformité réelle_ d'une implémentation (détail dans `01-md3-spec-foundations.md`) :

- **Color system HCT + dynamic color** : espace colorimétrique Hue-Chroma-Tone, génération de schémas tonaux à partir d'une couleur source ou d'une image, rôles sémantiques (`primary`, `on-primary`, `surface-container`, etc.).
- **Design tokens** : couleur, typescale, shape, elevation, motion, state layers — exposés comme variables consommables.
- **Composants** alignés sur la spec (forme, états, ripple/state layers, accessibilité).

À l'aune de ces critères, voici les options réellement disponibles et leur santé.

### Options de premier plan

| Option                                                                              | Nature                                        | Cadre                       | Santé 2026                                                            |
| ----------------------------------------------------------------------------------- | --------------------------------------------- | --------------------------- | --------------------------------------------------------------------- |
| **`@material/web` (Composants Web matériels, MWC)**                                 | Web components officiels Google (Lit)         | Agnostique (web components) | **Maintenance mode** depuis 2024, pas de roadmap active               |
| **MUI ($`@mui/material`)**                                                          | Lib React, la plus populaire                  | Réagir                      | **Très actif**, mais ancrée Material **2**, pas de M3 livré           |
| **Matériau angulaire**                                                              | Lib officielle Angular (Composants matériels) | Angulaire                   | **Actif** (maintenu par l'équipe Angular), aligné M3                  |
| **Tokens M3 personnalisés + composants Web/CSS** (incl. `material-color-utilities`) | Approche « faites-le vous-même »              | Agnostique                  | Les _briques_ (MCU) sont **actives**, l'assemblage est à votre charge |

### Alternatives notables (au-delà des repos locaux)

- **Beer CSS** — framework CSS léger, framework-agnostique, explicitement conçu pour M3 ; revendique _plus_ de composants que la version web officielle de Google. Bon candidat « pur M3, rapide à poser », communautaire. (https://github.com/beercss/beercss)
- **`@material/material-color-utilities` (MCU)** — la bibliothèque officielle Google qui _fait_ le dynamic color (HCT, schémas, extraction depuis image). C'est la pièce que toutes les approches sérieuses réutilisent, y compris le DIY. Versions récentes supportent `SpecVersion.SPEC_2025` (nouveaux variants `EXPRESSIVE`, etc.). (https://github.com/material-foundation/material-color-utilities, https://www.npmjs.com/package/@material/material-color-utilities)
- **Material Theme Builder** — outil web + plugin Figma officiel pour générer un thème M3 (HCT picker, export tokens). Non open source ; sorties parfois légèrement divergentes de MCU. (https://github.com/material-foundation/material-theme-builder)
- **Vuetify** — implémente M3 côté Vue, actif.
- **`material-tailwind` (Creative Tim)** — « inspiré Material » sur Tailwind, React/HTML. v3 toujours en **beta** (dernière release publique `v3.0.0-beta.6`) ; conformité M3 partielle/esthétique, pas de dynamic color HCT. (https://github.com/creativetimofficial/material-tailwind)
- **shadcn/ui + Tailwind** — _pas_ une lib M3, mais un _modèle de distribution_ (registry, « copie le code ») + des primitives a11y. C'est le socle idéal pour un design system M3 custom où **vous** détenez le code. (https://ui.shadcn.com)

### Versions constatées dans les repos locaux (`/home/ubuntu/md3/`)

| Dépôt local              | Emballer                             | Version constatée                              |
| ------------------------ | ------------------------------------ | ---------------------------------------------- |
| `matériel-web/`          | `@matériel/web`                      | **2.4.1** (dépend de `lit ^2.8 \|\| ^3`)       |
| `matériel-ui/`           | `@mui/matériel`                      | **9.0.1** (monorepo `@mui/monorepo` 9.0.1)     |
| `matériel-vent arrière/` | `@material-tailwind/react` / `-html` | **2.1.10** / **2.3.2** (v3 en beta hors arbre) |
| `shadcn-ui/`             | `ui` (registre/docs)                 | repo de référence shadcn                       |
| `tailwindcss/`           | `tailwindcss`                        | **4.3.0** (Tailwind v4)                        |

Chemins : `/home/ubuntu/md3/material-web/package.json`, `/home/ubuntu/md3/material-ui/packages/mui-material/package.json`, `/home/ubuntu/md3/material-tailwind/packages/material-tailwind-react/package.json`, `/home/ubuntu/md3/tailwindcss/packages/tailwindcss/package.json`.

---

## 2. Tableau comparatif maître

Colonnes : **MWC** = `@material/web` (Lit) · **MUI** = `@mui/material` (React) · **MT** = `material-tailwind` · **shadcn+TW** = shadcn-style + Tailwind · **DIY** = tokens M3 personnalisés + composants web/CSS (avec MCU).

| Critère                                         | MWC (@matériau/web)                                                                                           | MUI (Réagir)                                | matériau-vent arrière         | shadcn + vent arrière                       | DIY (jetons M3 + WC/CSS)            |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ----------------------------- | ------------------------------------------- | ----------------------------------- |
| **Exécution/framework**                         | Web components (Lit), agnostique                                                                              | Réagissez seulement                         | Réagir + HTML                 | React (registre), Tailwind                  | Agnostique                          |
| **Conformité M3 réelle**                        | **Élevée** (officielle M3)                                                                                    | **Faible** : basée Material **2**, pas M3   | **Partielle / cosmétique**    | Aucune par défaut (vous l'implémentez)      | **Aussi élevée que vous l'écrivez** |
| ↳ jetons couleur/typo/forme/élévation/mouvement | Jetons M3 natifs (accessoires personnalisés CSS)                                                              | Thème MUI ≠ tokens M3                       | Quelques tokens partiels      | À définir vous-même                         | À définir (basé spec `01-…`)        |
| ↳ HCT                                           | Oui (via MCU en amont)                                                                                        | Non natif                                   | Non                           | Non (ajout MCU possible)                    | **Oui** (MCU)                       |
| **Couleur dynamique**                           | Oui (theming via tokens générés MCU)                                                                          | Non (palette statique du thème)             | Non                           | Non — ajoutable via MCU                     | **Oui** (MCU au build/runtime)      |
| **Couverture composants**                       | **Partielle** : manquent data table, date/time picker, nav bar/rail/drawer, snackbar, tooltip, badge, search… | **Très large** (lib la plus complète React) | Grand (300+ blocs en v3 PRO)  | Large via primitives, mais à styliser       | Nulle au départ (vous construisez)  |
| **Thème**                                       | Jetons CSS, générateur de thèmes                                                                              | API `createTheme` puissante (mais M2)       | Configuration du vent arrière | Jetons CSS + Tailwind, contrôle total       | Total contrôle                      |
| **Accessibilité**                               | Bonne (intégrée aux WC)                                                                                       | Bonne (mûre)                                | Variable                      | Bonne (primitives Radix/Base sous-jacentes) | À votre charge                      |
| **Entretien 2026**                              | **Maintenance mode**, pas de roadmap                                                                          | **Très active**                             | Lentille bêta                 | **Très active** (modèle, pas dépendance)    | Dépend de MCU (actif) + vous        |
| **Courbe d'apprentissage**                      | Moyenne (web components, Lit en interne)                                                                      | Faible pour devs React                      | Faible                        | Moyenne (vous assemblez)                    | Élevée                              |

Détails par option : `02-material-web-deep-dive.md`, `03-mui-react-md3.md`, `04-shadcn-registry-tokens.md`, `05-tailwind-ecosystem-md3.md`.

---

## 3. Le constat clé : pas d'implémentation web first-party complète et maintenue

**Il n'existe pas, en 2026, de bibliothèque web officielle, complète _et_ activement développée de Material Design 3.** Trois faits convergent :

1. **`@material/web` (le candidat first-party) est en maintenance mode.** Le README du repo le dit noir sur blanc (« MWC is in maintenance mode pending new maintainers », `/home/ubuntu/md3/material-web/README.md`) et la roadmap officielle confirme « no current work planned for new features or components ». Le contexte : Google a réaffecté les ingénieurs vers son framework interne **Wiz**, et oriente désormais les utilisateurs Angular vers **Angular Material**. (https://github.com/material-components/material-web/discussions/5642, https://material-web.dev/about/roadmap/)

2. **`@material/web` est de toute façon incomplet.** La roadmap liste comme _non construits_ : data table, date picker, time picker, navigation bar/rail/drawer, snackbar, tooltip, badge, banner, bottom app bar/sheet, autocomplete, search, segmented button, top app bar. Pour un produit réel, ces absences sont bloquantes — et personne ne les comblera côté upstream.

3. **MUI, la lib la plus populaire, n'est pas M3.** Elle implémente Material **2**. L'adoption de M3 / Material You est une demande ouverte depuis 2021 (issue mui/material-ui#29345) **sans timeline confirmée** : en 2026 MUI a priorisé la stabilisation de **Base UI**, et a mis en pause Pigment CSS et Joy UI. (https://github.com/mui/material-ui/issues/29345, https://mui.com/blog/2026-and-beyond/)

**Ce que ça implique pour un projet « md3 » :**

- Aucune option « clé en main, officielle, complète » : tout choix exige un **arbitrage** (pureté M3 vs complétude vs vélocité vs framework).
- Le **dynamic color** (le différenciateur M3) ne vit pas dans une lib de composants prête à l'emploi côté React/Tailwind — il faut le brancher soi-même via **`material-color-utilities`** (la seule brique officielle réellement maintenue).
- Construire un **design system M3 maison** (tokens + composants détenus) devient l'option la plus pérenne pour qui veut de la conformité durable, car elle ne dépend pas d'une lib en sursis.

---

## 4. Scénarios d'architecture

### Scénario 1 — `@material/web` (Lit) + jetons M3 + Material Theme Builder

- **Stack** : `@material/web` 2.x (web components), tokens CSS générés via Material Theme Builder, `material-color-utilities` pour le dynamic color, Lit en dépendance interne. Consommable dans n'importe quel framework (ou sans framework).
- **Effort** : faible-moyen pour démarrer (web components prêts), **mais** moyen-élevé dès qu'il faut combler les composants manquants (data table, date picker, nav, snackbar…) à la main.
- **Conformité M3 atteignable** : **la plus haute** out-of-the-box — c'est la lib officielle, tokens et états M3 natifs.
- **Trade-offs** : projet en maintenance mode (risque de stagnation, bugs non corrigés), couverture incomplète, intégration React un peu rugueuse (wrappers, SSR avec web components à surveiller).
- **Quand le choisir** : besoin de **conformité M3 maximale** et **framework-agnostique**, périmètre de composants compatible avec ce que MWC couvre déjà, tolérance au risque de maintenance. Voir `02-material-web-deep-dive.md`.

### Scénario 2 — MUI (React) avec thème approchant M3

- **Stack** : `@mui/material` 9.x + thème custom (`createTheme`) calibré pour _ressembler_ à M3 (palette dérivée de MCU, formes arrondies, typescale Roboto/typo M3, élévations adaptées).
- **Effort** : faible côté composants (lib ultra-complète, DX React excellente), moyen pour le travail de theming visant l'approximation M3.
- **Conformité M3 atteignable** : **partielle, plafonnée**. On obtient un _look_ M3 sur une fondation Material 2 ; pas de dynamic color natif, pas de state layers/shape tokens M3 fidèles. C'est une _approximation_, pas du M3.
- **Trade-offs** : maturité, écosystème, vélocité maximale ; mais reniement de la pureté M3 et dépendance à une API (M2) qui pourrait diverger d'un futur M3 MUI.
- **Quand le choisir** : projet **React** où la **vélocité et la complétude composants** priment sur la conformité M3 stricte, et où « esthétique Material moderne » suffit. Voir `03-mui-react-md3.md`.

### Scénario 3 — Registre style shadcn + Tailwind + jetons M3 personnalisés + MCU (couleur dynamique)

- **Stack** : primitives accessibles (modèle shadcn/ui — code copié, détenu) + Tailwind v4 (`tailwindcss` 4.x) avec une **couche de tokens M3** (color roles, typescale, shape, elevation, motion) exposés en CSS custom props, **`material-color-utilities`** pour générer/appliquer les schémas HCT (statique au build ou dynamique au runtime/depuis image).
- **Effort** : **élevé au départ** (on construit le design system : tokens, mapping Tailwind, composants), faible ensuite (code détenu, évolutif).
- **Conformité M3 atteignable** : **aussi haute qu'on l'écrit**, dynamic color inclus — c'est la seule voie React/Tailwind qui offre du _vrai_ dynamic color HCT.
- **Trade-offs** : investissement initial important, responsabilité a11y et fidélité spec à votre charge ; en échange, **zéro dépendance à une lib en sursis**, contrôle total, pérennité.
- **Quand le choisir** : projet **React/Tailwind** voulant **dynamic color réel** et un design system **détenu et durable**, équipe prête à investir. Voir `04-shadcn-registry-tokens.md` et `05-tailwind-ecosystem-md3.md`.

### Scénario 4 — Hybride / fork de `@material/web` étendu

- **Stack** : `@material/web` comme base de composants M3 conformes + **fork/extension maison** pour les composants manquants (data table, date picker, nav…) et les correctifs, tokens via MCU/Theme Builder.
- **Effort** : **élevé et continu** : reprendre la maintenance là où Google s'arrête, suivre Lit, packager le fork.
- **Conformité M3 atteignable** : **très haute** (base officielle + extensions alignées spec).
- **Trade-offs** : vous devenez de facto mainteneur ; coût récurrent réel, mais conformité maximale sur tout le périmètre et indépendance vis-à-vis du gel upstream.
- **Quand le choisir** : organisation avec besoin **stratégique** de M3 framework-agnostique complet, ressources d'ingénierie pour porter la maintenance, horizon long terme. C'est l'option « si MWC n'avance plus, nous avançons ».

---

## 5. Recommandation finale (par profil de projet)

Il n'y a pas de gagnant universel — le bon choix dépend de trois axes : **framework (React vs agnostique)**, **besoin de dynamic color**, **vélocité vs pureté M3**.

- **Projet React, vélocité prioritaire, M3 « esprit » suffisant, pas de dynamic color** → **Scénario 2 (MUI)**. La complétude et la DX gagnent ; on assume une approximation M3. Ne pas attendre un M3 MUI à date connue (aucune timeline).

- **Projet React/Tailwind, dynamic color réel requis, design system durable** → **Scénario 3 (shadcn-style + Tailwind + tokens M3 + MCU)**. ✅ **Recommandation par défaut pour un projet « md3 » moderne côté React.** C'est la seule voie React qui livre du vrai HCT/dynamic color, sans s'enchaîner à une lib en maintenance ; le coût initial est compensé par la pérennité (code détenu, MCU activement maintenu).

- **Projet framework-agnostique (ou multi-framework / web components), conformité M3 maximale, périmètre couvert par MWC** → **Scénario 1 (`@material/web`)**, en acceptant le risque de maintenance et en se limitant aux composants existants (+ Beer CSS comme appoint pour combler des composants manquants si besoin, sans introduire une autre lib lourde).

- **Besoin stratégique de M3 agnostique _complet_, ressources d'ingénierie disponibles, horizon long** → **Scénario 4 (fork étendu de `@material/web`)**. À ne choisir que si l'on assume explicitement le rôle de mainteneur.

**Synthèse en une phrase** : pour la plupart des nouveaux projets web « md3 » en 2026, **construire son propre design system M3 sur Tailwind avec `material-color-utilities` pour le dynamic color (Scénario 3) est le meilleur compromis pérennité/conformité** ; on ne réserve `@material/web` (Scénario 1) qu'aux contextes framework-agnostiques tolérants au gel de maintenance, et MUI (Scénario 2) qu'aux projets React qui privilégient la vélocité sur la fidélité M3.

---

## Sources

**Repos locaux** : `/home/ubuntu/md3/material-web/` (README, `package.json` 2.4.1) · `/home/ubuntu/md3/material-ui/` (`@mui/material` 9.0.1) · `/home/ubuntu/md3/material-tailwind/` (react 2.1.10 / html 2.3.2) · `/home/ubuntu/md3/shadcn-ui/` · `/home/ubuntu/md3/tailwindcss/` (Tailwind 4.3.0).

**Internet** :

- Mode de maintenance MWC — https://github.com/material-components/material-web/discussions/5642
- Dépôt MWC et README — https://github.com/material-components/material-web
- MWC roadmap (composants non construits) — https://material-web.dev/about/roadmap/ · https://github.com/material-components/material-web/blob/main/docs/roadmap.md
- MUI adoption M3 (issue ouverte) — https://github.com/mui/material-ui/issues/29345
- Direction MUI 2026 (interface utilisateur de base, pauses) — https://mui.com/blog/2026-and-beyond/
- materials-color-utilities — https://github.com/material-foundation/material-color-utilities · https://www.npmjs.com/package/@material/material-color-utilities
- Générateur de thèmes matériels — https://github.com/material-foundation/material-theme-builder
- Bière CSS — https://github.com/beercss/beercss
- materials-tailwind (v3 bêta) — https://github.com/creativetimofficial/material-tailwind
- shadcn/ui — https://ui.shadcn.com
- Spécification M3 — https://m3.material.io/
