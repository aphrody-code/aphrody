<!-- SPDX-License-Identifier: Apache-2.0 -->

# Pratiques Rust Android applicables à aphrody

> **Portée.** Ce document distille la doc officielle « Building Rust modules »
> d'AOSP (Android Open Source Project) et n'en retient **que** ce qui est
> transposable à aphrody, qui est un **workspace Cargo pur** (cible #1 Linux
> Ubuntu, #2 Windows, #3 WASM) — et **pas** un build Soong/AOSP. Tout ce qui
> est spécifique au build system Android (modules `Android.bp`, `rust_*` Soong,
> `cargo_embargo`, intégration `make`/`soong`) est mentionné à titre de
> contexte mais **n'est pas** un livrable pour nous : on en extrait la
> *politique d'ingénierie* sous-jacente, pas l'outillage.
>
> Chaque point suit le format : **« Ce que fait Android »** → **« Ce qu'on en
> retient pour aphrody »**.

## Sources (consultées le 2026-05-23, version FR)

- Overview — <https://source.android.com/docs/setup/build/rust/building-rust-modules/overview?hl=fr>
- Modules Android Rust (propriétés, edition, lints) — <https://source.android.com/docs/setup/build/rust/building-rust-modules/android-rust-modules?hl=fr>
- Modules de bibliothèque (rlib/dylib/ffi) — <https://source.android.com/docs/setup/build/rust/building-rust-modules/library-modules?hl=fr>
- Modules binaires (linkage libstd, prefer_rlib, static) — <https://source.android.com/docs/setup/build/rust/building-rust-modules/binary-modules?hl=fr>
- Patterns Android Rust (CXX / bindgen / JNI) — <https://source.android.com/docs/setup/build/rust/building-rust-modules/android-rust-patterns?hl=fr>
- Modules Bindgen (interop C, wrapping sûr) — <https://source.android.com/docs/setup/build/rust/building-rust-modules/source-code-generators/bindgen-modules?hl=fr>

> Synthèse et reformulation : aucun texte source n'est reproduit verbatim. Les
> faits techniques sont attribués aux pages ci-dessus ; les extrapolations vers
> aphrody sont nos décisions d'ingénierie.

---

## 1. Politique de version / édition Rust (MSRV)

**Ce que fait Android.** AOSP ne publie pas de « MSRV » au sens crates.io : la
plateforme épingle **un seul toolchain Rust** distribué dans l'arbre source, et
tout le monde compile avec celui-ci (pas de matrice de versions). La propriété
`edition` des modules accepte `2015`/`2018`/`2021`, défaut **2021**, et reste
explicite par module.

**Ce qu'on en retient pour aphrody.** Même philosophie « single pinned
toolchain » : aphrody épingle déjà `nightly-2026-05-17` via
`rust-toolchain.toml` (re-pin = PR). On n'entretient donc **pas** de MSRV ni de
matrice multi-versions ; on documente la version unique et on bump par PR
revue. Édition : aphrody est en **Edition 2024**, plus récente que la 2021
d'Android, donc on garde notre choix (pas de régression).

## 2. Lints rustc + clippy imposés

**Ce que fait Android.** rustc-lint **et** clippy tournent **par défaut** sur
tous les modules (sauf source generators). Quatre jeux de lints : `default`
(selon emplacement), `android` (le plus strict, pour le code plateforme),
`vendor` (relâché, pour le code tiers), `none` (désactivé). Mettre `none`
pendant le dev est toléré, mais **un relâchement doit être justifié en revue de
code** ; le code généré (bindgen) a le droit de désactiver les lints car « non
garanti lint-free ».

**Ce qu'on en retient pour aphrody.** Le code first-party d'aphrody est
l'équivalent du jeu `android` (le plus strict) : on garde
`clippy --workspace --all-targets -D warnings` (déjà `cargo ci-offline`) comme
gate non négociable. Inversement, le **code généré** (sorties bindgen, codegen
`a2a-pb`, etc.) a le droit d'avoir ses lints relâchés/`#[allow]` ciblés, à
condition que le wrapper sûr autour, lui, repasse au strict. Tout `#[allow]`
large dans du code écrit à la main doit porter une justification en commentaire
(équivalent de la « justification en revue » d'Android).

## 3. Interop Rust ↔ C/C++ : cxx & bindgen

**Ce que fait Android.**
- **Rust → C++ (sûr)** : crate **CXX** pour un FFI sûr vers un sous-ensemble de
  C++ ; le pont C++ est généré (`cxxbridge`) puis lié statiquement.
- **Rust ↔ C (bas niveau)** : **bindgen** génère les bindings FFI bruts (support
  C++ limité via `cppstd`). Règle forte : **ne jamais exposer les bindings
  bruts** — fournir une **bibliothèque wrapper sûre dans le même arbre que les
  bindings**. Les bindings bruts sont par défaut en `visibility:
  [":__subpackages__"]` (accès restreint au voisinage) pour décourager leur
  usage direct ailleurs.
- **Rust ↔ Java** : crate `jni`, sans codegen.

**Ce qu'on en retient pour aphrody.** Aligne exactement notre politique §2 du
CLAUDE (C/C++ banni sauf wrappers FFI via `cxx::bridge`) :
1. Préférer **`cxx`** dès qu'il y a du C++ en jeu (FFI sûr, pas de `unsafe`
   manuel côté pont).
2. Pour du C pur, **bindgen** mais **toujours** encapsulé dans un module wrapper
   sûr ; les bindings bruts (souvent `unsafe extern`) ne sont **jamais** un
   point d'entrée public d'une crate aphrody.
3. Restreindre la visibilité des bindings bruts : en Cargo, l'équivalent est de
   les garder dans un module **non-`pub`** (ou `pub(crate)`) et de n'exporter
   que le wrapper. Mettre les bindings générés et leur wrapper dans le **même
   répertoire/crate**.
4. Les bindings générés peuvent désactiver les lints (cf. §2).

## 4. Politique panic : abort vs unwind

**Ce que fait Android.** Les pages « modules » consultées **ne documentent pas
explicitement** `panic=abort` vs `unwind` (absence notée sur binary-modules et
android-rust-modules). En pratique connue de la plateforme Android, le code
système privilégie `panic=abort` (pas de déroulement de pile à travers la
frontière FFI, taille binaire réduite), mais ce n'est **pas** affirmé par les
pages citées — on ne le présente donc pas comme une règle AOSP sourcée.

**Ce qu'on en retient pour aphrody.** Décision d'ingénierie aphrody (non
dérivée d'une source Android) : **`panic=abort` interdit de traverser une
frontière FFI** — un `panic` qui remonte dans du C/C++ est UB. Donc, sur toute
fonction `extern "C"` exportée, wrapper le corps Rust dans
`std::panic::catch_unwind` (ou compiler le profil concerné en `panic=abort`)
pour garantir qu'aucun unwind ne franchit la frontière. Pour les **binaires**
de release, envisager `panic="abort"` dans le profil (gain taille + pas
d'unwinder), à condition que `catch_unwind` ne soit pas requis ailleurs.
Sur **WASM** (cible #3), `panic=abort` est de toute façon le comportement
naturel. À trancher par profil dans `Cargo.toml`, pas globalement.

## 5. Types de bibliothèque : rlib / dylib / staticlib / cdylib

**Ce que fait Android.**
- `rust_library` produit **rlib + dylib** : c'est le **recommandé** car il
  fonctionne comme dépendance via `rustlibs`.
- `rust_library_rlib` / `_dylib` : une seule variante, sans garantie via
  `rustlibs`.
- `rust_ffi` (+ `_shared`/`_static`) : bibliothèques **C-compatibles** (statique
  & partagée) pour l'interop avec les modules CC, avec `export_include_dirs`
  pour les headers.
- **Linkage `libstd`** : device = **toujours dynamique** vers `libstd` ; host =
  **statique**. `prefer_rlib` / `static_executable` forcent rlib + libstd
  statique (binaire pleinement statique, restreint aux cibles bionic).

**Ce qu'on en retient pour aphrody.**
- Pour les crates **internes** du workspace : rester en **rlib** (défaut Cargo),
  c'est l'équivalent direct et le plus simple à lier — pas de `dylib` interne.
- Pour exposer une **API C** (consommateurs C/C++/autres langages des dépôts
  frères) : `crate-type = ["staticlib"]` (≈ `rust_ffi_static`) ou `["cdylib"]`
  (≈ `rust_ffi_shared`), en exportant des headers (cbindgen/cxx) — analogue à
  `export_include_dirs`.
- Linkage `libstd` : sur nos cibles, **statique** est la norme attendue (host-
  like) ; le `dylib`-libstd d'Android est un besoin device-spécifique sans objet
  pour aphrody. Pour un binaire pleinement statique (ex. cible musl Linux),
  l'équivalent de `static_executable` est une cible `*-musl`.

## 6. Vendoring & vetting des crates tierces

**Ce que fait Android.** AOSP **vendore** les crates tierces dans l'arbre
(`external/rust/crates`) et les importe avec revue ; l'import et la
régénération des fichiers de build sont outillés (historiquement `cargo2android`,
aujourd'hui **`cargo_embargo`**). Google **maintient ses propres feeds
`cargo vet`** (audits signés) pour les crates qu'il a auditées. La page
« managing deps » FR ciblée renvoyait un 404 au moment de la consultation ;
le mécanisme `cargo vet` / feeds Google est néanmoins un fait établi du
programme supply-chain de Google.

**Ce qu'on en retient pour aphrody.**
- aphrody fait le **choix inverse du vendoring** (politique §5 CLAUDE :
  *pas de `cargo vendor`*, repo lockfile-only sparse registry). On ne reprend
  donc pas l'approche « arbre vendoré » d'Android ni `cargo_embargo`.
- En revanche on **adopte pleinement `cargo vet`** : importer les **feeds
  d'audits de Google** (et Mozilla/Fuchsia) — c'est déjà la pratique aphrody
  (`cargo vet` alimenté par les feeds Google/Mozilla/Fuchsia, cf. §3 commandes).
  Toute nouvelle dep doit avoir un audit `cargo vet` ou une exemption explicite
  dans `config.toml`.
- Conserver `cargo deny check` (CVE + licences + bans + sources) comme gate
  complémentaire — pas d'équivalent direct côté Android mais cohérent avec leur
  exigence de revue d'import.

## 7. Revue du code `unsafe`

**Ce que fait Android.** La doctrine implicite des pages interop : le `unsafe`
est **confiné** aux bindings générés et **encapsulé** derrière un wrapper sûr ;
les bindings bruts sont à visibilité restreinte pour empêcher la diffusion du
`unsafe` ailleurs. La revue de code est le point de contrôle (justifications
exigées pour relâcher les lints / la sûreté).

**Ce qu'on en retient pour aphrody.**
- Chaque bloc `unsafe` écrit à la main porte un commentaire `// SAFETY:`
  justifiant l'invariant (convention Rust standard, alignée sur la doctrine
  « justification en revue » d'Android).
- Le `unsafe` issu de FFI est cantonné aux modules de bindings + wrapper sûr
  (cf. §3) ; aucune API publique de crate aphrody n'expose de `unsafe` non
  documenté.
- Activer `#![warn(unsafe_op_in_unsafe_fn)]` et viser `#![forbid(unsafe_code)]`
  sur les crates qui n'en ont pas besoin (la majorité), pour matérialiser le
  confinement.

## 8. Gestion des features

**Ce que fait Android.** Les features sont déclarées **explicitement par module**
(propriété `features` → flags activés à la compilation) ; rien n'est implicite
ni transitif via résolution Cargo (Soong n'a pas de feature unification). Les
`cfgs` (`--cfg`) sont eux aussi explicites par module.

**Ce qu'on en retient pour aphrody.** En Cargo on ne peut pas reproduire le
« tout explicite par module » de Soong, mais on en garde l'esprit :
- Features **additives** uniquement (ne jamais retirer d'API quand une feature
  s'active), pour rester compatible avec l'unification de features de Cargo.
- Garder les **features host-only** (ex. Magika/ort, `github`, gemini-cli)
  **opt-in** et `default = []` quand elles tirent des deps lourdes — pratique
  déjà en place dans aphrody.
- Documenter chaque feature dans le `Cargo.toml` de la crate (rôle + ce qu'elle
  active), équivalent lisible de la déclaration explicite Android.

---

## Recommandations actionnables pour aphrody (checklist)

- [ ] **Toolchain unique épinglé** (`rust-toolchain.toml`, `nightly-2026-05-17`,
      Edition 2024) — pas de MSRV ni de matrice multi-versions ; bump par PR.
- [ ] **Clippy strict obligatoire** sur le code first-party
      (`clippy --workspace --all-targets -D warnings`) ; lints relâchés tolérés
      **uniquement** sur le code généré, jamais sur le code écrit à la main sans
      `// justification`.
- [ ] **Interop FFI confinée** : `cxx` pour C++, `bindgen` pour C **toujours**
      derrière un wrapper sûr ; bindings bruts non-`pub` (ou `pub(crate)`) dans
      la même crate ; lints désactivables sur le code généré seulement.
- [ ] **Pas d'unwind à travers la FFI** : `catch_unwind` (ou profil
      `panic="abort"`) sur toute fonction `extern "C"` exportée ; trancher la
      politique panic par profil dans `Cargo.toml`.
- [ ] **rlib en interne, staticlib/cdylib pour exposer une API C** ; libstd
      lié statiquement (host-like) sur nos cibles ; cible `*-musl` pour un
      binaire pleinement statique.
- [ ] **Pas de vendoring** (lockfile-only, sparse registry) mais **`cargo vet`
      avec les feeds Google/Mozilla/Fuchsia** + `cargo deny check` (CVE,
      licences, bans, sources) comme double gate supply-chain sur toute nouvelle
      dep.
- [ ] **`unsafe` documenté & confiné** : `// SAFETY:` sur chaque bloc manuel,
      `#![forbid(unsafe_code)]` sur les crates qui n'en ont pas besoin,
      `unsafe_op_in_unsafe_fn` activé.
- [ ] **Features additives & host-only opt-in** (`default = []` pour les deps
      lourdes), documentées dans chaque `Cargo.toml`.
