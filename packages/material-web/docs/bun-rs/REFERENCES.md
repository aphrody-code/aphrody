# Référence technique de Bun-RS : C-ABI, FFI & Layouts mémoire

Ce document sert de source de vérité technique pour le développement des frontières FFI entre Rust et JavaScript/TypeScript dans Bun.

---

## 🦀 Directives C-ABI de Rust

Pour exporter avec succès une fonction Rust vers Bun FFI, elle doit être déclarée avec une interface compatible C stable et suivre les spécifications Rust 2024.

### 1. L'attribut `#[unsafe(no_mangle)]`

À partir de Rust 2024 (Édition 2024), la suppression du mangling est catégorisée comme unsafe. L'attribut doit être déclaré comme suit :

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ma_fonction() { ... }
```

Cela force le compilateur Rust à exporter la fonction avec son nom exact dans la table des symboles, permettant à `bun:ffi` de la localiser.

### 2. Convention d'appel Extern "C"

Les fonctions exportées doivent utiliser la convention d'appel standard `C` :

```rust
pub extern "C" fn bun_rs_add(a: i32, b: i32) -> i32 {
    a + b
}
```

---

## 🔀 Mappages de types : Rust vs Bun FFI

Le tableau suivant mappe les types courants de la bibliothèque dynamique Rust avec la configuration de signature `dlopen` en JavaScript :

| Type Rust       | Type de configuration Bun FFI | Type JavaScript / TypeScript | Description                                     |
| --------------- | ----------------------------- | ---------------------------- | ----------------------------------------------- |
| `i32`           | `"i32"`                       | `number`                     | Entier signé 32 bits                            |
| `u32`           | `"u32"`                       | `number`                     | Entier non signé 32 bits                        |
| `usize`         | `"usize"`                     | `number` / `bigint`          | Entier non signé de la taille d'un pointeur     |
| `isize`         | `"isize"`                     | `number` / `bigint`          | Entier signé de la taille d'un pointeur         |
| `*const u8`     | `"ptr"`                       | `TypedArray` / `Pointer`     | Pointeur brut vers un tableau de octets         |
| `*mut u8`       | `"ptr"`                       | `TypedArray` / `Pointer`     | Pointeur brut emprunté de manière mutable       |
| `*const c_char` | `"cstring"`                   | `string` / `Pointer`         | Chaîne de style C terminée par un caractère nul |
| `*mut c_char`   | `"cstring"`                   | `string` / `Pointer`         | Chaîne de style C allouée (doit être libérée)   |

---

## 🔒 Sécurité et gestion des pointeurs

Lors du passage de la frontière JS/Rust, la sécurité de la mémoire est primordiale. Suivez ces règles pour éviter les erreurs de segmentation et les fuites de mémoire.

### 1. Durée de vie des pointeurs et libération de la mémoire

JavaScript est géré par un Garbage Collector (GC), tandis que Rust utilise des durées de vie explicites.

- **Mémoire appartenant à JS** : Lors du passage d'un pointeur `TypedArray` à l'aide de `ptr(buffer)` à Rust, le buffer **ne doit pas** être collecté par le ramasse-miettes pendant que Rust s'exécute.
- **Mémoire appartenant à Rust** : Si Rust alloue de la mémoire (`String`, `Vec`) et la renvoie à JS sous forme de pointeur, Rust **doit** exposer une fonction de libération personnalisée pour la désallouer :

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: Reprise de propriété de la chaîne pour la laisser tomber (drop) et la désallouer.
    unsafe {
        let _ = std::ffi::CString::from_raw(ptr);
    }
}
```

### 2. Tranches (Slices) à partir de pointeurs bruts

Vérifiez toujours la présence de pointeurs nuls avant de convertir des pointeurs bruts en tranches Rust :

```rust
if data.is_null() || len == 0 {
    return 0;
}
// SAFETY: L'appelant doit garantir que la région mémoire est valide.
let slice = unsafe { std::slice::from_raw_parts(data, len) };
```

---

## 🌐 Mappages de types WebAssembly (WASM)

`wasm-bindgen` convertit et gère automatiquement le passage des types de données standard entre le tas du navigateur JavaScript et la mémoire WebAssembly. Voici les correspondances :

| Type Rust         | Type JavaScript    | Type TypeScript généré | Description / Rôle                                                         |
| ----------------- | ------------------ | ---------------------- | -------------------------------------------------------------------------- |
| `i32` / `u32`     | `number`           | `number`               | Entiers standards convertis par valeur.                                    |
| `f32` / `f64`     | `number`           | `number`               | Nombres flottants convertis par valeur.                                    |
| `bool`            | `boolean`          | `boolean`              | Valeur booléenne JS standard.                                              |
| `String` / `&str` | `string`           | `string`               | Chaînes de caractères encodées en UTF-8 copiées dans la mémoire WASM.      |
| `Vec<u32>`        | `Uint32Array`      | `Uint32Array`          | Tableaux typés d'entiers non signés 32 bits.                               |
| `Vec<f32>`        | `Float32Array`     | `Float32Array`         | Tableaux typés de nombres à virgule flottante.                             |
| `Result<T, E>`    | Valeur / Exception | `T` (or throws)        | Les `Result::Err` de Rust sont rejetés sous forme d'exceptions JS natives. |
