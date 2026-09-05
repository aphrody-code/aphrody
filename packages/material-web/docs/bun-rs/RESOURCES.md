# Ressources Bun-RS et outils de base

Ce document répertorie les dépendances externes, les packages internes et les systèmes de runtime utilisés par `bun-rs`.

---

## 📦 Dépendances Rust

### 1. `memchr` (Opérations de chaînes SIMD)

- **Rôle** : Fournit des capacités de recherche d'octets rapides et accélérées par SIMD.
- **Utilisation** : Utilisé dans [bun_rs_count_char](file:///home/ubuntu/material-web/packages/bun-rs/src/lib.rs#L31) et [bun_rs_find_bytes](file:///home/ubuntu/material-web/packages/bun-rs/src/lib.rs#L47) pour des analyses de texte/sous-chaînes ultra-rapides.
- **Référence** : [memchr sur crates.io](https://crates.io/crates/memchr)

### 2. `grass` (Compilateur Sass en Rust pur)

- **Rôle** : Remplace le binaire Sass Node/Dart.
- **Version cible** : `0.13.0` (ou version stable la plus récente).
- **Avantages** : Compile le Sass directement en CSS dans le processus, sans lancer de sous-processus lourd.
- **Référence** : [grass sur crates.io](https://crates.io/crates/grass)

### 3. `m3-tokens` (Crate de thème Material locale)

- **Rôle** : La bibliothèque d'espace de travail contenant les algorithmes de tokens Material Design 3, les utilitaires de palette de couleurs et les mappages personnalisés.
- **Source** : [m3-tokens (Rust Crate)](file:///home/ubuntu/aphrody/crates/m3-tokens)
- **Avantages** : Remplace les utilitaires de couleurs Material standards en C++ ou JS de Google par des structures Rust propres et natives.

---

## ⚙️ API de runtime Bun

### 1. `bun:ffi` (Interface de fonction étrangère)

- **Méthode** : `dlopen`
- **Utilité** : Charge dynamiquement le fichier d'objet partagé généré (`.so`, `.dylib` ou `.dll`) et résout les symboles exportés en fonctions JavaScript appelables.
- **Référence** : [Documentation de Bun FFI](https://bun.sh/docs/api/ffi)

### 2. Génération rapide de pointeurs

- **Méthode** : `ptr(buffer)`
- **Utilité** : Résout un TypedArray JS (`Uint8Array`, `Buffer`) en un pointeur entier natif qui peut être transmis directement aux paramètres C-ABI.
- **Avantages** : Évite la copie de données ; accède directement au tas (heap) V8.

### 3. Outils et API WebAssembly (WASM)

- **`wasm-bindgen`** : Génère automatiquement la couche d'interface de typage et de marshaling bidirectionnelle entre JS et Rust WASM.
- **`wasm-pack`** : Outil CLI d'orchestration qui compile Rust en WASM (`wasm32-unknown-unknown`), exécute la liaison et prépare la structure npm de sortie.
- **`wasm-opt` (Binaryen)** : Optimiseur post-compilation qui applique des passes d'optimisation de taille (`-Oz`) sur le code intermédiaire WASM pour alléger le bundle navigateur.

---

## 📊 Métadonnées de compilation et de benchmark

| Métrique / Cible         | Valeur                     | Description / Rôle                                                                  |
| ------------------------ | -------------------------- | ----------------------------------------------------------------------------------- |
| **Surcoût FFI (Native)** | `< 3ns`                    | Coût de latence pour traverser la frontière Rust et revenir en JS.                  |
| **Coût Dart VM**         | `~15ms`                    | Latence de démarrage Dart VM (Dart-Sass) évitée par l'utilisation de `grass` natif. |
| **Cible native**         | `x86_64-unknown-linux-gnu` | Cible de compilation FFI native principale (Linux x64).                             |
| **Binaire FFI produit**  | `libbun_rs.so`             | Bibliothèque partagée C-ABI chargée via `bun:ffi`.                                  |
| **Cible WASM**           | `wasm32-unknown-unknown`   | Cible de compilation standard pour exécution WebAssembly dans le navigateur.        |
| **Binaire WASM produit** | `bun_rs_bg.wasm`           | Binaire d'instructions WASM optimisé par `wasm-opt` (~1.9 Mo).                      |
