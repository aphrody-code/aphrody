<!-- SPDX-License-Identifier: Apache-2.0 -->
# Cargo Workspaces (Rust Monorepo)

Cargo (le package manager de Rust) dispose d'un support natif et robuste pour les monorepos via les **Cargo Workspaces**.

## Structure du Workspace

Un workspace Cargo est défini par un fichier `Cargo.toml` virtuel (ou un package racine) contenant une table `[workspace]`.

```toml
[workspace]
members = [
    "crates/core",
    "crates/ffi",
    "cli"
]
default-members = ["cli"]

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.40", features = ["full"] }
```

## Avantages en 2026

1.  **`workspace.dependencies`** : Partage de versions exactes de dépendances entre toutes les crates, évitant les conflits de compilation et accélérant le build (la dépendance n'est compilée qu'une fois).
2.  **Cargo Lock Unifié** : Un seul fichier `Cargo.lock` à la racine garantit la reproductibilité totale du projet.
3.  **Résolution de features** : Cargo unifie l'arbre de dépendances et combine intelligemment les features requises par chaque crate du workspace.

## Commandes

*   `cargo build --workspace` (Compile tout)
*   `cargo test -p core` (Teste uniquement la crate 'core')
*   `cargo clippy --workspace` (Lint global)
