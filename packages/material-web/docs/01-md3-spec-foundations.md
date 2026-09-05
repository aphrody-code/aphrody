---
title: "Fondations MD3"
nav_order: 3
---

# Material Design 3 — Fondations (référence web, 2026)

> Document de référence technique et autonome sur les fondations de Material Design 3 (Material You) appliquées au web, à jour de l'état réel du design system en mai 2026 : couleur (HCT, dynamic color, color roles), design tokens, typographie, élévation, forme, mouvement, state layers, layout/breakpoints et accessibilité.

---

## Table des matières

1. [Qu'est-ce que MD3 / Material You](#1-quest-ce-que-md3--material-you)
2. [Système de couleur](#2-système-de-couleur)
3. [Design tokens](#3-design-tokens)
4. [Typographie](#4-typographie)
5. [Élévation](#5-élévation)
6. [Forme (shape)](#6-forme-shape)
7. [Mouvement (motion)](#7-mouvement-motion)
8. [State layers (couches d'état)](#8-state-layers-couches-détat)
9. [Espacement & layout](#9-espacement--layout)
10. [Accessibilité](#10-accessibilité)
11. [Sources](#11-sources)

---

## 1. Qu'est-ce que MD3 / Material You

**Material Design 3 (MD3)**, aussi appelé **Material You**, est le système de design open source de Google. Il a été annoncé à Google I/O en mai 2021 avec Android 12. Sa caractéristique fondatrice est le **dynamic color** (thème dynamique) : le système extrait une palette de couleurs cohérente depuis le fond d'écran (ou une couleur source) de l'utilisateur et l'applique à toute l'interface.

### Historique : M2 → M3

| Axe              | Material 2 (2018)                                                                | Material 3 / Material You (2021+)                                                             |
| ---------------- | -------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Couleur          | Palette fixe (primary, secondary, accents) + overlays d'opacité pour l'élévation | **Dynamic color** via espace **HCT**, **color roles** sémantiques, palettes tonales 0-100     |
| Élévation        | Surcouches d'opacité blanches en dark                                            | **Tonal elevation** (surface tint) en plus des shadows                                        |
| Typographie      | Type scale H1-H6 / body / caption / button                                       | Type scale par **rôles** : display / headline / title / body / label × large / medium / small |
| Forme            | Échelle limitée                                                                  | **Shape scale** étendue, corner families                                                      |
| Tokens           | Implicites                                                                       | **Design tokens** à 3 niveaux (ref / sys / component) explicites et exportables               |
| Personnalisation | Branding statique                                                                | Thème généré depuis le wallpaper, contrast levels utilisateur                                 |

### Material 3 Expressive (2025) et statut en 2026

À **The Android Show: I/O Edition (mai 2025)**, Google a annoncé **Material 3 Expressive (M3E)** pour Android 16 et Wear OS 6. Ce n'est **pas** un nouveau design system mais une évolution de M3, issue de la plus grande campagne de recherche UX de Google (46 études, 18 000+ participants). Apports clés :

- **Motion physics system** : moteur d'animations à ressort (springs) qui remplace progressivement le modèle easing/duration historique.
- **Shape library étendue** : 35 nouvelles formes abstraites + **shape morphing** (transitions animées de forme), échelle de corner radius plus granulaire (10 paliers).
- **Typographie « emphasized »** : 15 styles miroirs de l'échelle de base, en plus gras / plus contrastés.
- **Nouveaux composants** : split buttons, button groups, docked toolbars, FAB menus, loading indicators.

Déploiement Android : Pixel sous Android 16 à partir de septembre 2025 (QPR1). Les apps Google (Gmail, Docs, Chrome, Keep, Files…) ont été migrées principalement fin 2025.

### Statut sur le web en 2026 (point critique)

L'implémentation **web** est largement en retard sur le natif :

- **`@material/web` (Material Web Components, MWC)** est en **maintenance mode**. La bibliothèque supporte le modèle de design tokens M3 et le styling M3 « classique », mais **M3 Expressive n'est pas implémenté sur le web** et il n'y a plus de développement de fonctionnalités actif.
- Les chantiers M3 réellement actifs sont **natifs** : **Jetpack Compose** (Android) et **Flutter**. Sur le web, certains outils recommandent encore des kits M2.
- Conséquence pratique : pour un projet web 2026, MD3 reste pleinement utilisable via ses **fondations tokenisées** (couleur HCT, type scale, élévation tonale, shape, motion easing/duration) implémentées en **CSS custom properties**, mais sans les nouveautés Expressive (springs, shape morphing, nouveaux composants).
- L'outil canonique pour générer un thème reste le **Material Theme Builder** et la bibliothèque **`material-color-utilities`**.

Sources principales : [m3.material.io](https://m3.material.io/), [material-web.dev](https://material-web.dev/), [blog.google — M3 Expressive launch](https://blog.google/products-and-platforms/platforms/android/material-3-expressive-android-wearos-launch/).

---

## 2. Système de couleur

### 2.1 L'espace colorimétrique HCT

MD3 repose sur **HCT** (**H**ue, **C**hroma, **T**one), un espace créé par Google et combinant le modèle d'apparence **CAM16** avec la luminosité perceptuelle **L\*** (CIELAB). Il tient compte des conditions de visualisation.

| Dimension                | Plage                              | Rôle                                                             |
| ------------------------ | ---------------------------------- | ---------------------------------------------------------------- |
| **Hue** (teinte)         | 0–360°                             | La couleur perçue (rouge, bleu…)                                 |
| **Chroma** (chrominance) | 0 → ~120 (variable selon hue/tone) | L'intensité / saturation                                         |
| **Tone** (ton)           | 0–100                              | La clarté perceptuelle (0 = noir, 100 = blanc) — alignée sur L\* |

Avantage clé sur HSL/HSV : **Tone** est perceptuellement uniforme, donc deux couleurs de tones différents garantissent un **contraste mesurable et prévisible**. C'est ce qui rend le système accessible par construction.

### 2.2 Tonal palettes (palettes tonales)

Depuis la couleur source, l'algorithme dérive **5 palettes tonales clés** : **Primary, Secondary, Tertiary, Neutral, Neutral Variant** (plus une palette **Error** prédéfinie). Une palette tonale est une gamme de couleurs qui ne varie **que par le tone** (hue et chroma fixés).

Chaque palette expose **13 tones** standard :

```
0 · 10 · 20 · 30 · 40 · 50 · 60 · 70 · 80 · 90 · 95 · 99 · 100
```

(0 = noir, 100 = blanc ; les tones intermédiaires servent de matière première aux color roles.)

### 2.3 Dynamic color & variantes de scheme

Le **dynamic color** génère un scheme complet à partir d'une couleur source (extraite d'un wallpaper, d'un logo de marque, ou de contenu). La bibliothèque `material-color-utilities` fournit plusieurs **constructeurs de scheme** (variantes), tous paramétrés par `(sourceColorHct, isDark, contrastLevel)` :

| Variante                     | Caractéristique                               |
| ---------------------------- | --------------------------------------------- |
| **TonalSpot**                | Scheme Material You **par défaut**, équilibré |
| **Neutral**                  | Palette désaturée, proche du gris             |
| **Vibrant**                  | Plus saturé / coloré                          |
| **Expressive**               | Couleurs plus variées et diverses             |
| **Fidelity**                 | Reste au plus près de la couleur source       |
| **Content**                  | Dérive la palette du contenu in-app           |
| **Monochrome**               | Niveaux de gris                               |
| **Rainbow** / **FruitSalad** | Variantes ludiques très colorées              |

Modules de `material-color-utilities` : `hct`, `palettes`, `scheme`, `dynamiccolor`, `blend` (harmonisation), `quantize` (image → palette), `score` (classement des couleurs candidates au theming), `contrast` (mesure et récupération de couleurs contrastées).

### 2.4 Color roles (rôles de couleur)

Les composants ne référencent **jamais** une couleur brute : ils référencent un **rôle sémantique**. Chaque rôle est rempli par un tone précis de la palette correspondante (avec un mapping différent en light et dark pour garantir le contraste).

**Familles d'accent (× primary, secondary, tertiary, error) :**

| Rôle                   | Usage                                                      | Tone light / dark (primary, indicatif) |
| ---------------------- | ---------------------------------------------------------- | -------------------------------------- |
| `primary`              | Couleur d'accent principale (FAB, boutons remplis, actifs) | 40 / 80                                |
| `on-primary`           | Contenu posé **sur** `primary`                             | 100 / 20                               |
| `primary-container`    | Conteneur tonal moins emphatique                           | 90 / 30                                |
| `on-primary-container` | Contenu sur le conteneur                                   | 30→10 / 90                             |

Idem pour `secondary`/`on-secondary`/`secondary-container`/`on-secondary-container`, `tertiary`/…, et `error`/`on-error`/`error-container`/`on-error-container`.

**Surfaces & neutres :**

| Rôle                           | Usage                                                                                  |
| ------------------------------ | -------------------------------------------------------------------------------------- |
| `surface`                      | Surface de fond par défaut des composants                                              |
| `surface-dim`                  | Surface la plus sombre (light)                                                         |
| `surface-bright`               | Surface la plus claire (light)                                                         |
| `surface-container-lowest`     | Conteneur le plus bas (le plus contrasté avec le fond)                                 |
| `surface-container-low`        | Conteneur bas                                                                          |
| `surface-container`            | Conteneur par défaut                                                                   |
| `surface-container-high`       | Conteneur haut                                                                         |
| `surface-container-highest`    | Conteneur le plus élevé                                                                |
| `surface-variant`              | Variante de surface (historique ; remplacée en pratique par les `surface-container-*`) |
| `on-surface`                   | Texte / icônes de premier plan sur surface                                             |
| `on-surface-variant`           | Texte / icônes secondaires, contours d'icônes                                          |
| `outline`                      | Bordures, séparateurs à fort contraste                                                 |
| `outline-variant`              | Séparateurs discrets, contours décoratifs                                              |
| `background` / `on-background` | Fond global (souvent aligné sur `surface`)                                             |

**Rôles inverses & utilitaires :**

| Rôle                 | Usage                                       |
| -------------------- | ------------------------------------------- |
| `inverse-surface`    | Surface inversée (snackbars)                |
| `inverse-on-surface` | Contenu sur surface inversée                |
| `inverse-primary`    | Primary lisible sur surface inversée        |
| `surface-tint`       | Couleur de teinte d'élévation (= `primary`) |
| `scrim`              | Voile opaque derrière les modales           |
| `shadow`             | Couleur d'ombre                             |

> Convention de nommage des rôles « on- » : `on-X` est toujours la couleur du **contenu** posé sur `X`, choisie pour atteindre un contraste suffisant.

### 2.5 Light / dark schemes

Un même set de **color roles** est rempli par deux mappings de tones distincts. Exemple typique pour `primary` : tone **40** en light, tone **80** en dark ; `surface` tend vers les tones hauts (98-99) en light et bas (6-10) en dark. Le composant ne connaît que le rôle, pas le tone — c'est le scheme qui résout.

### 2.6 Contrast levels (niveaux de contraste)

Introduits comme réglage utilisateur dans **Android 14**, les contrast levels modulent l'écart de tone entre rôles :

| Niveau             | Effet                        |
| ------------------ | ---------------------------- |
| **Standard** (0.0) | Apparence par défaut         |
| **Medium** (0.5)   | Contraste intermédiaire      |
| **High** (1.0)     | Contraste fortement augmenté |

Si l'app utilise déjà le dynamic color, le support du contraste est **gratuit** (les rôles ajustent automatiquement leurs tones). `contrastLevel` est un paramètre du constructeur de scheme (`-1.0` à `1.0`). Pour des thèmes de marque custom, l'API Android `ColorContrastOptions` permet de fournir des theme overlays `setMediumContrastThemeOverlay()` / `setHighContrastThemeOverlay()`.

---

## 3. Design tokens

Les **design tokens** sont la source unique de vérité des valeurs de style, partagée entre design (Figma), outils et code. Le nommage est auto-documenté, sous la forme **`md.<class>.<group>.<role>`** (ex. `md.sys.color.primary`).

### 3.1 Taxonomie à 3 niveaux

| Niveau               | Préfixe               | Nature                                                                              | Exemple                                                          |
| -------------------- | --------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| **Reference tokens** | `md.ref`              | Valeurs brutes, statiques, agnostiques du contexte (un hex, une font-family, un dp) | `md.ref.palette.primary40` = `#6750A4`                           |
| **System tokens**    | `md.sys`              | Décisions sémantiques (rôles) pointant vers des ref tokens                          | `md.sys.color.primary` → `md.ref.palette.primary40`              |
| **Component tokens** | `md.comp.<composant>` | Attributs d'un composant pointant vers des system tokens (ou des valeurs concrètes) | `md.comp.filled-button.container-color` → `md.sys.color.primary` |

C'est cette indirection qui fait fonctionner le dynamic color : un nouveau wallpaper régénère les **ref tokens**, les **sys tokens** restent stables, et tous les composants se mettent à jour automatiquement car ils ne référencent que des rôles.

### 3.2 Format & implémentation web

Sur le web, les tokens sont des **CSS custom properties** :

- Palette de référence : `--md-ref-palette-primary90`
- Typeface : `--md-ref-typeface-<token>`
- Couleur système : `--md-sys-color-<role>`
- Type scale système : `--md-sys-typescale-<scale>-<size>-<property>`
- Composant : `--md-<composant>-<propriété>`

Exemple de chaînage :

```css
:root {
  /* reference token */
  --md-ref-palette-primary90: #ffd7f0;
  /* system token (rôle) */
  --md-sys-color-primary-container: var(--md-ref-palette-primary90);
}

/* component token */
md-filled-button.error {
  --md-filled-button-container-color: var(--md-sys-color-error);
  --md-filled-button-label-text-color: var(--md-sys-color-on-error);
}
```

Workflow recommandé : tokens maintenus dans Figma/Penpot → export JSON au format **Design Token Community Group (DTCG)** → transformation par **Style Dictionary** / **Cobalt UI** vers les plateformes (web, Android, iOS).

> Limite web : MWC ne supporte pas les `--md-ref-palette` ; le theming par palette dynamique côté web passe donc par génération en amont (Theme Builder / `material-color-utilities`) et injection des `--md-sys-color-*`.

Source : [Design tokens — m3.material.io](https://m3.material.io/foundations/design-tokens/overview), [material-foundation/material-tokens](https://github.com/material-foundation/material-tokens/blob/main/tokens.md).

---

## 4. Typographie

### 4.1 Type scale M3

5 **rôles** (Display, Headline, Title, Body, Label) × 3 **tailles** (Large, Medium, Small) = **15 styles de base**. Police par défaut : **Roboto**. Seuls deux poids sont utilisés : **Regular (400)** et **Medium (500)**.

| Style           | Taille | Line height | Poids | Tracking (letter-spacing) |
| --------------- | ------ | ----------- | ----- | ------------------------- |
| Display Large   | 57 px  | 64 px       | 400   | −0.25 px                  |
| Display Medium  | 45 px  | 52 px       | 400   | 0 px                      |
| Display Small   | 36 px  | 44 px       | 400   | 0 px                      |
| Headline Large  | 32 px  | 40 px       | 400   | 0 px                      |
| Headline Medium | 28 px  | 36 px       | 400   | 0 px                      |
| Headline Small  | 24 px  | 32 px       | 400   | 0 px                      |
| Title Large     | 22 px  | 28 px       | 400   | 0 px                      |
| Title Medium    | 16 px  | 24 px       | 500   | 0.15 px                   |
| Title Small     | 14 px  | 20 px       | 500   | 0.1 px                    |
| Body Large      | 16 px  | 24 px       | 400   | 0.5 px                    |
| Body Medium     | 14 px  | 20 px       | 400   | 0.25 px                   |
| Body Small      | 12 px  | 16 px       | 400   | 0.4 px                    |
| Label Large     | 14 px  | 20 px       | 500   | 0.1 px                    |
| Label Medium    | 12 px  | 16 px       | 500   | 0.5 px                    |
| Label Small     | 11 px  | 16 px       | 500   | 0.5 px                    |

**Usages indicatifs** : Display → grands titres expressifs / hero ; Headline → titres de section ; Title → titres de composants / app bar ; Body → texte courant ; Label → texte de boutons, chips, onglets, captions interactifs.

### 4.2 Tokens de type scale (web)

```css
--md-sys-typescale-body-large-font: "Roboto", sans-serif;
--md-sys-typescale-body-large-size: 1rem; /* 16px */
--md-sys-typescale-body-large-line-height: 1.5rem; /* 24px */
--md-sys-typescale-body-large-weight: 400;
--md-sys-typescale-body-large-tracking: 0.5px;
```

### 4.3 Variable fonts (Roboto Flex) & emphasized

- **Roboto Flex** est la version **variable font** de Roboto : un seul fichier expose des axes continus (`wght`, `wdth`, `opsz`, `slnt`, et axes optiques fins). Elle permet d'interpoler poids et largeur sans charger plusieurs fichiers — utile pour la hiérarchie typographique adaptative de M3 et pour les motions de poids.
- Le rôle `--md-ref-typeface-*` change la font-family et les poids pour **tous** les tokens système et composants d'un coup.
- **M3 Expressive** ajoute **15 styles « emphasized »** : ils reprennent l'échelle de base avec un **poids plus élevé** et des ajustements mineurs, pour renforcer l'attention sur titres et actions clés. (Non disponible sur `@material/web` en 2026.)

Source : [Typography — m3.material.io](https://m3.material.io/styles/typography/applying-type), [Material Web — Typography](https://material-web.dev/theming/typography/).

---

## 5. Élévation

En MD3, l'élévation est la distance relative entre deux surfaces sur l'axe z. Elle s'exprime via **6 niveaux** mappés à des valeurs dp, et se rend par **deux mécanismes** :

- **Tonal elevation (surface tint)** : la surface est teintée d'une surcouche dérivée de `primary` (`surface-tint`). Plus le niveau est haut, plus la teinte est marquée. **Mécanisme recommandé par défaut en M3.**
- **Shadow elevation** : ombre portée traditionnelle. À réserver aux éléments nécessitant plus de focus, ou posés sur un fond chargé (photos, dégradés) où la teinte seule ne suffit pas à séparer visuellement.

| Niveau  | dp    | Surface tint (indicatif) | Composants typiques                        |
| ------- | ----- | ------------------------ | ------------------------------------------ |
| Level 0 | 0 dp  | 0 %                      | Boutons remplis « à plat », cards outlined |
| Level 1 | 1 dp  | ~5 %                     | Cards élevées, bottom sheets               |
| Level 2 | 3 dp  | ~8 %                     | Navigation bar, menus                      |
| Level 3 | 6 dp  | ~11 %                    | FAB, dialogs                               |
| Level 4 | 8 dp  | ~12 %                    | (navigation drawers, états transitoires)   |
| Level 5 | 12 dp | ~14 %                    | Élévation maximale                         |

> En dark theme, c'est la **tonal elevation** qui crée la hiérarchie (les ombres y sont peu visibles) ; en light theme et sur fonds chargés, les ombres restent pertinentes. La teinte d'élévation est de plus en plus remplacée, dans les implémentations récentes, par les rôles `surface-container-*` (un conteneur plus « haut » = un tone plus contrasté), qui obtiennent un effet d'élévation sans surcouche dynamique.

Source : [Elevation — m3.material.io](https://m3.material.io/styles/elevation/applying-elevation).

---

## 6. Forme (shape)

La **shape scale** définit le rayon d'arrondi des coins des conteneurs, du carré (none) au pilulaire (full).

### 6.1 Échelle M3 « classique »

| Token       | Corner radius             |
| ----------- | ------------------------- |
| None        | 0 dp                      |
| Extra Small | 4 dp                      |
| Small       | 8 dp                      |
| Medium      | 12 dp                     |
| Large       | 16 dp                     |
| Extra Large | 28 dp                     |
| Full        | 9999 dp (pilule / cercle) |

### 6.2 Échelle étendue (M3 Expressive)

M3 Expressive ajoute des paliers intermédiaires (échelle à ~10 crans) et 35 formes abstraites :

| Token additionnel     | Corner radius |
| --------------------- | ------------- |
| Large Increased       | 20 dp         |
| Extra Large Increased | 32 dp         |
| Extra Extra Large     | 48 dp         |

### 6.3 Corner families & morphing

- **Corner families** : deux familles de découpe des coins — **rounded** (arrondi, par défaut) et **cut** (chanfrein / coin coupé). On peut appliquer un radius par coin individuellement.
- **Shape morphing** (Expressive) : transitions animées d'une forme à l'autre (ex. carré → squircle au press d'un bouton). Implémenté en Compose ; **non disponible sur le web** en 2026.
- Tokens web : `--md-sys-shape-corner-<token>` (ex. `--md-sys-shape-corner-medium: 12px;`).

> Note : certaines implémentations Compose montrent `extraLarge = 24dp` — c'est une personnalisation d'app, la valeur de token par défaut reste **28 dp**.

Source : [Shape corner radius scale — m3.material.io](https://m3.material.io/styles/shape/corner-radius-scale).

---

## 7. Mouvement (motion)

Le système historique M3 combine **easing** (courbe) et **duration** (durée), appariés pour définir le ressenti d'une animation. M3 Expressive introduit en plus un **motion physics system** (ressorts/springs) qui remplace progressivement ce modèle — mais ce dernier n'est **pas implémenté sur le web** en 2026 ; le web utilise donc easing/duration.

### 7.1 Easing tokens (courbes)

| Token                 | Valeur                                                                                             |
| --------------------- | -------------------------------------------------------------------------------------------------- |
| Standard              | `cubic-bezier(0.2, 0, 0, 1)`                                                                       |
| Standard Decelerate   | `cubic-bezier(0, 0, 0, 1)`                                                                         |
| Standard Accelerate   | `cubic-bezier(0.3, 0, 1, 1)`                                                                       |
| Emphasized            | courbe en 2 segments — path `M 0,0 C 0.05,0 0.133333,0.06 0.166666,0.4 C 0.208333,0.82 0.25,1 1,1` |
| Emphasized Decelerate | `cubic-bezier(0.05, 0.7, 0.1, 1)`                                                                  |
| Emphasized Accelerate | `cubic-bezier(0.3, 0, 0.8, 0.15)`                                                                  |
| Linear                | `cubic-bezier(0, 0, 1, 1)`                                                                         |

- **Standard** : transitions communes qui commencent et finissent à l'écran.
- **Decelerate** (« ease out ») : éléments qui **entrent** à l'écran à pleine vitesse puis ralentissent.
- **Accelerate** (« ease in ») : éléments qui **sortent** de l'écran en accélérant (scale/opacity → 0).
- **Emphasized** : courbe expressive recommandée pour les transitions de premier plan (la courbe Emphasized standard n'est pas un simple bézier mais une spline en 2 segments).

> Astuce web : la courbe Emphasized full n'étant pas exprimable en un seul `cubic-bezier`, on l'approxime sur le web par **Emphasized Decelerate** pour les entrées et **Emphasized Accelerate** pour les sorties, ou par une animation `@keyframes` suivant le path.

### 7.2 Duration tokens

16 paliers, regroupés en Short / Medium / Long / Extra Long. Principe : **la durée croît avec l'aire / la distance parcourue** par l'animation.

| Token    | ms  |     | Token        | ms   |
| -------- | --- | --- | ------------ | ---- |
| Short 1  | 50  |     | Long 1       | 450  |
| Short 2  | 100 |     | Long 2       | 500  |
| Short 3  | 150 |     | Long 3       | 550  |
| Short 4  | 200 |     | Long 4       | 600  |
| Medium 1 | 250 |     | Extra Long 1 | 700  |
| Medium 2 | 300 |     | Extra Long 2 | 800  |
| Medium 3 | 350 |     | Extra Long 3 | 900  |
| Medium 4 | 400 |     | Extra Long 4 | 1000 |

### 7.3 Transitions M3

Patterns de transition standardisés (container transform, shared axis, fade through, fade) appariant ces tokens. Repères : micro-interactions (hover, state layer) → Short ; transitions de composant → Medium ; transitions de navigation / container transform → Long + Emphasized.

Source : [Easing and duration tokens — m3.material.io](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs), [material-components-android — Motion.md](https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md).

---

## 8. State layers (couches d'état)

Une **state layer** est une surcouche semi-transparente posée sur un élément pour signaler son état d'interaction. Elle utilise **la couleur du contenu** de l'élément (`on-*` du rôle concerné), à une **opacité fixe par état**. Une seule state layer s'applique à la fois.

| État            | Opacité de la state layer |
| --------------- | ------------------------- |
| Enabled (repos) | 0 % (aucune)              |
| **Hover**       | **8 %**                   |
| **Focus**       | **10 %**                  |
| **Pressed**     | **10 %**                  |
| **Dragged**     | **16 %**                  |

**Exemple de résolution** : un bouton dont le conteneur utilise `surface` et le contenu `primary` aura une state layer en `primary` à l'opacité de l'état. Un bouton `secondary-container` / `on-secondary-container` aura une state layer en `on-secondary-container`.

**État Disabled** (cas distinct, pas une state layer mais une opacité directe) :

| Élément désactivé              | Opacité  |
| ------------------------------ | -------- |
| Contenu (texte / icône)        | **38 %** |
| Conteneur (fond / remplissage) | **12 %** |

**Ripple** : l'effet d'ondulation au press (Android/Material) est le rendu animé de la state layer pressed — une expansion radiale de la couleur de contenu à l'opacité pressed depuis le point de contact. Sur le web, MWC fournit le composant `md-ripple`.

Source : [States — state layers — m3.material.io](https://m3.material.io/foundations/interaction/states/state-layers).

---

## 9. Espacement & layout

### 9.1 Grille de base & spacing

- MD3 s'appuie sur une grille de base **4 dp** ; les espacements et tailles sont des multiples de 4 (8, 12, 16, 24…). **M3 Expressive** met en avant un système d'espacement à pas de **8 dp** pour les layouts.
- Les marges et gouttières (gutters) varient selon le breakpoint (typiquement 16 dp de marge en compact, 24 dp+ en medium/expanded).

### 9.2 Window size classes (breakpoints adaptatifs)

Les **window size classes** sont des breakpoints d'opinion basés sur la **largeur de la fenêtre disponible** (pas la taille physique de l'appareil). Largeur et hauteur sont classées séparément ; la **largeur** est la plus déterminante pour l'UI. M3 définit **5 classes de largeur** :

| Window size class | Largeur (dp) | Cibles typiques                                     | Navigation recommandée                       |
| ----------------- | ------------ | --------------------------------------------------- | -------------------------------------------- |
| **Compact**       | < 600        | Téléphones en portrait                              | Navigation bar (bas)                         |
| **Medium**        | 600 – 839    | Petites tablettes, foldables, téléphones en paysage | Navigation rail                              |
| **Expanded**      | 840 – 1199   | Tablettes, desktops                                 | Navigation rail / drawer                     |
| **Large**         | 1200 – 1599  | Desktop, grands écrans                              | Navigation drawer permanent + multi-panneaux |
| **Extra-large**   | ≥ 1600       | Très grands écrans, displays connectés              | Drawer permanent + layouts multi-panneaux    |

Les classes **Large** et **Extra-large** (à partir de 1200 dp) ont été ajoutées après les 3 originelles (Compact/Medium/Expanded) pour cibler desktop et écrans externes. Elles pilotent des décisions de haut niveau : pattern de navigation, nombre de panneaux (panes), densité.

En Compose : `currentWindowAdaptiveInfo()` (`androidx.compose.material3.adaptive`) ; ajouter `supportLargeAndXLargeWidth = true` pour les deux plus grandes classes.

> Sur le web, ces dp se traduisent en breakpoints CSS (`min-width`) en px ; les valeurs canoniques 600 / 840 / 1200 / 1600 servent de seuils de media queries.

### 9.3 Canonical layouts & panes

MD3 propose des **canonical layouts** (list-detail, supporting pane, feed) qui répartissent le contenu en panneaux selon la window size class — un seul panneau en Compact, deux panneaux ou plus à partir de Medium/Expanded.

Source : [Window size classes — m3.material.io](https://m3.material.io/foundations/layout/applying-layout/window-size-classes), [Android — use window size classes](https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes).

---

## 10. Accessibilité

L'accessibilité est intégrée par construction dans MD3, principalement via le système tonal (contraste mesurable) et les contrast levels.

### 10.1 Contraste (WCAG)

| Cible                                               | Ratio minimum |
| --------------------------------------------------- | ------------- |
| Texte normal                                        | **4.5:1**     |
| Texte large (≥ 18 pt regular / 14 pt bold)          | **3:1**       |
| Éléments non-textuels (icônes, bordures, contrôles) | **3:1**       |

Le système de **tonal palettes** garantit ces ratios par défaut : un écart de tone suffisant entre un rôle et son `on-*` produit le contraste requis. Les **contrast levels** (Standard / Medium / High) permettent à l'utilisateur de renforcer encore le contraste. Les éléments purement décoratifs (logos, illustrations) sont dispensés de ces ratios sauf s'ils portent une fonction.

### 10.2 Cibles tactiles (touch targets)

- **Minimum 48 × 48 dp** par cible interactive, même si l'élément visuel est plus petit (ex. une icône 24 × 24 dp est entourée d'un padding pour atteindre 48 × 48 dp).
- **Espacement ≥ 8 dp** entre cibles tactiles pour éviter les erreurs de tap.
- Pour les entrées de précision (souris, trackpad), la cible peut être plus petite.

### 10.3 Bonnes pratiques complémentaires

- Ne pas coder une information par la **couleur seule** (ajouter icône / texte / forme).
- Respecter les rôles `on-*` pour le texte (jamais une couleur de fond comme couleur de contenu).
- Disabled : opacité 38 % (contenu) / 12 % (conteneur) — état explicitement exempté des ratios de contraste car non interactif.
- Supporter les tailles de texte dynamiques et le mode high-contrast système.

Source : [Accessibility — m3.material.io](https://m3.material.io/foundations/designing/structure), [Android — accessibility](https://developer.android.com/design/ui/mobile/guides/foundations/accessibility).

---

## 11. Sources

Sources officielles et de référence (consultées en mai 2026) :

- Material Design 3 — site officiel : <https://m3.material.io/>
- Color (HCT, roles, schemes) : <https://m3.material.io/styles/color/system/how-the-system-works>, <https://m3.material.io/styles/color/roles>
- Design tokens : <https://m3.material.io/foundations/design-tokens/overview>
- Typography : <https://m3.material.io/styles/typography/applying-type>
- Elevation : <https://m3.material.io/styles/elevation/applying-elevation>
- Shape : <https://m3.material.io/styles/shape/corner-radius-scale>
- Motion (easing & duration) : <https://m3.material.io/styles/motion/easing-and-duration/tokens-specs>
- States (state layers) : <https://m3.material.io/foundations/interaction/states/state-layers>
- Layout / window size classes : <https://m3.material.io/foundations/layout/applying-layout/window-size-classes>
- Accessibility : <https://m3.material.io/foundations/designing/structure>
- Material Web (implémentation web, maintenance mode) : <https://material-web.dev/>, theming : <https://material-web.dev/theming/material-theming/>
- `material-color-utilities` (HCT, schemes) : <https://github.com/material-foundation/material-color-utilities>
- `material-tokens` (référence des tokens) : <https://github.com/material-foundation/material-tokens/blob/main/tokens.md>
- `material-components-android` — Motion : <https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md>
- Jetpack Compose — Material 3 : <https://developer.android.com/develop/ui/compose/designsystems/material3>
- Window size classes (Android) : <https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes>
- M3 Expressive (annonce) : <https://blog.google/products-and-platforms/platforms/android/material-3-expressive-android-wearos-launch/>

> Note de fiabilité : les valeurs des type/shape/motion/state tables ont été vérifiées via les dépôts GitHub officiels (`material-components-android`, `material-color-utilities`, `material-tokens`) et `material-web.dev`, qui exposent les valeurs concrètes ; les pages `m3.material.io` sont rendues côté client (JS) et ne fournissent pas leur corps en texte brut via fetch. Les valeurs de surface-tint par niveau d'élévation sont indicatives (héritées de M2 et variables selon l'implémentation).
