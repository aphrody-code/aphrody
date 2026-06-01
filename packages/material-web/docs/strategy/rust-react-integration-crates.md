<!-- SPDX-License-Identifier: Apache-2.0 -->

# Intégration Rust & React — Sélection des meilleures crates (2026)

> Analyse stratégique et recommandations de crates Rust pour connecter notre monorepo React / Lit avec notre socle natif Rust (`bun-rs`).

---

## 1. Vision générale et objectifs

Dans le cadre du monorepo `material-web`, Rust intervient pour optimiser les tâches lourdes ou critiques (génération dynamique de thèmes HCT, compilation Sass, compilation/scaffolding de design, l'analyse statique ou la validation). Pour intégrer Rust à notre frontend React/TypeScript de manière performante et robuste, nous devons choisir des crates de liaison (WASM, FFI, sérialisation et synchronisation de types) répondant aux critères stricts de la stack 2026 :
1. **Performance maximale** : Zéro double-sérialisation inutile sur la frontière JS/Rust.
2. **Sécurité des types** : Zéro dérive de types (type drift) entre les structures de données Rust et l'application React en TypeScript.
3. **Robustesse cross-platform** : Compatibilité totale Linux (priorité #1), Windows et WASM.
4. **Licence permissive** : Licence Apache-2.0 ou MIT compatible avec le projet.

---

## 2. Tableau comparatif et recommandations

| Domaine | Crate recommandée | Version | Alternative rejetée | Raison de l'exclusion / Rationale |
| :--- | :--- | :--- | :--- | :--- |
| **Liaison WASM** | **`wasm-bindgen`** | `0.2` | Liaison C FFI brute | Standard de l'écosystème pour compiler du Rust vers le web. |
| **Sérialisation WASM** | **`serde-wasm-bindgen`** | `0.6` | `serde_json` | Évite l'allocation de chaînes intermédiaires en traduisant directement les structures Rust en objets JS (`JsValue`). |
| **Génération de Types** | **`ts-rs`** | `10.x` | `specta` / Synchro manuelle | `ts-rs` génère des interfaces `.ts` fiables via macros de dérivation, sans introduire la complexité RPC de `specta` (qui est plutôt orientée Tauri). |
| **Asynchronisme** | **`wasm-bindgen-futures`**| `0.4` | Promesses JS brutes | Permet de convertir les `Promise` JS en `Future` Rust et inversement pour un code asynchrone unifié. |
| **Background Tasks** | **`gloo-worker`** | `0.2` | Web Workers JS manuels | Abstraction de haut niveau de type *Worker* permettant de déporter les calculs lourds (ex: HCT/Sass) sur un thread d'arrière-plan sans bloquer la boucle d'événements de React. |
| **Shell Desktop/Mobile** | **`tauri`** | `2.x` | `electron` / `wry` brut | Tauri 2.0 offre une API stable desktop/mobile sécurisée avec une empreinte mémoire minimale, évitant les failles GTK3 de `wry` sur Linux. |

---

## 3. Focus Technique & Intégration

### A. Passage de données performant (WASM FFI)
Pour envoyer des structures de données complexes (configurations de thèmes, palettes HCT) de React vers Rust WASM :

- **Recommandation :** Utiliser **`serde-wasm-bindgen` 0.6**.
- **Pourquoi rejeté :** Utiliser `serde_json::to_string` côté JS puis `serde_json::from_str` côté Rust introduit une double-sérialisation inutile, dégradant les performances. `serde-wasm-bindgen` convertit directement les types Rust en objets structurés JS natifs via la structure `JsValue`.

**Exemple d'implémentation Rust :**
```rust
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct ThemeConfig {
    pub seed_color: String,
    pub dark_mode: bool,
    pub contrast_level: f64,
}

#[wasm_bindgen]
pub fn generate_dynamic_theme(config_val: JsValue) -> Result<JsValue, JsValue> {
    // Désérialisation directe optimisée depuis JsValue
    let config: ThemeConfig = serde_wasm_bindgen::from_value(config_val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
    // (Logique de calcul M3 / HCT...)
    
    // Sérialisation directe vers JsValue
    serde_wasm_bindgen::to_value(&config)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

---

### B. Synchronisation des types (React TSX ↔ Rust)
Pour garantir la cohérence des structures de données :

- **Recommandation :** Utiliser **`ts-rs` 10.x**.
- **Pourquoi rejeté :** L'écriture manuelle de fichiers `.d.ts` ou de types TypeScript séparés crée des erreurs d'intégration silencieuses lors du refactoring de notre socle Rust.

**Exemple d'implémentation Rust (dans notre workspace) :**
```rust
use ts_rs::TS;
use serde::Serialize;

#[derive(TS, Serialize)]
#[ts(export, export_to = "../react/src/types/ThemeConfig.ts")]
pub struct ThemeConfig {
    pub seed_color: String,
    pub dark_mode: bool,
    pub contrast_level: f64,
}
```

Les types TypeScript sont exportés automatiquement lors de l'exécution de `cargo test` ou d'une étape de compilation dédiée.

---

### C. Déchargement de threads (Calculs hors thread principal React)
Pour éviter de figer l'interface React lors de la génération de thèmes complexes ou de l'analyse statique :

- **Recommandation :** Utiliser **`gloo-worker` 0.2**.
- **Pourquoi rejeté :** L'implémentation manuelle de `postMessage` et des écouteurs d'événements côté React est verbeuse et propice aux fuites de mémoire. `gloo-worker` fournit une abstraction de type *Bridge* asynchrone et typée.

---

## 4. Licence

Toutes les crates sélectionnées sont publiées sous des licences permissives compatibles :
- **`wasm-bindgen`** : MIT / Apache-2.0
- **`serde-wasm-bindgen`** : MIT / Apache-2.0
- **`ts-rs`** : MIT
- **`gloo-worker`** : MIT / Apache-2.0
- **`tauri`** : MIT / Apache-2.0

*Aucune dépendance contaminante (de type GPL/AGPL) n'est introduite dans le cycle de build ou dans le binaire client.*
