<!-- SPDX-License-Identifier: Apache-2.0 -->
# Turborepo Documentation (Standard 2026)

Turborepo est le standard de l'industrie pour orchestrer des monorepos JavaScript/TypeScript massifs. Conçu en Rust, il excelle dans la parallélisation et la mise en cache.

## Concepts Fondamentaux

1.  **Pipeline (turbo.json)** : Définit l'ordre d'exécution des tâches (ex: `build`, `lint`, `test`) via un DAG (Directed Acyclic Graph).
2.  **Mise en cache (Caching)** : Ne recompile que ce qui a changé. Calcule un hash basé sur les fichiers sources, les variables d'environnement et les dépendances.
3.  **Filtrage (Filtering)** : Permet d'exécuter des tâches uniquement sur des packages spécifiques ou sur les paquets ayant subi des modifications (`--filter=...`).

## Exemple de `turbo.json`

```json
{
  "$schema": "https://turbo.build/schema.json",
  "globalEnv": ["NODE_ENV"],
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**", ".next/**"]
    },
    "lint": {
      "dependsOn": []
    },
    "dev": {
      "cache": false,
      "persistent": true
    }
  }
}
```

## Intégration hybride
Dans notre architecture, Turbo s'interface parfaitement avec Bun Workspaces. Bun gère la résolution des dépendances et Turbo gère l'orchestration des tâches complexes.
