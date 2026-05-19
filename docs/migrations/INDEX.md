# Plans de Migration Détaillés (TypeScript vers Rust)

Ce dossier documente les plans de refactoring avancés ("part-by-part") pour les packages historiques Bun/TypeScript du projet Aphrody qui nécessitent une attention particulière.

| Fichier | Cible d'Origine | Cible Rust | Priorité |
|---|---|---|---|
| [`01-gemini-ui-to-wasm.md`](01-gemini-ui-to-wasm.md) | `packages/gemini` | `crates/aphrody-wgpu-material` | 1 (Haute) |
| [`02-ui-shadcn-to-tui.md`](02-ui-shadcn-to-tui.md) | `packages/ui` | `crates/aphrody-tui` | 2 (Moyenne) |
| [`03-nextjs-rust-extraction.md`](03-nextjs-rust-extraction.md) | `packages/next.js` | `[workspace.dependencies]` purs | 3 (Basse) |

*Les autres packages (`mrx`, `bxc`, `n2b`, `aphrody-skills`, `aphrody-jsx`) ont déjà été migrés avec succès et les dossiers TS supprimés de l'arbre.*
