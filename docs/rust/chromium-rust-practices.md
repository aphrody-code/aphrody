<!-- SPDX-License-Identifier: Apache-2.0 -->

# Pratiques Rust de Chromium distillées pour aphrody

Synthèse des règles d'ingénierie Rust du projet Chromium, filtrées pour ce qui
est pertinent à **aphrody** (workspace Rust cross-platform, cible #1 Linux
Ubuntu 26.04 / #2 Windows MSVC / #3 wasm32, licence Apache-2.0, supply-chain via
`cargo deny` + `cargo vet`). Pour chaque point : *ce que fait Chromium* puis *ce
qu'on en retient pour aphrody*. Aucune copie verbatim — distillation.

## Sources

- Rust in Chromium (`docs/rust.md`) :
  <https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/rust.md>
  (version taguée lue : `.../+/refs/tags/135.0.7040.0/docs/rust.md`)
- Unsafe Rust policy (`docs/rust-unsafe.md`) :
  <https://chromium.googlesource.com/chromium/src/+/refs/tags/135.0.7040.0/docs/rust-unsafe.md>
- FFI C/C++ ↔ Rust (`docs/rust/ffi.md`) :
  <https://chromium.googlesource.com/experimental/chromium/src/+/HEAD/docs/rust/ffi.md>
- Politique d'import de crates tierces (`third_party/rust/README-importing-new-crates.md`) :
  <https://chromium.googlesource.com/chromium/src.git/+/HEAD/third_party/rust/README-importing-new-crates.md>
- Process général d'ajout de tiers (`docs/adding_to_third_party.md`), référencé par le doc ci-dessus.
- `cxx` (FFI sûr Rust↔C++) : <https://github.com/dtolnay/cxx>

## 1. Politique d'ajout de crates tierces

**Chromium.** Tout import de crate requiert l'approbation des
`//third_party/rust/OWNERS` plus le process tiers général
(`adding_to_third_party.md`). L'outillage est centré sur `gnrt` :
`run_gnrt.py add <crate>` met à jour `Cargo.toml`, `run_gnrt.py vendor`
télécharge les sources, `gnrt_config.toml` porte les réglages Chromium-specific,
`run_gnrt.py gen` génère les `BUILD.gn`, et chaque crate reçoit son propre
fichier `OWNERS`. Préférence explicite pour les crates **sans `unsafe`** afin de
réduire la charge de revue et faciliter la maintenance.

**aphrody.** On n'a pas de `gnrt` (build = Cargo natif, lockfile-only, sparse
registry, pas de `cargo vendor`), mais on garde le principe : ajout de dep =
décision tracée, passage obligatoire par `cargo deny check` (CVE + licences +
bans + sources) et un audit `cargo vet` (ou exemption documentée dans
`config.toml`). À privilégier : crates sans `unsafe`, ou dont l'`unsafe` est
minimal et audité. Réutiliser le skill `best-stack-2026` pour le choix.

## 2. Vetting / `cargo vet` / allowlist

**Chromium.** Les crates sous `chromium_crates_io` doivent avoir une couverture
d'audit `cargo vet` ; `run_cargo_vet.py check` doit passer. Les audits sont
**dispensés** (publishers de confiance) pour l'org GitHub `rust-lang`
(ex. `libc`, `hashbrown`) et pour les SDK OS (ex. crates `windows-*`).

**aphrody.** Aligné : `cargo vet` est déjà notre gate (feeds Google / Mozilla /
Fuchsia). On reprend l'idée d'une **trust-list de publishers** (rust-lang, SDK
OS) pour réduire le bruit d'audit, à matérialiser via `trusted`/exemptions dans
`supply-chain/config.toml`. Tout ajout non couvert = audit explicite avant merge.

## 3. Critères de sélection d'une crate (maintenance, soundness)

**Chromium.** Au-delà du process : préférer le code sans `unsafe`,
maintenabilité dans le temps (les changements *n'importe où* dans une crate à
`unsafe` peuvent casser ses invariants), et respect de la « rule of 2 » côté
sécurité (cf. §6).

**aphrody.** Critères de sélection : maintenance active (dernier commit récent),
soundness (`unsafe` minimal et justifié), licence OSI compatible Apache-2.0
(**aucune contamination GPL** — cf. ban `unicorn-engine`), absence d'advisory
RUSTSEC ouverte. Ces critères sont déjà encodés dans le skill `best-stack-2026`
et `deny.toml`.

## 4. Interop C++ ↔ Rust (cxx, autocxx, bindgen)

**Chromium.** L'outil **recommandé est `cxx`** : interface Rust↔C++ déclarée
manuellement dans un `#[cxx::bridge]`, générée dans un `namespace`
projet/crate-specific, idéalement un **seul bridge** par unité (le partage de
types multi-bridge a des « rough edges »). **`autocxx` a été abandonné** au
profit du `cxx` manuel. `bindgen` reste supporté (via
`//build/rust/rust_bindgen.gni`) quand générer des bindings depuis des headers
C/C++ est préférable à la déclaration manuelle (APIs nombreuses/complexes).
**Non supportés** : `cbindgen`, `zngur`, Crubit (intégration incomplète).
Côté style FFI : `From`/`TryFrom` pour les conversions, opérateur `?` et
`let Ok(..) = .. else { .. }` pour les erreurs.

**aphrody.** CLAUDE.md borne déjà le C/C++ aux wrappers FFI `cxx::bridge` : on
suit Chromium à la lettre — **`cxx` est l'outil canonique**, autocxx évité,
bindgen réservé aux gros surfaces de headers. Un seul bridge par module, sous
namespace dédié, conversions via `TryFrom`.

