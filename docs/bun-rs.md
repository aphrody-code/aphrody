# Bun-RS : Architecture Native & Développement de Librairies

Ce document détaille l'écosystème Rust interne de **Bun**, tel qu'il a été extrait et compilé depuis les sources officielles (`oven-sh/bun`) et intégré nativement dans **Google OS**.

Historiquement, Bun est perçu comme un projet Zig. Cependant, toute son architecture moderne (depuis la refonte de son bundler et de son parseur, ainsi que son intégration avec JavaScriptCore) repose sur une armada de **plus de 60 crates Rust** hautement optimisées.

Grâce à notre intégration, nous pouvons lier statiquement notre code Rust à ces crates, offrant des performances "Zero-Cost" sans précédent par rapport aux API C-FFI standards (Node-API / N-API).

## 1. Pourquoi développer en Rust natif pour Bun ?

- **Zéro Copie (Zero-Copy)** : Bun utilise `mimalloc` comme allocateur global, tout comme notre projet `google_os`. Une allocation mémoire en Rust est instantanément utilisable dans le moteur JavaScript via `TypedArray`, sans conversion ni copie.
- **Accès direct au moteur JSC (JavaScriptCore)** : Au lieu de passer par des API génériques lentes, vous accédez directement aux types natifs `JSValue`, `JSObject`, `JSString` via les crates `bun_jsc`.
- **Liaison Statique (LTO)** : Pas de `dlopen()` à l'exécution. Votre code Rust est inliné par le compilateur directement dans l'Event Loop de Bun.
- **SIMD Natif** : Bun embarque ses propres bindings pour `simdutf` et `highway`, permettant des traitements de chaînes et de buffers accélérés matériellement.

---

## 2. Cartographie des Crates Rust de Bun

L'écosystème Bun-RS est modulaire. Voici le détail factuel des crates majeures à votre disposition dans le workspace.

### A. Le Cœur du Moteur (Core & Runtime)
*Ces crates gèrent l'initialisation, la mémoire, et l'exécution globale du moteur JavaScript.*

* **`bun_core`** : La colonne vertébrale. Contient les primitives de base, la gestion des chaînes (BunString, ZigString), le formatage, et les abstractions des Feature Flags.
* **`bun_runtime`** : Implémente l'exécution des scripts (DevServer, Hot Reload, module resolution) et les API globales exposées à JavaScript (`Bun.serve`, `Bun.file`).
* **`bun_alloc`** : Fait le pont avec l'allocateur `mimalloc` et gère le Garbage Collector (GC) conjointement avec JavaScriptCore.
* **`bun_event_loop`** : L'intégration asynchrone bas-niveau (epoll/kqueue/IOCP selon l'OS) qui orchestre les I/O non-bloquants.

### B. Moteur JavaScriptCore (JSC & Bindings)
*Ces crates sont cruciales pour exposer des classes Rust directement au JavaScript.*

* **`bun_jsc`** : Les bindings Rust de bas niveau vers l'API C/C++ de WebKit/JavaScriptCore. Contient les types comme `JSGlobalObject`, `JSValue`, `JSString`. C'est l'équivalent de `v8-rs` pour l'écosystème WebKit.
* **`bun_jsc_macros`** : Macros procédurales très puissantes (ex: `#[jsc_class]`, `#[jsc_method]`) permettant de générer automatiquement tout le code "glue" (LUT - LookUp Tables) pour exposer une structure Rust en tant que classe JavaScript.

### C. Transpilation, AST & Bundling
*Bun est célèbre pour sa vitesse de transpilation TypeScript/JSX. C'est ici que ça se passe.*

* **`bun_ast`** : Le parseur d'Arbre Syntaxique Abstrait (AST) ultra-rapide. Il lit le TS/JS/JSX et le transforme en structures Rust orientées données (Struct-of-Arrays).
* **`bun_transpiler`** : Convertit l'AST (TS/JSX) en JavaScript pur exécutable par JSCore.
* **`bun_bundler`** : La logique de résolution de modules, de tree-shaking et de combinaison de fichiers (le cœur de `bun build`).
* **`bun_css`** / **`bun_css_jsc`** : Parseur et bundler natif pour les fichiers CSS (compatibilité Tailwind, modules CSS).

