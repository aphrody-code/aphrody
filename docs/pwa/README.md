# 🌐 Architecture des Progressive Web Apps (PWA) Chromium

> Documentation technique détaillée sur l'implémentation des PWA sous Windows via Chromium, et son intégration/parallèle avec l'architecture hybride de Google OS.
> Dernière mise à jour : 2026-05-15.

---

## Table des matières

1. [Anatomie d'un raccourci PWA](#1-anatomie-dun-raccourci-pwa)
2. [Le rôle de `chrome_proxy.exe`](#2-le-rôle-de-chrome_proxyexe)
3. [Génération et gestion des App IDs](#3-génération-et-gestion-des-app-ids)
4. [Isolation de Profil](#4-isolation-de-profil)
5. [Parallèle avec Google OS (Pilier I & III)](#5-parallèle-avec-google-os-pilier-i--iii)
6. [Perspectives Forensics](#6-perspectives-forensics)

---

## 1. Anatomie d'un raccourci PWA

Lorsqu'un utilisateur installe une PWA (comme Google Gemini, YouTube, ou Spotify) via Google Chrome ou Edge, le navigateur génère un raccourci système Windows (`.lnk`).

Voici l'anatomie canonique de la cible d'un raccourci PWA :

```powershell
"C:\Users\<USER>\AppData\Local\Google\Chrome SxS\Application\chrome_proxy.exe" --profile-directory=Default --app-id=gdfaincndogidkdcdkhapmbffkckdkhn
```

Ce processus est totalement distinct du lancement classique d'une session de navigation. Il est conçu pour donner l'illusion d'une application de bureau native.

---

## 2. Le rôle de `chrome_proxy.exe`

Google Chrome ne lance pas directement `chrome.exe` pour les applications web de bureau. Il utilise le binaire `chrome_proxy.exe`.

### Pourquoi un Proxy (Stub) ?

1. **Routage de processus** : `chrome_proxy.exe` est un "lanceur léger". Il n'embarque pas le moteur de rendu Blink ou le moteur JS V8. Son seul rôle est de localiser le processus principal `chrome.exe` (s'il est déjà en cours d'exécution en arrière-plan) et de lui transmettre une instruction IPC (Inter-Process Communication) via des pipes nommés.
2. **Isolation UI** : Il ordonne à Chrome d'ouvrir une fenêtre sans le "Chrome" habituel (la barre d'URL, les boutons de navigation, les onglets). Il impose le mode `display: standalone` ou `display: window-controls-overlay`.
3. **AppUserModelID** : Le proxy gère correctement l'AUMID (Application User Model ID) de Windows. Cela garantit que l'application a sa propre icône dans la barre des tâches, distincte de celle du navigateur Chrome, et qu'elle peut gérer ses propres notifications natives Windows.

---

## 3. Génération et gestion des App IDs

L'argument `--app-id` (ex: `gdfaincndogidkdcdkhapmbffkckdkhn`) est la clé de voûte de la PWA.

### L'Algorithme de Hachage

Cet ID n'est pas aléatoire. Il est généré de manière déterministe à partir de la **Start URL** définie dans le `manifest.json` de la web app.

L'algorithme interne de Chromium (historiquement hérité des extensions Chrome) :
1. Prendre la Start URL complète (ex: `https://gemini.google.com/`).
2. Calculer le hash **SHA-256** de cette chaîne.
3. Prendre les 16 premiers octets du hash.
4. Convertir ces octets en une chaîne alphanumérique en utilisant un encodage "Base32-like" personnalisé où l'alphabet est composé des caractères `a` à `p` (où `a`=0, `b`=1, ..., `p`=15).

### Stockage Local

Ces IDs sont référencés dans la base de données interne de Chrome. Spécifiquement dans :
`C:\Users\<USER>\AppData\Local\Google\Chrome SxS\User Data\Default\Preferences`

Sous l'objet JSON `web_app_install_metrics` et dans la base de données SQLite Web Data. C'est ici que Chrome stocke le nom de l'app, les chemins vers les icônes mises en cache (`.ico`), et le thème de couleur (Theme Color).

---

## 4. Isolation de Profil

L'argument `--profile-directory=Default` est crucial pour l'état de l'application.

Une PWA n'est pas "sandboxed" par rapport à vos données de navigation. En spécifiant le profil, le proxy s'assure que la PWA partage exactement le même contexte d'exécution que votre session web :
*   **Cookies** : Vous restez connecté à Gemini si vous êtes connecté sur Chrome.
*   **Stockage Local (IndexedDB/LocalStorage)** : Les états de l'application hors-ligne sont préservés.
*   **DPAPI** : L'accès aux tokens cryptés (Master Key) reste valide dans le contexte de l'utilisateur Windows.

---

## 5. Parallèle avec Google OS (Pilier I & III)

L'architecture God Mode de `aphrody` s'inspire de cette isolation, mais supprime l'intermédiaire massif qu'est le navigateur web, poussant la performance à sa limite théorique.

| Composant PWA Chrome | Équivalent Google OS (God Mode) | Avantage God Mode |
| :--- | :--- | :--- |
| `chrome_proxy.exe` + `chrome.exe` | **Pilier I (`gui.exe` en Rust)** | Un seul binaire léger (quelques Mo), démarrage instantané, pas de moteur V8 en arrière-plan. |
| Fenêtre Chrome sans UI | **Webview Native (`tao` / `wry`)** | Accès direct aux primitives de la fenêtre native OS. Accélération matérielle DirectX 12 intégrée sans l'overhead Blink. |
| Fichiers HTML/JS distants ou en cache | **Pilier III (Bun JSX intégré en `.rdata`)** | Le HTML/MD3 est compilé statiquement par Bun et inclus (*embedded*) dans le binaire Rust. Aucune lecture disque ou réseau requise au lancement. |
| `--app-id` & Web Manifest | **Code source natif** | Les icônes, le thème et le comportement sont définis directement dans l'exécutable Rust et les appels OS. |

### Conclusion de l'Architecture
Là où Chrome instancie un proxy IPC complexe pour ouvrir une vue restreinte du web, Google OS compile une interface native (via Bun JSX) et l'injecte dans un moteur d'exécution système (Rust) bénéficiant de privilèges Ring 0.

---

## 6. Perspectives Forensics

Dans le cadre de nos outils d'investigation (module `backend` Rust), les PWAs représentent une cible d'extraction de données à haute valeur ajoutée.

Puisque les PWAs utilisent le profil standard de Chrome, notre crate `backend` peut utiliser son accès natif pour contourner les protections et extraire les données d'une PWA :
1. Localiser les IDs d'applications installées dans le fichier `Preferences`.
2. Utiliser notre accès **Ring 0** (via `bun_ffi` / `NtOpenFile`) pour copier les bases de données IndexedDB associées à la PWA, même si l'application est en cours d'exécution (Bypass du File Lock).
3. Déchiffrer les tokens d'authentification (ex: Token de session Gemini) via notre implémentation DPAPI/AES-GCM native.

→ Voir [`FORENSICS.md`](./FORENSICS.md) pour les détails d'implémentation de l'extraction des données PWA.
