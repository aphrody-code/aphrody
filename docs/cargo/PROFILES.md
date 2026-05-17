# Build profiles

> Réf. : `[profile.*]` dans `Cargo.toml` racine.

## Vue d'ensemble

Le workspace définit **6 profils** spécialisés selon le cas d'usage :

| Profil | Cas d'usage | LTO | Codegen units | Strip | Panic | Debug |
|---|---|---|---|---|---|---|
| `dev` | Dev local, itération rapide | off | 256 | no | unwind | limited |
| `release` | Build de référence (`cargo build --release`) | fat | 1 | symbols | abort | false |
| `dist` | **Binaire distribuable production** | fat | 1 | symbols | abort | false |
| `release-fast` | **CI** (LTO thin pour vitesse) | thin | 16 | symbols | abort | false |
| `release-debug` | **Profiling / `perf`** | fat | 1 | none | abort | line-tables-only |
| `bench` | Criterion benchmarks | thin | 16 | none | abort | line-tables-only |

## Profil `dev`

```toml
[profile.dev]
opt-level         = 0
debug             = "limited"
debug-assertions  = true
overflow-checks   = true
lto               = false
codegen-units     = 256
incremental       = true
split-debuginfo   = "unpacked"
panic             = "unwind"

[profile.dev.build-override]
opt-level     = 3      # Build scripts compilés avec opts
codegen-units = 16

[profile.dev.package."*"]
opt-level     = 1      # Deps tierces compilées avec -O1
codegen-units = 16

[profile.dev.package.aes-gcm]   { opt-level = 3 }
[profile.dev.package.sha2]      { opt-level = 3 }
[profile.dev.package.rustls]    { opt-level = 3 }
```

**Pourquoi `package."*"` opt-level=1 ?** Énorme gain sur `cargo check` / `cargo build` répétés : les deps tierces (souvent énormes — serde, tokio, regex) sont compilées une seule fois avec optims légères et restent en cache. Le code workspace reste à `-O0` pour itération rapide.

## Profil `release` (par défaut `--release`)

```toml
[profile.release]
opt-level         = 3
debug             = false
debug-assertions  = false
overflow-checks   = false
lto               = "fat"
codegen-units     = 1
panic             = "abort"
strip             = "symbols"
incremental       = false
rpath             = false
split-debuginfo   = "off"
```

## Profil `dist` (ship target)

```toml
[profile.dist]
inherits          = "release"
opt-level         = 3
lto               = "fat"
codegen-units     = 1
panic             = "abort"
strip             = "symbols"
debug             = false
```

Identique à `release` pour l'instant — réservé aux ajouts PGO/BOLT futurs.

**Usage** : `cargo build --profile dist --workspace --locked`

## Profil `release-fast` (CI builds)

```toml
[profile.release-fast]
inherits          = "release"
lto               = "thin"          # ← key difference
codegen-units     = 16              # ← parallelism
incremental       = false
strip             = "symbols"
```

**Quand utiliser :** PR validation CI. LTO thin + 16 codegen units offrent ~80% des perfs `release` avec un build 3-5× plus rapide.

**Usage** : `cargo build --profile release-fast --workspace --locked`

## Profil `release-debug` (profiling)

```toml
[profile.release-debug]
inherits          = "release"
debug             = "line-tables-only"
strip             = "none"
split-debuginfo   = "packed"
```

**Quand utiliser :** debugging d'une race condition observée uniquement en release, profiling avec `perf` / VTune / Windows Performance Analyzer.

**Usage** : `cargo build --profile release-debug --workspace`

## Profil `bench` (Criterion)

```toml
[profile.bench]
inherits          = "release"
debug             = "line-tables-only"
lto               = "thin"
codegen-units     = 16
strip             = "none"
```

Compromis perf/build-time : suffisant pour les benchmarks Criterion qui amortissent le temps de build sur des milliers d'itérations.

**Usage** : `cargo bench --workspace --locked`

## Pourquoi `panic = "abort"` ?

- Binaire plus petit (~5-10%).
- Pas de stack unwinding → optimisations plus agressives.
- Comportement clair : un panic = un crash, pas de récupération.
- Compatible avec `catch_unwind` désactivé dans nos crates.

**Implication** : pas de `std::panic::catch_unwind` dans le code de prod. Les boundaries FFI doivent garantir qu'aucun panic ne traverse la frontière C (cf. `[workspace.lints.rust] ffi_unwind_calls = "warn"`).

## Quand changer un profil ?

Pour un cas particulier d'une seule crate, **n'éditez pas** `[profile.*]` dans Cargo.toml workspace — utilisez une override locale :

```toml
[profile.release.package.heavy-crate]
opt-level = 3
codegen-units = 1
```
