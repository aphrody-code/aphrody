---
name: rust-best-practices-2026
description: "Rust 1.95 features + 1.96 WASM breakage + Cargo CVE-2026-33056 + Edition 2024 async closures / precise capturing / Tokio discipline. Use when writing new Rust code in aphrody workspace, reviewing PRs, or upgrading toolchain."
version: 1.0.0
source: https://blog.rust-lang.org/ + rust-lang.org WebFetch 2026-05-19
---

# Rust best practices — May 2026 snapshot

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant complétion.

Synthèse source-of-truth pour tout code Rust écrit dans le workspace courant.
Couvre **Rust stable courant (1.95.0, 2026-04-16)** + **breakage à venir 1.96
(2026-05-28)** + **CVE-2026-33056** + **edition 2024 idioms matures**.

---

## 1. Toolchain pinning courant

| Slot | Version | Notes |
|---|---|---|
| Stable courant | **1.95.0** | shipped 2026-04-16 |
| Patch latest | **1.94.1** | shipped 2026-03-26 (tar/CVE fix backport) |
| Prochain stable | **1.96.0** | 2026-05-28 — **breakage WASM** (cf. §4) |
| Edition | **2024** | stable depuis 1.85 (2025-02), 2021 deprecated pour nouveau code |
| MSRV cible aphrody | nightly-2026-05-17 (cf. `rust-toolchain.toml`) | re-pin = PR |

---

## 2. Rust 1.95 — nouvelles APIs à utiliser

### Langage

| Feature | Remplace | Exemple |
|---|---|---|
| **`cfg_select!` macro** | `cfg-if` crate | `cfg_select! { unix => { mod posix; } windows => { mod win; } }` |
| **if-let guards dans match** | match nested + `if let` | `match x { Foo(y) if let Some(z) = y.bar() => …, _ => … }` |

⚠ **Exhaustiveness** : le compilateur **ne traite pas** les patterns d'`if let` guards
comme exhaustif. Toujours fournir un `_ =>` ou prouver l'exhaustivité ailleurs.

### Stable APIs nouvelles (utiliser plutôt que workarounds)

```rust
// MaybeUninit/Cell ↔ array conversions (zéro-copy, zéro-cost)
let a: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
let b: MaybeUninit<[u8; 4]> = a.into();                   // From conversion
let c: &MaybeUninit<[u8; 4]> = b.as_ref();                // AsRef

// Atomic update — CAS-loop intégré, plus de while-let manuel
let atomic = AtomicUsize::new(0);
let old = atomic.update(Ordering::SeqCst, Ordering::SeqCst, |v| Some(v + 1));
let opt = atomic.try_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_add(1));

// Vec / VecDeque / LinkedList push_mut / insert_mut (retourne &mut T)
let mut v = vec![1, 2];
let pushed: &mut i32 = v.push_mut(3);    // évite re-index après push
*pushed = 99;

// Pointer as_ref_unchecked / as_mut_unchecked (const + mut)
let raw: *const u8 = ptr::null();
unsafe { let _: &u8 = raw.as_ref_unchecked(); }            // ⚠ UB si null

// bool depuis entier (TryFrom)
let b = bool::try_from(0u8)?;                              // 0 → false, 1 → true, autre → Err

// core::range — ranges 1st-class avec iterator
use core::range::{Range, RangeFrom, RangeInclusive};
let r = Range { start: 0, end: 10 };

// core::hint::cold_path — branche froide pour branch prediction
if rare_condition {
    core::hint::cold_path();
    handle_rare_case();
}

// const-stabilizations utiles
const FN: fmt::Arguments = fmt::from_fn(|f| f.write_str("hi"));
const X: bool = ControlFlow::<()>::Break(()).is_break();
```

### Removals breakers

