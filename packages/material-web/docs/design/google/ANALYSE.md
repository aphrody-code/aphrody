<!-- SPDX-License-Identifier: Apache-2.0 -->

# Analyse design — captures Google (référence material-web)

> **Référence design.** Ce document distille la grammaire visuelle publiée par Google
> pour Google Search (dark mode) et « AI Mode » / Gemini. Il ne reproduit aucun texte
> d'article : ce sont uniquement des **faits et tokens** extraits par échantillonnage des
> captures (`-colors` ImageMagick + recalage sur les tokens canoniques Google dark).
> But pour material-web : montrer comment cette grammaire se transpose **fidèlement** sur
> notre stack (`--md-sys-color-*` de `@aphrody/m3-tokens` + composants `@aphrody/m3-react`),
> sans copier les couleurs en dur (le seul hex conservé est le gradient « sparkle » de marque Gemini).
>
> Démo live correspondante : section **« Gemini AI Mode »** de l'exemple `examples/showcase`
> (`@aphrody/m3-showcase`) — surface de recherche reconstruite avec de vrais composants Md\*.

Captures dans ce dossier : `google.png` (accueil), `ai_mode.png` (AI Mode + autocomplétion),
`search.png` (SERP classique), `result.png` (réponse synthétisée AI Mode).

## Tokens dark extraits (consolidés)

| Rôle                     | Hex                                                  | Usage observé                                                         |
| ------------------------ | ---------------------------------------------------- | --------------------------------------------------------------------- |
| `bg` (fond page)         | `#1f2026` → `#202124`                                | fond global, gouttières                                               |
| `bg-deep`                | `#161719`                                            | bandeau header SERP (légèrement plus sombre)                          |
| `surface` (barre/­carte) | `#303134`                                            | barre de recherche, cartes Knowledge/sources                          |
| `surface-hover`          | `#3c4043`                                            | survol barre, chips, boutons header home                              |
| `border`                 | `#3c4043` / `#5f6368`                                | contour barre focus, séparateurs                                      |
| `text` primaire          | `#e8eaed` (`#f4f4f4` au logo)                        | titres, réponse IA                                                    |
| `text` secondaire        | `#bdc1c6` (`#c0c3cb`)                                | snippets, URL/breadcrumb, labels                                      |
| `text` tertiaire         | `#9aa0a6` (`#73767b`)                                | placeholder, méta discrète                                            |
| `link` (bleu dark)       | `#8ab4f8` (échant. `#3087fd`/`#1b60cd` aux contours) | titres de résultats, liens                                            |
| `link-visited`           | `#c58af9`                                            | liens visités                                                         |
| **gradient Gemini / AI** | `#4285f4` → `#9b72cb` → `#d96570`                    | icône « sparkle », contour/halo « AI Mode », chips de citation actifs |

> **Décision material-web** : on conserve la **structure** Google à l'identique (la grammaire),
> mais chaque rôle de couleur observé ci-dessus est exprimé via le **rôle M3 équivalent**
> (`--md-sys-color-*`), jamais en dur. La table dark Google sert de _guide d'intention_ : elle
> dit quel **rôle sémantique** (surface, on-surface, on-surface-variant, primary, outline…) joue
> chaque élément, et le moteur Material You (`@aphrody/m3-tokens/dynamic-color`) en produit la
> valeur réelle (light + dark, WCAG-AA par construction). Le **seul hex conservé** est le gradient
> « sparkle » Gemini (`#4285f4 → #9b72cb → #d96570`), couleur de marque transverse exposée en une
> custom property unique `--gemini-sparkle`.

### Correspondance token Google dark → rôle M3

| Token Google dark         | Rôle `--md-sys-color-*` material-web                              |
| ------------------------- | ----------------------------------------------------------------- |
| `bg` / `bg-deep`          | `background` / `surface` (et `surface-dim` pour le header sombre) |
| `surface` (barre, cartes) | `surface-container-high` / `surface-container`                    |
| `surface-hover`           | state-layer (hover) sur la surface ci-dessus                      |
| `border`                  | `outline-variant` (séparateurs) / `outline` (contour focus)       |
| `text` primaire           | `on-surface`                                                      |
| `text` secondaire         | `on-surface-variant`                                              |
| `text` tertiaire          | `on-surface-variant` à opacité réduite (placeholder)              |
| `link` bleu dark          | `primary` (les liens de résultat)                                 |
| `link-visited`            | `tertiary`                                                        |
| gradient Gemini           | `--gemini-sparkle` (custom property dédiée, hors système M3)      |

