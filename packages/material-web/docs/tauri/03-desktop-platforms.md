<!-- SPDX-License-Identifier: Apache-2.0 -->

# 03. Développement & Déploiement sur Bureau (Desktop)

Ce guide détaille les spécificités de développement, d'optimisation visuelle et de packaging pour les cibles de bureau : **Windows**, **macOS** et **Linux**.

---

## 🎨 1. Effets de Translucidité et Vibrance (M3 Window Blur)

Pour obtenir un look premium d'inspiration Material 3, nous pouvons activer la transparence et le flou d'arrière-plan de la fenêtre native en fonction de la plateforme :

### A. Windows (Mica & Acrylic)
Windows 11 prend en charge l'effet de flou *Mica* (qui échantillonne l'arrière-plan du bureau) ou *Acrylic* (flou translucide).
Pour l'activer dans `tauri.conf.json` :
```json
{
  "app": {
    "windows": [
      {
        "decorations": false,
        "transparent": true,
        "windowEffects": {
          "effects": ["mica"],
          "state": "active"
        }
      }
    ]
  }
}
```
*Note : Côté CSS, assurez-vous que l'arrière-plan de votre balise `<body>` ou `<html>` utilise une couleur avec de l'opacité (ex: `rgba(var(--md-sys-color-background-rgb), 0.7)`) pour laisser transparaître l'effet.*

### B. macOS (Vibrancy)
macOS propose des effets de vibrance sous la fenêtre. Ajoutez ceci dans votre configuration de fenêtre :
```json
{
  "windowEffects": {
    "effects": ["underWindow", "hudWindow"],
    "state": "active"
  }
}
```

---

## 🏗️ 2. Assemblage et Packaging (Distribution)

Pour compiler et générer les installateurs de production, exécutez la commande suivante depuis le dossier de votre hôte :
```bash
bunx tauri build
```
Cette commande compile le frontend, compile le binaire Rust en mode `release` et produit les installateurs spécifiques à l'OS hôte.

### A. Windows (MSI & NSIS)
*   **Formats générés** : `.msi` (installateur standard Windows Installer via WiX Toolset v3) et `.exe` (via NSIS, plus rapide et léger).
*   **WiX Toolset** : Si WiX v3 n'est pas présent, Tauri le télécharge et l'installe automatiquement lors du premier build.
*   **Chemin des fichiers** : `src-tauri/target/release/bundle/msi/` et `src-tauri/target/release/bundle/nsis/`.

### B. macOS (App & DMG)
*   **Formats générés** : `.app` (binaire de l'application) et `.dmg` (image disque d'installation).
*   **Signature de code & Notarisation (Obligatoire pour la distribution hors Mac App Store)** :
    Vous devez définir les variables d'environnement avec vos identifiants Apple Developer avant de compiler :
    ```bash
    export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TeamID)"
    export APPLE_API_KEY_PATH="path/to/private/key.p8"
    export APPLE_API_KEY_ISSUER="issuer-uuid"
    export APPLE_API_KEY_ID="key-id"
    ```
    Lancez ensuite le build ; Tauri signera et enverra automatiquement le paquet à Apple pour validation (notarisation).

### C. Linux (Debian & AppImage)
*   **Formats générés** : `.deb` (paquet Debian/Ubuntu) et `.AppImage` (binaire portable autonome).
*   **Compilation croisée (Cross-Compilation)** : Pour compiler pour Linux, vous devez exécuter le build sur une machine Linux (ou une VM / container Docker).
*   **Note WebKitGTK** : Le paquet `.deb` généré listera automatiquement `libwebkit2gtk-4.1-0` dans ses dépendances d'installation pour s'assurer que le moteur de rendu HTML est installé chez le client final.
