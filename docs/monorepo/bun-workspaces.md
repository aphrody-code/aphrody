<!-- SPDX-License-Identifier: Apache-2.0 -->
# Bun Workspaces Documentation (Standard 2026)

Bun Workspaces est le gestionnaire de paquets natif et ultra-rapide intégré à l'environnement Bun. Il remplace `npm workspaces`, `yarn` et `pnpm` en éliminant les overheads Node.js.

## Structure et Configuration

La configuration se fait dans le `package.json` à la racine :

```json
{
  "name": "aphrody",
  "workspaces": [
    "packages/*",
    "apps/*"
  ]
}
```

## Fonctionnalités Clés

1.  **Résolution Hoistée** : Un seul `bun.lockb` (binaire et ultra-rapide) à la racine. Les dépendances communes sont remontées dans le `node_modules` global.
2.  **Liaison locale (Symlinking)** : Si `packages/a` dépend de `packages/b`, Bun crée automatiquement un lien symbolique interne. Aucun `npm link` n'est nécessaire.
3.  **Exécution ciblée** :
    *   `bun run --filter "packages/ui" build`
    *   `bun run --filter "*" test`

## Cas d'usage dans Aphrody

Nous utilisons Bun Workspaces pour l'ensemble de notre couche applicative TypeScript/JS (le CLI, le serveur MCP, le frontend UI). L'avantage principal réside dans la vitesse d'installation (quelques millisecondes) et l'accès direct aux API système natives de Bun (`Bun.file`, `Bun.spawn`).
