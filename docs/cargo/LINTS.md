# Lints policy

> Réf. : `[workspace.lints]` dans `Cargo.toml` racine, `.clippy.toml`.

## Stratégie globale

**Tight workspace-wide deny set, lenient style/pedantic groups, opt-in hardening per-crate.**

- Les antipatterns *toujours bugués* (panic, todo, dbg_macro, mem_forget) sont `deny`.
- Les lints de safety (unsafe_op_in_unsafe_fn, unused_must_use) sont `deny`.
- Les lints de style / pedantic / nursery / correctness sont `allow` au workspace (trop bruyants sur du code FFI / kernel / Bun-vendor).
- Les crates hardenées peuvent réactiver per-fichier : `#![warn(clippy::pedantic)]`.

## Workspace.lints.rust (rustc)

### Critical (deny)
```toml
unsafe_op_in_unsafe_fn         = "deny"
unused_must_use                = "deny"
non_ascii_idents               = "deny"
absolute_paths_not_starting_with_crate = "deny"
keyword_idents                 = { level = "deny", priority = -1 }
```

### Warn (informational)
```toml
ffi_unwind_calls               = "warn"
deprecated_in_future           = "warn"
unused_import_braces           = "warn"
macro_use_extern_crate         = "warn"
non_local_definitions          = "warn"
unreachable_pub                = "warn"
```

### Allow (FFI / kernel-bridge reality)
```toml
missing_debug_implementations  = "allow"
missing_copy_implementations   = "allow"
trivial_numeric_casts          = "allow"
trivial_casts                  = "allow"
unused_lifetimes               = "allow"
unused_qualifications          = "allow"
single_use_lifetimes           = "allow"
elided_lifetimes_in_paths      = "allow"
let_underscore_drop            = "allow"
explicit_outlives_requirements = "allow"
redundant_lifetimes            = "allow"
unused_features                = "allow"
```

## Workspace.lints.rustdoc

```toml
broken_intra_doc_links         = "deny"
private_intra_doc_links        = "warn"
missing_crate_level_docs       = "warn"
invalid_codeblock_attributes   = "warn"
invalid_html_tags              = "warn"
bare_urls                      = "warn"
```

## Workspace.lints.clippy

### Antipatterns toujours bugués (deny)
```toml
todo                           = "deny"
unimplemented                  = "deny"
dbg_macro                      = "deny"
mem_forget                     = "deny"
exit                           = "deny"
lossy_float_literal            = "deny"
float_cmp_const                = "deny"
```

### Lint groups (priority -1 → single-lint overrides win)
```toml
pedantic                       = { level = "allow", priority = -1 }
nursery                        = { level = "allow", priority = -1 }
suspicious                     = { level = "allow", priority = -1 }
style                          = { level = "allow", priority = -1 }
complexity                     = { level = "allow", priority = -1 }
correctness                    = { level = "allow", priority = -1 }
perf                           = { level = "warn",  priority = -1 }
```

### Overrides explicites (échapent au group muting)
```toml
collapsible_if                 = "allow"
collapsible_else_if            = "allow"
collapsible_match              = "allow"
```

## `.clippy.toml` — Thresholds

```toml
avoid-breaking-exported-api    = false   # canary cycle permits API churn
cognitive-complexity-threshold = 40      # relaxed (kernel/FFI complexity)
too-many-arguments-threshold   = 10      # FFI signatures often have ≥7 args
too-many-lines-threshold       = 200
type-complexity-threshold      = 350
enum-variant-name-threshold    = 4
struct-field-name-threshold    = 4
allow-unwrap-in-tests          = true    # tests can use unwrap
allow-expect-in-tests          = true
allow-panic-in-tests           = true
warn-on-all-wildcard-imports   = true    # warn level (group muted anyway)
msrv = "1.93"
```

## Opt-in hardening per crate

Quand une crate est jugée "hardened" (audit fait, unsafe documenté, etc.), elle peut réactiver pedantic localement :

```rust
// crates/base/src/lib.rs
#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]   // intentional
```

## Preset `android-strict` (équivalent du `lints: "android"` de Soong)

Pour les crates production-ready (crypto, parsing untrusted, FFI au-dessus du noyau), inspiré du niveau le plus strict d'AOSP. **Copier-coller** dans le `[lints]` de la crate hardenée — pas d'héritage workspace possible pour ce preset, c'est une décision per-crate consciente.

```toml
# crates/<hardened-crate>/Cargo.toml
[lints.rust]
unsafe_op_in_unsafe_fn         = "deny"
unused_must_use                = "deny"
missing_docs                   = "warn"
unreachable_pub                = "deny"
missing_debug_implementations  = "warn"
non_ascii_idents               = "deny"
trivial_casts                  = "warn"
trivial_numeric_casts          = "warn"
unused_qualifications          = "warn"
unused_lifetimes               = "warn"
single_use_lifetimes           = "warn"

[lints.rustdoc]
broken_intra_doc_links         = "deny"
missing_crate_level_docs       = "deny"
private_intra_doc_links        = "warn"

[lints.clippy]
pedantic                       = { level = "warn", priority = -1 }
nursery                        = { level = "warn", priority = -1 }
unwrap_used                    = "deny"
expect_used                    = "deny"
panic                          = "deny"
todo                           = "deny"
unimplemented                  = "deny"
indexing_slicing               = "deny"
missing_safety_doc             = "deny"
undocumented_unsafe_blocks     = "deny"
multiple_unsafe_ops_per_block  = "deny"
ptr_as_ptr                     = "deny"
cast_ptr_alignment             = "deny"
mem_forget                     = "deny"
dbg_macro                      = "deny"
print_stdout                   = "deny"
print_stderr                   = "deny"
module_name_repetitions        = "allow"   # intentional in our crates
```

### Quand appliquer ce preset
- Crate **stable**, pas de churn API en cours.
- Audit `unsafe` complet (chaque bloc a un SAFETY comment).
- Tous les `unwrap` / `expect` justifiés ou supprimés.
- Tests > 80% couverture.

### Crates actuellement en `android-strict`
| Crate | Statut |
|---|---|
| `crates/base` | candidat (crypto DPAPI, AES-GCM) |
| `crates/bun_ffi` | candidat (FFI zero-copy) |
| autres | TBD per-crate quand stabilisées |

## Lints Cargo (RFC 3389, unstable on stable)

Ces lints sont actifs depuis Cargo 1.93 :
- `unused_workspace_dependencies` — entrées de `[workspace.dependencies]` jamais héritées
- `unused_workspace_package_fields` — champs de `[workspace.package]` jamais hérités
- `missing_lints_inheritance` — crate sans `[lints] workspace = true` alors que le workspace définit `[workspace.lints]`
- `implicit_minimum_version_req` — encourage `serde = "1.0.219"` au lieu de `serde = "1.0"`
- `redundant_homepage`, `redundant_readme` — métadonnées inférables
- `non_kebab_case_bins`, `non_kebab_case_features`, `non_kebab_case_packages`

Pour les activer (nightly only) :
```toml
[workspace.lints.cargo]
implicit_minimum_version_req   = "warn"
unused_workspace_dependencies  = "deny"
missing_lints_inheritance      = "deny"
```

## Validation

```bash
cargo ci-offline       # = cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo clippy --fix     # auto-fix là où possible
cargo clippy --workspace --all-targets   # warnings only (sans -D)
```
