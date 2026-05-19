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
