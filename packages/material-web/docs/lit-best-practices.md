---
title: "Lit — Bonnes pratiques"
nav_order: 9
---

# Lit best practices — référentiel d'amélioration (source : lit.dev via context7)

Checklist appliquée à tous les composants Lit du fork material-web. Objectif : conformité Lit + perf + a11y, **sans casser l'API publique**.

## 1. Réactivité : `@property` vs `@state`

- **`@property`** = API publique (settable depuis HTML/JS externe). Options à utiliser à bon escient : `{type, attribute, reflect, converter, hasChanged}`. `reflect: true` **uniquement** si l'attribut doit refléter l'état (sélecteurs CSS, a11y) — pas par défaut (coût).
- **`@state`** = état réactif **interne** (hover, focus transitoire, valeurs dérivées, flags d'erreur). **Pas d'attribut**, ne doit jamais être lu de l'extérieur, peut être renommé par les minifiers. → Tout champ interne actuellement en `@property` doit passer `@state`.
- `type` correct (`Boolean`/`Number`/`String`/`Object`/`Array`) pour la conversion d'attribut. Booléens : présence d'attribut = true.

## 2. Events (Shadow DOM)

- Tout `CustomEvent` destiné à sortir du composant : **`{bubbles: true, composed: true}`** (sinon bloqué au shadow root). Données dans `detail`.
- Re-dispatch des events natifs (`input`/`change`) : conserver la sémantique native (`e.target.value`).
- Nommer les events de façon cohérente (les composants fork utilisent `comp:action`, ex `table:sort`).

## 3. Styles

- `static styles = css\`…\`` (le plus performant, mis en cache/adopté une fois). Pas de styles inline statiques.
- Dynamique : **`styleMap`** (`style=${styleMap({...})}`) et **`classMap`** (`class=${classMap({...})}`) plutôt que concaténation de chaînes.
- Theming par **CSS custom properties** (`--md-sys-*` avec fallbacks, + overrides `--md-<comp>-*`). Pas de valeurs en dur quand un token existe.
- `:host` pour le style du composant ; `:host([attr])` pour les variantes reflétées.

## 4. Lifecycle & perf

- Calculs dérivés dans **`willUpdate(changed)`** (avant render), pas dans `render()`. `render()` doit être pur.
- Garder via le système réactif ; éviter `querySelector` manuel quand un binding suffit. Utiliser `@query`/`@queryAssignedElements` si besoin DOM.
- Ne pas muter de propriété réactive dans `updated()` sans garde (boucle de rendu).
- `firstUpdated()` pour l'init DOM unique ; nettoyer les listeners/observers dans `disconnectedCallback`.

## 5. Accessibilité

- ARIA correct dans le shadow DOM (`role`, `aria-*`). Form controls : `ElementInternals` (`static formAssociated`, `attachInternals`, `setFormValue`, `setValidity`, `ariaLabel` via internals quand pertinent).
- Navigation clavier complète, focus visible (tokens M3 / `md-focus-ring`), `prefers-reduced-motion` respecté pour les animations.

## 6. Divers

- `static shadowRootOptions` avec `delegatesFocus: true` pour les composants focusables wrappant un contrôle natif.
- Pas d'`any` implicite (return types annotés) — le build lib est en `strict` + `noUnusedLocals`.
- Imports `lit`/`lit/decorators.js`/`lit/directives/*` ; pas de dépendance externe.
