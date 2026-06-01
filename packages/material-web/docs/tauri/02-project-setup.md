<!-- SPDX-License-Identifier: Apache-2.0 -->

# 02. Initialisation et Intégration dans le Monorepo Bun

Ce guide explique comment structurer et configurer Tauri 2.0 comme package ou extension de notre monorepo `material-web` géré par Bun.

---

## 📁 Structure du Monorepo conseillée

Pour conserver la modularité de notre monorepo, l'application conteneur de bureau/mobile (l'hôte) réside soit dans `examples/showcase` soit dans un nouveau workspace dédié `examples/desktop-app` :

```
material-web/
├── package.json                   # Gestionnaire de packages Bun + Workspace
├── packages/
│   ├── m3-theme/                  # Intégration thématique + Tauri hooks
│   └── material-web/              # Bibliothèque de composants web Lit
└── examples/
    └── showcase/                  # L'application React consommatrice
        ├── src/                   # Fichiers sources React/TypeScript
        ├── src-tauri/             # Code natif Rust pour Tauri
        │   ├── Cargo.toml         # Dépendances natives Tauri (crates)
        │   ├── tauri.conf.json    # Configuration globale de l'app Tauri
        │   └── src/main.rs        # Point d'entrée Rust
        └── package.json           # Scripts de build frontend
```

---

## 🚀 1. Installation de la CLI et Initialisation

1.  **Ajouter la CLI de Tauri** dans les dépendances de développement du package hôte (ex: `examples/showcase`) :
    ```bash
    cd examples/showcase
    bun add -d @tauri-apps/cli@latest
    ```
2.  **Initialiser le projet Tauri** :
    ```bash
    bunx tauri init
    ```
    Lors de l'initialisation, répondez aux invites avec les valeurs suivantes pour notre chaîne Bun native :
    *   **What is your app name?** : `M3 Showcase`
    *   **What is your window title?** : `Material Design 3 Showcase`
    *   **Where are your web assets?** : `../dist` (le dossier de build généré par `Bun.build`)
    *   **What is the URL of your dev server?** : `http://localhost:3000` (le port de notre serveur `Bun.serve`)
    *   **What is your frontend dev command?** : `bun run dev`
    *   **What is your frontend build command?** : `bun run build`

---

## 🛠️ 2. Configuration des scripts Bun

Dans le fichier `package.json` de votre package hôte (`examples/showcase/package.json`), ajoutez les raccourcis de scripts de compilation :

```json
{
  "name": "@aphrody-code/m3-showcase",
  "scripts": {
    "dev": "bun run src/server.ts",
    "build": "bun run build.ts",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build",
    "tauri:android": "tauri android dev",
    "tauri:ios": "tauri ios dev"
  }
}
```

Pour appeler ces scripts depuis la racine du monorepo via Turborepo, ajoutez la tâche dans `turbo.json` ou lancez-les directement avec Bun :
```bash
bun --filter @aphrody-code/m3-showcase run tauri:dev
```

---

## 📝 3. Configuration Globale de Tauri (`tauri.conf.json`)

Assurez-vous que les permissions d'invocation et les configurations de build correspondent à notre stack dans `examples/showcase/src-tauri/tauri.conf.json` :

```json
{
  "build": {
    "beforeDevCommand": "bun run dev",
    "beforeBuildCommand": "bun run build",
    "devUrl": "http://localhost:3000",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "M3 Showcase",
        "width": 1024,
        "height": 768,
        "resizable": true,
        "decorations": false,
        "transparent": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' data: https:; style-src 'self' 'unsafe-inline' https:; script-src 'self' 'unsafe-inline' 'unsafe-eval';"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "identifier": "com.aphrody.m3showcase",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

> [!IMPORTANT]
> L'option `"decorations": false` est obligatoire pour permettre à notre composant de barre de titre `<M3TauriTitlebar />` de remplacer la barre native du système d'exploitation.

---

## 🔒 4. Configuration des Permissions (Tauri 2.0 Security)

Dans Tauri 2.0, les droits d'accès aux plugins natifs (comme la gestion des fenêtres et des thèmes) sont soumis à un modèle de capacités strictes. Créez un fichier de permissions sous `src-tauri/capabilities/default.json` :

```json
{
  "$schema": "../gen/schemas/capability.json",
  "identifier": "main-capability",
  "description": "Permissions par défaut pour l'hôte de bureau M3",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-minimize",
    "core:window:allow-maximize",
    "core:window:allow-unmaximize",
    "core:window:allow-close",
    "core:window:allow-is-maximized",
    "core:window:allow-set-theme"
  ]
}
```

Ces autorisations permettent à notre hook `useM3TauriThemeSync()` et à la barre de titre `<M3TauriTitlebar />` d'appeler l'IPC système pour modifier la fenêtre native.