## 5. Revue du code `unsafe`

**Chromium.** Tout `unsafe` (first-party **et** third-party) doit être relu et
approuvé par `//third_party/rust/UNSAFE_RUST_OWNERS`. Process : ajouter un
commentaire non résolu « TODO: `unsafe` review » sur chaque bloc/fn/impl
`unsafe` nouveau ou modifié (outil `create_draft_comments.py`), ajouter
`chrome-unsafe-rust-reviews@google.com` en reviewer. SLA souple : ~1 jour
ouvré pour les changements incrémentaux. Les fichiers à `unsafe` peuvent
forwarder leur `OWNERS` vers `UNSAFE_RUST_OWNERS`.

**aphrody.** Pas de reviewers humains (autonomie totale, §0.1 CLAUDE.md), donc on
transpose en **discipline auto-imposée** : chaque bloc `unsafe` porte un
commentaire `// SAFETY:` justifiant l'invariant tenu, l'`unsafe` reste minimal et
isolé, et tout changement *ailleurs* dans une crate à `unsafe` impose de
re-vérifier les invariants. Mémoriser : « les invariants `unsafe` sont fragiles
aux changements à distance ».

## 6. Sécurité — « rule of 2 » et isolation

**Chromium.** Manipuler des données non fiables de façon non triviale dans un
process privilégié (Browser, GPU) est interdit sauf en langage memory-safe. Si
une lib *shipping* contient de l'`unsafe` et que la revue conclut qu'elle ne
respecte pas la rule of 2, elle est déplacée dans le groupe `"sandbox"` de
`gnrt_config.toml` (interdite en process privilégié).

**aphrody.** On n'a pas de modèle multi-process à sandbox, mais on garde le
principe : le **parsing de données non fiables** (réseau, fichiers, RE de
binaires) doit rester en Rust safe, l'`unsafe` confiné aux frontières OS/FFI.
Une crate à `unsafe` non sound ne doit pas se retrouver sur le chemin de données
attaquant-contrôlées.

## 7. MSRV / version de toolchain

**Chromium.** Toolchain Rust **vendorée** (`third_party/rust-toolchain/bin/...`),
imposée pour tous les contributeurs ; le doc ne fige pas de numéro de version
explicite mais bannit l'usage du Rust système.

**aphrody.** Toolchain **pinnée** via `rust-toolchain.toml`
(`nightly-2026-05-17`, Edition 2024) — un re-pin passe par PR. Même esprit que
Chromium : une seule toolchain canonique, reproductible, jamais le Rust système.

## 8. Lints obligatoires

**Chromium.** Le `rust.md` ne détaille pas de politique de lints explicite dans
le corps lu ; le style général est référencé séparément.

**aphrody.** Politique propre, plus stricte : `[workspace.lints]` centralisés,
`clippy --workspace --all-targets --locked --offline -- -D warnings`
(`cargo ci-offline`), `pedantic` activable per-crate via
`#[warn(clippy::pedantic)]`. À conserver comme gate de merge.

## 9. Gestion des panics à la frontière FFI

**Chromium.** Le doc FFI lu n'expose pas de règle explicite sur l'unwinding /
`panic=abort` à la frontière. En pratique, `cxx` gère le franchissement de
frontière de façon sûre (les panics ne se propagent pas en UB côté C++).

**aphrody.** Règle à tenir : **un panic ne doit jamais traverser une frontière
FFI** (UB / unwinding cross-language). Toute fn `extern "C"` ou exposée via
`cxx` enveloppe son corps dans `std::panic::catch_unwind` (ou `cxx::bridge` qui
convertit le panic en exception/Result), et convertit l'erreur en code de retour
ou `Result`. Pour les libs de bas niveau, envisager `panic = "abort"` sur les
profils release afin d'éliminer l'unwinding cross-FFI.

## Recommandations actionnables pour aphrody (checklist)

- [ ] **Ajout de dep** : passer par `cargo deny check` + audit `cargo vet` (ou
      exemption tracée) avant tout merge ; préférer une crate sans `unsafe`,
      maintenue, sans advisory RUSTSEC, licence non-GPL.
- [ ] **Trust-list publishers** : exempter d'audit `rust-lang/*` et les SDK OS
      (`windows-*`) dans `supply-chain/config.toml`, comme Chromium.
- [ ] **Interop C++** : utiliser `cxx` (`#[cxx::bridge]`, un seul bridge,
      namespace dédié) ; ne pas adopter `autocxx`/Crubit/`cbindgen` ; `bindgen`
      seulement pour de gros surfaces de headers.
- [ ] **`unsafe`** : chaque bloc porte un `// SAFETY:` justifiant l'invariant ;
      `unsafe` minimal, isolé ; re-vérifier les invariants à tout changement dans
      une crate qui en contient.
- [ ] **Frontière FFI** : zéro panic traversant (`catch_unwind` ou conversion
      `cxx`→`Result`/exception) ; envisager `panic = "abort"` en release pour les
      crates FFI bas niveau.
- [ ] **Données non fiables** : parsing en Rust safe, `unsafe` confiné aux
      frontières OS ; ne pas placer une crate `unsafe` non sound sur un chemin de
      données attaquant-contrôlées (esprit « rule of 2 »).
- [ ] **Toolchain & lints** : conserver la toolchain pinnée
      (`rust-toolchain.toml`) et le gate `clippy -D warnings` (`cargo ci-offline`)
      comme bloquants de merge.
