# FFI Policy — Rust ↔ C/C++ ↔ Bun

> Réf. : `crates/bun_ffi/`, `crates/google_os/`, `crates/python_ffi/`.
> Politique mémoire : `mimalloc` global, zero-copy, ownership transfert explicite.

## Principes

1. **Allocateur unique** : `mimalloc` est `#[global_allocator]` dans tout crate qui touche à la FFI. Pas de divergence entre deux allocateurs (jemalloc / system) qui causerait double-free ou leak silent.
2. **Zero-copy strict** : aucune copie mémoire entre Rust et le côté C/Bun/Python si évitable. Utilisation de raw pointers + `mem::forget` pour transférer ownership.
3. **Documentation `# Safety`** obligatoire sur chaque fonction `unsafe extern "C"`.
4. **Pas de panic à travers FFI** : `[workspace.lints.rust] ffi_unwind_calls = "warn"`. Toute fonction `pub extern "C" fn` doit catch les panics ou utiliser `panic = "abort"` (déjà notre cas).

## Pattern de référence — `bun_ffi::wc_alloc/wc_free`

```rust
// crates/bun_ffi/src/lib.rs
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Alloue `size` octets de mémoire partagée zero-copy.
/// # Safety
/// Le pointeur retourné doit être libéré par `wc_free` avec la même taille.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wc_alloc(size: usize) -> *mut u8 {
    let mut vec = Vec::<u8>::with_capacity(size);
    let ptr = vec.as_mut_ptr();
    // Ownership is transferred to the C caller; `wc_free` is responsible
    // for reconstructing the `Vec` with the exact `size` and dropping it.
    #[allow(clippy::mem_forget)]
    std::mem::forget(vec);
    ptr
}

/// Libère la mémoire allouée par `wc_alloc`.
/// # Safety
/// Le pointeur doit provenir de `wc_alloc` avec la taille exacte (`size`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wc_free(ptr: *mut u8, size: usize) {
    if ptr.is_null() { return; }
    unsafe { drop(Vec::from_raw_parts(ptr, 0, size)); }
}
```

**Pourquoi `mem::forget` est-il safe ici ?**
- Le `Vec<u8>` ne contient pas de heap allocations imbriquées (`u8` est Copy).
- L'ownership est transmis explicitement à `wc_free` qui reconstruit le Vec.
- Le lint `clippy::mem_forget` est `deny` workspace-wide → l'`#[allow]` local marque l'intention.

## Bridge Bun → Rust → C++

```
┌─────────────┐  raw ptr   ┌──────────────┐  raw ptr   ┌─────────────┐
│   Bun (JS)  │ ─────────► │ Rust bun_ffi │ ─────────► │ C++ vendor  │
│             │ ◄───────── │   (mimalloc) │ ◄───────── │  (mimalloc) │
└─────────────┘   free     └──────────────┘   free     └─────────────┘
```

Tous les acteurs partagent **le même allocateur mimalloc** → pas de cross-allocator free.

## Bridge POSIX → google_os → NT

`google_os` implémente le shim libc → Win32 :

```rust
// crates/google_os/src/libc/io.rs
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    // 1. Convertir path C en OsString Rust (sans allocation si possible)
    // 2. Appeler CreateFileW via windows-rs (NT path, pas Win32 layer)
    // 3. Mapper HANDLE → fd via une table fd → HANDLE thread-safe
    // 4. Set errno selon le résultat
}
```

Règles :
- Jamais de panic dans une fonction libc shim.
- `errno` thread-local correctement set sur erreur.
- HANDLE Windows masqué derrière un fd integer style POSIX.
- `[lib] crate-type = ["cdylib", "rlib"]` → produit `google_os.dll` + librairie Rust pour usage workspace.

## Bridge PyO3 (`python_ffi`)

```rust
// crates/python_ffi/src/lib.rs
use pyo3::prelude::*;

#[pymodule]
fn google_cli_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(some_func, m)?)?;
    Ok(())
}
```

- `mimalloc` global comme partout.
- `bun_jsc` + `bun_jsc_macros` pour le pont JSC (V8 alternative).
- `features = ["auto-initialize", "abi3-py311"]` — ABI stable, supporte Python 3.11+.

## Lints workspace pertinents pour FFI

```toml
[workspace.lints.rust]
unsafe_op_in_unsafe_fn = "deny"   # FORCE unsafe {} explicite dans les fn unsafe
ffi_unwind_calls       = "warn"   # warn si une fn FFI peut unwind

[workspace.lints.clippy]
mem_forget             = "deny"   # mem::forget rarement légitime
ptr_as_ptr             = "allow"  # FFI casts fréquents
cast_ptr_alignment     = "allow"  # idem
undocumented_unsafe_blocks = "allow"  # toléré pendant migration
multiple_unsafe_ops_per_block = "allow"
```

## Audit FFI obligatoire avant merge

- [ ] Chaque `unsafe fn` a une section `# Safety`.
- [ ] Chaque bloc `unsafe { ... }` non trivial a un commentaire SAFETY.
- [ ] Aucune allocation cross-allocator.
- [ ] Tests `proptest` ou `loom` pour les invariants concurrents.
- [ ] `cargo miri test -p <crate>` passe (UB detector).

## Outils nightly

```bash
cargo +nightly miri test -p bun_ffi         # détecte UB dans tests
cargo +nightly miri test -p google_os
RUSTFLAGS="-Z sanitizer=address" \
    cargo build -p google_os --target x86_64-pc-windows-msvc   # AddressSanitizer
```
