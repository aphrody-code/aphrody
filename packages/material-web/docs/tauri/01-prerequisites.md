<!-- SPDX-License-Identifier: Apache-2.0 -->

# 01. Prérequis & Configuration de l'Environnement

Pour développer et cross-compiler l'application `material-web` avec Tauri 2.0 pour les ordinateurs et mobiles, vous devez installer les outils de compilation spécifiques à chaque plateforme cible.

---

## 🛠️ Outils Communs (Toutes plateformes)

1.  **Rust & Cargo** : Installez la chaîne d'outils via [Rustup](https://rustup.rs/) (sélectionnez l'édition Rust 2024 active).
2.  **Bun** : Installez le runtime via la commande officielle :
    ```bash
    curl -fsSL https://bun.sh/install | bash
    ```

---

## 💻 Configuration pour le Bureau (Desktop)

### A. Windows (Hôte / Cible)
*   **Visual Studio Build Tools** : Installez [Visual Studio Community](https://visualstudio.microsoft.com/) ou ses Build Tools. Dans l'installateur, cochez l'option **"Développement Desktop en C++"** (qui inclut le SDK Windows et MSVC).
*   **Cibles Rust** (ajoutées automatiquement) : `x86_64-pc-windows-msvc`.

### B. macOS (Hôte / Cible)
*   **Xcode Command Line Tools** : Exécutez dans un terminal :
    ```bash
    xcode-select --install
    ```
*   **Cibles Rust** :
    ```bash
    rustup target add x86_64-apple-darwin      # Pour Mac Intel
    rustup target add aarch64-apple-darwin     # Pour Apple Silicon (M1/M2/M3)
    ```

### C. Linux (Hôte / Cible - Debian/Ubuntu)
Installez les dépendances système requises pour compiler l'API de fenêtrage et le Webview (Tauri 2.0 sous Linux utilise WebKitGTK) :
```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```
*   **Cibles Rust** : `x86_64-unknown-linux-gnu`.

---

## 📱 Configuration pour les Mobiles (Mobile)

### A. Android (Cible - Compilable sous Windows, macOS et Linux)
1.  **Java Development Kit (JDK)** : Installez OpenJDK 17 ou Zulu JDK 17 (obligatoire pour Gradle et Android SDK).
2.  **Android Studio** : Téléchargez et installez [Android Studio](https://developer.android.com/studio).
3.  **Android SDK & NDK** :
    *   Dans Android Studio, allez dans *SDK Manager* > *SDK Tools* et cochez :
        *   `Android SDK Command-line Tools`
        *   `Android NDK` (sélectionnez la version stable recommandée, par exemple `26.x.x`)
        *   `Android SDK Platform-Tools`
        *   `CMake`
4.  **Variables d'Environnement** : Ajoutez les chemins à votre fichier de configuration shell (`.bashrc` ou `.zshrc`) :
    ```bash
    export ANDROID_HOME=$HOME/Android/Sdk
    export NDK_HOME=$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk)
    export PATH=$PATH:$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin
    ```
5.  **Cibles Rust pour Android** :
    ```bash
    rustup target add aarch64-linux-android      # Téléphones modernes (64-bit)
    rustup target add armv7-linux-androideabi    # Téléphones anciens (32-bit)
    rustup target add x86_64-linux-android       # Émulateur (64-bit)
    rustup target add i686-linux-android         # Émulateur (32-bit)
    ```

### B. iOS (Cible - Compilable sur macOS uniquement)
1.  **Xcode complet** : Téléchargez Xcode sur le Mac App Store (les Command Line Tools seuls ne suffisent pas pour iOS).
2.  **CocoaPods** : Recommandé pour gérer les dépendances natives d'iOS. Installez-le via Homebrew ou RubyGems :
    ```bash
    brew install cocoapods
    ```
3.  **Cibles Rust pour iOS** :
    ```bash
    rustup target add aarch64-apple-ios          # Périphériques physiques iOS (iPhone/iPad)
    rustup target add aarch64-apple-ios-sim      # Émulateur iOS sur Apple Silicon (M1/M2/M3)
    rustup target add x86_64-apple-ios           # Émulateur iOS sur Mac Intel
    ```

---

## 🔍 Validation de l'environnement
Tauri propose une CLI de diagnostic très performante pour s'assurer que toutes les dépendances sont configurées. Pour vérifier votre configuration, exécutez à la racine du projet :
```bash
bunx tauri doctor
```
Assurez-vous que toutes les sections (en particulier Rust, Node/Bun, Android/iOS) sont cochées en vert avant de passer à l'étape suivante.