- **Custom target specs JSON** retirés de `rustc` **stable** (nightly only). Pour
  aphrody : aucun impact (`x86_64-{linux-gnu,pc-windows-msvc}` + `wasm32-unknown-unknown`
  sont des cibles built-in). Si quelqu'un essaie d'ajouter un target.json custom →
  refuser, utiliser un cfg ou un nouveau triple built-in.

---

## 3. Edition 2024 — patterns matures

### Async closures (stable 2024) — préférer aux `Box<dyn Future>`

```rust
// AVANT (boxed future, alloc + dyn dispatch)
fn middleware<F>(handler: F) where F: Fn() -> Pin<Box<dyn Future<Output=()>>> { … }

// APRÈS (2024 async closure, zéro alloc)
fn middleware<F>(handler: F) where F: AsyncFn() { … }
middleware(async || { do_work().await; });
```

Traits prelude 2024 : `AsyncFn`, `AsyncFnMut`, `AsyncFnOnce`.

### Précis capturing — `+ '_` souvent superflu en traits

```rust
// 2024 edition : capture rule = "toutes les input lifetimes captured"
trait Service {
    fn call(&self, req: Request) -> impl Future<Output = Response>;
    //                                ^ pas besoin de + '_  (capture &self auto)
}

// Syntaxe `use<'a, T>` pour capture explicite quand besoin
fn parse<'a>(s: &'a str) -> impl Iterator<Item=&'a str> + use<'a> { … }
```

### `unsafe_op_in_unsafe_fn` lint warn-by-default

```rust
unsafe fn raw_call(p: *const u8) {
    // ❌ 2024 warn : doit être dans un unsafe {} block explicite
    let v = *p;

    // ✅ explicite
    let v = unsafe { *p };
}
```

### `static mut` deny-by-default

Plus jamais `static mut FOO: Vec<u8> = …`. Utiliser `Mutex` / `RwLock` / `OnceLock`
/ `LazyLock` / `Atomic*`.

---

## 4. Rust 1.96 (2026-05-28) — WASM breakage à anticiper

**Changement** : retrait de `--allow-undefined` du linker `wasm-ld` pour TOUS les
targets `wasm32-*`. Tracking : [rust-lang/rust#149868](https://github.com/rust-lang/rust/pull/149868).

### Avant 1.96 (silent import)

```rust
unsafe extern "C" {
    fn foo();                          // imports `foo` depuis env (silently)
}
static nonexistent: u8;                // imports `nonexistent` (silently)
```

### À partir de 1.96 (link-error)

```rust
// ✅ Option A — annoter le module hôte explicitement
#[link(wasm_import_module = "host")]
unsafe extern "C" {
    fn foo();                          // imports "host"::foo
}

// ✅ Option B — escape hatch temporaire (à retirer ASAP)
// dans .cargo/config.toml :
// [target.wasm32-unknown-unknown]
// rustflags = ["-Clink-arg=allow-undefined"]
```

### Action pour aphrody

1. Audit `crates/aphrody-wasm/`, `crates/cli/` (si compile en wasm), et tout crate
   avec `extern "C"` blocks ciblant wasm.
2. Ajouter `#[link(wasm_import_module="…")]` partout où applicable.
3. Si breakage : pin temporairement `rust-toolchain.toml` ≤ `nightly-2026-05-27`
   le temps de migrer.

---

## 5. CVE-2026-33056 — Cargo tar extraction

**Affected** : `cargo` ≤ 1.94.0 avec **registries privés / alternate** (crates.io
patché côté serveur le 2026-03-13).

**Vector** : crate malveillant peut modifier permissions de répertoires arbitraires
du filesystem pendant `cargo build` / `cargo install` (via vulnérabilité crate `tar`).

**Fix** : bump à `cargo` ≥ **1.94.1** (sortie 2026-03-26) ou ≥ 1.95.

**Action pour aphrody** :

- Workspace utilise nightly-2026-05-17 → cargo ≥ 1.95 → **patched**.
- `deny.toml` doit déjà bannir `tar < 0.4.45` (à vérifier).
- Si on ajoute un registry alternate (`[registries]` dans `.cargo/config.toml`),
  confirmer auprès du fournisseur qu'il a re-scanné ses crates.