---

## 1. `google.png` — page d'accueil (état « repos »)

**Rôle** : point d'entrée vide, avant toute requête. C'est l'écran « hero » de la surface de recherche.

### Anatomie (grille verticale centrée)

1. **Top bar** (coin haut-droit, le reste vide) : liens texte `Gmail` `Images`, icône **Labs** (fiole),
   **app grid** (9 points 3×3), **avatar** (cercle). Alignement à droite, ~64 px de hauteur, padding ~16-24 px.
2. **Wordmark** centré (`Google` blanc `#f4f4f4`), large (~92 px de hauteur de glyphe), à ~38 % de la hauteur visible.
3. **Barre de recherche** : pill très arrondie (`border-radius` ≈ 24-28 px, hauteur ~46-52 px),
   largeur max ~584 px (desktop), fond `#303134`. Contenu :
   - gauche : icône **`+`** (ajout de contexte/sources — nouveau pattern AI),
   - centre : zone de saisie vide (placeholder absent ici),
   - droite : icône **micro** (couleur Google multicolore), icône **lens** (caméra), puis **pilule « AI Mode »**
     (icône sparkle + label), légèrement surélevée, fond translucide clair.
4. **Boutons** sous la barre : `Google Search` et `I'm Feeling Lucky` — surfaces `#3c4043`,
   coins arrondis 4 px, texte `#e8eaed`, padding ~9×16.
5. **Ligne de langue** : `Google offered in: Français` (lien bleu).

### Transposition material-web

- **Wordmark** → titre / greeting M3 (typographie display-large), couleur `on-surface`.
- **Barre de recherche pill** → `MdOutlinedTextField` ou `MdFilledTextField`, coins arrondis forcés via
  `--md-outlined-text-field-container-shape` (forme « full »), largeur max ~640 px centrée.
