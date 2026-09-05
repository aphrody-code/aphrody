# Aphrody Unified UI / GUI / Web App / Material Design Documentation

> **AVERTISSEMENT (mis à jour 2026-06-04) — document agrégé largement HISTORIQUE.**
> Ce fichier concatène d'anciennes sources (`DESIGN.md`, `DESIGN-GOOGLE.md`,
> `md3/*`, `terminal/*`) désormais supprimées. Plusieurs parties décrivent une
> architecture **abandonnée** : « God Mode / Google OS » et l'« Architecture à
> 3 Piliers » (Pilier II fork C++ de Windows Terminal, etc.). État réel actuel :
> - **Cœur 100 % Rust** : le binaire `aphrody`, le terminal LLM-first et le
>   systems/FFI restent 100 % Rust. Pas de fork C++ de Windows Terminal.
> - **UI = monorepo Material Design 3 Bun/TS** (pivot polyglotte 2026-05-21,
>   fusion `material-web` 2026-06-01) : les libs `@aphrody-code/*` dans
>   `packages/*` (Lit `material-web`, wrappers React `m3-react`, `m3-tokens`,
>   `m3-motion`, `m3-theme`, …) constituent la surface UI réutilisable.
>   Bun/TS est désormais citoyen de première classe — cf. [`CLAUDE.md`](../CLAUDE.md) §2.
> - **Crates supprimés/extraits** depuis la rédaction : `crates/gui` (wry+tao),
>   `mui-rs*`, la plupart des `crates/aphrody-terminal-*` (seul
>   `aphrody-terminal-backend` subsiste). Les sections ci-dessous qui les citent
>   sont obsolètes.
>
> État courant : [`ARCHITECTURE.md`](ARCHITECTURE.md),
> [`cargo/CRATES.md`](cargo/CRATES.md), [`SOURCE_OF_TRUTH.md`](SOURCE_OF_TRUTH.md).



<!-- ============================================== -->
<!-- SOURCE: docs/DESIGN.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->
# Material Design 3 & Google OS Native Architecture

Ce fichier sert de source de vérité absolue (Single Source of Truth) pour l'écosystème `aphrody` et la conception de l'OS. Il définit l'architecture hybride du God Mode, s'appuyant sur l'accélération matérielle et une implémentation **Material Design 3 (M3)** Desktop-First.

## 1. L'Architecture à 3 Piliers (The God Mode GUI)

L'interface graphique du système d'exploitation Google OS repose sur une fondation triptyque inébranlable, entièrement propulsée par le GPU.

### Pilier I : Rust (Performance & MD3 Natif)
Le socle applicatif et le moteur de rendu `WebView` (`wry` / `tao`) sont écrits en Rust pur. 
- **Objectif** : Vitesse d'exécution maximale, sécurité mémoire, et interopérabilité directe avec l'OS via FFI.
- **Rendu** : Hébergement des conteneurs UI et gestion du cycle de vie de la fenêtre.

### Pilier II : C++ (Terminal Windows Custom)
L'interface en ligne de commande (CLI) de l'OS est une évolution directe d'un **fork C++ de Windows Terminal**.
- **Cible** : `Microsoft.WindowsTerminalCanary_8wekyb3d8bbwe!App`
- **Rendu Extrême** : Utilisation stricte de l'**AtlasEngine** (Direct2D / Direct3D 11) pour un rendu de texte fluide à très haute fréquence d'images.
- **Objectif** : Un shell Google OS surpuissant, customisé au maximum, offrant l'expérience terminale la plus réactive au monde.

### Pilier III : Bun & JSX (Logique UI, CSS & Design Tokens)
La construction de l'interface, le templating, et la gestion dynamique des thèmes sont orchestrés par **Bun** avec du **JSX natif**.
- **Objectif** : Itération ultra-rapide, gestion simplifiée du CSS, et intégration parfaite des Design Tokens.
- **Composants** : `@material/web` natif enveloppé dans des composants JSX sans framework lourd (pas de React/Vue).

## 2. Le Moteur Graphique : D3D12 & WebGPU

Aucun rendu CPU toléré. L'ensemble de la surface graphique de Google OS est accéléré matériellement.

- **DirectX 12 / D3D11** : Backing natif ("dur") des surfaces applicatives via l'OS hôte.
- **WebGPU Natif** : Accélération des WebView et applications M3 via le processus GPU de Chromium.
- **Chrome Canary SxS** : Le backend de rendu pour les interfaces web/JSX s'appuie sur `Chrome SxS\chrome.exe (Canary)`, exploitant le processus GPU Chromium avec un backend **D3D11/12** (WebGPU et WebGL activés).
- **Vulkan** : L'intégration du SDK Vulkan est prévue comme évolution optionnelle future pour le cross-platform absolu.

## 3. Philosophie & Desktop Best Practices (M3)

- **Layout Adaptatif (Three-Pane)** : Grilles fluides et architectures en 3 panneaux (Navigation Rail, Content Area, Side Sheet).
- **Densité "High Density"** : Interface compacte, esthétique "Tooling" (dense, informative, industrielle).
- **Surface Containers** : Utilisation de `lowest`, `low`, `normal`, `high`, `highest`. L'élévation physique `<md-elevation>` est réservée aux modales et tooltips.
- **Survol et Focus** : Clic souris et clavier priorisés. États `:hover`, `:focus-visible` natifs.

## 4. Typographie (Google Sans)

- **UI & Display** : `Google Sans Flex` (Police variable, axes `wght`, `opsz`).
- **Éditeurs** : `Google Sans Text`.
- **CLI & Terminal (Pilier II)** : `Google Sans Mono`. Typographie inaltérable pour le rendu AtlasEngine.

## 5. Design Tokens (Dynamic Color) & Iconographie

- **CSS Variables exclusives** : `var(--md-sys-color-primary)`. Espace HCT (ou `oklch` pour les accents purs). Pas de Tailwind.
- **Icons** : `Material Symbols Rounded` (FILL 0/1, wght 400, opsz 24).

L'intégration de ces trois piliers propulse Aphrody et Google OS au-delà d'un simple wrapper, forgeant un environnement natif, résilient, et visuellement absolu.

## 6. Références Architectures Cibles

- Voir Architecture OS 2026 : Meilleures Pratiques Unix pour les détails sur le noyau, l'I/O et le modèle de processus.


<!-- ============================================== -->
<!-- SOURCE: docs/DESIGN-GOOGLE.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- GENERATED by .claude/skills/design-google-ingest + scripts/design-google-curate.ts
     DO NOT EDIT BY HAND — re-run /design-google-ingest. -->

# Aphrody Design Reference

Last refreshed: `2026-05-17T21:07:19.626Z`
Source: design.google + `scripts/edge-mass-scrape.ts` (Edge headless, virtual-time=15000)
Articles ingested: **19 / 19**

## 1. Quick-reference index

| Section | Title | URL | Bytes |
|---|---|---|---|
| foundations | Page not found - Google Design | <https://design.google/library/accessibility> | 138,983 |
| foundations | Page not found - Google Design | <https://design.google/library/design-systems> | 138,985 |
| foundations | Rethinking Color Theory - Google Design | <https://design.google/library/color-theory-ruxandra-duru> | 318,598 |
| foundations | True Design is Better than New Design - Google Design | <https://design.google/library/david-reinfurt-teaches-design> | 194,102 |
| foundations | Unboxing a New Collaboration - Google Design | <https://design.google/library/hardware-and-software-can-work-together> | 316,294 |
| foundations | UX Design as Dance Theater - Google Design | <https://design.google/library/ux-design-system-dance> | 233,502 |
| m3 | Expressive Design: Google's UX Research - Google Design | <https://design.google/library/expressive-material-design-google-research> | 307,904 |
| m3 | Material 3 Expressive – Design Notes - Google Design | <https://design.google/library/design-notes-material-3-expressive-liam-spradlin> | 201,730 |
| m3 | Page not found - Google Design | <https://design.google/library/material-3-design-tokens> | 138,987 |
| m3 | Page not found - Google Design | <https://design.google/library/material-design-3> | 138,989 |
| gemini | Gemini AI Visual Design - Google Design | <https://design.google/library/gemini-ai-visual-design> | 275,398 |
| brand | Google Sans: Evolving Google’s Typeface - Google Design | <https://design.google/library/google-sans-flex-font> | 320,643 |
| brand | How to Design for Transparent Screens - Google Design | <https://design.google/library/transparent-screens> | 319,666 |
| brand | Open Source Brand Fonts - Google Design | <https://design.google/library/open-source-custom-fonts-google> | 264,678 |
| site | About - Google Design | <https://design.google/about> | 160,584 |
| site | Google Design - Discover the people and stories behind the products | <https://design.google/> | 153,953 |
| site | Page not found - Google Design | <https://design.google/events> | 138,987 |
| site | Page not found - Google Design | <https://design.google/products> | 139,373 |
| site | UX Inspiration & Insights - Google Design | <https://design.google/library> | 178,696 |

## 2. Foundations

### Page not found - Google Design

- **Source:** <https://design.google/library/accessibility>
- **Bytes captured:** 138,983
- **Description:** Google Design is an editorial platform about design at Google. We open a window for designers and the design-curious to meet the people and processes behind the products.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

### Page not found - Google Design

- **Source:** <https://design.google/library/design-systems>
- **Bytes captured:** 138,985
- **Description:** Google Design is an editorial platform about design at Google. We open a window for designers and the design-curious to meet the people and processes behind the products.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

### Rethinking Color Theory - Google Design

- **Source:** <https://design.google/library/color-theory-ruxandra-duru>
- **Bytes captured:** 318,598
- **Description:** How to balance the emotional and rational to create powerful palettes
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#FFFFFF`, `#00363D`, `#000000`, `#DAE2FF`, `#EEF0FF`, `#97F0FF`, `#EDFCFF`, `#12110C`, `#32302A`, `#FAE366`, `#FFF1B3`

### True Design is Better than New Design - Google Design

- **Source:** <https://design.google/library/david-reinfurt-teaches-design>
- **Bytes captured:** 194,102
- **Description:** David Reinfurt discusses his latest book, A *Co-* Program for Graphic Design.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#D3EF46`, `#E1FE54`

### Unboxing a New Collaboration - Google Design

- **Source:** <https://design.google/library/hardware-and-software-can-work-together>
- **Bytes captured:** 316,294
- **Description:** If hardware and software can work together, so can their teams
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#B2C0FE`, `#0C2878`, `#000000`, `#FFD7F5`, `#FFEBF7`, `#9AF0FF`, `#EFDBFF`, `#FAF8FF`, `#12110C`, `#32302A`, `#FFFFFF`
- **Pull quote:** > “Just imagine a room with people who are looking at colors and saying, ‘I want to eat that.’ Then I have to figure out how to translate it into code.” —Ruxandra Duru, visual designer, Material Design

### UX Design as Dance Theater - Google Design

- **Source:** <https://design.google/library/ux-design-system-dance>
- **Bytes captured:** 233,502
- **Description:** Discover how principles of dance theater can be applied to UX design to create engaging and user-friendly digital experiences.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

## 3. Material 3

### Expressive Design: Google's UX Research - Google Design

- **Source:** <https://design.google/library/expressive-material-design-google-research>
- **Bytes captured:** 307,904
- **Description:** Google's research reveals how expressive design improves UX, usability, and evokes positive emotions in users
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110B`, `#FFF398`, `#FFFFFF`, `#406652`, `#284E3C`, `#C2ECD3`, `#FFDAD4`, `#B9F295`, `#FFF9E7`, `#12110C`, `#32302A`

### Material 3 Expressive – Design Notes - Google Design

- **Source:** <https://design.google/library/design-notes-material-3-expressive-liam-spradlin>
- **Bytes captured:** 201,730
- **Description:** Explore Material 3 Expressive with members of Google's Material Design team. Learn about emotion-driven UX, user research, and flexibility.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFF2A1`, `#000000`

### Page not found - Google Design

- **Source:** <https://design.google/library/material-3-design-tokens>
- **Bytes captured:** 138,987
- **Description:** Google Design is an editorial platform about design at Google. We open a window for designers and the design-curious to meet the people and processes behind the products.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

### Page not found - Google Design

- **Source:** <https://design.google/library/material-design-3>
- **Bytes captured:** 138,989
- **Description:** Google Design is an editorial platform about design at Google. We open a window for designers and the design-curious to meet the people and processes behind the products.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

## 4. Gemini visual identity

### Gemini AI Visual Design - Google Design

- **Source:** <https://design.google/library/gemini-ai-visual-design>
- **Bytes captured:** 275,398
- **Description:** Explore how Google designers use gradients, motion, and foundational shapes to build trust and intuition within the evolving Gemini AI assistant experience.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#000000`, `#FFECF0`, `#FFFFFF`, `#7C5635`, `#623F20`, `#FFDCC1`, `#FFD9E2`, `#FFF8F8`, `#12110C`, `#32302A`, `#FAE366`

## 5. Brand assets

### Google Sans: Evolving Google’s Typeface - Google Design

- **Source:** <https://design.google/library/google-sans-flex-font>
- **Bytes captured:** 320,643
- **Description:** Discover the inside story of Google Sans, Google’s iconic brand typeface. From product lockups to code, learn how the Google Sans font family has evolved to solve specific UX design problems.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

### How to Design for Transparent Screens - Google Design

- **Source:** <https://design.google/library/transparent-screens>
- **Bytes captured:** 319,666
- **Description:** Behind-the-scenes of designing the next generation of interfaces for AI glasses with displays—including Jetpack Compose Glimmer, the newly launched design system for Android extended reality (XR) experiences.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

### Open Source Brand Fonts - Google Design

- **Source:** <https://design.google/library/open-source-custom-fonts-google>
- **Bytes captured:** 264,678
- **Description:** Learn why top brands are open-sourcing custom fonts, boosting type accessibility, innovation, and brand recognition
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#081A3C`, `#EDF0FF`, `#FFFFFF`, `#725572`, `#593D5A`, `#FDD7FA`, `#6FF7F3`, `#E8DDFF`, `#FAF8FF`, `#12110C`, `#32302A`

## 6. Other library articles

_No entries in this bucket._

## 7. Site pages

### About - Google Design

- **Source:** <https://design.google/about>
- **Bytes captured:** 160,584
- **Description:** Explore Google Design. Discover design thinking, UX expertise, and how design shapes Google’s products.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

### Google Design - Discover the people and stories behind the products

- **Source:** <https://design.google/>
- **Bytes captured:** 153,953
- **Description:** Design resources and inspiration from Google — including the Material Design system, Google Fonts, and the people and processes behind the products.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#C8F0AE`, `#FFEE99`

### Page not found - Google Design

- **Source:** <https://design.google/events>
- **Bytes captured:** 138,987
- **Description:** Google Design is an editorial platform about design at Google. We open a window for designers and the design-curious to meet the people and processes behind the products.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

### Page not found - Google Design

- **Source:** <https://design.google/products>
- **Bytes captured:** 139,373
- **Description:** Google Design is an editorial platform about design at Google. We open a window for designers and the design-curious to meet the people and processes behind the products.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

### UX Inspiration & Insights - Google Design

- **Source:** <https://design.google/library>
- **Bytes captured:** 178,696
- **Description:** Get inspired by Google Design's story library. Discover articles, case studies, and resources to elevate your design expertise.
- **Excerpt:** design.google uses cookies from Google to deliver and enhance the quality of its services and to analyse traffic. Learn more
- **Color stops:** `#EFEFEF`, `#12110C`, `#32302A`, `#FFFFFF`, `#FAE366`, `#FFF1B3`, `#C3ECD0`, `#FFDAD3`, `#BFF28D`, `#FFF9EB`, `#FFEE99`

## 8. Raw failures (SPA shell or scrape error)

_None — every URL returned a fully-hydrated DOM._

## 9. Audit cross-reference

_coverage table marker not found_

Aphrody crate cross-references in lock-step with design.google intel:

- `crates/m3-tokens/src/gemini_brand.rs` ← `gemini-ai-visual-design`
- `crates/m3-tokens/src/google_sans_flex.rs` ← `google-sans-flex-font`
- `crates/m3-tokens/src/color.rs` ← `material-3-design-tokens`
- `crates/m3-tokens/src/{shape,state,tonal,motion,elevation,typography}.rs` ← M3 spec
- `crates/shadcn-bridge/src/gemini.rs` ← `gemini-ai-visual-design` composables
- `crates/aphrody-wasm/examples/gemini-clone-pixel-perfect.html` ← full Gemini clone
- `crates/aphrody-wasm/examples/m3-shadcn-pixel-perfect-v2.html` ← 30-component M3 demo

## 10. Open follow-ups

- Re-run with `--virtual-time=30000` after Edge ships a longer hydration budget so all SPA shell hits in §8 resolve.
- Port full CAM16 HCT pipeline so the dynamic palette in `crates/m3-tokens/src/dynamic.rs` matches Material Color Utilities round-trip within <1 sRGB unit (currently 5 round-trip tests `#[ignore]`).
- Re-scrape weekly via `/loop 7d /design-google-ingest` once a CI runner with Edge is provisioned.

---

_Skill: `.claude/skills/design-google-ingest/SKILL.md` · Agent: `.claude/agents/design-google-curator.md` · Generator: `scripts/design-google-curate.ts`_


<!-- ============================================== -->
<!-- SOURCE: docs/md3/components.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->
# Material Web Components (`@material/web`)

L'implémentation de MD3 dans notre projet repose sur la bibliothèque officielle `@material/web` de Google. 
Ces composants sont des Custom Elements HTML natifs. Ils fonctionnent avec Vanilla JS, Lit, React, Angular, Vue, ou tout autre framework web.

## Installation et Importation

Dans votre projet (`packages/ui`), assurez-vous d'importer les composants dont vous avez besoin. L'importation déclare automatiquement le Custom Element dans le registre du navigateur.

```javascript
// Exemple d'importation dans un fichier d'entrée (e.g. index.js ou main.ts)
import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';
import '@material/web/checkbox/checkbox.js';
import '@material/web/textfield/filled-text-field.js';
```

## Utilisation en HTML

Une fois importés, les composants s'utilisent comme des balises HTML standards, préfixées par `md-`.

### Boutons

```html
<md-filled-button>Action Principale</md-filled-button>
<md-outlined-button>Action Secondaire</md-outlined-button>
<md-text-button>Annuler</md-text-button>
<md-elevated-button>Sauvegarder</md-elevated-button>
```

### Champs de texte

```html
<md-filled-text-field label="Nom d'utilisateur" type="text"></md-filled-text-field>
<md-outlined-text-field label="Mot de passe" type="password"></md-outlined-text-field>
```

### Composants de sélection

```html
<label>
  <md-checkbox checked></md-checkbox>
  Activer l'option
</label>

<md-radio name="theme" value="dark"></md-radio>
<md-radio name="theme" value="light" checked></md-radio>
```

## Gestion des Événements et Propriétés

Puisqu'ils sont natifs, l'interaction se fait via l'API DOM standard :

```javascript
const button = document.querySelector('md-filled-button');

// Écouter un événement
button.addEventListener('click', () => {
  console.log('Bouton cliqué !');
});

// Modifier une propriété
button.disabled = true;

const textField = document.querySelector('md-filled-text-field');
textField.addEventListener('input', (e) => {
  console.log('Valeur:', e.target.value);
});
```

## Intégration A2UI (Agent-to-User Interface)

Ces composants sont parfaits pour le système `A2UI` cloné dans `packages/a2ui`. Les agents peuvent générer des schémas JSON qui se traduisent directement par ces balises `<md-*>` natives, garantissant un rendu sécurisé et universel sans exécution de code JavaScript dangereux.


<!-- ============================================== -->
<!-- SOURCE: docs/md3/index.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->
# Material Design 3 (MD3) - Overview

Bienvenue dans la documentation officielle de l'implémentation de Material Design 3 (MD3) pour le projet **Aphrody / Google OS**.
L'interface graphique est une expression native du **Pilier III (Bun/JSX)** de notre architecture God Mode. Elle génère le DOM utilisé par le **Pilier I (Rust Webview)**.

## L'Architecture FULL Bun JSX

Le package `packages/ui` a été entièrement refactorisé en pur Bun JSX :
1. **Zéro Dépendance React/Vue** : Un compilateur natif JSX vers HTML (`html.ts`) génère instantanément le balisage statique.
2. **Couverture Globale (Glossaire M3)** : 100% des concepts du glossaire officiel (Buttons, Navigation Rail, Dialogs, Cards) sont implémentés via les Custom Elements `@material/web`.
3. **God Mode Integration** : Le script de build Bun exporte directement le HTML dans le crate Rust `gui`, ce qui permet une compilation finale unifiée et une accélération matérielle (WebGPU/DX12) lors de l'exécution.

## Qu'est-ce que Material Design 3 ?

Material Design 3 (Material You) apporte :
1. **Personnalisation dynamique** : Espace colorimétrique HCT.
2. **Accessibilité renforcée** : Contraste défini algorithmiquement.
3. **Design Tokens** : `--md-sys-color-primary` et dérivés.

## Navigation

*   [**Global DESIGN.md**](../DESIGN.md) - Règles d'architecture globales, God Mode, et Desktop Best Practices.
*   Composants Natifs (Components) - Utilisation des wrappers JSX pour `@material/web`.
*   Le Système de Thème (Theming) - Design Tokens, HCT, et couleurs statiques.
*   Typographie & Icônes (Typography & Icons) - Google Sans et Material Symbols.


<!-- ============================================== -->
<!-- SOURCE: docs/md3/theming.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->
# Theming & Design Tokens (Material You)

L'un des plus grands apports de Material Design 3 est son système de Design Tokens basé sur le **Dynamic Color** et l'espace colorimétrique **HCT (Hue, Chroma, Tone)**.

## Design Tokens (CSS Custom Properties)

Les composants `@material/web` n'utilisent pas de classes CSS utilitaires complexes ou de préprocesseurs pour le thème global. Ils reposent entièrement sur les **CSS Custom Properties** (variables CSS).

Pour thémer l'application, vous modifiez les variables globales (préfixées par `--md-sys-color-`) dans votre feuille de style.

```css
:root {
  /* Couleurs primaires */
  --md-sys-color-primary: #006494;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-primary-container: #cae6ff;
  --md-sys-color-on-primary-container: #001e30;

  /* Arrière-plans et surfaces */
  --md-sys-color-background: #fdfcff;
  --md-sys-color-on-background: #1a1c1e;
  --md-sys-color-surface: #fdfcff;
  --md-sys-color-on-surface: #1a1c1e;

  /* Erreurs */
  --md-sys-color-error: #ba1a1a;
  --md-sys-color-on-error: #ffffff;
}

@media (prefers-color-scheme: dark) {
  :root {
    --md-sys-color-primary: #8dcdff;
    --md-sys-color-on-primary: #003450;
    --md-sys-color-primary-container: #004b71;
    --md-sys-color-on-primary-container: #cae6ff;
    
    --md-sys-color-background: #1a1c1e;
    --md-sys-color-on-background: #e2e2e5;
  }
}
```

## Personnalisation locale

Vous pouvez styliser un composant spécifique en surchargeant ses tokens locaux (préfixés selon le composant).

```css
md-filled-button.danger-btn {
  --md-filled-button-container-color: var(--md-sys-color-error);
  --md-filled-button-label-text-color: var(--md-sys-color-on-error);
  --md-filled-button-hover-container-color: #ff0000;
}
```

## Élévation (Elevation) et Surface Containers

Dans les versions récentes de MD3, le système d'élévation basé sur la teinte (Elevation 1 à 5) a évolué vers le concept explicite de **Surface Containers**. Au lieu d'assombrir/éclaircir dynamiquement la surface avec la couleur primaire selon son niveau d'élévation, on utilise des variables dédiées pour les conteneurs :

*   `--md-sys-color-surface-container-lowest`
*   `--md-sys-color-surface-container-low`
*   `--md-sys-color-surface-container`
*   `--md-sys-color-surface-container-high`
*   `--md-sys-color-surface-container-highest`

Pour gérer **l'ombre portée visuelle**, la bibliothèque `@material/web` fournit désormais un composant dédié `<md-elevation>` que vous pouvez inclure dans la structure DOM de vos éléments personnalisés, ou utiliser conjointement avec ces variables de surface.


<!-- ============================================== -->
<!-- SOURCE: docs/md3/typography-icons.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->
# Typographie et Iconographie (Material Symbols)

Le package `ui` intègre `material-symbols`, le standard absolu de l'iconographie Google pour Material Design 3.

## Material Symbols vs Material Icons

Les anciennes *Material Icons* étaient statiques. Les nouveaux **Material Symbols** utilisent la technologie des **Variable Fonts** (Polices Variables). Une seule police de caractères contient des millions de variations d'une icône modifiables via des propriétés CSS.

## Utilisation

Importez la police dans votre CSS ou HTML, puis utilisez la classe appropriée :

```html
<!-- HTML -->
<span class="material-symbols-outlined">search</span>
<span class="material-symbols-rounded">settings</span>
<span class="material-symbols-sharp">home</span>
```

### Axes de Police Variable

La puissance des Material Symbols réside dans les axes CSS `font-variation-settings`. Vous pouvez altérer l'icône à la volée de manière fluide.

1. **FILL (Remplissage)** `0` ou `1`
   Permet de passer d'une icône vide (outline) à une icône pleine. Souvent utilisé pour l'état actif/inactif d'un bouton.
2. **wght (Poids/Graisse)** `100` à `700`
   L'épaisseur des traits de l'icône.
3. **GRAD (Grade)** `-25` à `200`
   Ajuste visuellement l'épaisseur en fonction des fonds (sombre ou clair) sans modifier la taille physique de l'icône.
4. **opsz (Taille optique)** `20` à `48`
   Ajuste les détails de l'icône pour qu'elle reste lisible selon la taille d'affichage (20px, 24px, etc.).

### Exemple CSS

```css
.material-symbols-outlined {
  font-family: 'Material Symbols Outlined';
  font-variation-settings:
  'FILL' 0,
  'wght' 400,
  'GRAD' 0,
  'opsz' 24;
}

/* Au survol, l'icône se remplit et s'épaissit subtilement */
button:hover .material-symbols-outlined {
  font-variation-settings:
  'FILL' 1,
  'wght' 500,
  'GRAD' 20,
  'opsz' 24;
  transition: font-variation-settings 0.2s ease-in-out;
}
```

## Typographie (Google Sans & Roboto)

Dans le cadre du projet `aphrody` et de l'environnement WinClean, nous appliquons une charte typographique stricte basée sur l'écosystème **Google Sans** :

1. **Interface par défaut** : **Google Sans Flex**. Cette police variable permet un ajustement parfait de la graisse (wght), de la chasse (wdth) et de la taille optique (opsz) pour toutes les interfaces fluides MD3.
2. **Code et Éditeurs** : **Google Sans Text** / **Google Sans**. Utilisé pour l'affichage de code structuré dans les UI, garantissant une lisibilité maximale.
3. **Terminal** : **Google Sans Mono**. Exclusivement réservée aux CLI et aux affichages bruts.

