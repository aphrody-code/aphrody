# Bonnes pratiques de développement pour Bun-RS

Pour maintenir une frontière FFI sécurisée, stable et hautement performante, les développeurs doivent respecter les normes de codage suivantes lorsqu'ils contribuent à `bun-rs`.

---

## 🔒 Sécurité mémoire et durées de vie (Lifetimes)

Le code FFI désactive bon nombre des garanties de sécurité de Rust à la compilation. Suivez ces modèles stricts pour éviter tout comportement indéfini :

### 1. Vérification des pointeurs

Validez toujours que les pointeurs transmis depuis JavaScript sont non nuls et que les longueurs sont cohérentes avant de les convertir en tranches (slices).

```rust
if ptr.is_null() || len == 0 {
    return;
}
```

### 2. Ne jamais renvoyer de références allouées sur la pile (Stack)

Renvoyer un pointeur vers une variable locale créée à l'intérieur d'une fonction est un bug critique (pointeur suspendu / dangling pointer).

- **Correct** : Renvoyer un pointeur de chaîne littérale statique :
  ```rust
  c"1.0.0".as_ptr()
  ```
- **Correct** : Allouer sur le tas (heap) à l'aide de `Box` ou `CString` et renvoyer le pointeur brut, en veillant à implémenter une fonction de désallocation correspondante :
  ```rust
  let my_str = CString::new("dynamic content").unwrap();
  my_str.into_raw() // transfère la propriété à JS
  ```
- **Incorrect** : Renvoyer un pointeur vers une variable locale :
  ```rust
  let my_str = format!("version: {}", 1);
  my_str.as_ptr() // DANGER : mémoire désallouée dès que la fonction se termine !
  ```

---

## 🛑 Prévention des crashs et gestion des erreurs

Une panique à l'intérieur d'une fonction FFI interrompra l'ensemble du processus (faisant planter le runtime Bun).

### 1. Capturer les paniques (Catching Panics)

Utilisez `std::panic::catch_unwind` s'il y a le moindre risque de panique dans les bibliothèques Rust :

```rust
use std::panic::catch_unwind;

#[unsafe(no_mangle)]
pub extern "C" fn bun_rs_safe_divide(a: i32, b: i32) -> i32 {
    let result = catch_unwind(|| {
        a / b
    });
    match result {
        Ok(val) => val,
        Err(_) => -1, // gère la division par zéro en toute sécurité
    }
}
```

### 2. Renvoyer des enums d'erreur / codes d'état

Au lieu de paniquer en cas d'erreur, renvoyez des codes d'erreur ou des décalages d'état. Par exemple, les recherches de sous-chaînes renvoient `-1` pour signaler un échec plutôt que de propager une panique.

---

## 🧵 Sécurité des threads (Thread Safety)

Bun exécute JavaScript dans une boucle monothread, mais des tâches de longue durée ou des threads Worker peuvent déclencher des appels FFI concurrents.

- Assurez-vous que tout état global en Rust (`lazy_static`, `OnceLock`, etc.) utilise des primitives thread-safe (`Mutex`, `RwLock`, `Atomic`).
- Les fonctions exportées ne doivent pas bloquer le thread principal pendant de longues périodes. Si une opération prend plus de 1ms (ex. compilation de gros fichiers Sass), elle doit être déléguée à des workers asynchrones ou exécutée dans un thread d'arrière-plan à l'aide de canaux (channels).

---

## ⏱️ Discipline de benchmark et d'optimisation

Chaque nouvel ajout FFI doit être profilé.

- **Exécuter des benchmarks locaux** : Testez toujours la fonction JS FFI par rapport à son alternative JS pure dans `benchmark.js`.
- **Garder les appels FFI à gros grains** : La traversée de la frontière de langage (JS ↔ Rust) a un léger surcoût. Évitez d'appeler une fonction FFI des millions de fois dans une boucle JS serrée. Au lieu de cela, transmettez un grand buffer ou tableau à Rust une fois et traitez l'ensemble de l'ensemble dans un seul appel de fonction native.

---

## 🌐 WebAssembly (WASM) & Bonnes pratiques

La compilation pour le navigateur implique des contraintes différentes (taille du binaire, sécurité mémoire gérée par le runtime WASM). Respectez ces directives :

### 1. Préférer le passage de types managés par wasm-bindgen

Plutôt que d'échanger des pointeurs bruts comme en FFI, utilisez les conversions de type intégrées de `wasm-bindgen` (`String`, `Vec<T>`, slices comme `&str`) qui simplifient la gestion de la mémoire côté JavaScript et évitent les fuites.

### 2. Gérer proprement les erreurs via `Result`

Pour les fonctions WASM sujettes à échec (ex. compilation SCSS), renvoyez `Result<T, E>`. `wasm-bindgen` convertit automatiquement le retour en exception JavaScript standard que vous pouvez intercepter avec `try ... catch`.

### 3. Garder un œil sur la taille du bundle

L'inclusion de dépendances complexes (comme le parser Sass `grass`) augmente la taille finale du fichier `.wasm`.

- Activez toujours l'optimisation de taille `opt-level = 'z'` et LTO dans le profil de release Cargo.
- Nettoyez les binaires produits à l'aide de `wasm-opt -Oz`.
- Servez les fichiers `.wasm` compressés en Gzip ou Brotli côté serveur (le fichier `.wasm` de 1.9 Mo passe sous la barre des 400 Ko compressé).
