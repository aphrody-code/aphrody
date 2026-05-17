<!-- SPDX-License-Identifier: Apache-2.0 -->
# 🔍 PWA Forensics & Extraction

> Guide technique d'extraction des données isolées des Progressive Web Apps Chromium via le backend `aphrody`.
> Dernière mise à jour : 2026-05-15.

---

## 1. Vecteur d'Attaque

Les Progressive Web Apps (PWAs) basées sur Chromium (Chrome, Edge, Brave) stockent leurs données (IndexedDB, LocalStorage, Cache API) dans des dossiers isolés mais liés au profil utilisateur principal. Ces données sont souvent chiffrées à l'aide de l'API DPAPI de Windows.

Le crate Rust `backend` de Google OS dispose des capacités nécessaires pour extraire, déchiffrer et analyser silencieusement ces bases de données.

## 2. Emplacement des Données

L'espace de stockage des PWAs se trouve généralement sous :
`%LOCALAPPDATA%\Google\Chrome SxS\User Data\Default\IndexedDB\`

Les bases de données IndexedDB (LevelDB) sont classées par "Origin" (ex: `https_gemini.google.com_0.indexeddb.leveldb`).

### Résolution App ID vers Origin

Pour cartographier une PWA (`--app-id=gdfaincndogidkdcdkhapmbffkckdkhn`) à son point d'origine, le `backend` parse le fichier JSON `Preferences` :

1. Ouvrir le fichier `Preferences`.
2. Extraire les clés sous `web_app_install_metrics`.
3. Récupérer l'URL racine associée dans l'historique d'engagement.

## 3. Contournement des Verrous (File Lock Bypass)

Les bases de données IndexedDB et SQLite de Chromium sont verrouillées (`EXCLUSIVE LOCK`) lorsque le navigateur ou la PWA est en cours d'exécution.

### La méthode God Mode (`bun_ffi`)

Plutôt que d'attendre la fermeture de la PWA, Google OS utilise son module Ring 0 :
1. Activation de `SeBackupPrivilege`.
2. Utilisation de l'API NTDLL `NtOpenFile` ou du Volume Shadow Copy Service (VSS) si nécessaire.
3. Copie de l'intégralité du dossier `leveldb` vers un répertoire temporaire.

## 4. Déchiffrement AES-GCM (La Master Key)

Si la PWA utilise l'API de chiffrement de Chromium ou des tokens de session sécurisés :

1. **Extraction de la Master Key** :
   Le `backend` Rust extrait la clé encodée en base64 de `%LOCALAPPDATA%\Google\Chrome SxS\User Data\Local State`.

2. **Déchiffrement DPAPI** :
   Le `CryptoHelper` (crate `base`) invoque `CryptUnprotectData` pour déchiffrer la Master Key.

3. **Déchiffrement des Payload** :
   Tout blob de la PWA commençant par `v10` ou `v20` est passé dans notre implémentation native AES-GCM avec la Master Key déchiffrée.

## 5. Cas d'Usage : Google Gemini PWA

Dans le cas spécifique de la PWA Gemini, l'objectif d'investigation pourrait être de récupérer localement :
*   Les jetons d'authentification OAuth (permettant le détournement de session API).
*   Le cache hors-ligne des conversations (stocké en clair dans le Cache Storage ou IndexedDB).
*   Les préférences de l'utilisateur (Thème, Paramètres de langue).

**Outil CLI associé (prévu) :**
```bash
aphrody auth --target pwa --app-id gdfaincndogidkdcdkhapmbffkckdkhn
```
*Note : Cette fonctionnalité est en cours d'implémentation dans le crate `cli`.*
