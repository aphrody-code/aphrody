# Patterns Chromium / Android — adoption dans `aphrody`

> Synthèse appliquée des docs officielles :
> [`source.android.com/.../building-rust-modules`](https://source.android.com/docs/setup/build/rust/building-rust-modules/overview)
> et [`chromium.googlesource.com/.../docs/rust.md`](https://chromium.googlesource.com/chromium/src.git/+/HEAD/docs/rust.md).
> Dernière mise à jour : 2026-05-17.

---

## 1. Politique Rust (alignement Chromium)

Citation Chromium :
> *Handling untrustworthy data in non-trivial ways is a major source of security bugs. Rust gives a cross-platform, memory-safe language so that all platforms can handle untrustworthy data directly from a privileged process.*

**Notre application** :
- **Tout nouveau code en Rust** (cf. `CLAUDE.md`).
- **C/C++ retiré progressivement** par sous-système (cf. `docs/cargo/MIGRATION.md`).

## 2. Toolchain pinning (alignement Chromium `tools/rust/`)

Chromium pin sa toolchain dans `tools/rust/`. Nous, dans `rust-toolchain.toml` :

```toml
[toolchain]
channel    = "nightly"
profile    = "minimal"
components = ["rust-src", "rustfmt", "clippy", "miri",
              "llvm-tools-preview", "rust-analyzer",
              "rustc-codegen-cranelift-preview"]
targets    = [8 targets cross-platform]
```

→ Chaque dev / CI installe **exactement** la même nightly.

## 3. Workspace deps centralisées (alignement Android `rustlibs:`)

Android Soong utilise `rustlibs:` pour lister les deps Rust avec preference rlib+dylib :

```bp
rust_library {
    name: "libfoo",
    crate_name: "foo",
    srcs: ["src/lib.rs"],
    rustlibs: ["libserde", "libtokio"],
}
```

**Notre équivalent Cargo** :

```toml
# Cargo.toml workspace
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1.43", features = ["full"] }

# crates/cli/Cargo.toml
[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
```

→ 80 deps centralisées, mises à jour atomiques, audit unique (cf. `docs/cargo/DEPENDENCIES.md`).

## 4. Lints stricts (alignement Android `lints: "android"`)

Android Soong propose 4 niveaux : `default`, `android` (strictest), `vendor` (relaxed), `none`.

**Notre application** :
- **Workspace level** = équivalent `default` (relaxed pour FFI/kernel/Bun-vendor).
- **Preset `android-strict`** = opt-in per-crate hardenée, cf. `docs/cargo/LINTS.md § Preset android-strict`.

## 5. FFI C/C++ (alignement Chromium `cxx` primary)

Chromium recommande `cxx` comme outil principal, `bindgen` pour les APIs trop larges, `crubit` expérimental.

**Notre application** :
- `cxx` + `bindgen` ajoutés à `[workspace.dependencies]` (préparation).
- **Actuellement** : FFI manuelle via `windows-rs` (plus précis que bindgen pour WinAPI) + `bun_ffi` (zero-copy raw pointers + `mem::forget` pattern).
- **`crubit`** : non adopté (comme Chromium, statut expérimental).

Référence : [`docs/cargo/FFI_POLICY.md`](./FFI_POLICY.md).

### Pattern `cxx::bridge` à suivre (Chromium best practice)

Si on intègre `cxx` un jour pour bridger un sous-système C++ :

```rust
// crates/foo/src/lib.rs
#[cxx::bridge(namespace = "google::cli::foo")]
mod ffi {
    // C++ types exposed to Rust
    unsafe extern "C++" {
        include!("foo/foo.h");
        type CxxFoo;
        fn process(self: &CxxFoo, input: &[u8]) -> Vec<u8>;
    }
    // Rust types exposed to C++
    extern "Rust" {
        type RustBar;
        fn callback(bar: &RustBar, data: &[u8]);
    }
}
```

**Règles Chromium** (à appliquer) :
1. Un seul `#[cxx::bridge]` par module.
2. Toujours `namespace = "..."` projet-spécifique.
3. Conversions via `From`/`TryFrom` entre types FFI et types tiers.
4. `?` operator pour propagation propre des erreurs.

## 6. Tests (alignement Android `rust_test` + Chromium `rust_gtest_interop`)

Android Soong `rust_test` :
```bp
rust_test {
    name: "libfoo_test",
    srcs: ["src/lib.rs"],
    test_suites: ["general-tests"],
    auto_gen_config: true,
}
```

**Notre équivalent** : `cargo nextest`
```toml
# crates/foo/Cargo.toml
[dev-dependencies]
rstest    = { workspace = true }
proptest  = { workspace = true }
criterion = { workspace = true }
```

```bash
cargo nextest run -p foo --locked       # alias: cargo xt
```

→ Pas besoin d'`atest`/`TEST_MAPPING` — l'écosystème Cargo gère.

## 7. Cross-imports first-party (alignement Chromium `chromium::import!`)

Chromium force `chromium::import!` pour éviter les conflits de noms de crates dans le mixed-language build.

**Notre application** : pas nécessaire (Cargo gère le namespacing via `package = "..."` dans `[workspace.dependencies]`) :

```toml
[workspace.dependencies]
a2a-client = { package = "a2a-client-lf", path = "crates/a2a-client", default-features = false }
```

→ Code consommateur écrit `use a2a_client::...` même si le crate publié est `a2a-client-lf`.

## 8. Supply-chain (alignement Google `rust-crate-audits` + ChromeOS)

Chromium `//third_party/rust` + audits manuels.
Android : `Cargo.toml` formal review.
Fuchsia : `cargo vet` + `third_party/rust_crates/supply-chain/`.

**Notre application — synthèse moderne** : `cargo-vet` import des audits Google + Mozilla + Fuchsia + ChromeOS + Bytecode Alliance + Embark + Zcash. Cf. `docs/cargo/SUPPLY_CHAIN.md`.

```toml
# supply-chain/config.toml
[imports.google]
url = "https://raw.githubusercontent.com/google/rust-crate-audits/main/audits.toml"

[imports.fuchsia]
url = "https://fuchsia.googlesource.com/.../supply-chain/audits.toml?format=TEXT"
# ... etc.
```

## 9. Linkage (alignement Android device / host)

Android :
- Device : link `libstd` en **dynamic**, préfère `dylib` deps.
- Host : link `libstd` en **static**, préfère `rlib`.

**Notre application** :
- Le binaire `cli` est **host-style** (static `libstd`, prefere `rlib`) — c'est le défaut Cargo pour les bin targets.
- Les crates FFI (`bun_ffi`, `google_os`, `python_ffi`) sont `crate-type = ["cdylib", "rlib"]` → équivalent `rust_ffi` Soong (à la fois shared lib pour les consumers C/JS/Python et rlib pour les consumers Rust).

## 10. Unstable features (alignement Chromium policy)

Chromium : usage de features Rust unstable → approbation préalable de la Rust toolchain team.

**Notre application** :
- Nightly est notre canal — nous **utilisons** les unstable features (`#![feature(thread_local)]` dans `google_os`, etc.).
- **Avant `1.0.0-LTS`** : freeze des unstable features utilisées, audit pour s'assurer qu'elles sont en track de stabilisation OU avoir un fallback stable.
- Documenter dans `docs/PLAN.md` Phase P9 (Release LTS).

## 11. Différences délibérées

Là où on s'écarte des patterns Chromium/Android par choix :

| Sujet | Eux | Nous | Raison |
|---|---|---|---|
| Build system | GN/Ninja (Cr) / Soong (And) | **Cargo workspace** | Notre repo est mono-projet, pas besoin de la machinerie meta-build |
| Vendor source-replacement | Cr `//third_party/rust` + And `Cargo.toml` review | **Lockfile-only + cargo-vet** | Phase 1 du refactor 2026-05-16 (cf. SUPPLY_CHAIN.md) |
| WinAPI bindings | bindgen (Chromium) | **windows-rs** | Bindings officiels MS, précis sur l'API Windows |
| Crubit | Tracked (crbug/470466915) | Non adopté | Pas mature, pas besoin pour notre surface FFI |

## 12. Roadmap d'adoption complète

- [x] **Toolchain pinning** (rust-toolchain.toml)
- [x] **Workspace deps centralisées** (80 deps)
- [x] **Supply-chain audits** (cargo-vet + 7 feeds)
- [x] **Cross-platform `cli` binary** (platform.rs)
- [x] **Multi-target aliases** (.cargo/config.toml)
- [x] **`cxx`/`bindgen` workspace deps préparés**
- [ ] **CI multi-target** (`.github/workflows/cross-platform.yml`)
- [ ] **Premier `cxx::bridge` réel** (quand on intègre un binding C++ non trivial)
- [ ] **`cargo fuzz`** pour `google_os` / `bun_ffi` (équivalent `rust_fuzz` Soong)
- [ ] **`rust_bindgen`-style `build.rs`** quand on bind une API C large
- [ ] **Audits Google rust-crate-audits → notre repo audits publié**

## Références

- [Android Rust modules — Présentation](https://source.android.com/docs/setup/build/rust/building-rust-modules/overview?hl=fr)
- [Android Rust modules — Détails](https://source.android.com/docs/setup/build/rust/building-rust-modules/android-rust-modules)
- [Chromium Rust policy](https://chromium.googlesource.com/chromium/src.git/+/HEAD/docs/rust.md)
- [Chromium Rust FFI (experimental docs)](https://chromium.googlesource.com/experimental/chromium/src/+/HEAD/docs/rust/ffi.md)
- [Crubit (Google) — Bidirectional Rust↔C++](https://github.com/google/crubit)
- [Google rust-crate-audits](https://github.com/google/rust-crate-audits)
- [Fuchsia third_party/rust_crates/supply-chain](https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/third_party/rust_crates/supply-chain/)
