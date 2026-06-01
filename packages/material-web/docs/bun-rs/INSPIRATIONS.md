# Inspirations de conception de Bun-RS

`bun-rs` s'inscrit dans la lignée des outils web modernes à haute performance écrits dans des langages de programmation système (comme Rust ou Go) pour remplacer les pipelines hérités basés sur JavaScript.

---

## ⚡ Inspirations clés

### 1. LightningCSS

- **Concept** : Un analyseur, transformateur, bundler et minificateur CSS extrêmement rapide écrit en Rust.
- **Philosophie** : Le traitement CSS ne devrait pas nécessiter le lancement de chaînes d'outils JS lourdes (comme PostCSS). En compilant directement en code natif, LightningCSS fonctionne à des vitesses plusieurs ordres de grandeur plus rapides.
- **Influence** : A inspiré le choix de Rust pour le pré-traitement au sein de `material-web`, démontrant que la compilation de styles peut être rapide, propre et sûre.

### 2. SWC & Oxlint

- **Concept** : Outils de compilation et de peluchage (linter) de nouvelle génération écrits en Rust.
- **Philosophie** : Déplacer l'analyse syntaxique, le peluchage et la génération d'AST du JavaScript vers le Rust natif élimine les temps de chauffe de l'optimisation V8 et la surcharge du processeur.
- **Influence** : Valide le choix d'utiliser une crate C-ABI native et légère comme `bun-rs` pour contourner la surcharge JavaScript lors des étapes de packaging et de build.

### 3. Grass (Sass en Rust)

- **Concept** : Un compilateur pour le langage Sass écrit en Rust pur.
- **Philosophie** : Évite la dépendance de Node-Sass vis-à-vis des bibliothèques C++ natives (problèmes de compilation de node-gyp) et la lourde surcharge d'invocation de sous-processus de Dart-Sass.
- **Influence** : Grass est la principale bibliothèque cible à envelopper dans `bun-rs` pour permettre une compilation Sass transparente et sans sous-processus.

### 4. FFI sans surcoût de Bun (Zero-Overhead FFI)

- **Concept** : Le runtime de Bun génère des wrappers optimisés et compilés à la volée (JIT) pour les appels FFI dynamiques.
- **Philosophie** : Charger des bibliothèques `.so`/`.dll` et appeler des fonctions natives devrait être aussi rapide que du C natif, en contournant les couches de marshaling standard de Node.js N-API/addon.
- **Influence** : Fournit la base technique sous-jacente qui rend les appels FFI de `bun-rs` pratiquement gratuits (~1,5ns de latence).

### 5. Écosystème Rust/WASM (Écrire une fois, exécuter partout)

- **Concept** : L'écosystème d'outillage WebAssembly de Rust (`wasm-pack`, `wasm-bindgen`) permet de compiler le même code source Rust sans modifications pour cibler à la fois les architectures système natives et la machine virtuelle WASM du navigateur.
- **Philosophie** : Éviter de dupliquer les algorithmes critiques (génération d'espace de couleurs HCT, validation de spécifications complexes de conception) en JavaScript pour l'exécution côté client. Une source unique de vérité en Rust garantit une parité absolue des comportements entre l'environnement serveur/compilation et le navigateur de l'utilisateur.
- **Influence** : A inspiré la compilation hybride WASM de `bun-rs` qui propulse la page Showcase avec des performances et une taille de bundle optimales.
