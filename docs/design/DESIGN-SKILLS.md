# Agent-skills DESIGN / UI

Ce catalogue recense les agent-skills de design et d'UI installés sous
`.agents/skills/`. Ils alimentent la stack UI polyglotte d'aphrody, dont
l'implémentation s'appuie sur quatre piliers : le fork `packages/material-web`
(Material Web Components 3, wrappés en WASM Rust natif), le fork `packages/ui`
(shadcn/ui, source-en-projet via CLI), le fork `packages/tailwindcss`
(styling utility-first), et le crate Rust `m3-tokens` (tokens M3 canoniques :
couleur, forme, typographie, motion). Les skills Material Design 3 nourrissent
`m3-tokens` et `packages/material-web` ; les skills Tailwind CSS et shadcn
guident respectivement `packages/tailwindcss` et `packages/ui`.

## Material Design 3

| Skill | Description | Chemin d'install |
| --- | --- | --- |
| `material-3` | Implémente le système UI Material Design 3 (Material You) de Google. Cible principale Jetpack Compose Material3, aussi Flutter et web limité ; couvre tokens, 30+ composants, layout, theming, M3 Expressive, accessibilité. | `.agents/skills/material-3` |
| `material` | Material Design de Google avec surfaces en couches, theming dynamique, motion intégré et patterns responsive cross-platform. | `.agents/skills/material` |
| `material-design-3-components` | Guide complet des composants Material Design 3, de Material You à M3 Expressive : composants d'action, containment, communication, navigation, sélection et saisie texte, avec specs, états et implémentation. | `.agents/skills/material-design-3-components` |
| `material-design-3-guide` | Guide maître Material Design 3 (Material You jusqu'à M3 Expressive) : explique quel sous-skill M3 appliquer (couleur, motion, typographie, forme, layout, composants, icônes). | `.agents/skills/material-design-3-guide` |

## Tailwind CSS

| Skill | Description | Chemin d'install |
| --- | --- | --- |
| `tailwindcss` | Expert du styling utility-first TailwindCSS avec patterns de design responsive. | `.agents/skills/tailwindcss` |
| `tailwindcss-advanced-layouts` | Techniques de layout avancées Tailwind CSS : CSS Grid et Flexbox, grid-template-areas (v4), grilles responsive, container queries, subgrid, aspect-ratio, layouts magazine. | `.agents/skills/tailwindcss-advanced-layouts` |
| `tailwindcss-animations` | Animations et transitions Tailwind CSS : utilities animate-*, keyframes custom, transforms, prefers-reduced-motion, Framer Motion, animations scroll-driven et View Transitions API. | `.agents/skills/tailwindcss-animations` |
| `tailwindcss-mobile-first` | Patterns responsive mobile-first avec Tailwind CSS v4 : breakpoints, typographie fluide, container queries, safe-area insets, gestion touch/hover. | `.agents/skills/tailwindcss-mobile-first` |
| `nextjs-typescript-tailwindcss-supabase` | Développement full-stack Next.js 14 avec TypeScript, TailwindCSS et Supabase pour applications web prêtes pour la production. | `.agents/skills/nextjs-typescript-tailwindcss-supabase` |

## shadcn-ui

| Skill | Description | Chemin d'install |
| --- | --- | --- |
| `shadcn` | Gère les composants et projets shadcn : ajout, recherche, correction, debug, styling et composition d'UI ; fournit contexte projet, docs composants et exemples (registries, presets, `components.json`). | `.agents/skills/shadcn` |
