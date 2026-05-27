<!-- SPDX-License-Identifier: Apache-2.0 -->
# Analyse runtime des interactions design.google / Gemini → framework React

Analyse en profondeur (2026-05-23, via claude-in-chrome `javascript_tool` sur
les pages live + recoupement `crates/gemini-web`) du comportement **scroll /
clic / longue réflexion** des deux surfaces, et les primitives React qui en
découlent dans `apps/m3-react/src/interactions.tsx`.

## design.google — observé

| Aspect | Mesure live |
|---|---|
| Scroll-driven CSS | **0** élément avec `animation-timeline` / `scroll()` — pas de scroll timelines |
| Reveals au scroll | cartes hors-écran déjà `opacity:1`/`transform:none` → reveals (quand présents) en **IntersectionObserver**, pas dramatiques |
| Navigation au clic | **View Transitions API** : `view-transition-name: root` (1 élément racine) |
| Images | **10/10 `loading="lazy"`** |
| Header | `position: sticky` |
| Hover | `transition: color 0.165s cubic-bezier(0, 0.4, 0.2, 1)` (decelerate custom) |
| Hauteur doc | 4337 px (feed long, images chargées paresseusement) |

**Lecture** : design.google est volontairement sobre au scroll. La signature
interactionnelle est la **transition de vue au clic** (root) + le lazy-loading,
pas des animations de reveal tape-à-l'œil.

## gemini.google.com — observé (Angular, keyframes scoped `_ngcontent-*`)

État de **longue réflexion** (thinking) reconstitué depuis les keyframes :

| Keyframe | Rôle |
|---|---|
| `gem-shimmer-sweep` | **shimmer** du squelette pendant le chargement de la réponse |
| `animateGradient` / `gradientScroll` | **gradient de marque animé** (mouvement bleu→violet→rose) |
| `input-area-spin` | **anneau gradient animé autour du composer** pendant la génération |
| `mdc-circular-progress-*` (×6) | spinner circulaire 4-couleurs Google |
| `on-load-fade-in` / `lm-fade-in-up` / `on-load-slide-in` | messages qui **apparaissent en fade/slide-up** |

**Streaming** : la réponse arrive **token-par-token** (backend `StreamGenerate`,
chunks length-préfixés — cf. `crates/gemini-web/src/boq.rs`), chaque bloc
`fade-in-up` à l'arrivée. Cycle au clic d'envoi : composer → anneau gradient
animé (`input-area-spin`) + shimmer → premier token → reveal streaming → fin.

## Améliorations apportées à `apps/m3-react` (`interactions.tsx`)

Toutes reduced-motion aware, sur APIs plateforme standard (typecheck `tsc` 0) :

| Export | Source d'inspiration | Rôle |
|---|---|---|
| `startViewTransition()` / `useViewTransition()` | design.google nav (`root`) | enveloppe un update dans une View Transition (fallback sync) |
| `<ViewTransitionLink>` | clic design.google | `<a>` qui navigue en transition de vue + hover `0.165s` decelerate |
| `<Reveal>` | reveals IO | fade+lift à l'entrée via IntersectionObserver (pas de scroll timeline) |
| `<LazyImage>` | images lazy | `loading=lazy` + fade-in au décodage |
| `<ThinkingIndicator>` | longue réflexion | shimmer (`gem-shimmer-sweep`) + pastille gradient animée (`animateGradient`) |
| `<GradientBorder active>` | `input-area-spin` | anneau gradient de marque animé autour du composer pendant la génération |
| `<StreamingText>` / `useStreamingText()` | streaming token-par-token | reveal progressif + caret clignotant + `fade-in-up`, branché sur un `AsyncIterable<string>` (lecteur `StreamGenerate`) |

Exporté depuis `apps/m3-react/src/index.ts`. Exemple longue réflexion :

```tsx
const {text, done} = useStreamingText(stream); // stream du backend gemini-web
return generating
  ? <GradientBorder active><ThinkingIndicator label="Réflexion…" /></GradientBorder>
  : <StreamingText text={text} done={done} />;
```

## Pointeurs
- Primitives : `apps/m3-react/src/interactions.tsx`.
- Streaming backend : `crates/gemini-web/src/{boq,client}.rs`.
- Démos statiques : `packages/material-web/demos/`. Parité : `docs/design/angular-material-parity.md`.
