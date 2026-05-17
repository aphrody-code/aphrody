# Cargo Cheatsheet — Aphrody

> Réf. : `.cargo/config.toml` `[alias]` section pour les commandes custom.
> Dernière mise à jour : 2026-05-16.

## Build

```bash
cargo check  --workspace --locked                    # type-check rapide
cargo build  --workspace --locked                    # build dev (debug)
cargo build  --workspace --locked --release          # release par défaut
cargo build  --workspace --locked --profile dist     # release LTO fat (ship target)
cargo build  --workspace --locked --profile release-fast  # CI rapide LTO thin
cargo build  --workspace --locked --profile release-debug # release + symbols (profiling)
```

Aliases custom (de `.cargo/config.toml`) :

```bash
cargo dist             # = build --profile dist --workspace --locked
cargo release-fast     # = build --profile release-fast --workspace --locked
```

## Test

```bash
cargo nextest run --workspace --all-features --locked       # alias: cargo xt
cargo nextest run --workspace --all-features --locked --offline   # alias: cargo xt-offline
cargo test -p <crate> --lib                                 # tests d'une seule crate
cargo bench -p google_os                                    # benchmarks Criterion
```

## Lint

```bash
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo ci              # ↑ même chose, alias
cargo ci-offline      # ↑ + --offline (zéro réseau)
cargo ci-frozen       # = --frozen (no Cargo.lock update at all)

cargo clippy --fix --workspace --allow-dirty                # auto-fix
cargo fmt --all                                             # rustfmt
cargo fmt --all --check                                     # check format only
```

## Supply-chain audits (Google-grade)

```bash
cargo deny check                          # tous les axes (CVE+licence+bans+sources)
cargo deny check advisories               # CVE + yanked + unmaintained
cargo deny check licenses                 # licences whitelistées
cargo deny check bans                     # version dedup + denied
cargo deny check sources                  # registry/git origin

cargo vet                                 # audits signés ok
cargo vet suggest                         # quoi auditer ensuite
cargo vet certify <crate> <ver>           # marquer une dep auditée
cargo vet diff <crate> <v1> <v2>          # comparer 2 versions auditées
cargo vet fetch-imports                   # refresh feeds (imports.lock)

cargo audit-deny                          # alias = cargo deny check
cargo audit-vet                           # alias = cargo vet
cargo audit-machete                       # détecter unused deps
cargo audit-udeps                         # nightly unused deps detector
```

## Dependencies

```bash
cargo tree --workspace                                          # tree complet
cargo tree --workspace -i <crate>                               # qui dépend de <crate>
cargo tree --workspace --target all --all-features -i <crate>   # incluant deps target-spécifiques
cargo tree --workspace -e features -p <crate>                   # tree avec features
cargo tree --workspace -d                                       # duplicates

cargo update                                                    # update tout
cargo update -p <crate>                                         # update une seule dep
cargo update -p <crate> --precise <version>                     # pinner une version exacte
cargo generate-lockfile                                         # régénérer Cargo.lock from scratch

cargo info <crate>                                              # version dispo + features + license
cargo add <crate> --workspace                                   # ajouter via Cargo CLI
cargo remove <crate>                                            # retirer via Cargo CLI
```

## Workspace inspection

```bash
cargo metadata --format-version=1 | jq                          # JSON workspace info
cargo locate-project                                            # path du Cargo.toml courant
cargo pkgid <crate>                                             # identifier complet d'une dep
```

## Outils nightly

```bash
cargo +nightly miri test -p <crate>                             # UB detector
cargo +nightly udeps --workspace --all-targets                  # unused deps (nightly only)
cargo +nightly clippy --workspace -- -W clippy::pedantic        # pedantic warnings

RUSTFLAGS="-Z sanitizer=address" \
    cargo +nightly build -p <crate>                             # ASAN

cargo +nightly fmt -- --check                                   # rustfmt nightly
```

## Profile-Guided Optimization (PGO)

```bash
# 1. Instrumenter
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" \
    cargo build --profile dist --target x86_64-pc-windows-msvc

# 2. Run pour générer les traces
./target/x86_64-pc-windows-msvc/dist/cli.exe ...

# 3. Merger les profiles
llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

# 4. Re-compile avec les profiles
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata" \
    cargo build --profile dist --target x86_64-pc-windows-msvc
```

## Cross-compilation

```bash
cargo build --target aarch64-pc-windows-msvc          --workspace
cargo build --target x86_64-unknown-linux-gnu         --workspace
cargo build --target aarch64-unknown-linux-gnu        --workspace
cargo build --target x86_64-apple-darwin              --workspace
cargo build --target aarch64-apple-darwin             --workspace
cargo build --target wasm32-unknown-unknown           --workspace
cargo build --target wasm32-wasip1                    --workspace
```

Tous les targets sont déclarés dans `rust-toolchain.toml`.

## Documentation

```bash
cargo doc --workspace --no-deps --open                # ouvre la doc workspace
cargo doc --workspace --document-private-items        # inclut private items
```

## Cleanup

```bash
cargo clean --workspace                       # supprime target/
cargo clean -p <crate>                        # supprime target d'une crate
rm -rf target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-*  # cache CMake aws-lc-sys

cargo cache --remove-dir all                  # nettoie ~/.cargo/registry/cache
cargo cache --remove-if-younger-than 1week
```

## Quick env vars

```bash
# Build verbose
CARGO_LOG=debug cargo build

# Disable sccache temporairement
unset RUSTC_WRAPPER
cargo build

# Force offline
CARGO_NET_OFFLINE=true cargo build
```

## Diagnostic

```bash
# Pourquoi cette dep est-elle dans le graph ?
cargo tree --workspace --target all --all-features -i <crate>

# Pourquoi cette feature est-elle active ?
cargo tree --workspace --target all --all-features --edges features -i <crate>

# Versions multiples d'une même crate ?
cargo tree --workspace --target all --all-features --duplicates

# Que ferait `cargo update` ?
cargo update --dry-run

# Trace rust-analyzer / clippy
CARGO_LOG=cargo::core::resolver=trace cargo update
```
