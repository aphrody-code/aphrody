# Google Rust Crates — Evaluation & Integration

Évaluation de l'ensemble des crates Rust publiées par l'organisation GitHub `google`,
triées par étoiles. Résultat : 2 crates intégrées réellement dans `aphrody-re`.

---

## Tableau d'évaluation

| Crate | Stars | Licence | Dernière version | Dernier commit | Verdict |
|-------|-------|---------|-----------------|---------------|---------|
| `zerocopy` | 2 310 | BSD-2-Clause OR Apache-2.0 OR MIT | 0.8.48 (stable) | 2026-05-21 | **INTEGREE** |
| `googletest` | 414 | Apache-2.0 | 0.14.2 | 2026-05-22 | **INTEGREE** |
| `fancy-regex` | 165 | MIT | 0.18.0 | stable | Déjà intégrée (`google` module) |
| `autocxx` | 2 535 | MIT OR Apache-2.0 | 0.30.0 | 2025-03-05 | **REJETEE** — maintenance en doute (issue #1507 open), requiert bindgen/LLVM au build, incompatible wasm32 confirmé par issue #1508 |
| `tarpc` | 3 707 | MIT | 0.37.0 | 2026-03-25 | Non retenu — RPC framework non pertinent (aphrody utilise A2A/gRPC) |
| `argh` | 1 915 | BSD-3-Clause | 0.1.19 | 2026-05-06 | Non retenu — aphrody utilise `clap`, inutile d'ajouter un second parseur |
| `mundane` | 1 081 | non-standard | 0.5.0 | — | **REJETE** — licence non-standard (ni Apache, ni MIT, ni BSD) |
| `assertor` | 151 | Apache-2.0 | 0.0.4 | — | Non retenu — abandonné depuis 2022, supplanté par `googletest` |
| `forma` | 2 642 | Apache-2.0 | — | — | Non retenu — moteur de rendu 2D/GPU ; hors périmètre |
| `rust_icu` | 136 | Apache-2.0 | — | — | Non retenu — bindings ICU, pas de besoin dans aphrody-re |
| `shaderc-rs` | 286 | Apache-2.0 | — | — | Non retenu — compilation GLSL/HLSL ; hors périmètre |
| `native-pkcs11` | 72 | Apache-2.0 | — | — | Non retenu — PKCS#11 ; hors périmètre |
| `gpt-disk-rs` | 70 | Apache-2.0 | — | — | Non retenu — parsing GPT disk ; niche, déjà couvert par goblin pour PE/ELF |

---

## Crates intégrées

### 1. `zerocopy` 0.8.48 — Zero-copy header inspection

**Où :** `crates/aphrody-re/src/headers.rs` (nouveau module), déclarée dans
`crates/aphrody-re/Cargo.toml` (existait déjà en workspace dep, aucun code ne
l'utilisait avant cette PR).

**Ce qui a été implémenté :**

- `ElfIdent` — struct `#[repr(C)]` de 16 octets avec derives `FromBytes +
  IntoBytes + KnownLayout + Immutable`. Lit le tableau `e_ident` ELF par
  `zerocopy::Ref::from_bytes` sans copie ni allocation.
- `DosHeader` — struct `#[repr(C)]` de 64 octets utilisant `zerocopy::little_endian::*`
  pour tous les champs entiers (layout LE garanti statiquement, indépendant de
  l'endianness hôte). Expose `pe_offset()` via `e_lfanew`.
- `PeMagic` — enum résolvant le magic du PE optional header (0x010B = PE32,
  0x020B = PE32+) à partir des seuls premiers octets du fichier.
- `HeaderProbe` — façade publique zero-copy ; renvoie `Elf { is_64 }`,
  `Pe { is_64 }`, ou `Unknown` en lisant au plus 64 octets, zéro allocation.
- 18 tests unitaires avec `#[gtest]` + `expect_that!` (googletest).

**Compatibilité wasm32 :** confirmée (`cargo check -p aphrody-re --target
wasm32-unknown-unknown` vert).

**Note technique :** les macros `zerocopy-derive` 0.8 génèrent des identifiants
internes non-ASCII sur nightly 1.97, ce qui déclenche le lint workspace
`non_ascii_idents = "deny"`. Correction : `#![allow(non_ascii_idents)]` ajouté
au crate root `lib.rs` (standard documenté dans les output_tests de
zerocopy-derive).

---

### 2. `googletest` 0.14.2 — Matchers GoogleTest-style

**Où :** `crates/aphrody-re` (dev-dependency existante, aucun usage réel avant).
Usage concret dans deux fichiers :

- `crates/aphrody-re/src/headers.rs` — 18 tests `#[gtest]` avec `expect_that!`,
  `eq`, `none`, `some`, `predicate`.
- `crates/aphrody-re/src/lib.rs` — 16 tests `#[test]` migrés vers `#[gtest]` +
  `expect_that!` (anciens `assert_eq!` / `assert!` remplacés par des matchers
  lisibles : `contains(predicate(...))`, `len(eq(N))`, `empty()`, `le()`, etc.).

**Matchers utilisés :**
`eq`, `none`, `some`, `predicate`, `contains`, `len`, `le`, `not`, `empty`.

**Note :** `expect_that!` nécessite `#[gtest]` (ou `#[googletest::test]`) —
pas `#[test]` — pour configurer le contexte de test. Sans ce decorator,
le test panique avec "No test context found". Tous les tests ont été annotés
en conséquence.

---

## Sorties cargo vérifiées

```
# Check lib avec zerocopy + googletest
cargo check -p aphrody-re
    Checking aphrody-re v1.0.0-canary
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.36s

# Wasm32 cross-compile (cible #3)
cargo check -p aphrody-re --target wasm32-unknown-unknown --locked
    Checking aphrody-re v1.0.0-canary
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.95s

# Tests (nextest)
cargo nextest run -p aphrody-re --locked --no-fail-fast
    Summary   0.367s   113 tests run: 113 passed, 2 skipped

# CLI binary non cassé
cargo check -p aphrody --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4m 07s
```

---

## Ce qui reste à committer

Fichiers modifiés / créés (non committés) :

- `crates/aphrody-re/src/headers.rs` — nouveau module (zero-copy headers + tests googletest)
- `crates/aphrody-re/src/lib.rs` — `pub mod headers` ajouté, `#![allow(non_ascii_idents)]`
  au crate root, tests migrés vers `#[gtest]` + `expect_that!`
- `docs/rust/google-rust-crates.md` — ce fichier
