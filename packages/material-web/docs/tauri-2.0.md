<!-- SPDX-License-Identifier: Apache-2.0 -->

# Intégration de Tauri 2.0 & Material Design 3 (2026)

Ce guide détaille comment intégrer et optimiser le monorepo `material-web` (Lit components + React wrappers + Dynamic Theming) au sein d'une application de bureau ou mobile propulsée par **Tauri 2.0**.

---

## 1. Pourquoi associer Tauri 2.0 & `material-web` ?

Tauri 2.0 fournit un shell système ultra-léger (3 à 10 Mo par binaire, consommation mémoire réduite par rapport à Electron) basé sur la FFI native Rust. En combinant Tauri 2.0 avec notre architecture M3 :
*   **Performance FFI unifiée** : Notre crate `bun-rs` allie les performances FFI de Rust dans le processus Bun à la compilation. En production, notre code de thémation HCT s'intègre naturellement avec le backend natif de Tauri.
*   **Design cohérent** : Le style sans bordure (frameless) de Tauri s'habille parfaitement des composants Material 3, offrant une expérience bureau immersive.
*   **Zéro dérive de type** : Grâce à `ts-rs`, les configurations échangées entre le backend Rust de Tauri et le frontend React restent synchronisées statiquement.

---

## 2. Configuration de Tauri 2.0

### A. Désactiver les décorations système
Pour utiliser notre barre de titre personnalisée M3, vous devez désactiver la barre de titre classique de l'OS dans votre fichier `src-tauri/tauri.conf.json` :

```json
{
  "productName": "M3 Desktop App",
  "version": "1.0.0",
  "identifier": "com.m3.desktop",
  "build": {
    "beforeDevCommand": "bun run dev",
    "beforeBuildCommand": "bun run build",
    "devUrl": "http://localhost:3000",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "M3 Desktop Application",
        "width": 1000,
        "height": 700,
        "resizable": true,
        "fullscreen": false,
        "decorations": false,
        "transparent": true
      }
    ]
  }
}
```

> [!NOTE]
> `"decorations": false` indique à Tauri de ne pas afficher la barre de titre standard de Windows/macOS/Linux. Nous la remplacerons par le composant React `<M3TauriTitlebar />`.

---

## 3. Mise en œuvre dans React

Le package `@aphrody/m3-theme` expose le module `/tauri` qui regroupe les utilitaires d'intégration native.

### A. Initialisation et synchronisation du Thème (`useM3TauriThemeSync`)
Le hook `useM3TauriThemeSync` synchronise l'état du thème dynamique M3 (clair/sombre) avec le gestionnaire de fenêtre natif de Tauri via FFI.

```tsx
import React from "react";
import { M3ThemeProvider } from "@aphrody/m3-theme/react";
import { M3TauriTitlebar, useM3TauriThemeSync } from "@aphrody/m3-theme/tauri";
import "@aphrody/m3-theme/tokens.css";

function DesktopAppLayout() {
  // Synchronise automatiquement le thème résolu M3 vers Tauri
  useM3TauriThemeSync();

  return (
    <div 
      style={{ 
        display: "flex", 
        flexDirection: "column", 
        height: "100vh", 
        overflow: "hidden",
        backgroundColor: "var(--md-sys-color-background)" 
      }}
    >
      {/* Barre de titre personnalisée M3 */}
      <M3TauriTitlebar 
        title="Material Design 3 Desktop" 
        logo={<img src="/favicon.png" width="16" height="16" alt="logo" />} 
      />
      
      {/* Contenu principal */}
      <main style={{ flex: 1, overflow: "auto", padding: "16px" }}>
        <h1 style={{ color: "var(--md-sys-color-on-background)" }}>
          Bienvenue dans l'application Tauri 2.0 + M3
        </h1>
        <md-filled-button>Mon composant Lit M3</md-filled-button>
      </main>
    </div>
  );
}

export default function App() {
  return (
    <M3ThemeProvider defaultSeedColor="#6750a4" defaultThemeMode="system">
      <DesktopAppLayout />
    </M3ThemeProvider>
  );
}
```

---

## 4. Fonctionnement sous le capot (FFI & IPC)

La communication s'effectue sans dépendance lourde côté frontend via l'objet global injecté par le webview `window.__TAURI_INTERNALS__`.

### A. Synchronisation de Thème
Lorsqu'un changement de thème survient côté React, le hook appelle :
```typescript
window.__TAURI_INTERNALS__.invoke("plugin:window|set_theme", {
  value: resolvedTheme // "light" ou "dark"
});
```
*Note : Tauri 2.0 utilise le paramètre `value` (et non `theme`) pour son point d'entrée FFI natif.*

### B. Contrôles de Fenêtre
Les boutons d'agrandissement, de réduction et de fermeture interagissent directement avec l'IPC de la fenêtre active :
*   **Réduire** : `plugin:window|minimize`
*   **Fermer** : `plugin:window|close`
*   **Agrandir / Restaurer** : Le composant vérifie l'état actuel via `plugin:window|is_maximized` puis bascule en appelant `plugin:window|maximize` ou `plugin:window|unmaximize`.
*   **Déplacement (Drag)** : Utilise l'attribut standard HTML5 `data-tauri-drag-region` sur le conteneur pour déléguer la capture des clics et le mouvement à l'OS.

---

## 5. Synchronisation Typesafe avec `ts-rs`

Pour transmettre des objets de configuration complexes entre vos commandes Rust de Tauri et votre interface React :

1.  Déclarez vos structs dans la crate Rust en dérivant le trait `TS` de `ts-rs` :
    ```rust
    use ts_rs::TS;
    use serde::{Serialize, Deserialize};

    #[derive(TS, Serialize, Deserialize)]
    #[ts(export, export_to = "../src/bindings/AppConfig.ts")]
    pub struct AppConfig {
        pub theme_seed: String,
        pub enable_translucency: bool,
        pub update_channel: String,
    }
    ```
2.  Exécutez `cargo test` pour générer automatiquement l'interface TypeScript correspondante.
3.  Côté React, importez le type exporté pour typer vos appels `invoke` :
    ```typescript
    import type { AppConfig } from "./bindings/AppConfig";
    
    async function loadConfig(): Promise<AppConfig> {
      return window.__TAURI_INTERNALS__.invoke("get_app_config");
    }
    ```