### D. I/O, Réseau & Web
*Bun remplace Libuv par ses propres implémentations I/O.*

* **`bun_io`** : Primitives d'entrées/sorties natives (lecture de fichiers, streams).
* **`bun_uws`** & **`bun_uws_sys`** : Bindings vers *uWebSockets*, le moteur HTTP et WebSocket ultra-performant utilisé par `Bun.serve()`.
* **`bun_http`** / **`bun_http_types`** : Abstractions pour les requêtes/réponses HTTP internes.
* **`bun_dns`** : Résolution DNS asynchrone native.

### E. Utilitaires Haute Performance & Cryptographie
*Pour les algorithmes gourmands en CPU.*

* **`bun_simdutf_sys`** : Bindings vers la librairie C++ `simdutf` pour valider et transcoder de l'UTF-8 / UTF-16 à des vitesses de l'ordre du Gigaoctet par seconde.
* **`bun_highway`** : Exploitation de Google Highway pour du SIMD portable (AVX2, NEON, SVE).
* **`bun_wyhash`** & **`bun_hash`** : Algorithmes de hachage non-cryptographiques extrêmement rapides (Wyhash, CityHash, RapidHash) utilisés en interne pour les HashMaps de Bun.
* **`bun_boringssl`** : Bindings stricts vers le fork de BoringSSL utilisé par Google/Chromium pour la cryptographie (TLS, WebCrypto).

### F. Écosystème & Tooling
* **`bun_install`** : Le cœur de `bun install` (résolution de graphe de dépendances, lockfile, téléchargement parallèle).
* **`bun_sqlite`** : Driver natif pour `bun:sqlite` basé sur une intégration Zero-Copy de SQLite3.
* **`bun_lolhtml_sys`** : Bindings vers la librairie de Cloudflare pour la réécriture HTML à la volée (streaming HTML parser/rewriter).

---

## 3. Guide Pratique : Créer une classe JS en Rust

Avec cet arsenal, écrire une librairie pour Bun ne consiste plus à créer un module N-API isolé, mais à s'injecter dans JavaScriptCore en utilisant les macros de Bun.

Voici un exemple théorique d'implémentation d'une API Rust exposée à Bun :

```rust
use bun_jsc::{JSGlobalObject, JSValue, JSObject};
use bun_jsc_macros::{jsc_class, jsc_method};

// 1. Déclarer la structure Rust
#[jsc_class]
pub struct FastMath {
    multiplier: u32,
}

// 2. Implémenter la logique métier et l'exposer à JS
#[jsc_class]
impl FastMath {
    // Constructeur appelé via `new FastMath(5)` en JS
    #[jsc_method(constructor)]
    pub fn new(_global: &mut JSGlobalObject, args: &[JSValue]) -> Self {
        let multiplier = args.get(0).and_then(|v| v.as_u32()).unwrap_or(1);
        Self { multiplier }
    }

    // Méthode rapide (sans alloc, FFI direct)
    #[jsc_method]
    pub fn compute(&self, _global: &mut JSGlobalObject, input: u32) -> JSValue {
        let result = input.wrapping_mul(self.multiplier);
        JSValue::from_u32(result)
    }
}
```

### Comment l'intégrer ?
1. Ajoutez le module dans l'arbre principal (ex: dans `bun_ffi`).
2. Lors de l'initialisation du moteur (via `GoogleOS_Init` ou le `DevServer`), enregistrez la classe dans le `JSGlobalObject`.
3. Le code JS pourra faire :
   ```javascript
   const math = new FastMath(42);
   console.log(math.compute(10)); // 420
   ```

## 4. Précautions & Mode Nightly
Bun s'appuie fortement sur l'état de l'art du compilateur Rust (en l'occurrence l'édition 2024 via une version Nightly verrouillée, ex: `nightly-2026-05-15`). 
L'usage de fonctionnalités instables (`#![feature(type_info)]`, `#![feature(core_intrinsics)]`) pour la réflexion de types (utilisée par le Garbage Collector et l'AST SoA) implique que les mises à jour du compilateur doivent être gérées avec précaution via le fichier `rust-toolchain.toml`.
