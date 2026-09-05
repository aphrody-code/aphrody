<!-- SPDX-License-Identifier: Apache-2.0 -->

# 04. Développement & Déploiement sur Mobiles (Android & iOS)

Ce guide détaille la configuration, l'intégration des fonctionnalités d'OS mobiles et le packaging final pour **Android** et **iOS** sous Tauri 2.0.

---

## 🚀 1. Initialisation des Cibles Mobiles

Pour activer le support mobile dans votre projet Tauri, exécutez la commande d'initialisation suivante dans le dossier contenant le code Tauri (ex: `examples/showcase`) :

```bash
# Initialise les projets natifs Android (Gradle) et iOS (Xcode)
bunx tauri mobile init
```

Cette commande va créer un dossier `src-tauri/gen/android` et `src-tauri/gen/ios` contenant les structures d'applications natives respectives.

---

## 📱 2. Exécution et Débogage (Live Reload)

Pour tester l'application M3 sur un périphérique ou un simulateur en temps réel :

### Connexion Réseau (Obligatoire)
Le serveur de développement Bun s'exécutant sur votre machine de bureau, le téléphone mobile ou le simulateur doit pouvoir y accéder :
1.  Connectez votre machine et votre périphérique de test physique sur le **même réseau Wi-Fi**.
2.  Tauri détecte automatiquement l'adresse IP de votre machine et reconfigure la directive `devUrl` pour pointer sur votre adresse réseau (ex: `http://192.168.1.45:3000`).

### Lancement du Live Reload :
```bash
# Pour Android (lance l'émulateur ou déploie sur le téléphone connecté en USB)
bunx tauri android dev

# Pour iOS (lance le simulateur Simulator.app ou déploie via Xcode)
bunx tauri ios dev
```

---

## 🎨 3. Intégrations Spécifiques aux OS Mobiles

### A. Android : Teinter les barres système (M3 Status Bar & Navigation Bar)
Pour que la barre d'état (top status bar) et la barre de navigation (bottom navbar) d'Android adoptent dynamiquement la couleur de notre thème M3, nous pouvons modifier l'activité principale dans `src-tauri/gen/android/app/src/main/java/com/aphrody/m3showcase/MainActivity.kt`.

Ajoutez ce code Kotlin pour synchroniser les couleurs système lors du démarrage :
```kotlin
package com.aphrody.m3showcase

import android.os.Bundle
import android.view.Window
import android.view.WindowManager
import androidx.core.view.WindowCompat
import tauri.TauriActivity

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // Permet au contenu HTML de s'afficher derrière les barres système (Edge-to-Edge)
        WindowCompat.setDecorFitsSystemWindows(window, false)
        
        // Optionnel : Forcer une couleur de fond initiale assortie à notre thème M3 (ex: Surface #f3f3fa)
        val window: Window = this.window
        window.navigationBarColor = android.graphics.Color.parseColor("#f3f3fa")
        window.statusBarColor = android.graphics.Color.TRANSPARENT
    }
}
```

### B. iOS : Gestion des zones de sécurité (Safe Areas)
Sur les iPhones modernes avec encoche (Notch) ou Dynamic Island, le haut de l'écran et la barre d'accueil inférieure empiètent sur le conteneur web.
Pour éviter que l'interface de notre application ou notre barre de titre `<M3TauriTitlebar />` ne soient coupées, appliquez les directives CSS Safe Area dans votre feuille de style globale (`showcase.css` ou `index.css`) :

```css
/* S'applique à notre conteneur racine ou à notre entête */
header, .tauri-titlebar {
  padding-top: env(safe-area-inset-top, 0px);
}

footer, .bottom-navigation {
  padding-bottom: env(safe-area-inset-bottom, 0px);
}
```

---

## 🏗️ 4. Build de Production

### A. Android (APK & AAB)
Compilez l'application en mode production pour générer le package final :
```bash
bunx tauri android build
```
*   **Formats générés** :
    *   `.apk` (pour tester et installer en direct sur le périphérique).
    *   `.aab` (Android App Bundle, requis pour la publication sur le Google Play Store).
*   **Chemin des fichiers** : `src-tauri/gen/android/app/build/outputs/bundle/universalRelease/`.

### B. iOS (Xcode Archive & IPA)
Le build iOS génère le projet Xcode compilé en mode Release :
```bash
bunx tauri ios build
```
Une fois le build terminé :
1.  Ouvrez le projet généré dans Xcode :
    ```bash
    open src-tauri/gen/ios/Layout.xcodeproj
    ```
2.  Sélectionnez votre compte de développeur dans *Signing & Capabilities*.
3.  Sélectionnez la cible *Any iOS Device* et allez dans le menu *Product* > *Archive*.
4.  Une fois l'archive générée, cliquez sur *Distribute App* pour exporter votre fichier `.ipa` ou le publier directement sur TestFlight / App Store Connect.