- **`+`** → `MdIconButton` en leading (icône `add`) : ajout de contexte/filtres.
- **micro / lens** → deux `MdIconButton` trailing (`mic`, `photo_camera`) ; on reste **monochrome on-surface**
  (la lib n'embarque pas le logo multicolore Google).
- **« AI Mode »** → `MdAssistChip` (ou `MdFilterChip`) avec un `MdIcon` `auto_awesome` (sparkle), bordé/halo
  par `--gemini-sparkle`. C'est l'**unique affordance** qui porte le gradient de marque.
- **boutons** → deux `MdElevatedButton` (ou `MdFilledTonalButton`) sous la pill.

---

## 2. `ai_mode.png` — accueil AI Mode + autocomplétion

**Rôle** : entrée dédiée « AI Mode », avec **rail latéral gauche** et **dropdown de suggestions** pendant la frappe.

### Anatomie

1. **Rail gauche** (~112 px) : **G** coloré en haut, puis pile d'icônes — **liste+sparkle** (historique/découvrir),
   **compose/edit** (nouvelle conversation). Vertical, centré, espacé ~80 px.
2. **Greeting** centré : `Hi, Alex. What's on your mind?` — titre ~44-52 px, `#e8eaed` (personnalisé au prénom).
3. **Barre de prompt** (plus large que l'accueil classique, ~760 px) : `+` à gauche, texte saisi
   (curseur visible), **`x` clear**, pilule **« AI Mode → »** (flèche = soumettre).
4. **Dropdown autocomplétion** (attaché sous la barre, même largeur, fond `#303134`, coins bas arrondis) :
   5 lignes, chacune **icône sparkle** (suggestion générée) + texte. Hauteur de ligne ~52 px,
   hover = surface plus claire.
5. **Footer dropdown** : `Report inappropriate predictions` + `Learn more` (lien bleu), aligné droite.

### Détails d'interaction

- L'icône **sparkle** devant chaque suggestion signale une **prédiction générée** (vs loupe = historique).
- La pilule passe de `AI Mode` (repos) à `AI Mode →` (prête à soumettre) dès qu'il y a du texte.

### Transposition material-web

- **Rail gauche** → `MdNavigationRail` + `MdNavigationRailItem` (icônes `auto_awesome`/`history` puis `edit_note`),
  fond `surface`.
- **Greeting** → titre M3 (`display`/`headline`), couleur `on-surface`.
- **Barre de prompt** → même `MdOutlinedTextField`/`MdFilledTextField` que §1, plus large ; leading `add`,
  trailing `close` (clear) + le chip **« AI Mode »** (`MdAssistChip` + sparkle + flèche `arrow_forward`).
- **Dropdown autocomplétion** → `MdList` + `MdListItem type="button"`, chaque item préfixé d'un `MdIcon`
  `auto_awesome` teinté `--gemini-sparkle`. Fond `surface-container-high`, séparateurs `outline-variant`.
- **Footer** → texte `on-surface-variant` + lien `primary`.

---

## 3. `search.png` — page de résultats classique (SERP)

**Rôle** : le cœur fonctionnel — liste de résultats organiques + **panneau de connaissance** (Knowledge Panel). Vue « Tous ».

### Anatomie (2 colonnes)

1. **Header SERP** (fond `#161719`, sticky) : logo Google compact à gauche, **barre de recherche remplie**
   (texte + `x` + micro + lens + loupe), avatar à droite.
2. **Barre d'onglets** sous le header : `Mode IA` · **`Tous`** (actif, soulignement) · `Images` · `Shopping` ·
   `Vidéos` · `Vidéos courtes` · `Actualités` · `Plus ▾`. Texte `#bdc1c6`, actif `#e8eaed` + indicateur.
3. **Colonne gauche (résultats, ~600 px)** — chaque résultat :
   - ligne site : **favicon** (pastille ronde) + **nom du site** (`#e8eaed`) + sur 2nde ligne `URL/breadcrumb`
     (`#bdc1c6`),
   - **titre** lien (`#8ab4f8`, ~20 px, cliquable),
   - **snippet** (2-3 lignes, `#bdc1c6`),
   - menu `⋮` à droite.
4. **Colonne droite (Knowledge Panel, ~380 px)** : titre, **grille d'images** (mosaïque 2×2 + bouton « plus »),
   description, source, paires clé/valeur, bloc **`Recherches associées`**.

### Transposition material-web

- **Header SERP** → barre sticky `surface`/`surface-dim`, contenant la même pill `MdOutlinedTextField` qu'aux §1-2
  (état « rempli ») + `MdAvatar` à droite.
- **Onglets** → `MdTabs` + `MdPrimaryTab` (`Mode IA` · `Tous` · `Images` · `Shopping` · …) ; l'indicateur d'onglet
  natif M3 remplace le soulignement Google.
- **Résultats organiques** → liste de cartes/lignes : `MdAvatar` (favicon), nom du site `on-surface`, URL
  `on-surface-variant`, **titre lien** en couleur `primary` (équivalent du bleu `#8ab4f8`), snippet
  `on-surface-variant`, `MdIconButton` `more_vert` à droite.
- **Knowledge Panel** → `MdElevatedCard` : image(s), titre, description `on-surface-variant`, paires clé/valeur,
  et un bloc « Recherches associées » en `MdChipSet`/`MdSuggestionChip`.

---

## 4. `result.png` — AI Mode, réponse synthétisée

**Rôle** : réponse rédigée + citations + cartes de sources + relance conversationnelle.

### Anatomie

1. **Rail gauche** (collapse / compose) — idem AI Mode.
2. **Bulle requête** alignée à droite, style « message utilisateur » (chip surface claire).
3. **Réponse générée** : paragraphe d'intro avec **fragments soulignés** (entités cliquables) + **chip de citation**
   en fin de phrase (pastille favicon). Puis structure rédigée : titres, **listes à puces** avec termes en gras,
   et **chips de citation inline**.
4. **Colonne droite — carte « sources »** : en-tête **`◫ N sites`**, puis cartes empilées : titre + source +
   favicon/thumb.
5. **Barre de relance** en bas : `Ask anything` (champ pleine largeur, fond `#303134`).

### Transposition material-web

- **Rail gauche** → `MdNavigationRail` (idem §2).
- **Bulle requête** → `MdAssistChip`/`MdInputChip` ou une `MdElevatedCard` compacte alignée à droite,
  fond `surface-container-high`.
- **Réponse générée** → contenu M3 typographié sur `on-surface` ; les **fragments cliquables** sont des liens
  `primary` ; chaque **citation** est un `MdAssistChip` compact (favicon + nom de source) — purement présentationnel
  côté démo (aucun contenu d'article n'est reproduit).
- **Carte « sources »** → `MdElevatedCard` listant des `MdListItem` (favicon `MdAvatar` + titre + source).
- **Barre de relance** → un dernier `MdOutlinedTextField` pleine largeur, leading `add`, trailing `send`.

> Note material-web : cette section est une **démo de grammaire visuelle**, pas un produit de génération de
> contenu. Aucune réponse n'est synthétisée par un modèle ; les textes de la démo sont des libellés statiques
> illustrant la mise en page. Le sujet du document est la **grammaire** (structure, rôles de couleur, formes,
> composants), pas le contenu.

---

## Synthèse — système de composants cible (material-web)

| Élément de grammaire                                 | Réf. image              | Composants `@aphrody/m3-react`                                                               |
| ---------------------------------------------------- | ----------------------- | ------------------------------------------------------------------------------------------------- |
| Tokens (dark Google → rôles M3 + `--gemini-sparkle`) | toutes                  | `--md-sys-color-*` + custom property `--gemini-sparkle`                                           |
| Top bar (liens + app-grid + avatar)                  | google, search, ai_mode | `MdIconButton`, `MdAvatar`                                                                        |
| Search pill (`+`, mic, lens, AI Mode)                | google, ai_mode, search | `MdOutlinedTextField`/`MdFilledTextField`, `MdIconButton`, `MdAssistChip` + `MdIcon auto_awesome` |
| Dropdown autocomplétion                              | ai_mode                 | `MdList`, `MdListItem`, `MdIcon auto_awesome`                                                     |
| Home (wordmark/greeting + field + boutons)           | google, ai_mode         | titre M3 + `MdElevatedButton`                                                                     |
| Rail latéral AI Mode                                 | ai_mode, result         | `MdNavigationRail`, `MdNavigationRailItem`                                                        |
| Onglets SERP                                         | search                  | `MdTabs`, `MdPrimaryTab`                                                                          |
| Résultats organiques                                 | search                  | `MdAvatar`, `MdIconButton`, liens `primary`, `MdDivider`                                          |
| Knowledge Panel                                      | search                  | `MdElevatedCard`, `MdChipSet`, `MdSuggestionChip`                                                 |
| Réponse synthétisée + citations + sources            | result                  | `MdElevatedCard`, `MdListItem`, `MdAssistChip`                                                    |
| Relance                                              | result                  | `MdOutlinedTextField` + `MdIconButton`                                                            |

### Contraintes d'implémentation (material-web)

- **Tout via les rôles `--md-sys-color-*`** : aucune couleur en dur hors le gradient `--gemini-sparkle`
  (défini une seule fois en custom property). Fonctionne en light **et** dark (toggle + seed picker
  Material You déjà présents dans le showcase).
- **Composants réels uniquement** : n'utiliser que des exports `Md*` réellement présents dans
  `@aphrody/m3-react` (cf. `packages/react/index.ts`). Pas de composant inventé.
- **Icônes Material Symbols** via `MdIcon` (texte enfant = ligature) ; la police variable est chargée par
  le showcase (plages d'axes) — sparkle = `auto_awesome`.
- **Style** : tabs + double-quotes (formatter), header SPDX sur les fichiers `.ts/.tsx`, pas d'emoji, bun only.

### Démo

La transposition vivante est la section **« Gemini AI Mode »** de `examples/showcase`
(`@aphrody/m3-showcase`) : home pill + chip AI Mode (gradient sparkle), dropdown d'autocomplétion,
rail latéral, et un mock SERP/Knowledge-panel compact — le tout sur les tokens M3, vérifié en dark.
