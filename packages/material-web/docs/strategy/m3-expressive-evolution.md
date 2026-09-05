<!-- SPDX-License-Identifier: Apache-2.0 -->

# L'Évolution Material Design 3 (M3) Expressive (2026) : Rapport de Recherche & Mappage Technique

Ce rapport analyse en profondeur l'évolution de l'architecture **Material Design 3 (M3) Expressive** introduite par Google et son implémentation déclarative sous **Jetpack Compose** (Compose-first sur Android). Il établit les équivalences de parité technique, détaille les limites du Web, et propose une feuille de route d'implémentation pour notre monorepo Lit + React.

---

## 1. Synthèse Architecturale : Jetpack Compose vs. Web Monorepo

Sur Android, M3 Expressive consacre l'abandon définitif des vues XML au profit d'une approche **Compose-first**. L'ensemble de la thématisation est orchestré par le composable `MaterialExpressiveTheme` (ou par la configuration `motionScheme` de `MaterialTheme`). Ce changement se structure autour de quatre grands sous-systèmes :

1. **ColorScheme** : Couleurs dérivées de l'espace HCT via *Material You* (MCU).
2. **Typography** : Typographie variable dynamique gérée via `Typography`.
3. **Shapes** : Formes dynamiques prenant en charge le morphing.
4. **MotionScheme** : Nouveau sous-système de mouvement physique (ressorts/springs) remplaçant les courbes d'accélération statiques.

### Tableau de Parité des Concepts