### Intégration Terminal (Windows Terminal Canary)

Notre émulateur cible est **Windows Terminal Canary**, customisé spécifiquement avec les palettes de couleurs Material Design 3. L'intégration garantit que :
- La police par défaut est configurée sur `Google Sans Mono`.
- Les couleurs (ANSI) sont mappées sur les tokens `sys-color` de Material Design 3 (ex: le rouge ANSI pointe vers `--md-sys-color-error`).

Exemples de Tokens Typographiques MD3 :
*   `--md-sys-typescale-display-large-font-family` (Configuré sur `'Google Sans Flex', sans-serif`)
*   `--md-sys-typescale-body-medium-size`
*   `--md-sys-typescale-label-small-weight`


<!-- ============================================== -->
<!-- SOURCE: docs/terminal/ARCHITECTURE.md -->
<!-- ============================================== -->

# `vendor/terminal` — Architecture détaillée

Document écrit le 2026-05-16, valide pour le commit
`8fe6c21ef88a73a7985b5968ee18936928ccac69` du dépôt `microsoft/terminal`.

Sommaire :

1. [Vue d'ensemble](#1-vue-densemble)
2. [Cartographie du dépôt (racine)](#2-cartographie-du-dépôt-racine)
3. [Cartographie de `src/`](#3-cartographie-de-src)
4. [Architecture en couches](#4-architecture-en-couches)
5. [Composants réutilisables clés](#5-composants-réutilisables-clés)
6. [Build et toolchain](#6-build-et-toolchain)
7. [Tests](#7-tests)
8. [Politiques de code](#8-politiques-de-code)
9. [Specs et roadmap](#9-specs-et-roadmap)
10. [Licence et conformité](#10-licence-et-conformité)
11. [Risques d'intégration](#11-risques-dintégration)
12. [Conclusion : ce qu'on garde pour `google_os`](#12-conclusion--ce-quon-garde-pour-google_os)

---

## 1. Vue d'ensemble

`microsoft/terminal` est un monorepo qui produit conjointement
**plusieurs binaires** distincts, tous gouvernés par `OpenConsole.slnx`
à la racine.

### 1.1 Binaires produits

Recensés à partir de `vendor/terminal/OpenConsole.slnx` et de
l'inspection des `.vcxproj` de chaque cible.

| Binaire                                  | Type | Origine `.vcxproj`                                                          | Rôle                                                                                            |
|------------------------------------------|------|-----------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------|
| `OpenConsole.exe`                        | exe  | `vendor/terminal/src/host/exe/Host.EXE.vcxproj` (`TargetName=OpenConsole`)  | Build dev de `conhost.exe`, le serveur de console NT historique de Windows.                     |
| `WindowsTerminal.exe`                    | exe  | `vendor/terminal/src/cascadia/WindowsTerminal/WindowsTerminal.vcxproj`      | Hôte Win32 de l'application Terminal moderne. Ouvre une fenêtre XAML islands + DirectX surface. |
| `wt.exe` / `wtd.exe`                     | exe  | `vendor/terminal/src/cascadia/wt/wt.vcxproj`                                | Shim de 36 lignes qui redirige vers `WindowsTerminal.exe`. `wtd` = branding Dev.                |
| `conpty.dll`                             | dll  | `vendor/terminal/src/winconpty/dll/winconptydll.vcxproj`                    | Implémentation open-source du ConPTY (alias des exports de `kernel32`).                         |
| `conptylib.lib`                          | lib  | `vendor/terminal/src/winconpty/lib/winconptylib.vcxproj`                    | Version statique des mêmes symboles (sans `dllimport`, via `conpty-static.h`).                  |
| `OpenConsoleProxy.dll`                   | dll  | `vendor/terminal/src/host/proxy/Host.Proxy.vcxproj`                         | Proxy/stub MIDL pour les interfaces COM `IConsoleHandoff` et `ITerminalHandoff`.                |
| `console.dll`                            | dll  | `vendor/terminal/src/propsheet/propsheet.vcxproj`                           | Property sheet « clic droit > Propriétés » d'une fenêtre conhost.                               |
| `Microsoft.Terminal.Control.dll`         | dll  | `vendor/terminal/src/cascadia/TerminalControl/dll/TerminalControl.vcxproj`  | Contrôle WinUI 2 réutilisable (le « TermControl »), basé sur la TextBuffer + le renderer Atlas. |
| `Microsoft.Terminal.Settings.Model.dll`  | dll  | `vendor/terminal/src/cascadia/TerminalSettingsModel/dll/Microsoft.Terminal.Settings.Model.vcxproj` | Modèle de configuration JSON5 (héritage de profils, layouts, schemes).        |
| `Microsoft.Terminal.Settings.Editor.dll` | dll  | `vendor/terminal/src/cascadia/TerminalSettingsEditor/Microsoft.Terminal.Settings.Editor.vcxproj` | UI WinUI 2 d'édition des settings.                                              |
| `TerminalApp.dll`                        | dll  | `vendor/terminal/src/cascadia/TerminalApp/dll/TerminalApp.vcxproj`          | Application (tabs, panes, palette de commandes, JIT activation des profils).                    |
| `WindowsTerminalShellExt.dll`            | dll  | `vendor/terminal/src/cascadia/ShellExtension/WindowsTerminalShellExt.vcxproj` | Extension shell « Ouvrir dans Terminal » d'Explorer.                                          |
| `elevate-shim.exe`                       | exe  | `vendor/terminal/src/cascadia/ElevateShim/elevate-shim.vcxproj`             | Shim pour relancer Terminal en élévation UAC.                                                   |
| `UIHelpers.dll`, `UIMarkdown.dll`, `WinRTUtils.dll` | dll | `vendor/terminal/src/cascadia/{UIHelpers,UIMarkdown,WinRTUtils}/*.vcxproj` | Utilitaires WinRT internes.                                                                |
| `Microsoft.Terminal.Wpf.dll`             | dll  | `vendor/terminal/src/cascadia/WpfTerminalControl/WpfTerminalControl.csproj` | Wrapper WPF de `HwndTerminal`.                                                                  |
| `CascadiaPackage_*.msix`                 | msix | `vendor/terminal/src/cascadia/CascadiaPackage/CascadiaPackage.wapproj`      | Paquet MSIX qui agrège `WindowsTerminal.exe` + DLL + `OpenConsole.exe`.                         |
| `colortool.exe`                          | exe  | `vendor/terminal/src/tools/ColorTool/ColorTool.sln`                         | Petit utilitaire .NET pour appliquer des schemes XTerm à la palette conhost.                    |

À cela s'ajoutent une vingtaine d'utilitaires internes
(`vendor/terminal/src/tools/{benchcat,buffersize,ConsoleBench,nihilist,closetest,fontlist,RenderingTests,scratch,vtapp,vtpipeterm,U8U16Test,TerminalStress,…}/*.vcxproj`)
et tous les binaires de tests (`*.Unit.Tests.dll`, `*.Feature.Tests.dll`,
`*.UIA.Tests.dll`). La liste complète des projets est dans
`vendor/terminal/OpenConsole.slnx` (1060 lignes, ~70 projets).

### 1.2 Différence Terminal vs Console Host vs ConPTY

Trois entités à ne pas confondre :

- **Console Host (`conhost.exe`)** : le serveur de console historique
  de Windows. Il implémente le protocole `\Device\ConDrv\Server` côté
  user-mode (ALPC), porte la window proc historique avec rendu GDI et
  expose l'API Win32 Console (`ReadConsoleA`, `WriteConsoleA`,
  `GetConsoleScreenBufferInfo`, etc.). Source officielle du `conhost.exe`
  livré par l'OS = ce repo (`src/host/`). En dev on en produit
  `OpenConsole.exe` (cf. `Host.EXE.vcxproj`, `TargetName=OpenConsole`)
  pour éviter de remplacer celui de `System32`.

- **Windows Terminal (`WindowsTerminal.exe`)** : l'application
  utilisateur moderne. Elle gère les onglets, les panneaux, les profils,
  le rendu DirectX, etc. Elle **n'implémente pas** l'API Console : pour
  ça, elle ouvre un ConPTY et laisse `conhost.exe` (lancé en mode
  `--headless`) parler à l'application cliente. Du point de vue de
  `cmd.exe`, `powershell.exe`, `bash.exe`, le serveur reste donc
  `conhost`, mais le rendu visuel est dans Terminal.

- **ConPTY (`conpty.dll` + `OpenConsole.exe --headless`)** : la
  pseudo-console. C'est l'équivalent Windows de `forkpty()`. Elle
  expose en entrée/sortie deux pipes encodés UTF-8 + VT, et derrière
  spawn `conhost.exe --headless` qui traduit ces flux VT en API
  Console pour les vieux clients qui appellent `WriteConsoleA`. Cf.
  `vendor/terminal/src/winconpty/winconpty.cpp:_CreatePseudoConsole`.

Ces trois entités sont distinctes mais bâties sur les **mêmes libs
statiques** (`bufferout`, `parser`, `adapter`, `server`, etc.).

### 1.3 Versions Windows ciblées

Lecture de `vendor/terminal/src/common.build.pre.props` lignes 77-80 :

```xml
<WindowsTargetPlatformVersion Condition="'$(WindowsTargetPlatformVersion)' == ''">10.0.22621.0</WindowsTargetPlatformVersion>
<WindowsTargetPlatformMinVersion Condition="'$(WindowsTargetPlatformMinVersion)' == ''">10.0.18362.0</WindowsTargetPlatformMinVersion>
```

- **SDK de compilation** : 10.0.22621.0 (Windows 11 21H2). Notre wrapper
  surcharge avec 10.0.26100.0 (Windows 11 24H2).
- **OS min runtime** : 10.0.18362.0 (Windows 10 1903), mais le README
  upstream précise « Windows 10 2004 (build >= 19041) ou plus tard »
  pour Terminal lui-même.

---

## 2. Cartographie du dépôt (racine)

`vendor/terminal/` au niveau supérieur :

| Entrée                            | Rôle                                                                                                    |
|-----------------------------------|---------------------------------------------------------------------------------------------------------|
| `LICENSE`                         | MIT (copyright Microsoft Corporation).                                                                  |
| `NOTICE.md`                       | Notices tierces (jsoncpp, chromium/numerics, {fmt}, interval_tree, pcg, wyhash, stb, Oklab, ColorBrewer, cmark, fzf, GSL, MUX, VirtualDesktopUtils, WIL). |
| `README.md`                       | Présentation, install via Store/winget/Chocolatey/Scoop, build, FAQ.                                    |
| `OpenConsole.slnx`                | Solution unique du repo (format `.slnx` 1060 lignes, 4 `BuildType`, 4 `Platform`, 70 projets dans 15 dossiers). |
| `Scratch.sln`                     | Petite sln pour bidouiller du code expérimental sans charger tout `OpenConsole`.                        |
| `XamlStyler.json`                 | Config de formatting XAML (utilisé par `Invoke-XamlFormat`).                                            |
| `NuGet.Config`                    | Source NuGet **unique** : `https://pkgs.dev.azure.com/shine-oss/terminal/_packaging/TerminalDependencies%40Local/nuget/v3/index.json`. Mirror Azure DevOps Microsoft. |
| `vcpkg.json`                      | Manifest vcpkg : `fmt 12.1.0`, `ms-gsl 3.1.0` + feature `terminal` (jsoncpp 1.9.6, cli11 2.6.1, cmark 0.31.1), baseline `15e5f3820f0370f1ba…`. |
| `Directory.Build.props`           | Optionnellement active MSBuildCache (local/Pipeline).                                                   |
| `Directory.Build.targets`         | Idem côté targets.                                                                                      |
| `common.openconsole.props`        | Définit `$(OpenConsoleDir)` pour les `.wapproj` qui ne reçoivent pas correctement `$(SolutionDir)`.     |
| `custom.props`                    | Read by XES (release pipeline) : `XesBaseYearForStoreVersion=2026`, `VersionMajor=1`, `VersionMinor=26`. |
| `dirs`                            | Ancien marqueur Razzle (`DIRS=src`).                                                                    |
| `consolegit2gitfilters.json`      | Filtres utilisés par les outils internes Microsoft pour synchroniser conhost vers le repo OS.            |

Le wrapper de build aphrody est **hors sous-module**, à
`scripts/terminal/build.ps1`. Les patches locaux appliqués sous
`vendor/terminal/dep/vcpkg-overlay-triplets/*.cmake` et
`vendor/terminal/src/common.build.pre.props` sont archivés dans
`docs/terminal/PATCHES.diff` (cf. `BUILD.md`).

| Dossier                           | Rôle                                                                                                    |
|-----------------------------------|---------------------------------------------------------------------------------------------------------|
| `bin/`                            | Sorties MSBuild (`bin/<Platform>/<Configuration>/`).                                                    |
| `obj/`                            | Intermédiaires MSBuild + vcpkg installé (`obj/<Platform>/vcpkg/`).                                      |
| `packages/`                       | NuGet packages restored.                                                                                |
| `src/`                            | Code source des composants (voir § 3).                                                                  |
| `dep/`                            | Dépendances embarquées : `Console/` (headers internes), `NT/` (structs NT non publiques), `Win32K/` (headers privés window manager), `WinAppDriver/` (UI tests), `nuget/` (nuget.exe + `packages.config`), `telemetry/`, `vcpkg-overlay-ports/`, `vcpkg-overlay-triplets/`. |
| `tools/`                          | Scripts PowerShell (`OpenConsole.psm1`) et cmd (`razzle.cmd`, `bcz.cmd`, `runut.cmd`, `runft.cmd`, `runuia.cmd`, `bcx.cmd`, `bx.cmd`), `tests.xml`, `WindbgExtension.js`, génération de header (`Generate-CodepointWidthsFromUCD.ps1`, `Generate-FeatureStagingHeader.ps1`, `GenerateHeaderForJson.ps1`, `GenerateSettingsIndex.ps1`), profil WPR (`ConsolePerf.wprp`, `Terminal.wprp`), `StaticAnalysis.ruleset`. |
| `doc/`                            | Documentation : `STYLE.md`, `ORGANIZATION.md`, `EXCEPTIONS.md`, `WIL.md`, `Niksa.md`, `virtual-dtors.md`, `TAEF.md`, `feature_flags.md`, `building.md`, `Debugging.md`, `submitting_code.md`, `UniversalTest.md`, `WindowsTestPasses.md`, `bot.md`, `fuzzing.md`, `terminal-{a11y-2023,v1-roadmap,v2-roadmap}.md`, `roadmap-2022.md`, `roadmap-2023.md`, `color_nudging.html`, `creating_a_new_project.md`, `AddASetting.md`, `COOKED_READ_DATA.md`, `ConsoleCtrlEvent.md`, `ConsoleHostSettings.md`. Plus `specs/` (60+ specs détaillées), `cascadia/`, `reference/`, `user-docs/`, `images/`. |
| `samples/`                        | Exemples d'utilisation : `ConPTY/EchoCon` (créer un ConPTY et y lancer `ping localhost`), `ConPTY/MiniTerm` (petit terminal C++), `ConPTY/GUIConsole` (variante WPF/.NET), `PixelShaders` (HLSL custom pour AtlasEngine), `ReadConsoleInputStream`. |
| `oss/`                            | Bibliothèques tierces vendorisées en source : `chromium/` (numerics), `interval_tree/`, `pcg/`, `stb/`, `wyhash/`, `xorg_apps_rgb/` + `README.md`. |
| `build/`                          | Pipeline Azure DevOps : `pipelines/{ci,release}.yml`, `pipelines/templates/`, `scripts/{Create-AppxBundle,Index-Pdbs,Invoke-FormattingCheck,Run-Tests,Test-WindowsTerminalPackage}.ps1`, `rules/Branding.targets`, `rules/CollectWildcardResources.targets`, `config/`, `Fuzz/`, `Helix/`, `StoreSubmission/`, `packages.config`, `pgo/`. |
| `res/`                            | Ressources de branding : `LICENSE`, `README.md`, `console.ico`, `fonts/`, `terminal/`, `terminal.ico`, `truetype.bmp`. |
| `policies/`                       | Templates de stratégies de groupe : `WindowsTerminal.admx`, `en-US/`. |
| `scratch/`                        | Brouillons jetables.                                                                                    |

Le dossier `.config/` (mentionné dans le README pour les
`configuration.winget`) n'est pas présent dans notre checkout (probablement
masqué par `.gitignore` ou non poussé).

---

## 3. Cartographie de `src/`

Listing brut (`vendor/terminal/src/`) avec rôle synthétique. Sources :
inspection des `.vcxproj`, `doc/ORGANIZATION.md`, et lecture des
`README.md` quand ils existent.

### 3.1 Communs

| Sous-dossier             | Rôle                                                                                                              |
|--------------------------|-------------------------------------------------------------------------------------------------------------------|
| `src/inc/`               | Headers publics partagés : `DefaultSettings.h`, `HostAndPropsheetIncludes.h`, `HostSignals.hpp`, `LibraryIncludes.h`, `TestUtils.h`, `WilErrorReporting.h`, `conattrs.hpp`, `conint.h`, `conpty-static.h`, `consoletaeftemplates.hpp`, `cpl_core.h`, `unicode.hpp`, `winrtTaefTemplates.hpp`. Sous-dossier `til/` (37 headers : `at.h`, `atomic.h`, `bit.h`, `bytes.h`, `coalesce.h`, `color.h`, `colorbrewer.h`, `enumset.h`, `env.h`, `flat_set.h`, `generational.h`, `hash.h`, `io.h`, `latch.h`, `math.h`, `mutex.h`, `operators.h`, `pmr.h`, `point.h`, `rand.h`, `rect.h`, `regex.h`, `replace.h`, `rle.h`, `size.h`, `small_vector.h`, `spsc.h`, `static_map.h`, `string.h`, `throttled_func.h`, `ticket_lock.h`, `type_traits.h`, `u8u16convert.h`, `unicode.h`, `winrt.h`) ; `CppCoreCheck/warnings.h` ; `test/CommonState.hpp`.                                                                                                                                                                                                                                                                                                                                                                                  |
| `src/til/`               | Library cible pour `til/` : `precomp.{cpp,h}`, `dirs`, sous-dossier `ut_til/` (TAEF tests). Cible MSBuild = `til.unit.tests.dll`. |
| `src/internal/`          | `Internal.vcxproj` (`TargetName=ConInt`) : stubs pour les symboles internes Microsoft non redistribuables (`stubs.cpp`).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `src/staging/`           | Sous-dossier vide à part `makefile.inc` + `sources` (artefacts Razzle), non lié à la solution.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `src/features.xml`       | Source du système de feature flags : chaque feature génère un `Feature_XXX::IsEnabled()` + `TIL_FEATURE_XXX_ENABLED`. Doc : `vendor/terminal/doc/feature_flags.md`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src/testlist`           | Fichier de configuration utilisé par `TestTableWriter` pour générer la liste des suites TAEF.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `src/common.build.{pre,post,tests}.props`, `src/common.nugetversions.{props,targets}`, `src/cppwinrt.build.{pre,post}.props`, `src/wap-common.build.{pre,post}.props` | Couche commune MSBuild. `pre.props` impose `PlatformToolset=v143`, `LanguageStandard=stdcpp20`, options conformes (`/Zc:__cplusplus /Zc:__STDC__ /Zc:enumTypes /Zc:inline /Zc:templateScope /Zc:throwingNew`), warnings = errors (`TreatWarningAsError`), `EXTERNAL_BUILD`, `HybridCRT`, et configure vcpkg + 4 configs (Debug/Release/AuditMode/Fuzzing) × 3 plates-formes. |
| `src/unit.tests.{x64,x86}.runsettings` | Settings pour vstest.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `src/dirs`               | Marqueur Razzle.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `src/project.inc`, `src/project.unittest.inc` | Defaults pour les sources Razzle.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |

### 3.2 Sous-dossiers fonctionnels

| Dossier                      | `.vcxproj`(s) / cible                                                                                  | Dépendances majeures                                                  | Rôle                                                                                                                                          |
|------------------------------|--------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|
| `src/host/`                  | `lib/hostlib.vcxproj` (`ConhostV2Lib.lib`), `exe/Host.EXE.vcxproj` (`OpenConsole.exe`), `proxy/Host.Proxy.vcxproj` (`OpenConsoleProxy.dll`), `ft_host/`, `ft_uia/`, `ft_integrity/`, `ft_fuzzer/`, `ut_host/`, `ut_lib/`. | Quasi tous les autres modules.                                        | Le cœur de `conhost.exe`. ~80 fichiers `.cpp` (`_output.cpp`, `_stream.cpp`, `cmdline.cpp`, `consoleInformation.cpp`, `srvinit.cpp`, `handle.cpp`, `directio.cpp`, `getset.cpp`, `globals.cpp`, `screenInfo.cpp`, `cursor.cpp`, `selection.cpp`, `inputBuffer.cpp`, `clipboard.cpp` ailleurs, `registry.cpp`, `settings.cpp`, `outputStream.cpp`, `readDataCooked.cpp`, `readDataDirect.cpp`, `readDataRaw.cpp`, `VtIo.cpp`, `VtInputThread.cpp`, `PtySignalInputThread.cpp`, etc.). |
| `src/server/`                | `lib/server.vcxproj` (`ConServer.lib`).                                                                | `host/proxy` (pour les IDL `IConsoleHandoff`/`ITerminalHandoff`).     | Couche IPC user-mode parlant à `\Device\ConDrv\Server` via ALPC. Fichiers : `ApiDispatchers.cpp`, `ApiMessage.cpp`, `ApiSorter.cpp`, `ConDrvDeviceComm.cpp`, `ConsoleShimPolicy.cpp`, `DeviceHandle.cpp`, `Entrypoints.cpp`, `IoDispatchers.cpp`, `IoSorter.cpp`, `ObjectHandle.cpp`, `ObjectHeader.cpp`, `ProcessHandle.cpp`, `ProcessList.cpp`, `ProcessPolicy.cpp`, `WaitBlock.cpp`, `WaitQueue.cpp`, `WinNTControl.cpp` (chargement dynamique de `ntdll.dll`). |
| `src/winconpty/`             | `lib/winconptylib.vcxproj` (`conptylib.lib`), `dll/winconptydll.vcxproj` (`conpty.dll` + `winconpty.def`), `ft_pty/winconpty.FeatureTests.vcxproj`, `package/winconpty.nuspec`. | `server/DeviceHandle.cpp`, `server/WinNTControl.cpp`.                 | ConPTY userspace : `_CreatePseudoConsole` (création serveur ConDrv + pipe signal + spawn `conhost --headless`), `_ResizePseudoConsole`, `_ShowHidePseudoConsole`, `_ReparentPseudoConsole`, `_ClosePseudoConsoleMembers`. Exporte `ConptyCreatePseudoConsole` + alias `CreatePseudoConsole` (cf. `winconpty.def`).                                                                                                                                                                                                                                                                                                                                                            |
| `src/buffer/out/`            | `lib/bufferout.vcxproj` (`ConBufferOut.lib`), `ut_textbuffer/TextBuffer.Unit.Tests.vcxproj`.            | `types`.                                                              | Le **text buffer** : `Row.cpp` (ligne logique avec attributs SGR), `textBuffer.cpp` (buffer circulaire 2D), `textBufferCellIterator.cpp` + `textBufferTextIterator.cpp` (itérateurs zero-copy), `OutputCell.cpp` (cellule unitaire), `OutputCellIterator.cpp`, `OutputCellRect.cpp`, `OutputCellView.cpp`, `cursor.cpp`, `search.cpp`, `TextAttribute.cpp`, `TextColor.cpp` (16+RGB SGR), `ImageSlice.cpp` (rendu d'images Sixel/iTerm), `UTextAdapter.cpp` (passe le buffer à ICU). Headers principaux : `textBuffer.hpp`, `Row.hpp`, `LineRendition.hpp` (DECDHL/DECDWL), `Marks.hpp` (marks pour shell integration GH#11000), `DbcsAttribute.hpp`.                                                                                       |
| `src/terminal/parser/`       | `lib/parser.vcxproj` (`ConTermParser.lib`), `ft_fuzzer/VTCommandFuzzer.vcxproj`, `ft_fuzzwrapper/FuzzWrapper.vcxproj`, `ut_parser/Parser.UnitTests.vcxproj`. | aucune (header-only sur til, std).                                    | State machine ECMA-48 / VT100/220/320/420 + DEC + XTerm. Fichiers : `stateMachine.cpp` (entièrement la machine d'états), `OutputStateMachineEngine.cpp`, `InputStateMachineEngine.cpp`, `tracing.cpp`, `base64.cpp` (pour OSC 52 clipboard). Classe-clé : `Microsoft::Console::VirtualTerminal::StateMachine` (cf. `stateMachine.hpp`, support de `MAX_PARAMETER_VALUE=65535`, `MAX_PARAMETER_COUNT=32`, `MAX_SUBPARAMETER_COUNT=6`, gère C1, ANSI/VT52, OSC, DCS, SS3, mode `AcceptC1`).                                                                                                                                                                                                  |
| `src/terminal/adapter/`      | `lib/adapter.vcxproj` (`ConTermAdapt.lib`), `ut_adapter/Adapter.UnitTests.vcxproj`.                      | `types`, `terminal/input`.                                            | Adaptateur des verbes VT vers les calls API console. `adaptDispatch.cpp` (implémente `ITermDispatch`), `adaptDispatchGraphics.cpp` (SGR), `terminalOutput.cpp` (charsets G0..G3, designations DEC), `FontBuffer.cpp` (DECDLD soft-fonts), `MacroBuffer.cpp` (DECDMAC), `PageManager.cpp` (DECNCMR pages), `SixelParser.cpp` (DECSIXEL pour images). Interfaces : `ITerminalApi.hpp`, `ITermDispatch.hpp`, `IInteractDispatch.hpp`, `InteractDispatch.cpp`. |
| `src/terminal/input/`        | `lib/terminalinput.vcxproj` (`TerminalInput.lib`).                                                      | `types`.                                                              | Encodage clavier vers VT (xterm `modifyOtherKeys`, win32-input-mode = `CSI 9001`, SS3 pour fonctions), encodage souris SGR `\e[<…`. Fichiers : `terminalInput.cpp`, `mouseInput.cpp`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `src/types/`                 | `lib/types.vcxproj` (`ConTypes.lib`), `ut_types/Types.Unit.Tests.vcxproj`.                              | aucune (que til + std).                                               | Types utilitaires partagés : `Viewport.cpp` (rect typé en cells), `convert.cpp` (UTF-8↔UTF-16), `CodepointWidthDetector.cpp` (largeur Unicode + grapheme clusters via `unicode_width_overrides.xml` régénéré par `Generate-CodepointWidthsFromUCD.ps1`), `GlyphWidth.cpp`, `ColorFix.cpp`, `colorTable.cpp` (palettes 16-color + 256 XTerm + custom), `sgrStack.cpp` (XTPUSHSGR/XTPOPSGR), `ThemeUtils.cpp`, UIA helpers (`ScreenInfoUiaProviderBase.cpp`, `TermControlUiaProvider.cpp`, `TermControlUiaTextRange.cpp`, `UiaTextRangeBase.cpp`, `UiaTracing.cpp`).                                                                                                                                                  |
| `src/renderer/`              | `base/lib/base.vcxproj` (`ConRenderBase.lib`), `atlas/atlas.vcxproj` (`ConRenderAtlas.lib`), `gdi/lib/gdi.vcxproj` (`ConRenderGdi.lib`), `uia/lib/uia.vcxproj` (`ConRenderUia.lib`), `wddmcon/lib/wddmcon.vcxproj` (`wddmcon.lib`), `inc/` (`IRenderEngine.hpp`, `IRenderData.hpp`, `RenderSettings.hpp`, `Cluster.hpp`, `CSSLengthPercentage.h`, `FontInfo.hpp` etc.). | `types`, `buffer`.                                                | Pipeline de rendu. `base` est l'abstraction (transforme `IRenderData` en primitives `DrawString`/`DrawCursor`), `atlas` est le moteur DirectWrite/D2D/D3D11 avec cache de glyphes GPU (cf. `vendor/terminal/src/renderer/atlas/README.md`, schémas Mermaid), `gdi` le rendu GDI classique de conhost, `uia` le « rendu » virtuel pour UIA, `wddmcon` un rendu DXGK pour environnement de boot. AtlasEngine inclut des shaders HLSL (`shader_ps.hlsl`, `shader_vs.hlsl`, `custom_shader_{ps,vs}.hlsl`). |
| `src/interactivity/base/`    | `lib/InteractivityBase.vcxproj` (`ConInteractivityBaseLib.lib`).                                        | aucune directe (que les interfaces dans `inc/`).                      | Service locator + interfaces (`IConsoleControl`, `IConsoleInputThread`, `IConsoleWindow`, `IHighDpiApi`, `IInteractivityFactory`, `ISystemConfigurationProvider`, `IWindowMetrics`). Fichiers : `ApiDetector.cpp`, `EventSynthesis.cpp`, `HostSignalInputThread.cpp`, `InteractivityFactory.cpp`, `PseudoConsoleWindowAccessibilityProvider.cpp`, `RemoteConsoleControl.cpp`, `ServiceLocator.cpp`, `VtApiRedirection.cpp`.                                                                                                                                                                                                                                                                  |
| `src/interactivity/win32/`   | `lib/win32.LIB.vcxproj` (`ConInteractivityWin32Lib.lib`), `ut_interactivity_win32/Interactivity.Win32.UnitTests.vcxproj`. | `renderer/atlas`.                                       | Implémentation Win32 des interfaces ci-dessus. Fichiers : `Clipboard.cpp`, `ConsoleControl.cpp`, `ConsoleInputThread.cpp`, `ConsoleKeyInfo.cpp`, `Find.cpp` (popup de recherche), `Icon.cpp`, `Menu.cpp`, `screenInfoUiaProvider.cpp`, `SystemConfigurationProvider.cpp`, `uiaTextRange.cpp`, `Window.cpp`, `WindowDpiApi.cpp`, `WindowIo.cpp`, `WindowMetrics.cpp`, `WindowProc.cpp`, `windowUiaProvider.cpp`.                                                                                                                                                                                                                                                                                  |
| `src/interactivity/onecore/` | `lib/onecore.LIB.vcxproj` (`onecore.lib`).                                                              | `interactivity/base`.                                                 | Variante OneCore (Windows sans user32, IoT/HoloLens, etc.).                                                                                  |
| `src/propsheet/`             | `propsheet.vcxproj` (`console.dll`).                                                                    | `propslib`, `internal`.                                               | Property sheet « clic droit > Propriétés ». Fichiers : `console.cpp`, `globals.cpp`, `dbcs.cpp`, `dll.cpp`, `fontdlg.cpp`, `init.cpp`, `misc.cpp`, `preview.cpp`, `PropSheetHandler.cpp`, `OptionsPage.cpp`, `LayoutPage.cpp`, `ColorsPage.cpp`, `ColorControl.cpp`, `TerminalPropsheetPage.cpp`, `registry.cpp`, `util.cpp`.                                                                                                                                                                                                                                                                                                                                                                |
| `src/propslib/`              | `propslib.vcxproj` (`ConProps.lib`).                                                                    | aucune.                                                              | Sérialisation des prefs console dans HKCU + `.lnk` : `DelegationConfig.cpp`, `RegistrySerialization.cpp`, `ShortcutSerialization.cpp`, `TrueTypeFontList.cpp`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `src/tsf/`                   | `tsf.vcxproj` (`ConTSF.lib`).                                                                           | aucune (que win32 TSF).                                              | Bridge IME via Text Services Framework. Fichiers : `Handle.cpp`, `Implementation.cpp`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src/audio/midi/`            | `lib/midi.vcxproj` (`MidiAudio.lib`).                                                                   | `winmm.lib`.                                                          | DECPSO (commande de musique VT). `MidiAudio.cpp`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `src/cascadia/`              | (voir § 3.3)                                                                                            | (voir § 3.3)                                                          | Tout Terminal moderne (WinUI 2 + WinRT).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src/tools/`                 | `benchcat`, `buffersize`, `closetest`, `ConsoleBench`, `ConsoleMonitor`, `echokey`, `fontlist`, `nihilist`, `RenderingTests`, `scratch`, `TerminalStress`, `U8U16Test`, `vtapp`, `vtpipeterm`, plus `ColorTool/` (.NET) et `GraphemeTableGen`, `GraphemeTestTableGen`, `ansi-color`, `lnkd`, `pixels`, `schemes-fragment`, `test`, `texttests`, `vttests`, `integrity`. | divers, surtout en console. | Utilitaires internes : bancs perf, générateurs de table Unicode, traceurs VT, tests visuels.                                                                                                                                                                                                                                                                                                                                                                                                                                          |

### 3.3 `src/cascadia/` (Terminal moderne)

Cf. `vendor/terminal/doc/ORGANIZATION.md` § *cascadia*. Sous-dossiers :

| Sous-dossier                              | `.vcxproj` / cible                                                              | Rôle                                                                                                                                       |
|-------------------------------------------|---------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------|
| `TerminalConnection/`                     | `TerminalConnection.vcxproj` (`Microsoft.Terminal.TerminalConnection.dll`)      | Abstractions de connexion : `ITerminalConnection.idl`, `ConptyConnection` (Win32 ConPTY), `AzureConnection` (Azure Cloud Shell), `EchoConnection` (debug), `BaseTerminalConnection.h`. Handoff inbound : `CTerminalHandoff.cpp`. |
| `TerminalCore/`                           | `lib/TerminalCore-lib.vcxproj` (`TerminalCore.lib`), `pch.*`, `terminalcore-common.vcxitems` | Classe `Microsoft::Terminal::Core::Terminal` qui compose buffer + parser + adapter + input sans rendu ni UI. Implémente `ITerminalApi`, `ITerminalInput`, `IRenderData` (cf. `Terminal.hpp`). `ICoreSettings.idl` (WinRT settings). |
| `TerminalControl/`                        | `dll/TerminalControl.vcxproj` (`Microsoft.Terminal.Control.dll`), `TerminalControlLib.vcxproj` | UI WinUI 2 du TermControl : `TermControl.xaml/.cpp/.h/.idl`, `ControlCore.cpp` (composition non-UI réutilisable), `ControlInteractivity.cpp`, `HwndTerminal.cpp` (variante Win32 pure, sans XAML), `SearchBoxControl.xaml`, `TermControlAutomationPeer.cpp`. |
| `TerminalApp/`                            | `TerminalAppLib.vcxproj`, `dll/TerminalApp.vcxproj` (`TerminalApp.dll`)         | L'application Terminal en WinUI 2 : `App.xaml`, `TerminalPage.xaml/.cpp`, `Tab.cpp`, `Pane.cpp`, `CommandPalette.xaml`, `SuggestionsControl.xaml`, `MinMaxCloseControl.xaml`, `MarkdownPaneContent.xaml`, `AboutDialog.xaml`, `AppLogic.cpp`, `AppCommandlineArgs.cpp` (parser CLI `wt`), `Jumplist.cpp`, `Toast.cpp`, `Remoting.cpp` (multi-instance via WinRT). |
| `TerminalSettingsModel/`                  | `Microsoft.Terminal.Settings.ModelLib.vcxproj`, `dll/Microsoft.Terminal.Settings.Model.vcxproj` | Modèle de settings JSON5 (héritage de profils), `profiles.schema.json`. |
| `TerminalSettingsEditor/`                 | `Microsoft.Terminal.Settings.Editor.vcxproj`                                    | Settings UI WinUI 2. |
| `TerminalSettingsAppAdapterLib/`          | `TerminalSettingsAppAdapterLib.vcxproj`                                         | Adaptateur entre l'App et le Settings Model. |
| `WindowsTerminal/`                        | `WindowsTerminal.vcxproj` (`WindowsTerminal.exe`)                               | Hôte Win32 + XAML islands : `AppHost.cpp`, `BaseWindow.h`, `IslandWindow.cpp` (XAML island host), `NonClientIslandWindow.cpp` (titlebar custom), `VirtualDesktopUtils.cpp` (extrait de PowerToys), `WindowEmperor.cpp` (multi-fenêtre), `icon.cpp`, `main.cpp`. |
| `WindowsTerminal_UIATests/`               | `WindowsTerminal.UIA.Tests.csproj`                                              | Tests UIA via Appium WebDriver. |
| `CascadiaPackage/`                        | `CascadiaPackage.wapproj`                                                       | MSIX packaging. `Package*.appxmanifest` (Dev/Preview/Canary/Release branding). |
| `ShellExtension/`                         | `WindowsTerminalShellExt.vcxproj` (`WindowsTerminalShellExt.dll`)               | Extension Explorer « Ouvrir dans Terminal ». |
| `ElevateShim/`                            | `elevate-shim.vcxproj` (`elevate-shim.exe`)                                     | Élévation UAC. |
| `Remoting/`                               | (pas de vcxproj, inclus dans TerminalApp)                                       | Resources WinRT du modèle d'inter-fenêtre. |
| `UIHelpers/`                              | `UIHelpers.vcxproj`                                                             | Utilitaires UI WinRT. |
| `UIMarkdown/`                             | `UIMarkdown.vcxproj`                                                            | Rendu Markdown WinUI (utilise cmark). |
| `WinRTUtils/`                             | `WinRTUtils.vcxproj`                                                            | Utilitaires WinRT divers. |
| `WpfTerminalControl/`                     | `WpfTerminalControl.csproj` (`Microsoft.Terminal.Wpf.dll`)                      | Wrapper WPF de `HwndTerminal`. |
| `WpfTerminalTestNetCore/`                 | `WpfTerminalTestNetCore.csproj`                                                 | Banc test .NET Core WPF. |
| `wt/`                                     | `wt.vcxproj` (`wt.exe`, `wtd.exe`)                                              | Shim de 36 lignes (`shim.cpp`) qui réécrit `argv[0]` et lance `WindowsTerminal.exe` via `CreateProcessW`. Astuce pour que `wt.exe` AppX-aliased apparaisse dans le PATH. |
| `fzf/`                                    | (inclus dans `TerminalApp`)                                                     | Fuzzy finder pour la palette de commandes (`fzf.cpp`, `fzf.h`, MIT). |
| `inc/`                                    | (header-only)                                                                   | `ControlProperties.h`, `cppwinrt_utils.h`. |
| `LocalTests_TerminalApp/`                 | `TerminalApp.LocalTests.vcxproj`, `TestHostApp/TestHostApp.vcxproj`             | TAEF locaux pour TerminalApp. |
| `UnitTests_Control/`                      | `Control.UnitTests.vcxproj`                                                     | Tests unit du TermControl. |
| `UnitTests_SettingsModel/`                | `SettingsModel.UnitTests.vcxproj`                                               | Tests unit du Settings Model. |
| `UnitTests_TerminalCore/`                 | `UnitTests.vcxproj`                                                             | Tests unit du TerminalCore. |
| `ut_app/`                                 | `TerminalApp.UnitTests.vcxproj`                                                 | Tests unit additionnels TerminalApp (`FzfTests.cpp`, `JsonUtilsTests.cpp`). |

Dépendances NuGet majeures du `cascadia/` (cf.
`vendor/terminal/dep/nuget/packages.config`) :

- `Microsoft.Windows.CppWinRT 2.0.250303.1` — code-gen C++/WinRT ;
- `Microsoft.UI.Xaml 2.8.4` — **WinUI 2** (pas 3), c'est-à-dire WinUI
  XAML Islands. Terminal n'est pas porté sur WinUI 3 ;
- `Microsoft.Web.WebView2 1.0.1661.34` ;
- `Microsoft.Windows.ImplementationLibrary 1.0.250325.1` — WIL ;
- `Microsoft.Internal.Windows.Terminal.ThemeHelpers 0.8.250811004` ;
- `Microsoft.Internal.PGO-Helpers.Cpp 0.2.34` (interne Microsoft).

---

## 4. Architecture en couches

```
                ┌──────────────────────────────┐
                │ Apps CLI / wWinMain / WinUI  │
                │  (cmd, bash, pwsh, vim, …    │
                │   ou WindowsTerminal.exe)    │
                └──────────────┬───────────────┘
                               │
                  appel API Win32 Console
                  (WriteConsoleA/W, ReadConsoleA/W,
                   GetConsoleScreenBufferInfo, …)
                               │
                               ▼
                ┌──────────────────────────────┐
                │ kernelbase.dll  (Console API)│
                └──────────────┬───────────────┘
                               │
                               │ NtDeviceIoControlFile
                               │ vers \Device\ConDrv
                               ▼
                ┌──────────────────────────────┐
                │ Driver console : condrv.sys  │
                │ (kernel-mode, hors ce repo)  │
                └──────────────┬───────────────┘
                               │
                               │ ALPC msg ring
                               ▼
   ┌───────────────────────────────────────────────────────┐
   │      conhost.exe  (= OpenConsole.exe en dev)         │
   │  - server/  : décode les API msg                     │
   │  - host/    : ApiDispatchers → ApiRoutines           │
   │  - buffer/  : TextBuffer, Row, OutputCell, …         │
   │  - terminal/parser/  : StateMachine VT (entrée+sortie)│
   │  - terminal/adapter/ : verbes VT → API console        │
   │  - terminal/input/   : clavier/souris → VT            │
   │  - renderer/         : rendu (GDI / Atlas / UIA / …)  │
   │  - interactivity/    : window proc, clipboard, IME    │
   └───────────┬──────────────────────────────┬────────────┘
               │                              │
               │ rendu GDI                    │ pipes UTF-8 + VT
               ▼                              │ (mode --headless)
   ┌─────────────────────┐                    │
   │ fenêtre conhost     │                    │
   │ (USER32 + GDI)      │                    │
   └─────────────────────┘                    │
                                              │
                                              ▼
                            ┌───────────────────────────────────┐
                            │  WindowsTerminal.exe              │
                            │  - cascadia/TerminalConnection :  │
                            │      ConptyConnection ↔ pipes     │
                            │  - cascadia/TerminalCore :        │
                            │      Terminal { Buffer + Parser   │
                            │      + Adapter + Input }          │
                            │  - cascadia/TerminalControl :     │
                            │      TermControl (WinUI 2 XAML)   │
                            │      + AtlasEngine (D3D11/D2D)    │
                            │  - cascadia/TerminalApp :         │
                            │      Tabs, Panes, CommandPalette  │
                            │  - cascadia/WindowsTerminal :     │
                            │      hôte Win32 + XAML islands    │
                            └───────────────────────────────────┘
```

### 4.1 Flux IPC

Trois canaux d'IPC coexistent :

1. **ALPC `\Device\ConDrv\Server`**. Pipe historique entre une
   application console et `conhost.exe`. Le serveur côté user-mode est
   `vendor/terminal/src/server/lib/` (`ApiMessage.cpp`, `ApiSorter.cpp`,
   `IoSorter.cpp`, `IoDispatchers.cpp`). Le « server handle » de
   `\Device\ConDrv` est créé par le driver `condrv.sys` (hors repo) ;
   `winconpty.cpp:CreateServerHandle` est un wrapper qui appelle
   `NtCreateFile` sur ce path.

2. **`\Device\ConDrv\Reference`**. Handle enfant du précédent, hérité
   par le processus client via `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`.
   Quand son refcount tombe à 0, conhost se ferme (voir le bloc de
   commentaire en haut de `vendor/terminal/src/winconpty/winconpty.h:_PseudoConsole`).

3. **Pipe anonyme « signal »** entre Terminal et conhost (créé par
   `_CreatePseudoConsole` via `CreatePipe`). Sert à envoyer des
   `PTY_SIGNAL_RESIZE_WINDOW`, `PTY_SIGNAL_CLEAR_WINDOW`,
   `PTY_SIGNAL_SHOWHIDE_WINDOW`, `PTY_SIGNAL_REPARENT_WINDOW` (cf.
   `vendor/terminal/src/winconpty/winconpty.h` lignes 44-49).

À noter pour `Cascadia` (Terminal moderne) :

- `TerminalConnection/ConptyConnection.cpp` instancie un ConPTY puis
  lit/écrit dans les deux pipes UTF-8 (`hInput`, `hOutput`).
- `TerminalCore/Terminal.cpp` traite ces flux via la `StateMachine`
  (depuis `terminal/parser`) qui appelle l'`AdaptDispatch` (depuis
  `terminal/adapter`) qui modifie le `TextBuffer` (depuis `buffer/out`).

### 4.2 Pipeline de rendu (AtlasEngine)

Cf. `vendor/terminal/src/renderer/atlas/README.md`. Schéma simplifié :

```
TermControl (WinUI 2 XAML)
        │ (DispatcherTimer ou VSync)
        ▼
Renderer (base/renderer.cpp)
        │ casse le buffer en "DrawString" / "FillBackground" / "DrawCursor"
        ▼
AtlasEngine (atlas/AtlasEngine.cpp)
        │ regroupe en DWRITE_GLYPH_RUNs
        │ split api.cpp (sous console lock) / r.cpp (hors lock)
        ▼
  ┌─────────────────────┬────────────────────┐
  ▼                     ▼                    │
BackendD2D           BackendD3D              │
(pur Direct2D,       (Direct3D 11 +          │
 fallback RDP /       glyph atlas GPU +      │
 vieux GPU)           HLSL shaders)          │
  │                     │                    │
  └────────┬────────────┘                    │
           ▼                                 │
     IDXGISwapChain                          │
           │                                 │
           ▼                                 │
     compositeur Windows  ◀──────────────────┘
                          (custom shaders)
```

### 4.3 Threads

`conhost.exe` (résumé d'après `src/host/`):

- **API thread** : reçoit les API messages depuis ConDrv via
  `IoSorter::ServiceIoOperation` ; appelle un `ApiDispatcher` qui
  modifie le buffer.
- **Input thread** : `ConsoleInputThread.cpp` lit les messages
  `WM_KEYDOWN`/`WM_INPUT` depuis la window proc puis pousse dans
  `inputBuffer`.
- **VT Input thread** : `VtInputThread.cpp` (`pVtInputThread`). Lit
  l'`hInput` pipe et délègue à la `StateMachine` (engine input).
- **Pty Signal thread** : `PtySignalInputThread.cpp` lit le signal pipe.
- **Render thread** : alimente le renderer engine actif (GDI ou Atlas).

`WindowsTerminal.exe` :

- **UI thread** (XAML) : WinUI 2 / dispatcher.
- **Render thread** : AtlasEngine.
- **Connection thread** : par TermControl, lit le pipe ConPTY UTF-8.

---

## 5. Composants réutilisables clés

Pour chaque composant : chemin du `.vcxproj`, headers exposés, sortie,
couplage Win32, exemple d'utilisation **issu du code réel** ou esquisse
FFI Rust.

### 5.1 `til` — Terminal Implementation Library

- **Chemin** : `vendor/terminal/src/til/` (lib unit-tests),
  `vendor/terminal/src/inc/til/*.h` (headers publics) + `src/inc/til.h`.
- **Sortie** : header-only (sauf `precomp.cpp` pour les unit tests qui
  produit `til.unit.tests.dll`).
- **Couplage Win32** : partiel. Plusieurs headers utilisent WIL et
  `wchar_t`, mais `at.h`, `small_vector.h`, `flat_set.h`,
  `rle.h`, `hash.h`, `bit.h`, `math.h`, `bytes.h`, `point.h`,
  `rect.h`, `size.h`, `static_map.h`, `enumset.h`, `generational.h`,
  `type_traits.h`, `coalesce.h`, `latch.h`, `mutex.h`, `pmr.h`,
  `ticket_lock.h`, `spsc.h`, `rand.h`, `replace.h`, `regex.h`,
  `u8u16convert.h`, `unicode.h`, `string.h`, `color.h`, `operators.h`
  sont en pratique du C++20 portable.
- **Exemple** : `til::small_vector<Injection, 8>` est utilisé par la
  `StateMachine` pour stocker les injections VT (cf.
  `vendor/terminal/src/terminal/parser/stateMachine.hpp:87`).
- **Idée FFI Rust** : pas nécessaire — équivalents directs en Rust
  (`smallvec`, `hashbrown`, etc.).

### 5.2 `vtparser` (`terminal/parser`)

- **Chemin** : `vendor/terminal/src/terminal/parser/lib/parser.vcxproj`.
- **Sortie** : `ConTermParser.lib` (statique).
- **Headers exposés** : `vendor/terminal/src/terminal/parser/stateMachine.hpp`
  (classe `Microsoft::Console::VirtualTerminal::StateMachine`),
  `IStateMachineEngine.hpp` (interface),
  `OutputStateMachineEngine.hpp`, `InputStateMachineEngine.hpp`,
  `base64.hpp`, `ascii.hpp`, `tracing.hpp`.
- **Couplage Win32** : non (`std::wstring_view`, til). Le seul lien à
  Win32 est via WIL pour les macros d'erreur.
- **API publique condensée** (extrait de `stateMachine.hpp`) :

  ```cpp
  namespace Microsoft::Console::VirtualTerminal {
      class StateMachine final {
      public:
          template<typename T>
          StateMachine(std::unique_ptr<T> engine) noexcept;
          void ProcessCharacter(const wchar_t wch);
          void ProcessString(const std::wstring_view string);
          void SetParserMode(const Mode mode, const bool enabled) noexcept;
          void InjectSequence(InjectionType type);
          const til::small_vector<Injection, 8>& GetInjections() const noexcept;
          void ResetState() noexcept;
          bool FlushToTerminal();
      };
  }
  ```

- **Idée FFI Rust** : wrapper `extern "C"` minimal autour de
  `StateMachine::ProcessString` avec callback C++ qui pousse les
  actions dispatched dans un canal vers Rust. Voir `INTEGRATION.md` §
  2.2.

### 5.3 `bufferout` (text buffer)

- **Chemin** : `vendor/terminal/src/buffer/out/lib/bufferout.vcxproj`.
- **Sortie** : `ConBufferOut.lib` (statique).
- **Headers exposés** : `vendor/terminal/src/buffer/out/textBuffer.hpp`
  (classe `TextBuffer`), `Row.hpp`, `cursor.h`, `OutputCell.hpp`,
  `OutputCellIterator.hpp`, `OutputCellRect.hpp`, `OutputCellView.hpp`,
  `TextAttribute.hpp` (SGR), `TextColor.h` (16/256/RGB),
  `LineRendition.hpp` (DECDHL/DECDWL), `ImageSlice.hpp` (Sixel),
  `Marks.hpp` (shell-integration), `DbcsAttribute.hpp` (CJK), `search.h`,
  `UTextAdapter.h` (intégration ICU), `textBufferCellIterator.hpp`,
  `textBufferTextIterator.hpp`.
- **Couplage Win32** : partiel (utilise WIL).
- **Exemple** : la classe `TextBuffer` (utilisée par toutes les
  consoles) est instanciée par `host/screenInfo.cpp` et par
  `cascadia/TerminalCore/Terminal.cpp`.

### 5.4 `winconpty` (ConPTY)

- **Chemin lib statique** : `vendor/terminal/src/winconpty/lib/winconptylib.vcxproj`
  → `conptylib.lib`.
- **Chemin DLL** : `vendor/terminal/src/winconpty/dll/winconptydll.vcxproj`
  → `conpty.dll` + `winconpty.def` (exports `ConptyCreatePseudoConsole`,
  `ConptyCreatePseudoConsoleAsUser`, `ConptyResizePseudoConsole`,
  `ConptyClosePseudoConsole`, `ConptyClearPseudoConsole`,
  `ConptyShowHidePseudoConsole`, `ConptyReparentPseudoConsole`,
  `ConptyReleasePseudoConsole`, `ConptyPackPseudoConsole` + alias compat
  `CreatePseudoConsole`, `ResizePseudoConsole`, `ClosePseudoConsole`,
  `ClearPseudoConsole`, `ReleasePseudoConsole`).
- **Header public** :
  `vendor/terminal/src/inc/conpty-static.h` (déclare les `Conpty*` sans
  `dllimport` pour usage en static-link), et
  `vendor/terminal/src/winconpty/winconpty.h` (struct interne
  `PseudoConsole { HANDLE hSignal; HANDLE hPtyReference; HANDLE hConPtyProcess; }`).
- **Couplage Win32** : total (`NtCreateFile`, `CreateProcessAsUserW`,
  `\Device\ConDrv\Server`, `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`).
- **Exemple d'utilisation officiel** :
  `vendor/terminal/samples/ConPTY/EchoCon/EchoCon/EchoCon.cpp` (extrait,
  vérifié) :

  ```cpp
  // Crée un ConPTY et y attache "ping localhost"
  HRESULT CreatePseudoConsoleAndPipes(HPCON* phPC, HANDLE* phPipeIn, HANDLE* phPipeOut)
  {
      HANDLE hPipePTYIn{ INVALID_HANDLE_VALUE };
      HANDLE hPipePTYOut{ INVALID_HANDLE_VALUE };
      if (CreatePipe(&hPipePTYIn, phPipeOut, NULL, 0) &&
          CreatePipe(phPipeIn, &hPipePTYOut, NULL, 0))
      {
          COORD consoleSize{};
          CONSOLE_SCREEN_BUFFER_INFO csbi{};
          GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &csbi);
          consoleSize.X = csbi.srWindow.Right - csbi.srWindow.Left + 1;
          consoleSize.Y = csbi.srWindow.Bottom - csbi.srWindow.Top + 1;
          return CreatePseudoConsole(consoleSize, hPipePTYIn, hPipePTYOut, 0, phPC);
      }
      return E_FAIL;
  }
  ```

- **Esquisse Rust** (avec `windows-rs`) :

  ```rust
  use windows::Win32::System::Console::{
      CreatePseudoConsole, ClosePseudoConsole, HPCON, COORD,
  };
  use windows::Win32::System::Pipes::CreatePipe;
  ```

### 5.5 `terminal/adapter`

- **Chemin** : `vendor/terminal/src/terminal/adapter/lib/adapter.vcxproj`.
- **Sortie** : `ConTermAdapt.lib` (statique).
- **Headers exposés** : `adaptDispatch.hpp` (`AdaptDispatch` :
  implémente `ITermDispatch`), `ITermDispatch.hpp`, `ITerminalApi.hpp`,
  `IInteractDispatch.hpp`, `InteractDispatch.hpp`, `DispatchTypes.hpp`,
  `FontBuffer.hpp`, `MacroBuffer.hpp`, `PageManager.hpp`,
  `SixelParser.hpp`, `terminalOutput.hpp`, `charsets.hpp`,
  `termDispatch.hpp`.
- **Couplage Win32** : partiel (via WIL).
- **Dépend** de : `types`, `terminal/input` (cf. `adapter.vcxproj`).

### 5.6 `terminal/input`

- **Chemin** : `vendor/terminal/src/terminal/input/lib/terminalinput.vcxproj`.
- **Sortie** : `TerminalInput.lib` (statique).
- **Headers** : `terminalInput.hpp` (classe `TerminalInput` qui produit
  des séquences VT à partir d'événements clavier/souris).
- **Couplage Win32** : partiel (uses `VK_*` constants).
- **Dépend** de : `types`.

### 5.7 `types`

- **Chemin** : `vendor/terminal/src/types/lib/types.vcxproj`.
- **Sortie** : `ConTypes.lib` (statique).
- **Headers** : `vendor/terminal/src/types/inc/`:
  `CodepointWidthDetector.hpp` (calcul largeur Unicode + graphemes),
  `Viewport.hpp` (rectangle en cellules), `ColorFix.hpp`,
  `colorTable.hpp`, `convert.hpp` (UTF-8 ↔ UTF-16),
  `GlyphWidth.hpp`, `IInputEvent.hpp`, `sgrStack.hpp`, `ThemeUtils.h`,
  `utils.hpp`.
- **Couplage Win32** : partiel.

### 5.8 `renderer` (résumé pour mémoire)

- **`renderer/base`** (`ConRenderBase.lib`) : pur C++, interface
  `IRenderEngine` + `Renderer` qui orchestre. Dépend de `types` et
  `buffer`.
- **`renderer/atlas`** (`ConRenderAtlas.lib`) : Direct3D 11 + Direct2D
  + DirectWrite + shaders HLSL. **Non portable.**
- **`renderer/gdi`** (`ConRenderGdi.lib`) : Win32 GDI. **Non portable.**
- **`renderer/uia`** (`ConRenderUia.lib`) : UI Automation. **Non
  portable.**

### 5.9 `interactivity`

- **`interactivity/base`** : `ServiceLocator` + interfaces. Utile pour
  comprendre l'architecture de plugin du conhost.
- **`interactivity/win32`** et **`interactivity/onecore`** :
  implémentations Win32 / OneCore.

### 5.10 `server` (ConDrv user-mode)

- **Chemin** : `vendor/terminal/src/server/lib/server.vcxproj` →
  `ConServer.lib`.
- **Headers** : `ApiMessage.h`, `ApiDispatchers.h`, `IApiRoutines.h`,
  `IoDispatchers.h`, `ConsoleShimPolicy.h`, `DeviceComm.h`,
  `DeviceHandle.h`, `ObjectHandle.h`, `ObjectHeader.h`, `ProcessHandle.h`,
  `ProcessList.h`, `ProcessPolicy.h`, `WaitBlock.h`, `WaitQueue.h`,
  `WaitTerminationReason.h`, `WinNTControl.h`.
- **Couplage Win32** : total (chargement dynamique de `ntdll.dll` pour
  les fonctions `Nt*` non publiques).
- **Intérêt pour nous** : **référence d'implémentation** du serveur
  console NT pour aider `google_os` à simuler un PTY cohérent côté
  Linux/POSIX.

---

## 6. Build et toolchain

(Détails complets dans `BUILD.md`. Résumé ici.)

### 6.1 Pipeline officiel

```powershell
Import-Module .\tools\OpenConsole.psm1
Set-MsbuildDevEnvironment
Invoke-OpenConsoleBuild
```

Pré-requis : VS 2022 (toolset v143), Windows SDK 10.0.22621.0,
PowerShell 7+, .NET Framework Targeting Pack.

### 6.2 Notre pipeline

```powershell
pwsh -File scripts/terminal/build.ps1
```

Forces `PlatformToolset=v145` + `WindowsTargetPlatformVersion=10.0.26100.0`
parce que la machine n'a que VS 2026 Insiders. Force `nuget-latest.exe`
parce que `dep/nuget/nuget.exe` (4.1) ne comprend pas `.slnx`.

### 6.3 Configurations

Définies dans `vendor/terminal/src/common.build.pre.props` :

- **Debug** : `_DEBUG;DBG`, optimisations off, link incrémental.
- **Release** : `NDEBUG`, `/O2 /Ot /GL` (WPO), COMDAT folding, `OPT:REF`.
- **AuditMode** : Release + CppCoreCheck + PREfast.
- **Fuzzing** : ASAN (`/fsanitize=address`) + coverage tracing
  (`/fsanitize-coverage=…`), CRT statique, ne supporte pas HybridCRT.

Plates-formes : `x64`, `x86` (alias `Win32` côté MSBuild), `ARM64`.
**Pas** de `Any CPU` pour les C++.

---

## 7. Tests

Framework : **TAEF** (Test Authoring and Execution Framework de
Microsoft). NuGet `Microsoft.Taef 10.100.251104001` (cf.
`dep/nuget/packages.config`). Runner : `te.exe`.

Doc upstream : `vendor/terminal/doc/TAEF.md`.

Scripts d'invocation :

- `vendor/terminal/tools/runut.cmd` : unit tests
- `vendor/terminal/tools/runft.cmd` : feature tests
- `vendor/terminal/tools/runuia.cmd` : UIA tests
- depuis PowerShell : `Invoke-OpenConsoleTests` (cf.
  `vendor/terminal/tools/OpenConsole.psm1` ligne 163).

Liste des binaires de tests (depuis `vendor/terminal/tools/tests.xml`) :

| Nom logique         | Type | Binaire                                              | Suites notables                                                                                          |
|---------------------|------|------------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| `host`              | unit | `Conhost.Unit.Tests.dll`                             | `AliasTests`, `ApiRoutinesTests`, `ClipboardTests`, `ConsoleArgumentsTests`, `HistoryTests`, `InitTests`, `InputBufferTests`, `ObjectTests`, `OutputCellIteratorTests`, `ScreenBufferTests`, `SearchTests`, `SelectionTests`, `TextBufferIteratorTests`, `TextBufferTests`, `TitleTests`, `UtilsTests`, `ViewportTests`, `VtIoTests`. |
| `textBuffer`        | unit | `TextBuffer.Unit.Tests.dll`                          | Buffer interne (Row, OutputCellIterator, textBufferTextIterator, search, attributs).                     |
| `terminalCore`      | unit | `UnitTests_TerminalCore\Terminal.Core.Unit.Tests.dll` | Classe `Microsoft::Terminal::Core::Terminal`.                                                            |
| `terminalApp`       | unit | `UnitTests_TerminalApp\Terminal.App.Unit.Tests.dll`  | Logique de l'App Terminal (sans XAML).                                                                   |
| `localTerminalApp`  | unit | `TestHostApp\TerminalApp.LocalTests.dll`             | Tests locaux UI TerminalApp dans un host XAML local.                                                     |
| `unitSettingsModel` | unit | `UnitTests_SettingsModel\SettingsModel.Unit.Tests.dll` (isolated TAEF) | Parser JSON + héritage de profils.                                                            |
| `unitControl`       | unit | `UnitTests_Control\Control.Unit.Tests.dll`           | TermControl + ControlCore.                                                                               |
| `interactivityWin32`| unit | `Conhost.Interactivity.Win32.Unit.Tests.dll`         | Window proc, clipboard.                                                                                  |
| `terminal`          | unit | `ConParser.Unit.Tests.dll`                           | StateMachine VT (`Base64Test`, `InputEngineTest`, `OutputEngineTest`, `StateMachineTest`).               |
| `adapter`           | unit | `ConAdapter.Unit.Tests.dll`                          | `AdaptDispatch`, SGR, charsets.                                                                          |
| `types`             | unit | `Types.Unit.Tests.dll`                               | `CodepointWidthDetector`, `Viewport`, `convert`, `colorTable`.                                           |
| `til`               | unit | `til.unit.tests.dll`                                 | Tous les headers `til/*.h`.                                                                              |
| `feature`           | ft   | `Conhost.Feature.Tests.dll`                          | API tests bout-en-bout (`API_Alias`, `API_Buffer`, `API_Cursor`, `API_Dimensions`, `API_File`, `API_FillOutput`, `API_Font`, `API_Input`, `API_Mode`, `API_MultipleInflightMessage`, `API_Output`, `API_Policy`, `API_RgbColor`, `API_Title`, `CJK_Dbcs`, `Canary`, `Message_KeyPress`). |
| `uia`               | ft   | `Conhost.UIA.Tests.dll` (C#)                         | UI Automation via Appium WebDriver.                                                                      |
| `winconpty`         | ft   | `winconpty.Feature.Tests.dll`                        | Smoke tests bout-en-bout du ConPTY.                                                                      |

Fuzzers : `vtparser/ft_fuzzer/VTCommandFuzzer.vcxproj`,
`host/ft_fuzzer/Host.FuzzWrapper.vcxproj`, ASAN activé en config
Fuzzing.

---

## 8. Politiques de code

Sources : `vendor/terminal/doc/STYLE.md`, `ORGANIZATION.md`,
`EXCEPTIONS.md`, `WIL.md`, `Niksa.md`, `virtual-dtors.md`.

### 8.1 Style

- Modern C++ pour tout code neuf (cf. `STYLE.md` : « Modern C++ … and
  reference the C++ Core Guidelines as much as you possibly can »).
- **WIL obligatoire** pour les appels Win32/NT (`wil::unique_handle`,
  `RETURN_IF_WIN32_BOOL_FALSE`, `THROW_IF_FAILED`, etc.).
- `HRESULT` préféré à `NTSTATUS`. Les fonctions retournant un code
  d'erreur doivent être `noexcept` et `[[nodiscard]]`.
- C++/WinRT : utiliser les `weak_ref` correctement, comprendre la
  concurrence cppwinrt (cf. `STYLE.md`).

### 8.2 Organisation

Règles de `vendor/terminal/doc/ORGANIZATION.md` :

- chaque projet a un sous-dossier `ut_<name>` (unit tests) ;
- les feature tests vont en `ft_<name>` ;
- les scripts de build par type de sortie : `/dll`, `/exe`, `/lib` ;
- les interfaces publiques vont dans `inc/` ;
- groupez les libs liées (ex. `terminal/parser` + `terminal/adapter`).

### 8.3 Exceptions

Cf. `vendor/terminal/doc/EXCEPTIONS.md` :

1. **Ne pas** laisser une exception fuir du code neuf vers le vieux
   code.
2. **Retourner** `HRESULT` (préféré) ou `NTSTATUS`.
3. **Encapsuler** tout comportement d'exception dans la classe qui
   l'utilise.
4. **Ne pas** introduire d'exceptions modernes dans le vieux code.
5. **Utiliser WIL** pour les facilités modernes non-throwing
   (`wil::make_unique_nothrow`, `wistd::unique_ptr`).

### 8.4 WIL — Windows Implementation Library

Cf. `vendor/terminal/doc/WIL.md`. Patterns :

- `wil::unique_handle` (auto-`CloseHandle`), `wil::unique_process_information`,
  `wil::unique_process_heap_string`, `wil::scope_exit` (RAII custom).
- `RETURN_IF_WIN32_BOOL_FALSE(call)` : wrap autour de calls Win32 qui
  retournent `BOOL`. Sur false → `RETURN_HR(HRESULT_FROM_WIN32(GetLastError()))`.
- `LOG_IF_*` : équivalent loggant qui continue.
- `wil::make_unique_nothrow<T>()` : `std::make_unique` sans exception.

### 8.5 Destructeurs virtuels pour interfaces

Cf. `vendor/terminal/doc/virtual-dtors.md`. Pattern strict :

```cpp
class IRenderData {
public:
    virtual ~IRenderData() = 0;
};
inline IRenderData::~IRenderData() {}
```

Définir le destructeur pur virtuel hors de la classe. Sans ça, des
segfaults occasionnels au destructeur (l'interface est appelée à la
place de la classe dérivée).

### 8.6 Niksa.md

Récap de longs commentaires de Dustin Howett et Mike « Niksa » Griese
sur :

- pourquoi on ne touche pas à `cmd.exe` (compat 30+ ans) ;
- pourquoi les perfs typing-to-screen sont exceptionnelles
  (`PolyTextOut` GDI direct, pas de framework) ;
- comment Win32 USER32/GDI32 sont stratifiés ;
- l'histoire « Far East » vs « Western » dans `_stream.cpp` ;
- pourquoi pas de mixed elevated/non-elevated tabs (faille de
  sécurité) ;
- différence shell vs terminal (cf. `Niksa.md#shell-vs-terminal`,
  reproduit dans `INTEGRATION.md` § 1.2).

### 8.7 Linting / formatting

- `clang-format` (config dans `.clang-format` à la racine, fourni par
  VS dans `packages/clang-format.win-x86.10.0.0/`). `Invoke-CodeFormat`
  reformate tout (cf. `OpenConsole.psm1` ligne 411).
- `XamlStyler` (`tools/Test-XamlFormat` + `Invoke-XamlFormat`).
- `clang-format` est imposé par la CI (`build/scripts/Invoke-FormattingCheck.ps1`).
- Treat warnings as errors (`common.build.pre.props` ligne 119).

---

## 9. Specs et roadmap

### 9.1 Specs `doc/specs/`

60+ documents Markdown ; sélection notable :

- `#1043 - Set the initial position of the Terminal`
- `#11000 - Marks` (shell integration)
- `#1142 - Keybinding Arguments`
- `#1235 - Azure cloud shell connector`
- `#12570 - Show Hide operations on GetConsoleWindow via PTY`
- `#13000 - In-process ConPTY`
- `#1337 - Per-Profile Tab Colors`
- `#1502 - Advanced Tab Switcher`
- `#1564 - Settings UI`
- `#1571 - New Tab Menu Customization`
- `#1595 - Suggestions UI`
- `#16599 - Quick Fix`
- `#1790 - Font features and axes-spec`
- `#2046 - Command Palette`, `#2046 - Unified keybindings…`
- `#2325 - Default Profile Settings`
- `#2563 - closeOnExit and TerminalConnection evolution`
- `#2871 - Pane Navigation`
- `#3062 - Appearance configuration object for profiles`
- `#4066 - Theme-controlled color scheme switch`
- `#4191 - Formatted Copy`
- `#492 - Default Terminal`
- `#4993 - Keyboard Selection`
- `#4999 - Improved keyboard handling in Conpty`
- `#5000 - Process Model 2.0`
- `#532 - Panes and Split Windows`
- `#597 - Tab Sizing`
- `#605 - Search`
- `#607 - Commandline Arguments for the Windows Terminal`
- `#653 - Quake Mode`
- `#6899 - Action IDs`, `#6900 - Actions Page`
- `#7335 - Console Allocation Policy`
- `#754 - Cascading Default Settings`
- `#8324 - Application State (TSM)`
- `#885 - Terminal Settings Model`
- `#976 - VT52 escape sequences`
- `#980 - SnapOnOutput`
- `Keybindings-spec.md`
- `Proto extensions-spec.md`
- `TerminalSettings-spec.md`
- `portable-mode-spec.md`
- `settings-spec-template.md`, `spec-template.md`

Brouillons dans `doc/specs/drafts/` :
`#1256 - Tab tearoff`, `#2634 - Broadcast Input`,
`#3327 - Application Theming`, `#642 - Buffer Exporting and Logging`,
`#997 Non-Terminal-Panes.md`, `576-ProfilesJumplistSpec.md`.

### 9.2 Roadmaps

- `doc/terminal-v1-roadmap.md`, `doc/terminal-v2-roadmap.md` :
  feuilles de route historiques (v1 = 2019-2020, v2 = 2020-2021).
- `doc/roadmap-2022.md` : milestones 1.13 → 1.18, planning des
  semesters 22H1 / 22H2.
- `doc/roadmap-2023.md` : feuille de route active la plus récente
  (la 2024+ semble ne pas avoir été poussée publiquement).

### 9.3 Feature flags (`doc/feature_flags.md`)

Système `til::feature` : `src/features.xml` génère, via
`tools/Generate-FeatureStagingHeader.ps1`, un header avec :

```cpp
class Feature_Xxx {
public:
    static bool IsEnabled();
};
#define TIL_FEATURE_XXX_ENABLED 1   // ou 0 selon la cible
```

Stages : `AlwaysEnabled` / `AlwaysDisabled`. Filtres par branche
(`alwaysDisabledBranchTokens`, `alwaysEnabledBranchTokens`) et par
branding (`Dev`, `Preview`, `Release`, `WindowsInbox`). Précédence :
`alwaysDisabledReleaseTokens` > branches enabled > branches disabled
(plus longue match gagne) > brandings enabled > brandings disabled >
défaut.

---

## 10. Licence et conformité

- **Licence** : MIT (`vendor/terminal/LICENSE`). Copyright Microsoft.
- **Notices** : `vendor/terminal/NOTICE.md`. Composants tiers :

  - `jsoncpp` (MIT) ;
  - `chromium/base/numerics` (BSD-3) ;
  - `{fmt}` (MIT + exception optionnelle) ;
  - `interval_tree` (MIT) ;
  - `pcg-cpp` (MIT) ;
  - `wyhash` (public domain) ;
  - `stb` (public domain) ;
  - `Oklab` (MIT) ;
  - `ColorBrewer` (Apache-2.0) ;
  - `cmark` (BSD-2 + parties MIT) ;
  - `fzf` (MIT) ;
  - `GSL` (MIT) ;
  - `Microsoft-UI-XAML` (MIT) ;
  - `VirtualDesktopUtils` (extrait de PowerToys, MIT) ;
  - `wil` (MIT).

- **Notice spéciale** : « Notwithstanding any other terms, you may
  reverse engineer this software to the extent required to debug
  changes to any libraries licensed under the GNU Lesser General Public
  License » (NOTICE.md). Aucun composant LGPL embarqué actuellement
  mais la clause est défensive.

---

## 11. Risques d'intégration

### 11.1 Portabilité

- `cascadia/*` → entièrement Win32 + WinUI 2 + DirectX. Portabilité
  Linux = **0**.
- `host/`, `interactivity/win32/`, `propsheet/`, `propslib/`, `tsf/`,
  `audio/midi/` → Win32 only.
- `renderer/atlas/`, `renderer/gdi/`, `renderer/uia/`,
  `renderer/wddmcon/` → Win32/DirectX/GDI only.
- `winconpty/` → utilise `\Device\ConDrv` qui n'existe pas hors
  Windows. À ré-implémenter sur `posix_openpt`/`forkpty` côté
  `google_os`.
- `vtparser`, `bufferout`, `types`, `terminal/adapter`,
  `terminal/input` → C++ portable en théorie (uses `wchar_t` 16-bit
  toutefois, ce qui pose problème sur Linux où `wchar_t` est 32-bit).

### 11.2 WinUI 2 (pas 3)

`Microsoft.UI.Xaml 2.8.4`. WinUI 2 est en **maintenance** et ne reçoit
plus de nouvelles features. Microsoft ne migrera pas Terminal sur WinUI
3 à court terme (cf. issues GitHub) car ça impliquerait de réécrire le
host XAML islands. Pour nous : pas d'avenir à investir dans une
intégration directe Cascadia.

### 11.3 Feed NuGet privé

`vendor/terminal/NuGet.Config` :

```xml
<add key="TerminalDependencies"
     value="https://pkgs.dev.azure.com/shine-oss/terminal/_packaging/TerminalDependencies%40Local/nuget/v3/index.json" />
```

C'est l'unique feed. Tous les paquets sont récupérés là, y compris :

- `Microsoft.UI.Xaml` (public mais épinglé) ;
- `Microsoft.Internal.PGO-Helpers.Cpp` (interne Microsoft) ;
- `Microsoft.Internal.Windows.Terminal.ThemeHelpers` (interne Microsoft) ;
- `Microsoft.MSBuildCache.*` (preview public).

**Risque** : si ce feed disparaît ou bascule en privé restreint, le
build casse. Le projet pourrait avoir besoin d'un mirror local
(`dep/packages/` activable via `NuGet.Config` § « Static Package
Dependencies »).

### 11.4 Poids du build

- Toolchain VS 2026 + SDK 26100 + .NET 10 ≈ 25 Go disque.
- vcpkg installed (`obj/x64/vcpkg/`) ≈ 1-2 Go par config.
- `packages/` NuGet ≈ 500 Mo.
- `bin/x64/Release/` ≈ 200 Mo.
- Premier build complet (`-Project ""`) ≈ 30-60 minutes sur 8 cores ;
  build incremental d'un module ≈ 10-60 s.

### 11.5 PGO

`Microsoft.Internal.PGO-Helpers.Cpp 0.2.34` est interne Microsoft. Tant
qu'on désactive PGO (`-Project Conhost\Host_EXE` n'active pas PGO ; la
prop `PgoTarget=true` n'est utilisée que dans le pipeline Microsoft
officiel), pas de blocage.

### 11.6 XAML islands + multi-monitor DPI

`WindowsTerminal.exe` utilise `Microsoft.UI.Xaml.Hosting.WindowsXamlManager`
pour héberger du XAML 2 dans un HWND Win32 classique. Cette stack
exige Windows 10 1903+ et impose des contraintes DPI fortes
(`NonClientIslandWindow.cpp`). Non transposable.

---

## 12. Conclusion : ce qu'on garde pour `google_os`

Mapping explicite **Terminal → aphrody** :

| Composant Terminal              | Use-case `aphrody`                                                                                  | Action concrète                                                                                                          |
|---------------------------------|--------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
| `vtparser` (`ConTermParser.lib`)| Émulateur de terminal pour notre futur PTY POSIX dans `google_os` (côté Linux) et bridge interactif dans `crates/cli/`. | Linker en static, exposer via `extern "C"` minimal dans une future crate `terminal_ffi`.                                  |
| `bufferout` (`ConBufferOut.lib`)| Modèle de référence pour `crates/cli` (buffer interactif type REPL). On peut **soit** linker, **soit** ré-implémenter en Rust sur le même modèle. | Étudier le modèle `Row`/`TextBuffer`/`OutputCell` pour s'inspirer. Implémentation full-Rust préférable à terme.            |
| `winconpty` (`conptylib.lib` ou `conpty.dll`) | Couche ConPTY pour spawn de `cmd`/`bash`/`pwsh` depuis nos crates Rust. | Sur Windows : utiliser `kernel32!CreatePseudoConsole` directement (équivalent), ou linker `conpty.dll` open-source pour bénéficier des fixes récents. |
| `terminal/adapter` (`ConTermAdapt.lib`) | Référence pour traduire VT → API console lors de l'émulation conhost dans `google_os`. | Référence + tests TAEF à étudier ; ré-implémentation Rust à terme.                                                       |
| `terminal/input` (`TerminalInput.lib`) | Encodage clavier/souris VT pour notre REPL Rust (envoyer du « \e[A » sur flèche haut, par exemple). | Réimplémenter en Rust (algorithme trivial) ou wrapper FFI minimal.                                                       |
| `types` (`ConTypes.lib`)        | `CodepointWidthDetector` (largeur Unicode) très utile. | Soit linker `ConTypes.lib`, soit utiliser la crate Rust `unicode-width` + grapheme. La crate Rust est probablement suffisante. |
| `server` (`ConServer.lib`)      | **Référence** pour notre émulateur de protocole ConDrv si on porte un userland POSIX qui s'attend à parler à conhost (peu probable mais possible). | Référence d'algorithme uniquement.                                                                                       |
| `til` (header-only)             | Patterns C++20 (`small_vector`, `flat_set`, `generational`, etc.). | Pas nécessaire en Rust pur (équivalents `smallvec`, `hashbrown`).                                                        |
| `host` (`ConhostV2Lib.lib`) + `OpenConsole.exe` | Référence d'implémentation complète de conhost. Très précieux pour l'équivalent côté `google_os`. | Référence pure. Pas de link.                                                                                             |
| `cascadia/wt/` (`wt.exe` shim)  | Modèle minimal pour notre propre alias AppX si on package un jour. | Référence (36 lignes de C++).                                                                                            |

### Ce qu'on **n'utilisera pas** :

- toute la pile **Cascadia/WinUI** (`TerminalApp`, `TerminalControl`,
  `TerminalSettingsModel`, `TerminalSettingsEditor`, `WindowsTerminal`,
  `CascadiaPackage`, `ShellExtension`, `WpfTerminalControl`,
  `Remoting`) : ce sont des couches UI Win32-only et WinUI 2 que nous
  n'avons pas vocation à reproduire dans un userland POSIX ;
- les **renderers** (`atlas`, `gdi`, `uia`, `wddmcon`) : trop liés à
  DirectX/GDI ;
- `propsheet`, `propslib`, `tsf`, `audio/midi`, `interactivity/win32`,
  `interactivity/onecore` : Win32-only et hors scope ;
- `colortool` (.NET), tous les `tools/*` (benchcat, scratch, etc.).

### Verdict global

L'unique investissement direct rentable est dans **un crate
`terminal_ffi` à venir** qui linke statiquement `vtparser.lib`,
`bufferout.lib`, `terminal/adapter.lib`, `terminal/input.lib`,
`types.lib`, plus éventuellement `winconpty.lib` côté hôte Windows.

Le reste est consommé en **lecture** : ce dépôt sert de référence
canonique pour comprendre comment Microsoft a résolu les problèmes
d'émulation VT, de buffer de texte, de codepage CJK, de Unicode width,
de Sixel, de signal pipe et de ConPTY. Cette connaissance alimentera
directement les modules `google_os::libc::io`, `google_os::libc::ipc`
et `google_os::libc::process` quand on devra émuler un PTY côté Linux
hôte.
</content>
</invoke>


<!-- ============================================== -->
<!-- SOURCE: docs/terminal/BUILD.md -->
<!-- ============================================== -->

# `vendor/terminal` — Procédure de build

Document écrit le 2026-05-16, valide pour le commit
`8fe6c21ef88a73a7985b5968ee18936928ccac69` (cf. `README.md`).

## 1. Pré-requis effectifs sur la machine

| Composant                       | Version utilisée localement      | Source                            |
|---------------------------------|----------------------------------|-----------------------------------|
| OS                              | Windows 11 Home 10.0.28020       | machine de dev                    |
| Visual Studio                   | 2026 Community Insiders 18.7     | preview                           |
| MSVC toolset                    | v145 (cl 14.51)                  | composant VS Insiders             |
| Windows SDK                     | 10.0.26100.0                     | inclus avec VS Insiders           |
| .NET                            | 10.0.300                         | required pour `Microsoft.Taef`    |
| PowerShell                      | 7.6.1                            | requis par `OpenConsole.psm1`     |
| NuGet                           | 7.6.0.59 via `dep/nuget/nuget-latest.exe` | téléchargé par le wrapper ; le 4.1 embarqué dans le repo ne parse pas `.slnx` |
| MSBuild                         | 18.7.1 (VS 2026 Insiders)        | requis pour parser `.slnx` (cf. note ci-dessous) |

Pré-requis upstream officiels (cf. `vendor/terminal/README.md` §
*Prerequisites* et `vendor/terminal/doc/building.md`) :

- Windows 10 2004 (build >= 19041) ou ultérieur ;
- Developer Mode activé pour pouvoir déployer `CascadiaPackage` ;
- PowerShell 7+ ;
- Windows 11 SDK 10.0.22621.0 ;
- VS 2022 minimum ;
- workload « Desktop Development with C++ » + « Universal Windows
  Platform Development » ;
- composant individuel « C++ (v143) Universal Windows Platform Tools » ;
- .NET Framework Targeting Pack pour les projets de tests managés.

Le repo embarque ses dépendances natives via NuGet
(`vendor/terminal/dep/nuget/packages.config`) et vcpkg (manifest
`vendor/terminal/vcpkg.json`, baseline figée `15e5f3820f0370f1ba…`).
Les paquets vcpkg utilisés : `fmt 12.1.0`, `ms-gsl 3.1.0`, plus dans la
feature `terminal` : `jsoncpp 1.9.6`, `cli11 2.6.1`, `cmark 0.31.1`.

## 2. Procédure officielle Microsoft

Depuis PowerShell :

```powershell
Import-Module .\tools\OpenConsole.psm1
Set-MsbuildDevEnvironment
Invoke-OpenConsoleBuild
```

`Set-MsbuildDevEnvironment` (`vendor/terminal/tools/OpenConsole.psm1`)
utilise `VSSetup` + `Microsoft.VisualStudio.DevShell.dll` pour exporter
les variables d'environnement de `vcvarsall.bat` dans le shell courant.
Sans le flag `-Prerelease`, seules les installs stables de VS sont
considérées : VS 2026 Insiders est ignoré, d'où le wrapper local.

`Invoke-OpenConsoleBuild` (même fichier) appelle :

1. `nuget.exe restore OpenConsole.slnx` ;
2. `nuget.exe restore dep\nuget\packages.config` ;
3. `msbuild.exe OpenConsole.slnx @args` (où `@args` reçoit tout ce qu'on
   passe à la fonction PowerShell).

Depuis `cmd.exe` :

```cmd
.\tools\razzle.cmd
bcz
```

`razzle.cmd` est l'équivalent `cmd` de `Set-MsbuildDevEnvironment`. `bcz`
(`tools/bcz.cmd`) est l'alias clean + build.

## 3. Procédure utilisée chez nous

Le repo `microsoft/terminal` pin :

```xml
<PlatformToolset>v143</PlatformToolset>
<WindowsTargetPlatformVersion>10.0.22621.0</WindowsTargetPlatformVersion>
```

(`vendor/terminal/src/common.build.pre.props`, lignes 78 et 98). Or
notre machine ne possède **ni** le toolset v143, **ni** le SDK 22621 :
elle est equipée du toolset v145 et du SDK 10.0.26100.0 livrés avec VS
2026 Insiders. Il faut donc surcharger les deux variables au moment de
l'invocation MSBuild, sans modifier les `.props` upstream (lecture
seule). C'est exactement le rôle de
`scripts/terminal/build.ps1` (à la racine du repo, hors sous-module).

### 3.1 Anatomie de `scripts/terminal/build.ps1`

Le script (90 lignes) fait :

1. `Import-Module ./tools/OpenConsole.psm1 -Force` ;
2. `Set-MsbuildDevEnvironment -Prerelease` : ce flag pousse `VSSetup` à
   inclure les builds Insiders (le `-Prerelease` n'est utilisé que par
   ce wrapper, jamais par le script upstream) ;
3. force `$env:PlatformToolset = 'v145'` et
   `$env:WindowsTargetPlatformVersion = '10.0.26100.0'` ;
4. télécharge `nuget-latest.exe` depuis
   `https://dist.nuget.org/win-x86-commandline/latest/nuget.exe` s'il
   est absent (version 7.6.0.59 au moment de l'écriture). Raison : le
   `dep/nuget/nuget.exe` embarqué dans le repo est en version 4.1.x,
   antérieure au format `.slnx`, et plante avec « Invalid input
   'OpenConsole.slnx'. The file type was not recognized. » ;
5. exécute deux `nuget restore` (slnx + packages.config). Le restore
   `.slnx` est passé avec `-MSBuildPath "$env:VSINSTALLDIR\MSBuild\Current\Bin"` :
   `nuget.exe` n'a toujours pas de parser `.slnx` natif (issue
   NuGet/Home #14034 ouverte), mais quand on lui passe `-MSBuildPath`
   vers MSBuild 17.13+ (ici 18.7 / VS 2026), MSBuild s'occupe du parse ;
6. lance MSBuild avec :

```text
msbuild.exe OpenConsole.slnx
    /p:Configuration=<Debug|Release|AuditMode|Fuzzing>
    /p:Platform=<x64|x86|ARM64>
    /p:PlatformToolset=v145
    /p:WindowsTargetPlatformVersion=10.0.26100.0
    /p:AppxSymbolPackageEnabled=false
    /m /nologo /v:minimal
    [/t:<Project>]
```

Paramètres du wrapper :

| Paramètre        | Défaut             | Valeurs acceptées                    |
|------------------|--------------------|--------------------------------------|
| `-Project`       | `Conhost\Host_EXE` | nom de cible MSBuild, ou `""` (tout) |
| `-Configuration` | `Release`          | `Debug`, `Release`, `AuditMode`, `Fuzzing` |
| `-Platform`      | `x64`              | `x64`, `x86`, `ARM64`                |

La cible `Conhost\Host_EXE` est un *fast smoke test* qui ne reconstruit
que `OpenConsole.exe` (le `conhost.exe` local) et ses dépendances
directes. Pour tout construire :

```powershell
pwsh -File scripts/terminal/build.ps1 -Project ""
```

`/p:AppxSymbolPackageEnabled=false` désactive la génération du
`.appxsym` du packaging MSIX, qui exige des certificats que nous
n'avons pas.

### 3.2 Pourquoi écrire un wrapper plutôt que patcher les `.props`

`vendor/terminal/src/common.build.pre.props` est imposé par toutes les
`.vcxproj` du repo via :

```xml
<Import Project="$(SolutionDir)src\common.build.pre.props" />
```

Le modifier reviendrait à committer dans le sous-module et à diverger
de l'upstream. MSBuild laisse heureusement gagner toute variable passée
en `/p:` sur la valeur définie dans une `<PropertyGroup Label="Configuration">`,
ce qui est exactement le cas ici. La méthode est documentée par
Microsoft : voir
<https://learn.microsoft.com/en-us/cpp/build/reference/setting-additional-msbuild-properties>.

## 4. Configurations disponibles

Définies dans `vendor/terminal/src/common.build.pre.props` (lignes 187 →
269) et listées dans `vendor/terminal/OpenConsole.slnx` (`<BuildType>` :
`AuditMode`, `Debug`, `Fuzzing`, `Release`).

| Configuration | Particularités                                                                                       |
|---------------|------------------------------------------------------------------------------------------------------|
| `Debug`       | `_DEBUG;DBG`, optimisations désactivées (`/Od`), CRT debug, link incrémental, `DebugFastLink`        |
| `Release`     | `NDEBUG`, `/O2 /Ot /GL`, WPO, COMDAT folding, `/OPT:REF`, full PDB                                   |
| `AuditMode`   | identique à Release plus `CppCoreCheck` + `PREfast` (`/analyze`) via `src/StaticAnalysis.ruleset`    |
| `Fuzzing`     | `/fsanitize=address /fsanitize-coverage=…`, CRT statique, `libsancov.lib` + `clang_rt.asan_dynamic`, désactive HybridCRT |

Plateformes : `x64`, `x86` (= `Win32` côté MSBuild), `ARM64`. La
plate-forme `Any CPU` n'est pas supportée pour les projets C++ (voir
README upstream).

HybridCRT (`EnableHybridCRT`) est activé par défaut sauf en `Fuzzing` :
il fait disparaître la dépendance `vcruntime140.dll` en linkant
statiquement la STL et en réimportant les symboles `vcruntime` depuis
`ucrtbase.dll`. C'est pour ça que ConPTY peut tourner dans
`kernelbase.dll` sans dépendances DLL exotiques.

Commande type :

```powershell
# Release x64, tout le monde
pwsh -File scripts/terminal/build.ps1 -Project "" -Configuration Release -Platform x64

# Debug x64, juste OpenConsole.exe + ses libs (rapide)
pwsh -File scripts/terminal/build.ps1 -Configuration Debug

# Mode Fuzzing pour ASAN (utile sur le parser VT)
pwsh -File scripts/terminal/build.ps1 -Project "TerminalParser_FT_Fuzzer" -Configuration Fuzzing
```

## 5. Sorties

`vendor/terminal/src/common.build.pre.props` (lignes 5 → 24) impose :

```text
OutDir = $(SolutionDir)\bin\$(Platform)\$(Configuration)\
IntDir = $(SolutionDir)\obj\$(Platform)\$(Configuration)\$(ProjectName)\
```

Pour C++/WinRT, `OutDir` reçoit un suffixe `\$(ProjectName)\` pour ne
pas écraser les `.winmd` entre projets. Tous les exécutables et DLL
atterrissent donc dans `vendor/terminal/bin/<Platform>/<Configuration>/`.

Cibles installées pour `Conhost\Host_EXE` (chaîne typique) :

```
bin/x64/Release/OpenConsole.exe        (= conhost local)
bin/x64/Release/conptylib.lib          (statique, namespace winconpty.LIB)
bin/x64/Release/conpty.dll             (= winconpty.DLL)
bin/x64/Release/OpenConsoleProxy.dll   (interface IDL Console/Terminal Handoff)
bin/x64/Release/ConhostV2Lib.lib       (statique, hostlib)
bin/x64/Release/ConBufferOut.lib       (statique, bufferout)
bin/x64/Release/ConTermParser.lib      (statique, terminal/parser)
bin/x64/Release/ConTermAdapt.lib       (statique, terminal/adapter)
bin/x64/Release/TerminalInput.lib      (statique, terminal/input)
bin/x64/Release/ConTypes.lib           (statique, types)
bin/x64/Release/ConRenderBase.lib      (statique, renderer/base)
bin/x64/Release/ConRenderAtlas.lib     (statique, renderer/atlas, dépend de D3D11/D2D)
bin/x64/Release/ConRenderGdi.lib       (statique, renderer/gdi)
bin/x64/Release/ConRenderUia.lib       (statique, renderer/uia)
bin/x64/Release/ConServer.lib          (statique, server, IPC ConDrv)
bin/x64/Release/ConTSF.lib             (statique, Text Services Framework)
bin/x64/Release/ConInteractivityBaseLib.lib  (statique, interactivity/base)
bin/x64/Release/ConInteractivityWin32Lib.lib (statique, interactivity/win32)
bin/x64/Release/MidiAudio.lib          (statique, audio/midi)
bin/x64/Release/console.dll            (propsheet)
bin/x64/Release/ConProps.lib           (propslib)
```

Pour `CascadiaPackage` (target « tout Terminal moderne »), s'ajoutent :

```
bin/x64/Release/WindowsTerminal/WindowsTerminal.exe
bin/x64/Release/wt.exe                       (shim de redirection)
bin/x64/Release/wtd.exe                      (variante Dev branding)
bin/x64/Release/CascadiaPackage_*.msix       (paquet MSIX signé/non signé — requiert UAP patch sur wap-common.build.pre.props, cf. PATCHES.diff)
bin/x64/Release/WindowsTerminalShellExt.dll
bin/x64/Release/Microsoft.Terminal.Control.dll
bin/x64/Release/Microsoft.Terminal.Settings.Model.dll
bin/x64/Release/Microsoft.Terminal.Settings.Editor.dll
bin/x64/Release/TerminalApp.dll
bin/x64/Release/elevate-shim.exe
bin/x64/Release/UIHelpers.dll, UIMarkdown.dll, WinRTUtils.dll
```

## 6. Tests

Voir `vendor/terminal/doc/TAEF.md` et `vendor/terminal/tools/tests.xml`.

Lancer les tests unitaires depuis PowerShell après un build :

```powershell
Import-Module vendor/terminal/tools/OpenConsole.psm1
Invoke-OpenConsoleTests                # tous les unit tests x64 Debug
Invoke-OpenConsoleTests -Test til      # juste les unit tests TIL
Invoke-OpenConsoleTests -FTOnly        # tous les feature tests
Invoke-OpenConsoleTests -Test uia      # UI automation (déplace la souris)
```

`Invoke-OpenConsoleTests` charge `tools/tests.xml`, qui décrit chaque
binaire de test (`Conhost.Unit.Tests.dll`, `TextBuffer.Unit.Tests.dll`,
`til.unit.tests.dll`, `ConParser.Unit.Tests.dll`,
`ConAdapter.Unit.Tests.dll`, `Types.Unit.Tests.dll`,
`Terminal.Core.Unit.Tests.dll`, etc.). Chaque suite tourne via
`te.exe`, fourni par le NuGet `Microsoft.Taef 10.100.251104001`.

Variantes ligne de commande :

```cmd
.\tools\runut.cmd           :: unit tests
.\tools\runft.cmd           :: feature tests
.\tools\runuia.cmd          :: UIA tests
```

## 7. Troubleshooting (erreurs rencontrées)

### 7.1 « MSB8020 : The build tools for v143 cannot be found »

Cause : `common.build.pre.props` pin v143, mais la machine n'a que v145.
Solution : ajouter `/p:PlatformToolset=v145` (déjà géré par le wrapper).

### 7.2 « MSB4019 : The imported project … 10.0.22621.0 was not found »

Cause : SDK 22621 absent. Solution : `/p:WindowsTargetPlatformVersion=10.0.26100.0`
(géré par le wrapper).

### 7.3 « Unable to parse solution file 'OpenConsole.slnx' »

Cause : NuGet 4.1 embarqué dans `dep/nuget/nuget.exe` ne comprend pas
`.slnx`. **Subtilité** : `nuget.exe` lui-même n'a pas de parser `.slnx`
natif à ce jour (cf. issue NuGet/Home #14034, toujours ouverte mai
2026). Ce qui fait fonctionner notre pipeline, c'est l'argument
`-MSBuildPath "$env:VSINSTALLDIR\MSBuild\Current\Bin"` passé à
`nuget-latest.exe` : NuGet délègue alors le parse de `.slnx` à MSBuild
18.7 (VS 2026), qui le supporte nativement depuis MSBuild 17.13.

Solutions :
1. **Notre wrapper** : télécharge `nuget-latest.exe` (7.6.0.59) +
   `-MSBuildPath` vers MSBuild 18.7 — fonctionne.
2. **Alternative officielle .NET** (non utilisée ici car le restore
   doit aussi traiter `dep/nuget/packages.config` qui est l'ancien
   format) : `dotnet restore OpenConsole.slnx`, supporté depuis
   .NET SDK 9.0.200.

### 7.4 Warning bénin `vswhere.exe not found in PATH` au démarrage du shell

`vswhere` n'est nécessaire qu'à `Invoke-CodeFormat` (clang-format),
pas au build. Ignorable.

### 7.5 `DEP0700 : Registration of the app failed` au déploiement
`CascadiaPackage`

Cf. `vendor/terminal/doc/building.md` § *Are you seeing DEP0700* : le
`OpenConsoleProxy.dll` est verrouillé par une instance Terminal Dev
restée ouverte. Tuer les processus `WindowsTerminalDev.exe` puis
relancer le deploy.

### 7.6 vcpkg : `error: failed to download cmark`

Cause : `vcpkg.json` impose un baseline figé
(`15e5f3820f0370f1ba7150853762cec0688cd396`) qui peut bouger côté
upstream. Solution : `set VCPKG_BINARY_SOURCES=clear;` puis relancer.

### 7.7 `error LNK2019 unresolved external symbol __imp_CreatePseudoConsole`

Cause : on link `conptylib.lib` mais on inclut `<consoleapi.h>` qui
déclare les symboles comme `dllimport`. Solution : utiliser
`vendor/terminal/src/inc/conpty-static.h` qui redéclare les symboles
sans `dllimport`, ou linker `conpty.dll` à la place.

### 7.8 `error APPX3217 : UAP 10.0.22621.0 introuvable`

Cause : `vendor/terminal/src/wap-common.build.pre.props` hard-code
`TargetPlatformVersion=10.0.22621.0` sans condition, donc l'override
MSBuild `/p:TargetPlatformVersion=...` est écrasé. Solution : appliquer
le patch local qui ajoute `Condition="'$(TargetPlatformVersion)' == ''"`
sur cette ligne (cf. `PATCHES.diff`). Le script `scripts/terminal/build.ps1`
passe ensuite `/p:TargetPlatformVersion=10.0.26100.0`.

## 7bis. État du build vérifié (2026-05-16, machine de référence)

Après application de `PATCHES.diff` et exécution de
`scripts/terminal/build.ps1 -Project "" -Configuration Release -Platform x64`,
les artefacts suivants sont produits dans
`vendor/terminal/bin/x64/Release/` :

**Exécutables vérifiés**

| Binaire                                 | Origine                          |
|-----------------------------------------|----------------------------------|
| `OpenConsole.exe`                       | `src/host/exe/Host.EXE.vcxproj` (= `conhost.exe` local) |
| `OpenConsoleProxy.dll`                  | `src/host/proxy/Host.Proxy.vcxproj` |
| `conpty.dll`                            | `src/winconpty/dll/winconptydll.vcxproj` |
| `wt.exe`                                | shim de redirection vers WindowsTerminal |
| `WindowsTerminal/WindowsTerminal.exe`   | `src/cascadia/WindowsTerminal/WindowsTerminal.vcxproj` |
| 263 DLL + 56 EXE au total (incluant tests) | |

**Bibliothèques statiques utiles pour l'intégration `google_os`**

| Lib                              | Source                                  |
|----------------------------------|-----------------------------------------|
| `ConTermParser.lib`              | `src/terminal/parser/lib/`              |
| `ConBufferOut.lib`               | `src/buffer/out/lib/`                   |
| `conptylib.lib`                  | `src/winconpty/lib/`                    |
| `ConTermAdapt.lib`               | `src/terminal/adapter/lib/`             |
| `ConTypes.lib`                   | `src/types/lib/`                        |
| `ConServer.lib`                  | `src/server/lib/`                       |
| `ConhostV2Lib.lib`               | `src/host/lib/` (référence seulement)   |
| `ConRenderAtlas.lib`             | `src/renderer/atlas/`                   |
| `ConRenderBase.lib`              | `src/renderer/base/lib/`                |
| `ConInteractivityWin32Lib.lib`   | `src/interactivity/win32/lib/`          |
| `ConInt.lib`                     | `src/internal/`                         |

**Erreur résiduelle (non bloquante pour notre cas d'usage)**

`CascadiaPackage.wapproj` produit un `.msix` signé seulement si :
- patch UAP de `PATCHES.diff` appliqué (déjà fait sur la machine de référence) ;
- ET un certificat de code signing valide est installé.

Sans cert, le MSIX échoue mais `WindowsTerminal.exe` lui-même est
opérationnel et lançable directement (pas via Store).

## 8. Recettes utiles

### Recompiler uniquement un sous-projet

Tout `.vcxproj` du repo est buildable via son chemin solution :

```powershell
pwsh -File scripts/terminal/build.ps1 -Project "Conhost\TerminalParser"
pwsh -File scripts/terminal/build.ps1 -Project "Conhost\BufferOut"
pwsh -File scripts/terminal/build.ps1 -Project "Conpty\winconpty_LIB"
```

(Les noms exacts viennent des attributs `ProjectName` dans chaque
`.vcxproj`. La hiérarchie dans la sln correspond aux `<Folder Name="…">`
de `OpenConsole.slnx`.)

### Profile-Guided Optimization

Désactivée par défaut sur build externe. Activable via
`/p:PgoBuildType=Instrument` ou `Optimize`, en présence du NuGet
`Microsoft.Internal.PGO-Helpers.Cpp 0.2.34`. Ce paquet est interne
Microsoft et n'est pas redistribuable : le feed
`pkgs.dev.azure.com/shine-oss/terminal/_packaging/TerminalDependencies`
fait office de mirror public, mais sa pérennité n'est pas garantie pour
nous.

### Reformater le code

Depuis `Set-MsbuildDevEnvironment` :

```powershell
Invoke-CodeFormat        # clang-format + xstyler sur tout le repo
```

(Inutile dans notre setup : on ne committe pas dans le sous-module.)
</content>
</invoke>


<!-- ============================================== -->
<!-- SOURCE: docs/terminal/GEMINI_CLI.md -->
<!-- ============================================== -->

# Gemini CLI sur Windows Terminal — diagnostic crash et workaround Bun

Document écrit le **2026-05-16**. Cible : faire fonctionner
`packages/gemini-cli/` (workspace npm vendoré dans le monorepo) sur la
machine de dev qui tourne Node v26.1.0, npm 11.13.0, Bun 1.3.14.

## 1. Le problème

Sur cette machine, lancer `gemini-cli` selon n'importe laquelle des
méthodes upstream (`npm install -g @google/gemini-cli`, `npx`,
`node scripts/start.js`) crash systématiquement avec l'un des trois
patterns suivants :

| # | Symptôme observé                                                 | Cause racine                                                                |
|---|------------------------------------------------------------------|------------------------------------------------------------------------------|
| A | `ECOMPROMISED` à `npm install` ou `npx`                          | bug Node v24/v25/v26 + npm v11 sur Windows (file lock du cache npm) : [google-gemini/gemini-cli#14149](https://github.com/google-gemini/gemini-cli/issues/14149) |
| B | `ReferenceError: agent is not defined` dans `windowsTerminal.js` | binding `node-pty` non recompilé pour `NODE_MODULE_VERSION` 137 (Node 26)   |
| C | `Cannot resize a pty that has already exited` (`WindowsPtyAgent.resize`) | race condition `@lydell/node-pty` + ConPTY sous Node 24+ : [#12045](https://github.com/google-gemini/gemini-cli/issues/12045) |
| D | Freeze à `Initializing…` au premier boot                         | binding pty bloqué sur OpenConsole handshake : [#19248](https://github.com/google-gemini/gemini-cli/issues/19248) |

**Validation locale** :

```
$ node --version
v26.1.0
$ npm --version
11.13.0
```

C'est exactement la combinaison cassée. `@lydell/node-pty@1.1.0`
(consommée par `packages/gemini-cli/packages/core/package.json`) n'a
pas de prebuilt pour `NODE_MODULE_VERSION 137` au moment de l'écriture.

## 2. Décision projet (2026-05-16) : on utilise Bun

Plutôt que d'installer un Node manager (nvm-windows / fnm / volta) pour
downgrader vers Node 22 LTS, **on utilise `bun` (1.3.14)** déjà présent
sur la machine. Bun :

- gère `"workspaces"` et la spec `workspace:*` que npm 11 refuse ;
- résout son propre cache (pas de file lock Windows partagé avec npm) ;
- a une ABI N-API compatible avec les modules natifs Node, dont
  `@lydell/node-pty-win32-x64` (testé à `1.2.0-beta.12` upstream) ;
- réduit drastiquement le temps d'install (`bun install` ≈ 5 s vs
  `npm install` ≈ 90 s sur ce monorepo).

### 2.1 Recette `bun` — état des essais 2026-05-16

Trois recettes ont été testées sur cette machine, par ordre de complexité
croissante :

| # | Commande                                                             | Résultat                                                                                                |
|---|----------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|
| 1 | `bun install`                                                        | échoue dans le `prepare` script qui appelle `execSync('npm install')` (Node v26)                        |
| 2 | `bun install --ignore-scripts`                                       | OK, lockfile écrit. Mais le `prepare` skip empêche aussi l'install transitif des packages workspace     |
| 3 | `bun esbuild.config.js`                                              | échoue : `Could not resolve "execa"`, `"tinycolor2"`, `"ws"` (deps présentes dans `.bun/` cache mais non symlinkées dans `node_modules/`) |

**Conclusion** : le pipeline officiel `gemini-cli` n'est pas réellement
compatible Bun sans patch upstream. Les `scripts/build.js` et
`scripts/build_package.js` font des `execSync('npm install')` et
`execSync('npm run build')` en dur, ce qui force un retour à
Node v26 + npm 11 qui casse.

**Recette qui fonctionnerait** (à valider) — patch minimal de
`scripts/build.js` pour remplacer `npm install` par `bun install` :

```powershell
cd C:\src\aphrody\packages\gemini-cli

# Patch local (pas remonté upstream) : remplacer "npm install" par "bun install"
# dans scripts/build.js et scripts/build_package.js, et "npm run build" par "bun run build".
# Lignes concernées : build.js:30, build.js:42, build_package.js (chercher npm run).

bun install
bun run bundle
bun bundle/gemini.js --version
```

Ce patch est **bloqué tant que `packages/gemini-cli` est vendoré
tel quel** : modifier ces scripts est traçable mais dirty.
À discuter avec l'amont (issue à ouvrir : « support `BUN_INSTALL`
env var to use bun for installs from scripts/build.js »).

### 2.1bis Recette de contournement immédiate

En attendant le patch, **la solution qui marche aujourd'hui sur cette
machine** est le Plan C (Node v22 LTS, cf. § 3). Une fois Node v22
installé :

```powershell
# Avec Node v22 LTS actif (via volta/fnm/PATH dédié) :
cd C:\src\aphrody\packages\gemini-cli
bun install                  # bun gère mieux workspace:* que npm 11
bun run bundle               # scripts/build.js voit Node v22 → npm install OK
node bundle/gemini.js --version
```

Le mix `bun install` + `node bundle/gemini.js` capture le meilleur des
deux : install rapide, runtime Node 22 LTS stable pour `@lydell/node-pty`.

### 2.2 Profil Windows Terminal dédié

Le `settings.json` de Windows Terminal Canary
(`%LOCALAPPDATA%\Packages\Microsoft.WindowsTerminalCanary_8wekyb3d8bbwe\LocalState\settings.json`)
peut recevoir un profil prêt à l'emploi :

```jsonc
{
  "guid": "{a8b1c2d3-4e5f-4a6b-9c8d-ef0123456789}",
  "name": "Gemini CLI (bun)",
  "commandline": "%USERPROFILE%\\.bun\\bin\\bun.exe run %REPO_ROOT%\\packages\\gemini-cli\\bundle\\gemini.js",
  "startingDirectory": "%REPO_ROOT%",
  "icon": "ms-appx:///ProfileIcons/{0caa0dad-35be-5f56-a8ff-afceeeaa6101}.png",
  "colorScheme": "Campbell Powershell",
  "font": { "face": "Google Sans Code", "size": 18, "weight": "bold" },
  "hidden": false
}
```

À insérer dans `profiles.list[]` du `settings.json`.

## 3. Plans de repli

### Plan B — Désactiver complètement node-pty

`gemini-cli` lit la variable d'environnement `GEMINI_PTY_INFO` (cf.
`packages/gemini-cli/packages/core/dist/src/utils/getPty.js`). Si on la
positionne à `child_process`, le CLI tombe back sur un `spawn` standard
sans ConPTY :

```powershell
$env:GEMINI_PTY_INFO = 'child_process'
gemini
```

Conséquence : on perd l'interactivité fine (VT input, redimensionnement
PTY), mais `gemini` reste utilisable pour les requêtes one-shot.

### Plan C — Downgrade Node v22 LTS

Si Bun pose problème sur une dépendance future :

```powershell
winget install OpenJS.NodeJS.LTS  # installe Node 22.x LTS
node --version                     # v22.x
npm install -g @google/gemini-cli
```

À partir de mai 2026, Node 22 LTS est supporté jusqu'à avril 2027.

## 4. Pourquoi pas WSL ?

WSL est une option valide (Node 22 LTS sous Ubuntu, ConPTY remplacé par
PTY POSIX, donc plus de race condition `WindowsPtyAgent.resize`), mais :

- l'expérience est lente côté FS (cross-mount `\\wsl$\`) si le projet vit
  sur `C:\src\` ;
- on perd l'intégration native avec les outils Windows (Visual Studio,
  notebooks `.ps1`, MCP `windows-mcp`) ;
- ce sera de toute façon adressé par `crates/google_os` qui implémente
  un userland POSIX natif sur NT.

## 5. Crash *résiduel* éventuel côté Windows Terminal

Indépendamment de `gemini-cli`, Windows Terminal **lui-même** peut
crasher dans deux cas connus :

1. **Driver GPU intermittent** : « A handful of Intel & Radeon drivers
   intermittently drop the resize event that Atlas needs. » Si tu
   observes des freezes au resize, bascule `rendering.graphicsAPI` de
   `"direct3d11"` vers `"direct2d"` (cf. `BUILD.md` § 4.1 de Terminal
   docs).
2. **WindowsTerminalDev** local non installé après un build : tant que
   `scripts/terminal/build.ps1` produit le `.msix` mais que
   `Add-AppDevPackage.ps1` n'a pas été exécuté, lancer
   `WindowsTerminal.exe` directement échoue avec
   `class not registered`. Voir `docs/terminal/BUILD.md` § 7.5.

## 6. Références

- Issue [#14149](https://github.com/google-gemini/gemini-cli/issues/14149) — ECOMPROMISED Node 24/25/26
- Issue [#19248](https://github.com/google-gemini/gemini-cli/issues/19248) — Freeze Initializing Node 20
- Issue [#14619](https://github.com/google-gemini/gemini-cli/issues/14619) — `agent is not defined`
- Issue [#12045](https://github.com/google-gemini/gemini-cli/issues/12045) — `Cannot resize a pty`
- Issue [#9054](https://github.com/google-gemini/gemini-cli/issues/9054) — EPERM + ERR_INVALID_ARG_TYPE
- [Microsoft Learn — Windows Terminal Rendering Settings](https://learn.microsoft.com/en-us/windows/terminal/customize-settings/rendering)
- [Bun docs — Workspace support](https://bun.sh/docs/install/workspaces)


<!-- ============================================== -->
<!-- SOURCE: docs/terminal/INTEGRATION.md -->
<!-- ============================================== -->

# `vendor/terminal` — Stratégie d'intégration avec `aphrody` / `google_os`

Document écrit le 2026-05-16, valide pour le commit
`8fe6c21ef88a73a7985b5968ee18936928ccac69`.

Le but : énumérer quels composants de Microsoft Terminal sont
réellement reprenables dans le contexte de `aphrody` (monorepo Rust
qui porte un userland POSIX sur Windows via la crate `google_os` →
syscalls NT natifs via `windows-rs`), et lesquels sont à laisser de
côté. Politique du projet : **zéro stub, zéro mock, 100 % production**
(`CLAUDE.md`).

## 1. Matrice détaillée des composants

Légende :

- **Sortie** : `.lib` statique, `.dll` dynamique, `.exe`, ou en-têtes.
- **Couplage Win32** : oui (utilise `kernel32`/`user32`/`gdi32`/`d3d11`/`d2d`/`dwrite`/`comctl32`/`WinUI`), partiel (Win32
  uniquement par RAII WIL, portable en théorie), non (header-only C++20 STL).
- **Intérêt** : raison concrète pour l'intégrer dans notre stack.
- **État** : `gardé`, `optionnel`, `écarté`.

| Composant Terminal                | Chemin                                       | Sortie                  | Couplage Win32 | Intérêt pour aphrody                                                                  | État        |
|-----------------------------------|----------------------------------------------|-------------------------|---------------|-------------------------------------------------------------------------------------------|-------------|
| `til` (Terminal Implementation Library) | `vendor/terminal/src/inc/til/` + `src/til/` | en-têtes + lib unit-test | partiel (WIL via headers) | utilitaires C++20 : `til::small_vector`, `til::flat_set`, `til::generational`, `til::rect`, `til::point`, `til::color`, `til::env`, `til::rle`, `til::throttled_func`, `til::ticket_lock`, `til::winrt` | **gardé** : utiles si on a un module C++ FFI, *non* utiles côté pur Rust |
| `vtparser` (`TerminalParser`)     | `vendor/terminal/src/terminal/parser/lib/`   | `ConTermParser.lib`     | non           | state machine xterm / ECMA-48 / DEC complète, séquences CSI/OSC/DCS/SS3, supporte Win32-Input-Mode | **gardé** : référence n°1 pour notre futur émulateur de PTY Linux côté `google_os` |
| `bufferout` (`BufferOut`)         | `vendor/terminal/src/buffer/out/lib/`        | `ConBufferOut.lib`      | partiel       | modèle de text buffer UTF-16 / UTF-8 avec attributs SGR, surrogate pairs, lignes DECDHL/DECDWL, recherche, double-width CJK | **gardé** : modèle de référence pour le buffer de nos consoles |
| `winconpty` LIB                   | `vendor/terminal/src/winconpty/lib/`         | `conptylib.lib`         | oui           | implémentation production du ConPTY (handles `\Device\ConDrv\Server` + `\Reference` + pipe de signal, `CreateProcessAsUserW` + `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`) | **gardé** : référence n°2 pour notre crate ConPTY syscall dans `google_os` |
| `winconpty` DLL                   | `vendor/terminal/src/winconpty/dll/`         | `conpty.dll` + `.def`   | oui           | équivalent open-source de `kernel32!CreatePseudoConsole` ; exporte `Conpty*` + alias compat `CreatePseudoConsole` | **optionnel** : utilisable directement par P/Invoke depuis nos crates Rust |
| `terminal/adapter` (`TerminalAdapter`) | `vendor/terminal/src/terminal/adapter/lib/` | `ConTermAdapt.lib` | partiel | mapping verbes VT → calls API console, gère SGR, modes DEC, polices, macros (DECDMAC), Sixel | **gardé** : essentiel si on émule le serveur console côté `google_os` |
| `terminal/input` (`TerminalInput`) | `vendor/terminal/src/terminal/input/lib/`   | `TerminalInput.lib`     | partiel       | encodage clavier→VT (xterm modifyOtherKeys, win32-input-mode, mouse SGR `\e[<…`) | **gardé** : utile pour synthétiser de l'input VT depuis `google_os` |
| `types` (`Types`)                 | `vendor/terminal/src/types/lib/`             | `ConTypes.lib`          | partiel       | `Viewport`, `CodepointWidthDetector` (largeur Unicode + grapheme clusters), `convert.cpp` (UTF-8↔UTF-16), `colorTable`, `sgrStack`, `Cluster`, `IInputEvent`, palettes UIA | **gardé** : `CodepointWidthDetector` est portable, utile au pur layout |
| `renderer/base` (`RendererBase`)  | `vendor/terminal/src/renderer/base/lib/`    | `ConRenderBase.lib`     | partiel       | abstraction `IRenderEngine` + `Renderer` (transforme `IRenderData` en primitives `DrawString`/`DrawCursor`) | **optionnel** : utile uniquement si on garde une UI Win32 ; sinon ré-écrire en Rust |
| `renderer/atlas` (`RendererAtlas`) | `vendor/terminal/src/renderer/atlas/`       | `ConRenderAtlas.lib`    | oui           | moteur DirectWrite + Direct2D + Direct3D 11 + cache de glyphes GPU + custom HLSL shaders | **écarté** : trop spécifique Win32/DirectX, non portable |
| `renderer/gdi` (`RendererGdi`)    | `vendor/terminal/src/renderer/gdi/lib/`    | `ConRenderGdi.lib`      | oui           | rendu GDI (utilisé par `conhost.exe` historique)                                          | **écarté** : Win32 only, perf < Atlas |
| `renderer/uia` (`RendererUia`)    | `vendor/terminal/src/renderer/uia/lib/`    | `ConRenderUia.lib`      | oui           | « rendu » virtuel pour UI Automation                                                       | **écarté** : Win32-only, hors scope |
| `renderer/wddmcon`                | `vendor/terminal/src/renderer/wddmcon/lib/` | `wddmcon.lib`           | oui           | rendu DXGK pour environnement de boot (avant que le compositeur ne tourne)                  | **écarté** : usage interne kernel |
| `server` (`Server`)               | `vendor/terminal/src/server/lib/`           | `ConServer.lib`         | oui (ConDrv)  | implémente côté user-mode le protocole ALPC `\Device\ConDrv\Server` (`ApiDispatchers`, `IoDispatchers`, `WaitBlock`, `ProcessHandle`) | **gardé** : référence d'implémentation du serveur console NT |
| `interactivity/base`              | `vendor/terminal/src/interactivity/base/lib/` | `ConInteractivityBaseLib.lib` | partiel | abstraction `IConsoleControl`, `IConsoleWindow`, `IInteractivityFactory` ; `ServiceLocator` ; `VtApiRedirection` ; `RemoteConsoleControl` | **optionnel** : si on rebondit sur conhost à distance |
| `interactivity/win32`             | `vendor/terminal/src/interactivity/win32/lib/` | `ConInteractivityWin32Lib.lib` | oui | clipboard, dpi, window proc, IME, UIA, sélection, fenêtre `conhost` | **écarté** |
| `interactivity/onecore`           | `vendor/terminal/src/interactivity/onecore/lib/` | `onecore.lib` | oui | équivalent OneCore (sans `user32`)                                                          | **écarté** : niche |
| `propslib` (`PropertiesLibrary`)  | `vendor/terminal/src/propslib/`             | `ConProps.lib`          | oui (registry/LNK) | lecture/écriture des prefs console depuis HKCU + `.lnk`                                  | **écarté** : registry Windows only |
| `propsheet`                       | `vendor/terminal/src/propsheet/`            | `console.dll`           | oui           | property sheet « clic droit propriétés »                                                  | **écarté** |
| `tsf` (`TextServicesFramework`)   | `vendor/terminal/src/tsf/`                  | `ConTSF.lib`            | oui           | bridge IME (CJK, pen, touch)                                                              | **écarté** : nécessite TextServicesFramework Windows |
| `audio/midi`                      | `vendor/terminal/src/audio/midi/lib/`       | `MidiAudio.lib`         | oui (winmm)   | implémentation du DECPSO (commande de musique VT)                                          | **écarté** : usage curiosité |
| `host` (`Host`)                   | `vendor/terminal/src/host/lib/`             | `ConhostV2Lib.lib`      | oui           | tout `conhost.exe` (boucle d'événements, dispatch API, clipboard, fenêtre)                  | **gardé en référence** uniquement |
| `host/exe`                        | `vendor/terminal/src/host/exe/`             | `OpenConsole.exe`       | oui           | `conhost` rebuildable en dev                                                              | **gardé en référence** |
| `host/proxy`                      | `vendor/terminal/src/host/proxy/`           | `OpenConsoleProxy.dll`  | oui           | proxy COM IDL (`IConsoleHandoff`, `ITerminalHandoff`)                                       | **écarté** |
| `cascadia/TerminalConnection`     | `vendor/terminal/src/cascadia/TerminalConnection/` | `Microsoft.Terminal.TerminalConnection.dll` | oui (WinRT) | implémente `ConptyConnection`, `AzureConnection`, `EchoConnection` côté WinRT | **écarté** : WinRT + WinUI 2 obligatoire |
| `cascadia/TerminalCore`           | `vendor/terminal/src/cascadia/TerminalCore/lib/` | `TerminalCore.lib`  | partiel       | classe `Microsoft::Terminal::Core::Terminal` qui composite buffer + parser + adapter + input sans UI ; `ITerminalApi` + `IRenderData` | **optionnel** : c'est une glue C++ propre, utile si on garde une UI |
| `cascadia/TerminalControl`        | `vendor/terminal/src/cascadia/TerminalControl/` | `Microsoft.Terminal.Control.dll` | oui (XAML islands) | `TermControl` (WinUI 2), `HwndTerminal` (Win32 pur via `HwndTerminal.cpp`), automation peer | **optionnel** : `HwndTerminal` est utilisable en C++ pur sans XAML |
| `cascadia/TerminalApp`            | `vendor/terminal/src/cascadia/TerminalApp/` | `TerminalApp.dll`       | oui (WinUI 2) | l'application Terminal (tabs, panes, palette, settings UI)                                  | **écarté** : 100 % WinUI 2 + XAML islands |
| `cascadia/TerminalSettingsModel`  | `vendor/terminal/src/cascadia/TerminalSettingsModel/` | `Microsoft.Terminal.Settings.Model.dll` | oui | parseur JSON5 + héritage de profils, schéma `profiles.schema.json` | **écarté** : couplé WinRT |
| `cascadia/WindowsTerminal`        | `vendor/terminal/src/cascadia/WindowsTerminal/` | `WindowsTerminal.exe` | oui (XAML islands) | hôte Win32 de l'application WinUI                                                          | **écarté** |
| `cascadia/CascadiaPackage`        | `vendor/terminal/src/cascadia/CascadiaPackage/` | `*.msix`              | oui (AppX)    | packaging MSIX, jumplist                                                                  | **écarté** |
| `cascadia/wt`                     | `vendor/terminal/src/cascadia/wt/`          | `wt.exe`, `wtd.exe`     | oui           | shim 36-lignes : redirige `wt args` → `WindowsTerminal.exe wt args` via `CreateProcessW`     | **gardé en référence** : exemple minimal de redirection AppX |
| `cascadia/WpfTerminalControl`     | `vendor/terminal/src/cascadia/WpfTerminalControl/` | `Microsoft.Terminal.Wpf.dll` | oui (WPF .NET) | contrôle terminal pour applications WPF, basé sur `HwndTerminal`                          | **écarté** : .NET WPF |
| `tools/ColorTool`                 | `vendor/terminal/src/tools/ColorTool/`      | `colortool.exe`         | oui           | applique des schemes XTerm dans la palette `conhost`                                      | **écarté** : niche |
| `tools/scratch`, `tools/nihilist`, etc. | `vendor/terminal/src/tools/`           | divers `.exe`           | oui           | bancs d'essai, jouets internes                                                            | **écarté** |

Conclusion résumée :

- **Composants gardés en intégration directe** (link statique) :
  `vtparser`, `bufferout`, `winconptylib`, `terminal/adapter`,
  `terminal/input`, `types`, `server`.
- **Composants utilisés via DLL** : `conpty.dll` (P/Invoke depuis Rust
  est trivial, et c'est aussi ce qu'expose `kernel32` nativement).
- **Composants gardés en référence d'implémentation** : `host`, `wt`,
  `interactivity/base`.
- **Composants écartés** : toute la pile `cascadia/`, `renderer/atlas`,
  `renderer/gdi`, `renderer/uia`, `propsheet`, `propslib`, `tsf`,
  `audio/midi`, `interactivity/win32`.

## 2. Stratégie d'intégration FFI

### 2.1 Cible : crate Rust `crates/terminal_ffi/` (à créer plus tard)

Modèle préconisé :

1. linker les `.lib` statiques C++ depuis `bin/x64/Release/` via une
   `build.rs` qui appelle `cc-rs` (pour les wrappers `extern "C"`) +
   `bindgen` (pour générer les FFI sur les en-têtes `conpty-static.h`,
   `winconpty.h`, et nos propres wrappers) ;
2. exposer une surface C `extern "C"` minimale écrite par nous (côté
   C++) dans un fichier `wrapper.cpp`, pour cacher la machinerie WIL /
   exceptions / RAII C++ ;
3. allouer les tampons via `mimalloc` (cf. `CLAUDE.md`, contrainte zero-copy
   FFI), exposer des pointeurs bruts safe-wrapped côté Rust.

### 2.2 Modèle d'API FFI minimale

Exemple à viser pour un wrapper Rust autour du parser VT, sans
introduire de stub :

```rust
// crates/terminal_ffi/src/parser.rs (à créer plus tard, pas maintenant)
unsafe extern "C" {
    fn gcli_vt_parser_new() -> *mut GcliVtParser;
    fn gcli_vt_parser_free(p: *mut GcliVtParser);
    fn gcli_vt_parser_feed(p: *mut GcliVtParser,
                           data: *const u16, len: usize,
                           callback: GcliVtCallback,
                           user: *mut std::ffi::c_void) -> u32;
}
```

`wrapper.cpp` côté C++ instancie un
`Microsoft::Console::VirtualTerminal::StateMachine` (cf.
`vendor/terminal/src/terminal/parser/stateMachine.hpp`, classes
publiques `StateMachine`, `OutputStateMachineEngine`,
`InputStateMachineEngine`) et redispatche les actions via la callback.

### 2.3 ConPTY direct depuis Rust (sans terminal_ffi)

Le DLL `conpty.dll` exporte des symboles équivalents à
`kernel32!CreatePseudoConsole`. Liaison via `windows-rs` :

```text
use windows::Win32::System::Console::{
    CreatePseudoConsole, ResizePseudoConsole, ClosePseudoConsole, HPCON, COORD,
};
```

Tant qu'on tourne sur Windows 10 19041+, on peut utiliser le ConPTY
système. Pour bénéficier des correctifs récents (notamment GH#12977 sur
le win32-input-mode), il faut charger `conpty.dll` produit par
`winconpty/dll` depuis `bin/`. Cf. `vendor/terminal/src/winconpty/dll/winconpty.def`
pour la liste exacte d'exports.

### 2.4 Lecture du text buffer en zero-copy

`vendor/terminal/src/buffer/out/textBuffer.hpp` expose des itérateurs
(`TextBufferCellIterator`, `TextBufferTextIterator`) qui retournent des
`std::wstring_view` sur les cellules. Pour rester zero-copy entre Rust
et C++ via `mimalloc`, l'approche correcte :

1. wrapper C++ qui prend un `std::function<void(const wchar_t*, size_t, TextAttribute)>`
   et l'appelle pour chaque run ;
2. la callback C `extern "C"` côté Rust reçoit un slice `&[u16]` et un
   `u64` d'attributs SGR encodés ;
3. allocation des buffers de cellules dans l'arène `mimalloc` partagée
   (`mi_malloc` / `mi_free`).

## 3. Sécurité

### 3.1 Modèle C++ Terminal incompatible avec `no_std` Rust

Terminal repose massivement sur :

- **WIL** (`vendor/terminal/dep/wil/`) : macros `RETURN_IF_*`,
  `THROW_IF_*`, smart handles (`wil::unique_handle`,
  `wil::unique_process_information`, etc.). WIL utilise les exceptions
  C++ pour propager des erreurs depuis les helpers
  `THROW_IF_WIN32_BOOL_FALSE`.
- **RAII C++** partout (`std::filesystem::path`, `std::unique_ptr`,
  scope guards).
- **Exceptions C++** internes, encapsulées dans les classes mais
  jamais converties en codes d'erreur pour les fonctions publiques (cf.
  `vendor/terminal/doc/EXCEPTIONS.md`).

Conséquence : exposer une surface `extern "C"` est **obligatoire** pour
toute consommation depuis Rust. Aucun symbole C++ ne doit traverser le
FFI directement. Les exceptions doivent être interceptées dans le
wrapper et converties en codes d'erreur :

```cpp
// wrapper.cpp esquisse (à écrire le moment venu, pas maintenant)
extern "C" int32_t gcli_pty_create(/* args */, HPCON* out) noexcept try
{
    return SUCCEEDED(ConptyCreatePseudoConsole(/*…*/, out)) ? 0 : -1;
}
catch (...)
{
    return -2;
}
```

`noexcept` + try/catch sur la totalité du corps est le seul moyen
sûr d'éviter de laisser remonter une exception C++ jusqu'à l'unwinder
Rust (UB).

### 3.2 Plateformes

`google_os` vise à terme MUSL Linux et WebAssembly (cf. commit
`f986583c` de notre repo : « configure ultra-minimal release profiles
and isolate Windows dependencies to enable Linux MUSL and WebAssembly
cross-compilation »). Tous les composants Terminal listés « gardés »
sont C++ portable **sur le papier** mais en pratique câblés à Win32 :

- `vtparser` n'a pas de dépendance Win32 hors `wchar_t` 16-bit (à
  vérifier précisément avec `nm` après build), c'est le plus
  portable ;
- `bufferout` dépend de `til/u8u16convert.h` qui n'utilise pas Win32 ;
- `winconpty` est intrinsèquement Win32 (utilise `NtCreateFile`,
  `CreateProcessAsUserW`, `\Device\ConDrv`). **Non portable.** Seul
  utilisable en condition d'hôte Windows.

**Décision projet (2026-05-16)** : pour les cibles non-Windows
(MUSL Linux, WebAssembly, BSD à venir), **aucun composant C++ de
Microsoft Terminal n'est porté ni linké**. Tout est ré-implémenté en
**Rust pur** dans `crates/google_os/` :

- VT parser → réécrire en Rust (s'inspirer de `vte` crate ou repartir
  de zéro à partir de `vendor/terminal/src/terminal/parser/lib/` pour
  le comportement, pas pour le code) ;
- TextBuffer → modèle data-structure en Rust (`crates/google_os/src/`
  + éventuel sous-crate `crates/terminal_buffer/`) ;
- ConPTY → ré-implémenter au-dessus de `posix_openpt` / `forkpty` (Linux)
  ou shim WASI (WASM), en s'inspirant de l'**algorithme** documenté
  dans `vendor/terminal/src/winconpty/winconpty.cpp` (séquence
  `CreateServerHandle` + `CreateClientHandle(\Reference)` + signal
  pipe + `CreateProcessAsUserW(--headless)`) — pas du code.

Conséquence : Terminal Microsoft reste **strictement Windows-hôte**
dans notre stack. Le futur `crates/terminal_ffi` (si on le crée) ne
compile que sous `--target x86_64-pc-windows-msvc`, guard par
`#[cfg(target_os = "windows")]`. Tout chemin Linux/WASM est servi par
nos crates Rust natives.

Cette décision tranche le débat `wchar_t` 16-bit vs 32-bit et
`-fshort-wchar` : on n'y touche pas, car aucun code C++ Terminal ne
traverse la frontière OS.

### 3.3 ABI

Microsoft Terminal compile en x64 par défaut. `common.build.pre.props`
définit `_WINDOWS;EXTERNAL_BUILD;_SILENCE_STDEXT_ARR_ITERS_DEPRECATION_WARNING`
et impose `stdcpp20`. ABI MSVC C++20, non compatible MinGW. On linkera
donc nos crates Rust avec la même toolchain MSVC (`stable-x86_64-pc-windows-msvc`).

## 4. Aspects légaux

`vendor/terminal/LICENSE` : MIT. Permet :

- distribution binaire dans `aphrody` (avec mention de copyright) ;
- modification (mais on ne modifie pas l'upstream ; on vendore et on
  surcharge MSBuild via wrapper) ;
- link statique dans nos binaires Rust → OK ;
- relicensing impossible (rester sous MIT pour la portion redistribuée).

`vendor/terminal/NOTICE.md` liste les composants tiers embarqués sous
MIT ou Apache-2.0 compatibles : `jsoncpp`, `chromium/base/numerics`,
`{fmt}`, `interval_tree`, `pcg`, `wyhash`, `stb`, `Oklab`,
`ColorBrewer` (Apache-2.0), `cmark`, `fzf`, GSL, Microsoft-UI-XAML,
VirtualDesktopUtils, WIL. **Si on redistribue un binaire embarquant
ces composants, il faut reproduire les notices.**

Risque : le NuGet `Microsoft.Internal.PGO-Helpers.Cpp` est interne
Microsoft, non MIT. La PGO build doit rester désactivée par défaut
chez nous (déjà le cas — pas dans le wrapper).

### 4.1 Décisions projet 2026-05-16 sur les dépendances tierces

**Feed NuGet privé `pkgs.dev.azure.com/shine-oss/terminal/_packaging/TerminalDependencies`**
(défini dans `vendor/terminal/NuGet.Config`).

- **Décision** : accepté tel quel, pas de mirroir proactif.
- **Plan de repli** : les packages restaurés vivent dans
  `vendor/terminal/packages/` (créé par `nuget restore` au premier
  build) et restent localement disponibles. Si Microsoft restreint
  un jour l'accès au feed, on peut soit (a) committer le dossier
  `packages/` une fois pour toutes (gros mais portable), soit
  (b) push les `.nupkg` vers un feed NuGet interne (ex. GitHub
  Packages, Azure Artifacts).
- **Pourquoi pas mirroir maintenant** : le feed est public en lecture,
  rien ne casse aujourd'hui, et mirroir 200+ MB de NuGet sans raison
  pollue le repo.

**Packages internes Microsoft** : `Microsoft.Internal.PGO-Helpers.Cpp`,
`Microsoft.Internal.Windows.Terminal.ThemeHelpers`.

- **Décision** : acceptés tant qu'ils sont publics depuis le feed
  ci-dessus (ils le sont actuellement, le restore réussit).
- **Si on perd l'accès** : PGO est non activée chez nous
  (`PgoBuildType` non set → `Microsoft.Internal.PGO-Helpers.Cpp` est
  inerte). `ThemeHelpers` est consommé par `CascadiaPackage` ; si on
  ne build pas le UI Terminal (et notre intégration cible parser/buffer/conpty
  uniquement), on peut désactiver `CascadiaPackage` via
  `/p:BuildProjectReferences=false` sur le subgraph.

## 5. Checklist go/no-go pour intégrer Terminal

Avant de tirer le moindre `.lib` Terminal dans un crate :

- [ ] le composant figure-t-il dans la colonne « **gardé** » de la
      matrice § 1 ? Si non → stop.
- [ ] le composant peut-il être enveloppé dans un `extern "C"`
      `noexcept try/catch` ?
- [ ] la cible Rust est-elle compilée avec
      `stable-x86_64-pc-windows-msvc` (toolset compatible MSVC v145) ?
- [ ] le crate qui consomme respecte-t-il la directive `CLAUDE.md` :
      pas de stub, pas de TODO, allocation via `mimalloc` ?
- [ ] les tests TAEF du composant (cf. `tools/tests.xml`) tournent-ils
      proprement en local ?
- [ ] si on redistribue, `NOTICE.md` est-il mis à jour ?
- [ ] si le composant est Win32-only (`winconpty`, `host`,
      `interactivity/win32`), a-t-on un plan B pour Linux/WASM
      (cf. § 3.2) ?

Si toutes les cases sont cochées → go. Sinon, garder le composant en
« référence » et ré-implémenter en Rust dans `google_os`.

## 6. Conclusion

L'intégration ciblée *value* de Terminal sur **Windows hôte** se
réduit à un ensemble clair : **parser VT, adapter, input, buffer,
types, server, conpty**.

Sur **toutes les autres cibles** (Linux MUSL, WebAssembly, BSD…),
décision projet 2026-05-16 : **Rust pur, point final**. Les composants
Terminal ne servent que de référence algorithmique ; ils ne sont ni
portés, ni linkés. La parité fonctionnelle (VT parsing, TextBuffer,
PTY) est livrée par les crates `google_os` et compagnie. Cela libère
le projet de l'ABI MSVC C++, de `wchar_t` 16-bit, de WIL et des
exceptions C++ partout où Windows n'est pas l'hôte.
</content>
</invoke>


<!-- ============================================== -->
<!-- SOURCE: docs/terminal/README.md -->
<!-- ============================================== -->

# `vendor/terminal` — Microsoft Terminal en sous-module

Document d'index, écrit le **2026-05-16**.

## Pourquoi Terminal est-il dans `vendor/`

Microsoft Terminal est cloné comme sous-module Git à l'emplacement
`vendor/terminal/`. C'est un dépôt monolithique qui contient à la fois
`conhost.exe` (le serveur de console NT historique de Windows), la pile
ConPTY (`winconpty.dll`), la nouvelle application `WindowsTerminal.exe`
(WinUI 2 + DirectWrite + Direct3D) et un ensemble de bibliothèques
réutilisables : parser VT, text buffer, framework `til`, etc. Cette base
est utile à `aphrody` pour trois raisons :

1. fournir un terminal d'avant-plan moderne pour nos CLIs / REPL ;
2. récupérer les composants C++ statiques (`vtparser`, `bufferout`,
   `winconptylib`, `til`) à linker depuis nos crates Rust via
   `windows-rs` + `cc-rs` + `mimalloc` ;
3. servir de référence d'implémentation du serveur ConDrv NT pour aider
   `google_os` à émuler un pseudo-terminal cohérent côté Linux/POSIX.

Le sous-module est **en lecture seule** côté `aphrody`. Toute
modification se fait amont, dans le fork si nécessaire.

## Version clonée

```
commit  8fe6c21ef88a73a7985b5968ee18936928ccac69
date    2026-05-15 13:56:48 -0500
title   Keep the font size delta across settings reloads (#20230)
```

Branche officielle : `microsoft/terminal:main`. Licence : **MIT**
(`vendor/terminal/LICENSE`). Notices tierces : `vendor/terminal/NOTICE.md`.

## Documents disponibles

| Fichier                                  | Rôle                                                                                                            |
|------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| [`ARCHITECTURE.md`](./ARCHITECTURE.md)   | Cartographie complète : binaires produits, dossiers de `src/`, couches, composants réutilisables, tests, specs. |
| `BUILD.md`                 | Build officiel Microsoft + procédure spécifique `scripts/terminal/build.ps1` (toolset v145 + SDK 10.0.26100.0). |
| `INTEGRATION.md`     | Matrice d'intégration pour `google_os` / `aphrody` : composants utiles, stratégie FFI, sécurité, go/no-go.   |
| `PATCHES.diff`         | Patches locaux appliqués au sous-module (overlays vcpkg v143→v145, warning C4875). Réappliquer : `cd vendor/terminal && git apply ../../docs/terminal/PATCHES.diff`. |
| `GEMINI_CLI.md`       | Diagnostic du crash `gemini-cli` sur Windows (Node v26 + npm 11 incompatibles `node-pty`). Workaround Bun documenté + plans de repli. |

## Build local

Pré-requis sur la machine cible : VS 2026 Community Insiders 18.7 (toolset
v145, MSVC 14.51), Windows SDK 10.0.26100.0, PowerShell 7.6.1, .NET 10.0.300.

Construction d'un sous-projet de fumée (par défaut : `OpenConsole.exe`,
alias dev de `conhost.exe`) :

```powershell
pwsh -File scripts/terminal/build.ps1
```

Plein détails dans `BUILD.md`.

## Politiques de lecture / écriture

- **Aucun fichier de notre cru** ne vit dans `vendor/terminal/`. Le
  wrapper de build (`scripts/terminal/build.ps1`) et notre doc
  (`docs/terminal/`) sont à la racine du repo parent.
- Le sous-module reste **dirty** après build : patches obligatoires
  (`PATCHES.diff`) et artifacts générés (`bin/`, `obj/`, `packages/`).
  Ces patches sont locaux, jamais propagés à l'upstream Microsoft.
- Pour mettre à jour la version vendorisée :
  `git submodule update --remote vendor/terminal`, ré-appliquer
  `PATCHES.diff`, puis `git add` du sous-module dans le parent.
- Toute documentation française additionnelle va dans `docs/terminal/`,
  jamais dans `vendor/terminal/`.

## À consulter en complément

- `../design/aphrody-terminal-spec.md` :
  **spec normative aphrody-terminal LLM-first** (5 piliers, WASM-native,
  M3-themed) — successeur Rust pur du modèle vendor/terminal Win-only.
- `../design/aphrody-terminal-integration-matrix.md` :
  matrice contract-de-vie par crate (chaque crate du workspace a un slot
  dans `aphrody-terminal`).
- [`../PLAN-MOONSHOT.md`](PLAN-MOONSHOT.md) : plan 30 jours qui drive
  l'ambition `aphrody-terminal`.
- `vendor/terminal/README.md` : README officiel Microsoft.
- `vendor/terminal/doc/building.md` : procédure de build amont.
- `vendor/terminal/doc/ORGANIZATION.md` : description de l'organisation
  du code par Microsoft.
- `vendor/terminal/doc/STYLE.md`, `EXCEPTIONS.md`, `WIL.md`,
  `virtual-dtors.md` : règles de codage.
- `vendor/terminal/doc/specs/` : 60+ specs de features VT et UI.
</content>
</invoke>


<!-- ============================================== -->
<!-- SOURCE: docs/design/aphrody-terminal-integration-matrix.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody-terminal — integration matrix (every crate has a job)

> **Ambition** : aucun crate du workspace ne reste orphelin. aphrody-terminal
> est le **front-end intégrateur** qui consomme chaque pièce du puzzle. Si une
> crate ne sert pas à la terminal, elle prouve sa valeur ailleurs (ex. `gui`
> desktop Wry+Tao, `aphrody-translate` CLI standalone) — sinon elle a un slot
> ici.

Cette matrice est la **contrat-de-vie** des crates aphrody. Chaque tick de
développement doit pousser plus de cellules de la colonne "Consumer status"
de `⏳` vers `✅`.

## Matrice principale

| Crate                  | Rôle dans aphrody-terminal                                                                | Consumer status                                  |
|---|---|---|
| `base`                 | Primitives no_std (consommé transitivement par `aphrody-terminal-vt`)                     | ✅ via vt                                         |
| `backend`              | `aphrody-terminal-browser` réutilise `backend::network` pour HTTP fetch fallback + `backend::process` pour sub-agent process inspection (pid/name/cmdline → sub-agent pane enrichment) | ⏳ T-INT-backend |
| `a2a`                  | `aphrody-terminal-llm` publie chaque `LlmEvent` AUSSI comme A2A envelope (parallèle au broadcast tokio interne)                                                                       | ⏳ T-INT-a2a     |
| `a2a-client`           | `aphrody-terminal-llm` envoie au peer winclean via `a2a-client::http_jsonrpc`             | ⏳ T-INT-a2a     |
| `a2a-server`           | `aphrody term` héberge un endpoint A2A pour envelopes entrantes (sub-agents externes)     | ⏳ T-INT-a2a     |
| `a2a-grpc`             | Transport gRPC optionnel pour l'event bus terminal-llm (haut-débit)                       | ⏳ T-INT-a2a     |
| `a2a-pb`               | Protos partagés entre `aphrody-terminal-llm::LlmEvent` et A2A envelope                    | ⏳ T-INT-a2a     |
| `mrx-core`             | (transitif via `mrx-watch`)                                                               | ⏳ T-INT-mrx     |
| `mrx-detect`           | (transitif via `mrx-watch`)                                                               | ⏳ T-INT-mrx     |
| `mrx-audit`            | Pane "workspace health" — score audit live dans terminal                                  | ⏳ T-INT-mrx     |
| `mrx-watch`            | Stream watch → pane "Workspace activity" (live diff, file changes, build status)          | ⏳ T-INT-mrx     |
| `mrx-cli`              | Spawnable depuis terminal command palette (`Ctrl+Shift+P → scan workspace`)               | ⏳ T-INT-mrx     |
| `aphrody-translate`    | i18n labels FR/EN switchables dans command palette (`aphrody-translate::convert`)         | ⏳ T-INT-i18n    |
| `aphrody-wasm`         | `aphrody-terminal-wasm` réexporte les helpers crypto/encoding (aes_gcm, base64)           | ⏳ T-INT-wasm    |
| `ievr-tools`           | Pane optionnelle IEVR (game RE workflow, gated `--feature ievr`)                          | ⏳ T-INT-ievr (low prio) |
| `aphrody-summary`      | Pane "docs preview" — `aphrody_summary::generate()` rendu inline via `DocsPreviewPane` (publish `LlmEvent::Markdown` sur l'event bus) | ✅ T-INT-summary |
| `m3-tokens`            | Tokens couleur/typo consommés par `aphrody-terminal-wasm` (déjà câblé)                    | ✅                                                |
| `shadcn-bridge`        | Chrome terminal : header bar, tab strip, command palette, dialogs (M3 segmented buttons, list, fab) | ⏳ T-INT-chrome  |
| `a2a-ui`               | **Embedded as the "A2A coord channel" pane** — mailbox viewer JSONL en live              | ⏳ T-INT-a2a-ui  |
| `aphrody-memory`       | Session memory (JSONL + brute-force HNSW pour semantic recall des commandes passées)      | ✅ T-INT-memory (`SessionMemoryPane` in `aphrody-terminal-llm`, publishes `LlmEvent::SessionRecall`) |
| `aphrody-gateway`      | Routes les LLM calls du browser pane + sub-agent dispatch vers Cloudflare/Vercel/OpenAI-BYOK | ⏳ T-INT-gateway |
| `aphrody-mcp`          | OAuth 2.1 client pour MCP HTTP/SSE servers du pane MCP status                             | ⏳ T-INT-mcp     |
| `aphrody-voice`        | TTS pour hook event audio notifications (configurable per-event)                          | ⏳ T-INT-voice   |
| `aphrody-voice-stt`    | Push-to-talk prompt input (Ctrl+Shift+V long press → STT → inject dans terminal stdin)    | ⏳ T-INT-voice   |
| `gemini-runtime`       | **First-class "spawn Gemini CLI" panel** — bouton dans command palette → tab dédiée       | ⏳ T-INT-gemini  |
| `agui-bridge`          | Sub-agent task tree parle AG-UI protocol au renderer (compat avec écosystème agui)        | ⏳ T-INT-agui    |
| `aphrody-channels`     | Terminal hook bridge : événements hook (build done, test fail, sub-agent complete) → Slack/Telegram/Matrix | ⏳ T-INT-channels |
| `google_mcp`           | Default MCP server registered dans le pane MCP status (Google APIs surface)               | ⏳ T-INT-google-mcp |
| `gui`                  | **Out-of-scope terminal** — desktop Wry+Tao standalone, vit hors aphrody-terminal         | N/A (standalone) |
| `cli`                  | Hôte du subcommand `aphrody term` + autres                                                | ✅                                                |
| `aphrody-terminal-vt`  | VT parser, foundation                                                                     | ✅                                                |
| `aphrody-terminal-wasm`| WASM renderer, foundation                                                                 | ✅                                                |
| `aphrody-terminal-backend` | pty + WS server, foundation                                                           | ✅                                                |
| `aphrody-terminal-llm` | event bus + registries (en vol)                                                           | ⏳ in-flight                                      |
| `aphrody-terminal-browser` | bxc/agent-browser/edge bridge (en vol)                                                | ⏳ in-flight                                      |

## Crates "showcase" wiring (ordre de leverage)

L'objectif est d'ordonner les ticks d'intégration par démo-value-per-effort.

1. **T-INT-a2a-ui** : embed `a2a-ui` comme pane "Coord channel". Visible immédiat, leverage un crate WASM existant.
2. **T-INT-chrome** : `shadcn-bridge` pour le header/tabs/command palette. Polish visible.
3. **T-INT-mcp + T-INT-google-mcp** : pane MCP status alimentée par `aphrody-mcp` (OAuth) + `google_mcp` (server) — un seul tick, deux crates wired.
4. **T-INT-gemini** : "spawn Gemini CLI" tab. Démo concrète "voici ton agent dans ton terminal aphrody".
5. **T-INT-a2a + T-INT-mrx** : workspace activity pane via `mrx-watch` + A2A bridge (4 crates wired ensemble).
6. **T-INT-memory** : session semantic recall — démo "tape Ctrl+R, fuzzy-find ta commande d'il y a 3 semaines".
7. **T-INT-gateway + T-INT-voice + T-INT-channels** : tick "notifications + AI routing" (3 crates).
8. **T-INT-i18n + T-INT-summary + T-INT-wasm** : tick "polish" (3 crates).
9. **T-INT-agui** : AG-UI protocol compat (intéropérabilité écosystème).
10. **T-INT-backend** : process inspection + network fallback (peu visible mais robuste).
11. **T-INT-ievr** : low prio, optionnel (game RE workflow).

## Règles de wiring

- **Pas de stub** : chaque intégration ship réel code production, pas de
  `unimplemented!()` derrière un feature flag.
- **Chaque pane est optionnelle** via `terminal.json` `llm.<pane>: bool`.
  Default = on pour les 4 piliers (sub-agent / mcp / hook / skill), off pour
  les compléments (voice / channels / memory).
- **Toute intégration nouvelle crate** ajoute une row dans cette matrice ET
  un tick `T-INT-<slug>` dans `docs/PLAN.md` Phase T.
- **Test d'intégration obligatoire** : chaque T-INT-* crate doit avoir au
  moins un test qui prouve que la pane reçoit/affiche les events du crate.
  Ex. `cargo test -p aphrody-terminal-llm a2a_envelope_published_on_event`.

## Justification (pourquoi cet effort)

Aujourd'hui le workspace a 28+ crates dont une bonne moitié n'a qu'un
consommateur faible (un `README.md`, un test isolé). Pour le moonshot 100k
stars : un repo qui présente 28 crates dont 18 sont "academic exercises"
perd. Un repo qui présente 28 crates dont **chaque crate alimente une
feature visible du binaire showcase** gagne.

aphrody-terminal devient l'**intégration test live** du workspace entier.
Si on casse un crate, on casse une pane visible → CI le voit, l'utilisateur
le voit. Chaque ticket "ajoute une feature à aphrody-terminal" pousse de
facto sur 1-3 crates.

## Trade-offs assumés

- **Surface API agrandie** : l'utilisateur Linux qui veut "juste un terminal"
  peut désactiver toutes les panes via `terminal.json` `llm.*: false`. Le
  binaire reste lean (lazy-load des panes par feature flag).
- **Maintenance crates × consumers** : chaque crate qui bouge force un test
  côté aphrody-terminal. C'est le but — feedback loop court, pas de drift
  silencieux.
- **`gui` reste standalone** : pas de wrapping forcé. Wry+Tao a un cycle de
  vie différent (desktop natif vs WASM). Mais `gui` peut devenir l'**hôte
  desktop d'aphrody-terminal-wasm** (charge le bundle WASM dans Wry) si on
  veut une app desktop tout-en-un.


<!-- ============================================== -->
<!-- SOURCE: docs/design/aphrody-terminal-spec.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody-terminal — LLM-first terminal specification

> **One-line positioning**: the terminal designed for sub-agents, skills,
> hooks, MCP servers and Ink/React TUIs — JSON output everywhere, markdown
> rendered inline, JSON-config full, WASM-native + M3-themed.

## What this terminal is NOT

- **Not a Windows Terminal clone.** Tabs/panes/profiles exist only because
  LLM tooling needs them (one pane per sub-agent stream, one pane per MCP
  server status, etc.) — not as a generic productivity feature.
- **Not a wterm port.** `vercel-labs/wterm` is the WASM-emulator API
  reference; we replace it with pure Rust to add the LLM-first surface.
- **Not a Warp-style "AI in your terminal".** The terminal is *for* LLMs
  running underneath (Claude Code, Gemini CLI, codex, etc.) and *for*
  humans collaborating with them — not a vertical AI chatbox.

## What this terminal IS

A WASM-native, M3-themed terminal whose every design decision answers one
question: **does this make life easier for the LLM running inside it?**

### Five pillars

1. **JSON output on every channel.** Every command exposes `--json`. The
   terminal frames non-JSON output into JSON envelopes too (stdout/stderr
   chunks, exit codes, timing, environment). Sub-agents can consume the
   terminal's session log without re-parsing ANSI.
2. **Markdown rendered inline.** When the underlying program emits
   markdown (`# Heading`, ` ```rust `, `- list`), the WASM renderer
   detects and renders it natively: headings, fenced code blocks with
   syntax highlight via `syntect`, lists, links, images. Toggleable via
   `aphrody-md` ANSI extension OSC sequence.
3. **JSON config full.** No YAML, no TOML for terminal config. One
   `~/.aphrody/terminal.json` with strict schema. Compatible with the
   patterns of `claude.json`, `settings.json`, `mcp.json`, `.gemini/`,
   so an LLM that knows one knows them all.
4. **Sub-agent + MCP + hooks + skills as first-class concepts.** The
   terminal exposes panes/regions for:
   - Live sub-agent task tree (one row per task, status + last log)
   - MCP server status bus (one row per server, last RPC + state)
   - Hook firing log (one row per hook event)
   - Active skill surface (one row per loaded skill, last invocation)
5. **Ink/React TUI compatibility.** Claude Code and Gemini CLI both
   render via Ink (React TUI). The VT must support: alternate screen
   buffer (`\e[?1049h`), cursor save/restore (`\e[s/u`), bracketed paste
   (`\e[?2004h`), mouse SGR (`\e[?1000h..1006h`), focus in/out
   (`\e[?1004h`), 24-bit true color SGR (`\e[38;2;r;g;b`,
   `\e[48;2;r;g;b`), 256-color (`\e[38;5;n`), DECSTBM scroll regions
   (`\e[1;24r`), insert/delete line (`\e[L`/`\e[M`), erase character
   (`\e[X`). Without these, Ink renders garbled.

## Crate stack

```
aphrody-terminal-vt          (no_std, pure Rust)
  └─ vte parser + ScreenBuffer + ALL Ink-essential CSI/SGR/DCS
aphrody-terminal-wasm        (wasm32-unknown-unknown)
  └─ DOM renderer + M3 colors + keyboard + mouse + bracketed paste
     + markdown overlay layer + JSON inspect panel
aphrody-terminal-backend     (native)
  └─ portable-pty (ConPTY/openpty) + tokio-tungstenite WS server
     + JSON resize/data protocol
aphrody-terminal-llm         (native + wasm)
  └─ Sub-agent stream multiplexer
  └─ MCP server status event bus (poll mcp.json servers, surface state)
  └─ Hook event surface (subscribe to hook firings, render)
  └─ Skill activation slot (loaded skill registry, last invocation)
aphrody-terminal-markdown    (no_std capable)
  └─ comrak CommonMark + syntect highlighter
  └─ OSC sequence detector: `\e]aphrody-md;...\a` enters markdown mode
aphrody-terminal-json-out    (no_std)
  └─ Frame stdout/stderr chunks into JSONL envelopes
  └─ Detect application-emitted JSON and pass through unmodified
aphrody-terminal-config      (native)
  └─ ~/.aphrody/terminal.json strict schema (serde + jsonschema)
  └─ Compat shims: import from settings.json, claude.json, mcp.json
aphrody-terminal-browser     (native + wasm)
  └─ Bridge: terminal LLM event bus <-> bxc (in-process) / agent-browser (RPC)
  └─ Native LLM <-> DOM automation: nav, eval JS, query selectors, screenshot,
     extract structured data, intercept requests, replay sessions
  └─ Surfaces a browser pane in the terminal (mini-viewport + DOM tree + console)
```

## JSON config schema (v1, normative)

```jsonc
{
  "$schema": "https://aphrody.dev/schemas/terminal/v1.json",
  "version": 1,
  "appearance": {
    "theme": "m3-dark-tonal",          // m3-{dark,light}-{tonal,vibrant,expressive}
    "scheme_seed": "#1A73E8",          // generates full M3 palette
    "font_family": "google-sans-flex",
    "font_size_px": 14,
    "line_height": 1.4,
    "cursor": "block-blink"            // block|underline|bar × blink|steady
  },
  "shell": {
    "default": "$SHELL",               // resolved at runtime
    "argv": ["-l"],
    "env": { "TERM": "aphrody-256color" }
  },
  "llm": {
    "sub_agent_pane": true,
    "mcp_status_pane": true,
    "hook_event_pane": true,
    "skill_pane": true,
    "json_output_default": true,
    "markdown_inline": true,
    "markdown_code_theme": "github-dark"
  },
  "integrations": {
    "claude_code": { "settings_path": "~/.claude/settings.json" },
    "gemini_cli":  { "config_path": "~/.gemini/" },
    "mcp":         { "config_path": "~/.aphrody/mcp.json" }
  },
  "keybindings": [
    { "id": "command-palette",   "binding": "ctrl+shift+p" },
    { "id": "toggle-sub-agents", "binding": "ctrl+shift+a" },
    { "id": "toggle-mcp",        "binding": "ctrl+shift+m" },
    { "id": "toggle-hooks",      "binding": "ctrl+shift+h" },
    { "id": "toggle-skills",     "binding": "ctrl+shift+s" },
    { "id": "toggle-markdown",   "binding": "ctrl+shift+d" },
    { "id": "json-export-session", "binding": "ctrl+shift+j" }
  ]
}
```

## Ink/React TUI compatibility checklist

These must work for Claude Code + Gemini CLI to render correctly. They
form the `aphrody-terminal-vt` acceptance criteria.

| Sequence | Name | Mandatory |
|---|---|---|
| `\e[?1049h/l`            | Alternate screen buffer enter/leave              | yes |
| `\e[?25h/l`              | Show/hide cursor                                  | yes |
| `\e[?2004h/l`            | Bracketed paste mode                              | yes |
| `\e[?1000;1002;1003;1006h/l` | Mouse reporting (any-event + SGR)            | yes |
| `\e[?1004h/l`            | Focus in/out events                               | yes |
| `\e[s` / `\e[u`          | Cursor save / restore (SCO)                       | yes |
| `\e7` / `\e8`            | Cursor save / restore (DEC)                       | yes |
| `\e[<top>;<bot>r`        | DECSTBM scroll region                             | yes |
| `\e[<n>S` / `\e[<n>T`    | Scroll up / down                                  | yes |
| `\e[<n>L` / `\e[<n>M`    | Insert / delete line                              | yes |
| `\e[<n>@` / `\e[<n>P`    | Insert / delete character                         | yes |
| `\e[<n>X`                | Erase character                                   | yes |
| `\e[<r>;<c>H`            | Cursor position (CUP)                             | yes |
| `\e[<n>A..D`             | Cursor up/down/right/left                         | yes |
| `\e[<n>G` / `\e[<n>d`    | Horizontal / vertical position                    | yes |
| `\e[<n>m` SGR full       | Bold/italic/underline/inverse/strike/dim          | yes |
| `\e[38;2;r;g;b m`        | 24-bit RGB foreground                             | yes |
| `\e[48;2;r;g;b m`        | 24-bit RGB background                             | yes |
| `\e[38;5;n m`            | 256-color indexed foreground                      | yes |
| `\e[48;5;n m`            | 256-color indexed background                      | yes |
| `\e[<n>J` / `\e[<n>K`    | Erase display / line                              | yes |
| `\e]0;TITLE\a`           | OSC 0 set title                                   | yes |
| `\e]52;c;BASE64\a`       | OSC 52 clipboard read/write                       | yes |
| `\eP` ... `\e\`          | DCS string passthrough (sixel/kitty optional)     | optional |

## LLM-extension ANSI sequences (aphrody-specific)

We reserve a single OSC namespace prefix `aphrody-*` for LLM-aware
extensions. All optional, all detected, all gracefully ignored by
non-aphrody terminals.

| Sequence | Meaning |
|---|---|
| `\e]aphrody-md;<base64-markdown>\a`                  | Render markdown block inline |
| `\e]aphrody-json;<base64-json>\a`                    | Surface JSON in inspect panel |
| `\e]aphrody-sub-agent;<id>;<status>;<text>\a`        | Sub-agent status update |
| `\e]aphrody-mcp;<server>;<state>;<rpc>\a`            | MCP server activity |
| `\e]aphrody-hook;<event>;<payload>\a`                | Hook firing log entry |
| `\e]aphrody-skill;<name>;<phase>;<payload>\a`        | Skill invocation log |
| `\e]aphrody-task;<id>;<status>;<subject>\a`          | Task tree update |
| `\e]aphrody-browser-nav;<url>\a`                     | Navigate active browser to URL |
| `\e]aphrody-browser-eval;<base64-js>\a`              | Eval JS in browser, response via JSON pane |
| `\e]aphrody-browser-dom;<base64-selector>\a`         | Query DOM (CSS selector), surface result tree |
| `\e]aphrody-browser-screenshot;<area>\a`             | Capture viewport / element / full-page, render inline |
| `\e]aphrody-browser-intercept;<base64-rule>\a`       | Install request interception rule |
| `\e]aphrody-browser-extract;<base64-schema>\a`       | Structured extraction (schema-driven, returns JSON) |
| `\e]aphrody-browser-record;<id>;<state>\a`           | Start/stop session recording for replay |
| `\e]aphrody-jsx-mount;<id>;<base64-tree>\a`          | Layer B: initial React VDOM mount (Yoga-laid-out JSON tree) |
| `\e]aphrody-jsx-update;<id>;<base64-patch>\a`        | Layer B: VDOM diff patch (Yoga deltas + style changes) |
| `\e]aphrody-jsx-unmount;<id>\a`                      | Layer B: VDOM teardown |
| `\e]aphrody-jsx-input;<id>;<base64-event>\a`         | Layer B: useInput keyboard event injection |
| `\e]aphrody-jsx-window-size;<cols>;<rows>\a`         | Layer B: useWindowSize push |
| `\e]aphrody-jsx-focus;<id>;<true|false>\a`           | Layer B: useFocus state push |

## Architectural invariants

1. **No JS in the core path.** TS only allowed in `packages/` for non-core
   helpers; the core renderer pipeline is pure Rust + WASM.
2. **No `unsafe` outside FFI boundaries.** `#![deny(unsafe_code)]` on
   every crate except where `wasm-bindgen` requires it.
3. **JSON config is the only config.** No YAML, no TOML, no INI for the
   terminal user-facing config.
4. **Apache-2.0 SPDX header line 1** of every file.
5. **Linux is target #1.** If a feature can't ship on Linux, it doesn't
   ship.
6. **No emoji in source or docs.** (CLAUDE.md §6 invariant.)

## Browser automation extensions (LLM ↔ DOM, native)

The terminal exposes a **browser pane** driven by two pluggable backends:

| Backend | Transport | Mode | Use case |
|---|---|---|---|
| `bxc` (aphrody-code/bxc @ aphrody) | In-process via `crates/bxc-runtime` | Lightpanda (Linux/Mac) or curl-impersonate (HTTP) | Fast scrape, static + light JS, no GPU needed |
| `agent-browser` (vercel-labs) | RPC (stdio JSON-RPC) | Full Chromium via CDP | Real SPA, WebGPU, video, complex auth flows |
| `edge` (built-in Win fallback) | spawn msedge `--headless=new --dump-dom` | DOM snapshot only | When neither above is installed |

**Selection policy** — `terminal.json` `llm.browser.preferred` chooses
default. The terminal probes availability at startup and surfaces the
chosen backend in the browser pane header. Sub-agents emit
`\e]aphrody-browser-*\a` sequences; the LLM bridge dispatches to the
active backend.

**Native LLM-DOM round-trip** (sub-second on bxc, < 3 s on agent-browser):

```
LLM sub-agent          aphrody-terminal-llm        aphrody-terminal-browser
     │                          │                              │
     │ "extract pricing table"  │                              │
     ├─────────────────────────►│                              │
     │                          │ \e]aphrody-browser-extract;  │
     │                          │  <schema-base64>\a           │
     │                          ├──────────────────────────────►│
     │                          │                              │ bxc.fetch(url)
     │                          │                              │ + schema-driven
     │                          │                              │   extraction
     │                          │     {rows: [...], meta:{...}}│
     │                          │◄──────────────────────────────│
     │       JSON envelope      │                              │
     │◄─────────────────────────│                              │
```

## Ink / React-TUI fusion strategy (3-layer)

`vadimdemedes/ink` est l'écosystème React TUI dominant : react-reconciler +
Yoga flexbox + ANSI stdout. **gemini-cli + Claude Code l'utilisent
massivement** (App.tsx, AppContainer, render.tsx, examples). Officiellement
Node only, mais sans bindings natifs critiques — il marche dans Bun.

Trois angles complémentaires :

### Layer A — Compat (must-have, tick T-2)

aphrody-terminal-vt parse l'ANSI émis par Ink. Tant que le VT couvre les 22
séquences Ink-essentials (table ci-dessus), **toute app Ink run inside
aphrody-terminal sans modification**. C'est la garantie de compatibilité —
gemini-cli, Claude Code, n'importe quel `useInput()` Ink fonctionne.

**Status** : T-2 tick — extension VT à shipper en priorité.

### Layer B — Bun-JSX → aphrody-OSC bridge (différenciateur, new tick T-9)

`packages/aphrody-jsx` — un **custom react-reconciler** alternatif à Ink,
écrit en TS Bun-natif (pas de babel, JSX direct), qui émet des séquences
**`aphrody-jsx-*` OSC** au lieu d'ANSI brut. aphrody-terminal reçoit l'OSC
et rend natively via `taffy` (Yoga-équivalent Rust) + M3 tokens.

Avantages :
- **Bun JSX natif** : pas de babel/swc step, démarrage 10x plus rapide
  qu'Ink+Node.
- **React DX préservée** : `<Box flexDirection="column"><Text bold>Hi</Text></Box>`
  identique à Ink — courbe d'apprentissage zéro pour les devs Ink.
- **Skip Ink's node-binding**: pas de `react-reconciler` npm transitif,
  pas de Yoga WASM côté JS, pas de native modules — portable n'importe où
  Bun tourne.
- **M3 styling natif** : `<Text color="primary">` mappe vers
  `m3_tokens::dynamic::primary()` directement dans le renderer Rust, pas
  de translation ANSI lossy.
- **WASM rendering possible** : le même fichier `.tsx` rend dans
  aphrody-terminal-wasm (browser) ET dans aphrody-terminal-vt (native pty).
  Un seul source-of-truth, deux render targets.

API surface cible (mimics Ink) :

```tsx
import { render, Box, Text, useInput, useApp } from "@aphrody/jsx";

function App() {
  const { exit } = useApp();
  useInput((input) => { if (input === "q") exit(); });
  return (
    <Box flexDirection="column">
      <Text bold color="primary">Hello from aphrody-jsx</Text>
      <Text dimColor>Press q to exit</Text>
    </Box>
  );
}
render(<App />, { target: "aphrody-terminal" });
```

Le reconciler :
1. Reçoit le VDOM React via `react-reconciler` (peer dep, partagé avec Ink).
2. Computes layout via `taffy` côté Rust (FFI Bun→aphrody-terminal-llm).
3. Émet `\e]aphrody-jsx-mount;<id>;<base64-json-tree>\a` à chaque commit React.
4. aphrody-terminal-{vt,wasm} reçoivent l'OSC, hydrate le DOM virtuel,
   render via M3 + dirty-region updates.

**Status** : tick T-9 (new, à scaffolder).

### Layer C — Pure Rust DSL (canonical long-term, new tick T-10)

`crates/aphrody-tui` — un crate Rust ratatui-style avec Builder DSL ou
proc-macro JSX-like syntax. Pas de JS. Canonical aphrody path per
CLAUDE.md §2 ("WASM Rust natif pour TOUT nouveau projet web").

Use cases : TUIs aphrody-internes performance-critiques (le browser pane
60fps, le sub-agent task tree avec 1000+ rows live).

**Status** : tick T-10 (long-term, optional initially).

### Decision matrix

| Goal | Path |
|---|---|
| "Mon app Ink doit tourner inside aphrody-terminal" | A (T-2) |
| "Je veux écrire une nouvelle TUI en TS Bun + JSX sans Ink" | B (T-9) |
| "Je veux le max de perf + zéro JS, en Rust pur" | C (T-10) |

## Reference upstreams (read-only)

- `C:/worktree/wterm/` — Apache-2.0, TS+Zig WASM. API surface reference.
- `C:/worktree/terminal/` — MIT, C++. Buffer/Renderer/AtlasEngine/
  ConPTY/profiles.schema.json algorithmic reference.
- `C:/worktree/gemini-cli/` — Ink + React TUI. Compatibility test target.
- `C:/worktree/bxc/` — bxc in-process browser. Primary LLM-DOM backend.
- `C:/worktree/agent-browser/` — vercel-labs full Chromium. Heavy-SPA backend.
- (Anticipated, not yet cloned) — Anthropic Claude Code Ink TUI. Same.

## Roadmap (tick-sized)

| Tick | Deliverable | Status |
|---|---|---|
| T-1  | Worktrees added, foundation 3 crates scaffolded | in-flight |
| T-2  | VT extended with full Ink/React essentials (table above) | queued |
| T-3  | `aphrody-terminal-llm` — sub-agent + MCP + hooks + skills surfaces | queued |
| T-4  | `aphrody-terminal-markdown` — comrak + syntect inline renderer | queued |
| T-5  | `aphrody-terminal-json-out` — JSONL session framing | queued |
| T-6  | `aphrody-terminal-config` — JSON schema + claude.json/mcp.json compat | queued |
| T-6b | `aphrody-terminal-browser` — bxc + agent-browser + edge fallback, OSC `aphrody-browser-*` | queued |
| T-7  | `aphrody term` CLI subcommand + WASM demo HTML | queued |
| T-8  | Demo gif: Claude Code running inside aphrody-terminal w/ live sub-agent pane + browser pane scraping a real site | queued |


<!-- ============================================== -->
<!-- SOURCE: docs/migrations/01-gemini-ui-to-wasm.md -->
<!-- ============================================== -->

# Migration 01 : `packages/gemini` vers `crates/aphrody-wgpu-material`

**Priorité :** 1 (Critique)
**Statut :** Planifié
**Cible :** `wasm32-unknown-unknown` + WebGPU

## 1. État des Lieux (TS/Next.js)
Le package `packages/gemini` contient ~4 500 lignes de code React (TSX) reproduisant l'interface web Gemini au pixel près.
- **Routage :** App Router Next.js (`app/api/*`, `app/page.tsx`).
- **Composants :** 41 composants UI (`MessageBubble`, `VoiceWaveform`, `PromptBar`).
- **Core Logic :** Intégration Whisper, MCP, Gateway asynchrone (`core/`).
- **Assets :** Polices Google Sans Flex (~41k lignes binaire), M3 Tokens.

## 2. Problématique
Ce package contrevient à la politique 100% Rust (`CLAUDE.md §2`). L'exécution dépend de Node.js/Next.js et embarque un runtime JS lourd qui est incompatible avec la distribution via un binaire CLI natif cross-platform.

## 3. Plan de Migration Rust
Le portage s'appuiera sur `crates/aphrody-react-reconciler` (déjà implémenté) et `crates/aphrody-wgpu-material`.

### Étape A : Core & API (Backend)
- Migrer `core/auth.ts`, `core/mcp.ts` vers des handlers HTTP dans `crates/backend/src/routes/`.
- Migrer la logique Whisper (`core/whisper.ts`) vers `crates/aphrody-voice-stt`.
- Remplacer les routes API Next.js par des endpoints Axum ou un canal WebSocket direct.

### Étape B : Composants & Rendu (Frontend WASM)
- Transcrire les hooks `useChat.ts`, `useVoiceInput.ts` en structs Rust gérant leur state interne (via `yew` ou le reconciler maison `aphrody-react-reconciler`).
- Convertir la surcouche CSS complexe (`globals.css`) et les dégradés WebGPU (`lib/webgpu-gradient.ts`) en shaders WGSL natifs dans `crates/aphrody-wgpu-material`.
- Remplacer les primitives UI React (`PromptBar.tsx`, `MessageBubble.tsx`) par des composants Rust générant du DOM via `web-sys` et `wasm-bindgen`.

### Étape C : Intégration
- Compiler le client en `aphrody-terminal-wasm.wasm`.
- L'injecter via le daemon HTTP Rust.

## 4. Critères de Succès
- [ ] L'interface s'affiche dans un navigateur sans charger un seul script `.js` (hormis le glue code `wasm-bindgen`).
- [ ] La communication se fait en WebSocket vers le binaire Rust local, sans serveur Node.
- [ ] Suppression complète du dossier `packages/gemini/`.


<!-- ============================================== -->
<!-- SOURCE: docs/migrations/02-ui-shadcn-to-tui.md -->
<!-- ============================================== -->

# Migration 02 : `packages/ui` vers Terminal UI et Natif

**Priorité :** 2 (Modérée)
**Statut :** Suppression / Remplacement
**Cible :** `crates/aphrody-tui` / `crates/shadcn-bridge`

## 1. État des Lieux
Le package `packages/ui` contient 127 fichiers répartis entre des composants React Shadcn (`components/`, `src/components/`), des variables de design (`tokens/`), et d'énormes collections de sprites et d'assets PWA (`assets/`).
- Aucun consommateur actif dans l'application CLI principale.
- Code mort accumulé suite à des tests d'interface disparates.

## 2. Problématique
Ce dossier TS/CSS viole la directive "Rust Only". De plus, Shadcn est conçu pour le DOM web, alors que l'expérience Aphrody cible avant tout le Terminal (TUI) et des overlays natifs. Garder cette bibliothèque de composants React locale est redondant.

## 3. Plan de Migration Rust

### Étape A : Extraction des Design Tokens
- Convertir `tokens/colors.json`, `tokens/spacing.json`, `tokens/typography.json` vers un format chargé nativement par `crates/aphrody-tui/src/theme.rs`.
- Les tokens Material Design (M3) seront gérés par `crates/m3-tokens`.

### Étape B : Assets & Sprites
- Déplacer `assets/sprites/` et `assets/thumbnails/` vers le dossier racine `/assets/` qui est scanné lors du build `build.rs` du CLI pour l'inclusion dans le binaire ou le bundle d'installation.
- PWA manifest et icones web-specific : suppression pure et simple, Aphrody n'est plus distribué en PWA.

### Étape C : Composants UI
- Purger tous les fichiers `.tsx` et `.ts` (`button.tsx`, etc.).
- Le paradigme de composants sera remplacé par `ratatui` dans `crates/aphrody-tui` pour l'interface en ligne de commande.
- S'il faut une interface GUI riche desktop, elle s'appuiera sur `crates/gui` (TAO/WRY) et WebGPU.

## 4. Critères de Succès
- [ ] Les design tokens sont parsés dynamiquement par le code Rust.
- [ ] Le répertoire `packages/ui` est totalement supprimé.
- [ ] Le binaire CLI n'a perdu aucune fonctionnalité visuelle et arbore un TUI pixel-perfect via `ratatui`.


<!-- ============================================== -->
<!-- SOURCE: docs/migrations/03-nextjs-rust-extraction.md -->
<!-- ============================================== -->

# Migration 03 : Isolation des Crates Vercel (`packages/next.js`)

**Priorité :** 3 (Basse - Maintenance)
**Statut :** Partiellement Fait (JS ignoré)
**Cible :** `[workspace.dependencies]`

## 1. État des Lieux
L'énorme dossier `packages/next.js` (~336 000 lignes) est un sous-module ou fork du projet amont Vercel. Il contient à la fois l'implémentation JS historique de Next.js (`packages/`, `apps/`, `bench/`) et le nouvel outillage Rust Turbopack (`turbopack-*`, `next-core`, etc.).

## 2. Problématique
Aphrody n'a aucune intention de devenir ou de forker Next.js au sens Node.js du terme. L'objectif unique de ce dossier est d'extraire les puissantes bibliothèques de compilation Rust (`swc`, `turbopack`, `lightningcss`, `oxc`) pour le tooling de build interne (ex: transpiler du JSX en Rust, optimiser du CSS natif). Le volume de JS pollue le monorepo et viole symboliquement la charte 100% Rust.

## 3. Plan de Migration Rust

### Étape A : Filtrage Git (Git Sparse Checkout / Submodule)
- Plutôt que d'héberger le code JS mort, configurer un Sparse Checkout ou récupérer les crates Rust via des `git = "https://github.com/vercel/next.js"` dans le `Cargo.toml` racine.
- Alternative : Extraire manuellement les dossiers `crates/` de `packages/next.js` et les placer directement sous `crates/vercel-tools/` pour couper le lien TS/JS amont.

### Étape B : Suppression du Bruit
- Supprimer les dossiers `apps/`, `bench/`, `test/`, `packages/` internes au dossier `next.js` qui contiennent du code JS.
- Supprimer le fichier `bun.lock` (qui tente de résoudre l'arbre Node de Next.js) et les multiples `package.json` imbriqués.

### Étape C : Finalisation Cargo
- Assurer que `Cargo.toml` (`[workspace] members` ou `workspace.dependencies`) pointent exclusivement vers les crates Turbopack purs.
- Éradiquer `packages/next.js` et libérer 300 Mo de la base de code git locale.

## 4. Critères de Succès
- [ ] Le code JS de Vercel/Next.js n'existe plus dans le repository local Aphrody.
- [ ] La compilation `cargo build -p aphrody` réussit toujours et parvient à linker `turbopack`.
- [ ] Le répertoire racine `packages/` peut être définitivement supprimé.



<!-- ============================================== -->
<!-- SOURCE: docs/WASM/bun-native-wasm.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->
# Bun — Native WASM Loading

Source : Bun 1.3+ runtime docs, verified 2026-05-17.

Bun is the **fastest runtime** in the aphrody stack to load WASM, mostly because there is no bundler / loader configuration to fight. WASM is a first-class import.

## Direct `.wasm` import

```ts
// src/api/handler.ts
import { add, Counter } from './engine.wasm'

console.log(add(2, 3))
const c = new Counter()
c.increment()
```

Bun parses the import, instantiates the module synchronously at module load, and re-exports its declared symbols. No `init()` boilerplate.

This works because Bun ships a built-in **wasm loader** that wraps `WebAssembly.instantiate` and uses the wasm-bindgen JS shim if it's adjacent (`engine.js` next to `engine.wasm`).

## `wasm-pack` output integration

```bash
wasm-pack build --target bundler --out-dir ./pkg crates/my-crate
```

Then in Bun :

```ts
import init, { Counter, transform } from './pkg/my_crate.js'

await init()                       // wasm-bindgen target=bundler still needs init()
const c = new Counter()
console.log(transform("hi"))
```

For maximum speed, prefer `--target nodejs` when shipping to a Bun runtime — it avoids the bundler glue :

```bash
wasm-pack build --target nodejs --out-dir ./pkg crates/my-crate
```

```ts
const { Counter, transform } = require('./pkg/my_crate')
```

## `bunfig.toml` — preload-style optimization

If a WASM module is on the hot path, pre-instantiate at server boot :

```toml
# bunfig.toml
preload = ["./src/preload-wasm.ts"]
```

```ts
// src/preload-wasm.ts
import './heavy.wasm'                // instantiated once, cached for the rest of the process
```

## Async `WebAssembly.instantiate` — when you need control

For arbitrary `.wasm` files (not `wasm-bindgen` output) :

```ts
const wasmBytes = await Bun.file('./module.wasm').arrayBuffer()
const { instance } = await WebAssembly.instantiate(wasmBytes, {
  env: {
    // import object — fill with the imports declared in the WASM module
    abort: () => { throw new Error('wasm abort') },
    log: (ptr: number, len: number) => {
      const mem = new Uint8Array((instance.exports as any).memory.buffer, ptr, len)
      console.log(new TextDecoder().decode(mem))
    },
  },
})

const exports = instance.exports as Record<string, Function>
console.log(exports.compute(42))
```

## Streaming compile (large modules)

```ts
const response = await fetch('https://cdn.example/heavy.wasm')
const { instance } = await WebAssembly.instantiateStreaming(response, importObject)
```

Bun supports `instantiateStreaming` since 1.0 — use it for anything > 1 MB.

## SharedArrayBuffer — threading

```toml
# bunfig.toml
[serve.static]
"Cross-Origin-Opener-Policy" = "same-origin"
"Cross-Origin-Embedder-Policy" = "require-corp"
```

These headers unlock `SharedArrayBuffer`, which is what `wasm-bindgen-rayon` needs for browser parallelism. On the Bun server side, threads work through `worker_threads` (Node-compat).

## Build-time pipeline (cross-platform)

Build once on each platform of the matrix, ship from one place :

```jsonc
// package.json
{
  "scripts": {
    "wasm:build:linux": "cargo build --release --target wasm32-unknown-unknown --manifest-path crates/my-crate/Cargo.toml",
    "wasm:bindgen":     "wasm-bindgen --target bundler --out-dir pkg target/wasm32-unknown-unknown/release/my_crate.wasm",
    "wasm:opt":         "wasm-opt -Oz pkg/my_crate_bg.wasm -o pkg/my_crate_bg.wasm",
    "build":            "bun run wasm:build:linux && bun run wasm:bindgen && bun run wasm:opt && tsc"
  }
}
```

`wasm-opt -Oz` after `wasm-bindgen` typically halves the bundle size.

## Comparison vs Node / Deno

| Runtime | `.wasm` import | Loader needed | wasm-bindgen target |
|---------|----------------|---------------|---------------------|
| Bun 1.3+ | ✓ Native | None | `nodejs` or `bundler` |
| Node 22+ | ✗ Need loader hook or `--experimental-wasm-modules` | Yes | `nodejs` |
| Deno 1.40+ | ✓ Native via `import` | None | `bundler` |

Per the org policy (`feedback-bun-only` memory), Bun is the only allowed runtime — Node is banned. WASM workflow is therefore Bun-first.

## Profiling Bun + WASM

```bash
bun --inspect server.ts
```

Hits the Chrome DevTools inspector. WASM frames show up in the perf timeline with Rust function names if you built with `wasm-bindgen --debug`.


<!-- ============================================== -->
<!-- SOURCE: docs/WASM/nextjs-integration.md -->
<!-- ============================================== -->

<!-- SPDX-License-Identifier: Apache-2.0 -->
# Next.js 16 + WASM

Source : `vercel/next.js` 16.2+ official docs, verified 2026-05-17.

## Current state of WASM support in Next.js 16 (verified 2026-05-17)

| Bundler | WASM bundling | WASM in Web Workers | Edge runtime WASM |
|---------|---------------|---------------------|-------------------|
| **Webpack 5** (Next 16 with `--webpack`) | ✅ Full `asyncWebAssembly` experiment, `import './x.wasm'` works | ✅ | ✅ |
| **Turbopack** (Next 16 default) | ❌ `.wasm` import not resolved ; `new URL("x.wasm", import.meta.url)` fails ; tracked in vercel/next.js#84972 + discussion#75430 | ✅ since 16.2 (Web Worker Origin relaxed) | ⚠️ partial |

What Next.js 16.2 (2026-03) did fix on the Turbopack side :
- Web Worker Origin restriction relaxed → `crypto-wasm`, `@tensorflow/tfjs-backend-wasm`
  and similar libs now run inside Web Workers without extra config.
- This unblocks the *runtime* execution of WASM imported through other means
  (CDN, `fetch` + `WebAssembly.instantiateStreaming`), but **not** the
  bundler-resolved `import './x.wasm'` syntax.

Until Turbopack ships full WASM resolution :
- Apps with a `wasm-pack` output to bundle → opt out of Turbopack (`--webpack`).
- Apps that just call `WebAssembly.instantiateStreaming(fetch('/static/x.wasm'))` at runtime → Turbopack is fine.
- Apps with no WASM at all → keep Turbopack (the dev-startup gains are real, ~400 % faster on 16.2).

## Opt-out of Turbopack — `package.json`

```json
{
  "scripts": {
    "dev": "next dev --webpack",
    "build": "next build --webpack",
    "start": "next start"
  }
}
```

## Webpack config — enable WASM async

`next.config.ts` :

```ts
import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
  webpack: (config, { isServer }) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      layers: true,
    }

    // Required for top-level await in dynamic wasm imports
    config.output = { ...config.output, webassemblyModuleFilename: 'static/wasm/[hash].wasm' }

    return config
  },
}

export default nextConfig
```

## Importing a wasm-bindgen output in a React component

After `wasm-pack build --target bundler` produced `pkg/my_crate.js` and `pkg/my_crate_bg.wasm` :

```tsx
'use client'

import { useEffect, useRef, useState } from 'react'

export default function CounterClient() {
  const [counter, setCounter] = useState<unknown>(null)
  const [value, setValue] = useState(0)

  useEffect(() => {
    let mounted = true
    ;(async () => {
      const wasm = await import('@aphrody-code/my-crate-pkg')   // points to pkg/
      await wasm.default()                                       // bootstrap (target=bundler)
      if (!mounted) return
      const c = new wasm.Counter()
      setCounter(c)
    })()
    return () => { mounted = false }
  }, [])

  if (!counter) return <div>loading…</div>

  return (
    <button onClick={() => setValue((counter as any).increment())}>
      count: {value}
    </button>
  )
}
```

Notes :
- The dynamic `import()` keeps the WASM out of the initial bundle.
- `'use client'` is required — WASM that uses DOM / Canvas / WebGPU can't run in RSC.
- Pin the pkg/ directory as a workspace package, **don't** commit `target/`.

## Server Components with WASM

WASM **can** run in RSC if it's pure compute (no DOM). The Node.js runtime resolves `.wasm` via Webpack's `asyncWebAssembly`. Use `wasm-pack build --target nodejs` for the pkg consumed server-side. The edge runtime supports a smaller subset — verify your wasm-bindgen output doesn't use any Node-only APIs (it shouldn't by default).

## Server Actions + WASM

```ts
'use server'

import { transform } from '@aphrody-code/my-server-wasm-pkg'

export async function processAction(input: string): Promise<string> {
  return transform(input)
}
```

Behavior :
- The `'use server'` module resolves through Next.js's **server-only webpack layer** — WASM imported here stays server-side.
- Caching is fine — the WASM instance is reused across invocations on the same Node process.

## Edge runtime caveats

`export const runtime = 'edge'` constrains what you can ship :
- No filesystem, no Node-specific APIs.
- WASM works if it's `wasm32-unknown-unknown` (not `wasm32-wasi`).
- Bundle limit is **1 MB after gzip** on Vercel Edge. WASM bundles tend to bust this — measure first with `next build` output.

## Bundle analysis

```bash
ANALYZE=true next build --webpack
```

Add `@next/bundle-analyzer` (already in the workspace catalog of `aphrody-code/vps`). Watch the static/wasm chunk — > 500 KB is usually a sign you forgot `wasm-opt -Oz` or pulled in too many `web-sys` features.

## Turbopack roadmap note

Turbopack's WASM support is tracked upstream. When parity lands, the dev script can drop `--webpack`. Monitor [vercel/next.js Turbopack docs](https://github.com/vercel/next.js/blob/canary/docs/01-app/03-api-reference/08-turbopack.mdx) for the green check on async WASM.