```toml
# deny.toml — ajouter si pas déjà
[[bans.deny]]
name = "tar"
version = "<0.4.45"
reason = "CVE-2026-33056 — directory permission tampering during extraction"
```

---

## 6. Send-bound problem (async traits + tower)

**Problème** : pas moyen aujourd'hui d'écrire une fn générique qui exige `impl Trait`
async returning `Send` futures → bloque `tower` middleware sur AFIT.

**Workaround courant 2026** :

| Cas | Solution |
|---|---|
| Lib veut `dyn Trait` async | crate **`dynosaur`** v0.3 (proc macro, parité `async-trait` mais natif AFIT) |
| Lib veut bound `Send` sur `impl Trait` | trait + `trait MyServiceSend: MyService<…: Send> {}` (verbeux mais portable) |
| Service tower | continuer `async-trait` 0.1 jusqu'à RFC `return_type_notation` stable |

---

## 7. Tokio discipline 2026

```rust
// ❌ kill performance — bloque worker thread
async fn read_config() -> Vec<u8> {
    std::fs::read("config.toml").unwrap()      // syscall blocking !
}

// ✅ offload sur thread pool dédié
async fn read_config() -> Vec<u8> {
    tokio::task::spawn_blocking(|| std::fs::read("config.toml").unwrap())
        .await
        .unwrap()
}

// ✅ ou utiliser tokio::fs (async wrappers)
async fn read_config() -> std::io::Result<Vec<u8>> {
    tokio::fs::read("config.toml").await
}
```

### `tokio::select!` cancellation safety

```rust
// ⚠ chaque branch non-completed est DROPPED — l'opération est annulée mid-flight
tokio::select! {
    r = reader.read_buf(&mut buf) => { … }    // ✅ cancellation-safe (Tokio AsyncRead)
    _ = tokio::time::sleep(d) => { … }
}

// ❌ NON cancellation-safe — perd des données si timeout race
tokio::select! {
    items = collect_into_vec() => { … }       // collect partiel dropé, items perdus
    _ = sleep(d) => { … }
}
```

Règle : seules les opérations documentées **cancellation-safe** vont dans
`tokio::select!`. Sinon, encapsuler dans `tokio::spawn` + abort handle.

---

## 8. Outils workflow conseillés (cf. skill `cross-platform-cli-toolbelt`)

| Tâche | Outil 2026 |
|---|---|
| Compile/check rapide | `cargo check --offline` + `cargo nextest run --offline` |
| Benchmark | `criterion` + `hyperfine` (CLI comparison) |
| Unsafe verify | `cargo +nightly miri test --workspace --lib` |
| Lint strict | `cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic` |
| Macro debug | `cargo expand` |
| Cross-compile Linux ← Windows | `cargo zigbuild --target x86_64-unknown-linux-gnu` |
| Doc | `cargo doc --no-deps --workspace` |
| Supply-chain | `cargo deny check` + `cargo vet` + `cargo audit` |
| Install binaire | `cargo binstall <crate>` (pré-build, **pas** `cargo install`) |

---

## 9. Sources

- [Rust Blog](https://blog.rust-lang.org/)
- [Announcing Rust 1.95.0 (2026-04-16)](https://blog.rust-lang.org/2026/04/16/Rust-1.95.0/)
- [Security advisory CVE-2026-33056 (2026-03-21)](https://blog.rust-lang.org/2026/03/21/cve-2026-33056/)
- [WASM target changes (2026-04-04)](https://blog.rust-lang.org/2026/04/04/changes-to-webassembly-targets-and-handling-undefined-symbols/)
- [Modern Rust Best Practices in 2026 (onehorizon.ai)](https://onehorizon.ai/blog/modern-rust-best-practices-in-2026-beyond-the-borrow-checker)
- [Announcing Rust 1.85.0 and Rust 2024](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)