| Concept Jetpack Compose (M3) | Équivalent Monorepo Web (`material-web` / `m3-react`) | Statut de Parité |
| :--- | :--- | :--- |
| `MaterialExpressiveTheme` | `M3ThemeProvider` (dans [react.tsx](file:///home/ubuntu/material-web/packages/m3-theme/src/react.tsx)) | **Partielle** : Gère les variables CSS claires/sombres et la génération dynamique depuis un seed, mais sans les ressorts physiques natifs CSS. |
| `ColorScheme` (MCU) | [dynamic-color.ts](file:///home/ubuntu/material-web/packages/m3-tokens/src/dynamic-color.ts) | **Complète** : Supporte les 7 variantes de schémas MCU dont `expressive`. |
| `Typography` | `<md-type>` (dans [md-type.ts](file:///home/ubuntu/material-web/packages/material-web/typography/internal/md-type.ts)) | **Complète** : Implémente le rendu avec axes variables `Google Sans Flex` et les styles. |
| `shapes` (échelle 10 niveaux) | `shape.rs` / CSS custom properties | **Partielle** : Échelle 7 étapes actuellement dans les tokens Web ; les 3 ajouts M3 Expressive (20dp, 32dp, 48dp) doivent être intégrés. |
| `motionScheme` (physique springs) | [packages/m3-motion/](file:///home/ubuntu/material-web/packages/m3-motion/) | **Partielle** : Interpolation physique sous React (via Framer Motion) mais retombée (fallback) sur des beziers CSS standard pour le Shadow DOM de Lit. |
| `State Layers` (+8% / +10% / +16%) | `<md-ripple>` ou classes de pseudo-état CSS | **Complète** : Géré via les couches d'état de l'élément de ripple natif. |
| `TextFieldState` | Custom React Hook `useTextFieldState` | **Proposition** : Hook de contrôle d'état atomique pour le wrapper React du champ textuel. |

---

## 2. Architecture Fondamentale et Thématisation Dynamique

### Jetpack Compose : Résolution à la compilation & runtime
En Kotlin Compose, la hiérarchie des jetons se divise en :
- **Reference tokens** (jetons de référence) : valeurs brutes de couleurs/tailles (ex: `Palette.primary40`).
- **System tokens** (jetons système) : rôles sémantiques contextuels (ex: `MaterialTheme.colorScheme.primary`).
- **Component tokens** (jetons de composant) : liaison finale au niveau du composant (ex: `ButtonDefaults.filledButtonColors(...)`).

Cette approche résout les tokens sous forme de variables en mémoire Compose lors de la construction de l'arbre sémantique, assurant un **zéro-runtime overhead** pour l'affichage final sans parsing de CSS.

### Web Mapping : Theming Dynamique
Dans notre monorepo, la thématisation dynamique s'effectue au runtime par injection de variables d'environnement CSS dans le DOM. Notre module [dynamic-color.ts](file:///home/ubuntu/material-web/packages/m3-tokens/src/dynamic-color.ts) enveloppe le compilateur de couleur de Google (`@material/material-color-utilities`) et dérive ~47 rôles CSS `--md-sys-color-*` à partir de n'importe quel code hexadécimal.

Le sous-système M3 Expressive introduit 7 variantes de génération chromatique que nous supportons via la propriété `SchemeVariant` :
1. `tonalSpot` : Schéma standard à faible chroma.
2. `content` : Fidélité maximale à la couleur d'origine.
3. `fidelity` : Reste fidèle à la source mais ajuste les accents.
4. `expressive` : Palette audacieuse et dynamique, détachée de la teinte source pour créer un contraste plus vif.
5. `vibrant` : Chroma maximum à toutes les étapes.
6. `neutral` : Presque monochrome (très faible chroma).
7. `monochrome` : Échelle de gris pure.

#### Exemple d'application dynamique Web
```typescript
import { applyDynamicColor } from "@aphrody/m3-tokens/dynamic-color";

// Application instantanée d'un schéma expressif issu d'un seed de couleur
applyDynamicColor("#9B72CB", {
  dark: true,
  contrastLevel: 0.5, // Niveau moyen (M3 Expressive)
  variant: "expressive"
});
```

---

## 3. Système de Mouvement, "State Layers" et Animations Physiques

### Les interactions fluides et "State Layers"
Pour garantir un retour visuel sans ralentissement (maintien des 120 fps sur écrans mobiles et XR), M3 spécifie des couches d'interaction semi-transparentes superposées au conteneur. Cela élimine la nécessité de recalculer ou de générer de nouvelles teintes de couleur lors d'un hover ou d'un focus.
Les opacités d'état sémantiques sont strictement définies comme suit :
*   **Hover (survol)** : `+8%` (`0.08` d'opacité de la couleur du texte/icône parent).
*   **Focus (clavier/accessibilité)** : `+10%` (`0.10` d'opacité).
*   **Pressed (pression tactique)** : `+10%` (`0.10` d'opacité).
*   **Dragged (glisser-déposer)** : `+16%` (`0.16` d'opacité).

### Implémentation Web
Dans notre catalogue Lit, cela correspond à l'application de variables CSS associées aux états interactifs :
```css
.indicator::before {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background: var(--md-sys-color-primary);
  opacity: 0;
  transition: opacity 150ms cubic-bezier(0.2, 0, 0, 1);
}
.target:hover .indicator::before { opacity: 0.08; }
.target:focus-visible .indicator::before { opacity: 0.10; }
.target:active .indicator::before { opacity: 0.10; }
```

### Limitations Web sur le mouvement Expressive
1. **Ressorts physiques (Springs)** : Compose utilise un moteur physique interne pour animer les layouts (amortissement, rigidité). CSS ne gère pas nativement les équations de ressort. Le contournement dans le package [m3-motion](file:///home/ubuntu/material-web/packages/m3-motion/) consiste à utiliser Framer Motion pour les wrappers React et à fournir des approximations avec des transitions `cubic-bezier(0.05, 0.7, 0.1, 1.0)` (courbe de décélération expressive) pour Lit.
2. **Morphing de forme (Shape Morph)** : Très développé sous Compose (conversion vectorielle fluide d'un bouton vers un FAB ou une carte), le morphing sur le Web se heurte à l'isolation du Shadow DOM et à la limitation des tracés SVG complexes. Nous utilisons à la place des transitions `clip-path` ou des animations de taille combinées à des arrondis dynamiques.

---

## 4. Composants "State-Based" et Saisie de Texte (TextFieldState)

Dans Jetpack Compose (1.4.0-alpha14+), le modèle traditionnel de saisie reposant sur des callbacks asynchrones (`value: String` associé à `onValueChange: (String) -> Unit`) provoquait des recompositions inutiles de tout l'arbre de composants à chaque frappe.

Pour y remédier, Google a introduit `TextFieldState`. C'est un conteneur d'état persistant qui encapsule :
1. Le texte brut sous forme de tampon modifiable (`TextFieldCharSequence`).
2. Les métadonnées de sélection (curseur, sélection active).
3. Les transactions atomiques via une méthode `.edit { ... }`.

### Transposition technique pour React (`m3-react`)
Pour répliquer ce paradigme orienté état sans subir de multiples rafraîchissements de composants React (avoiding re-renders) tout en conservant l'intégrité de la sélection et du curseur sur le Web component `<md-text-field>`, nous concevons le hook suivant :

```typescript
import { useState, useRef, useCallback, useEffect } from "react";

export interface TextFieldSelection {
  start: number;
  end: number;
}

/**
 * Conteneur d'état imitant le TextFieldState de Jetpack Compose pour le Web.
 * Gère le texte, la sélection et permet des modifications transactionnelles atomiques.
 */
export class WebTextFieldState {
  private _text: string;
  private _selection: TextFieldSelection;
  private _listeners: Set<() => void> = new Set();

  constructor(initialText = "", initialSelection: TextFieldSelection = { start: 0, end: 0 }) {
    this._text = initialText;
    this._selection = initialSelection;
  }

  get text() { return this._text; }
  get selection() { return this._selection; }

  /** Met à jour la sélection de manière atomique */
  setSelection(start: number, end: number) {
    this._selection = { start, end };
    this.notify();
  }

  /** Applique des modifications de texte transactionnelles */
  edit(block: (buffer: { text: string; selection: TextFieldSelection }) => void) {
    const buffer = { text: this._text, selection: { ...this._selection } };
    block(buffer);
    this._text = buffer.text;
    this._selection = buffer.selection;
    this.notify();
  }

  subscribe(listener: () => void) {
    this._listeners.add(listener);
    return () => this._listeners.delete(listener);
  }

  private notify() {
    this._listeners.forEach(l => l());
  }
}

/** Hook React recréant rememberTextFieldState() */
export function useTextFieldState(initialValue = ""): WebTextFieldState {
  const [state] = useState(() => new WebTextFieldState(initialValue));
  const [, forceUpdate] = useState({});

  useEffect(() => {
    return state.subscribe(() => {
      forceUpdate({});
    });
  }, [state]);

  return state;
}
```

Ce hook s'intègre avec nos wrappers React comme suit :
```tsx
import React, { useRef, useEffect } from "react";
import { MdOutlinedTextField } from "@aphrody/m3-react";
import { useTextFieldState } from "./useTextFieldState";

export function ExpressiveInput() {
  const state = useTextFieldState("Valeur initiale");
  const fieldRef = useRef<any>(null);

  // Synchronisation bidirectionnelle atomique avec le Shadow DOM Lit
  useEffect(() => {
    const el = fieldRef.current;
    if (!el) return;

    const handleInput = (e: Event) => {
      const target = e.target as HTMLInputElement;
      state.edit((buf) => {
        buf.text = target.value;
        buf.selection = {
          start: target.selectionStart ?? 0,
          end: target.selectionEnd ?? 0
        };
      });
    };

    el.addEventListener("input", handleInput);
    return () => el.removeEventListener("input", handleInput);
  }, [state]);

  return (
    <MdOutlinedTextField
      ref={fieldRef}
      value={state.text}
      label="Saisie Expressive"
    />
  );
}
```

---

## 5. Layouts Adaptatifs et Transitions de Navigation

L'évolution M3 Expressive redéfinit la transition entre les types d'écrans en s'appuyant sur les **Window Size Classes** :
*   **Compact** (largeur < 600dp) : Téléphones classiques.
*   **Medium** (largeur entre 600dp et 840dp) : Petites tablettes, écrans pliables fermés.
*   **Expanded** (largeur > 840dp) : Grandes tablettes, ordinateurs.

### Le nouveau paradigme de navigation latérale
En mode compact, la barre de navigation inférieure (`NavigationBar`, 3 à 5 destinations) reste la norme ergonomique. 
Cependant, pour les écrans Medium et Expanded, la navigation bascule sur un rail vertical. M3 Expressive modifie ce rail de la manière suivante :
1. **Élargissement** : Le rail passe d'une largeur classique de 80dp à **96dp** (dans [navigation-rail-styles.ts](file:///home/ubuntu/material-web/packages/material-web/navigationrail/internal/navigation-rail-styles.ts)).
2. **Cibles tactiles agrandies** : La hauteur minimale des éléments passe de 60dp à **64dp** pour une meilleure préhension ergonomique.
3. **Suppression du NavigationDrawer** : Le tiroir de navigation standard (`NavigationDrawer`) est remplacé par une version **expansible** du `NavigationRail` (d'une largeur d'au moins 220dp, gérée par l'attribut `expanded` de notre composant Lit). Cela offre une transition continue de layout : au lieu de faire apparaître un volet modal masquant le contenu, le rail de navigation s'élargit en repoussant ou en adaptant la grille de contenu.

Dans notre structure de mise en page, ces transitions sont orchestrées par `<md-scaffold>` (dans [md-scaffold.ts](file:///home/ubuntu/material-web/packages/material-web/layout/md-scaffold.ts)) qui calcule dynamiquement la largeur de l'écran ou écoute des conteneurs parents (via des requêtes de conteneur, *Container Queries*) pour basculer automatiquement entre barre inférieure, rail 96dp et rail étendu.

---

## 6. Typographie Variable (Google Sans Flex / Roboto Flex)

### Optimisation du bundle et flexibilité
Historiquement, charger un style typographique complet (Light, Regular, Medium, SemiBold, Bold, Italic) nécessitait le téléchargement de plusieurs fichiers de police (parfois plus de 1 Mo de bundle).
L'évolution M3 s'appuie désormais sur des **polices variables** (comme `Google Sans Flex` ou `Roboto Flex`). Un seul fichier de police compact (~150 ko) expose plusieurs axes modifiables dynamiquement par le moteur de rendu graphique.

Les axes de variations supportés par notre composant `<md-type>` (dans [md-type.ts](file:///home/ubuntu/material-web/packages/material-web/typography/internal/md-type.ts)) sont :
*   `wght` (Weight / Poids) : 100 à 900.
*   `opsz` (Optical Size / Taille optique) : 9 à 144. Ajuste la graisse et l'espacement selon la taille réelle d'affichage pour une lisibilité maximale.
*   `wdth` (Width / Largeur) : 75% à 125%.
*   `GRAD` (Grade / Contraste) : -200 à 150. Modifie l'épaisseur sans changer la largeur globale du glyphe (utile pour les états survolés/actifs).
*   `slnt` (Slant / Inclinaison) : -10° à 0°.
*   `ROND` (Roundness / Arrondi) : 0 à 100. Utilisé pour donner un aspect doux, caractéristique de l'identité visuelle de Gemini.

### Échelle des 15 styles Expressive
M3 Expressive dédouble l'échelle typographique en introduisant **15 styles emphasized (expressifs)** en plus des 15 styles baseline d'origine (totalisant 30 tokens). Ces styles se distinguent par un poids plus fort (`wght` supérieur) et des ajustements sur l'axe d'arrondi (`ROND`).

```typescript
// Extrait conceptuel des nouveaux rôles typographiques dans notre type-scale.ts
export const EXPRESSIVE_TYPE_SCALE = {
  "display-large-emphasized": {
    sizePx: 57,
    lineHeightPx: 64,
    trackingPx: -0.25,
    axes: { wght: 700, opsz: 96, grad: 50, rond: 80 } // Emphasized : wght 700, ROND 80
  },
  "headline-large-emphasized": {
    sizePx: 32,
    lineHeightPx: 40,
    trackingPx: 0,
    axes: { wght: 800, opsz: 36, rond: 60 }
  }
};
```

Sur le Web, les changements d'axes sont animés de manière fluide via des transitions CSS natives (car `font-variation-settings` est une propriété entièrement interpolable par les navigateurs modernes), offrant des effets de respiration textuelle de haute qualité lors des survols ou des changements d'états actifs.

---

## 7. Feuille de Route d'Implémentation pour le Monorepo

Pour intégrer pleinement les capacités de M3 Expressive au sein de notre monorepo, voici la feuille de route technique proposée :

### Phase 1 : Alignement des Formes et de la Typographie (Priorité Haute)
1.  **Mise à jour de l'échelle des formes (Shapes)** : Étendre les définitions de tokens dans notre sous-système de design pour passer de l'ancienne échelle à 7 niveaux vers l'échelle M3 Expressive à **10 niveaux** en ajoutant les tokens de rayon pour `large-increased (20dp)`, `extra-large-increased (32dp)` et `extra-extra-large (48dp)`.
2.  **Intégration des 15 styles Emphasized** : Compléter [type-scale.ts](file:///home/ubuntu/material-web/packages/material-web/typography/internal/type-scale.ts) pour exporter les 15 rôles `*-emphasized` associés et régénérer le fichier global de variables CSS de typographie.

### Phase 2 : Composants de Navigation et Saisie (Priorité Moyenne)
1.  **Élargissement du Navigation Rail** : Modifier [navigation-rail-styles.ts](file:///home/ubuntu/material-web/packages/material-web/navigationrail/internal/navigation-rail-styles.ts) pour passer la largeur par défaut (`--_width`) de `80px` à `96px` et la hauteur de l'item à `64px`. Mettre en place un modificateur ou une variable permettant l'expansion en volet latéral fluide à la place de l'ancien `NavigationDrawer`.
2.  **Création du package d'état `useTextFieldState`** : Créer le fichier utilitaire ou le hook `useTextFieldState` dans `@aphrody/m3-react` pour offrir l'API d'état atomique aux consommateurs React.

### Phase 3 : Mouvement Physique et Effets Avancés (Priorité Basse)
1.  **Interpolateurs physiques Web** : Intégrer un parseur léger dans `@aphrody/m3-motion` capable de traduire des paramètres de ressorts physiques (damping, stiffness) en approximations complexes à base de splines de bézier ou en animations programmées avec le Web Animations API.
2.  **Stabilisation des composants Labs** : Finaliser l'intégration des composants issus de `labs/gb` (comme les split-buttons, button-groups) au sein du bundle principal stable de `@material/web`.
