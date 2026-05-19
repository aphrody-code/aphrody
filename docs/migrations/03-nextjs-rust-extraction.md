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
