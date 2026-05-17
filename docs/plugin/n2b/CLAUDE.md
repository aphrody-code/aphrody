# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Principe directeur

**Rust = moteur unique. TypeScript = façade thin.** Toute la logique métier (scan, règles, rendu) vit dans `crates/n2b-core`. Le package TS `@n2b/core` ne fait que spawn le binaire `n2b` et parser son JSON. Ne jamais réintroduire de scanners/règles côté TS.

## Commandes essentielles

```bash
# Build
cargo build --release -p n2b                      # binaire CLI seul
cargo build --release --workspace                 # + cdylib
sudo install -m755 target/release/n2b /usr/local/bin/n2b

# Tests (34 tests au total répartis comme suit)
cargo test --workspace                            # 14 Rust (schema_test + contract + 3 proptest)
bun test packages/n2b/                            # 14 TS (cli + shims)
bash tests/compare-baseline.sh                    # 13 assertions snapshot CLI-as-API

# Test ciblé
cargo test --workspace contract                   # juste les contract tests
cargo test -p n2b-core --test proptest_source     # une proptest précise
bun test packages/n2b/test/shims/env.test.ts      # un fichier TS

# Lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
bun run typecheck

# Codegen depuis schema/v2.json (source unique)
bun run codegen:schema                            # régénère schema.rs + schema.ts
bun run codegen:schema:check                      # --check (CI drift detection)

# Régénérer les baselines après bump version ou changement de sortie légitime
PATH="$PWD/target/release:$PATH" N2B=./target/release/n2b \
  bash -c 'for f in text json jsonl md sarif; do
    ext=$f; [[ $f == text ]] && ext=txt
    $N2B test/fixture --report=$f > tests/snapshots/baseline/fixture.$ext
  done'
```

## Contrat externe gelé — ne jamais casser

Ces surfaces sont consommées par `/home/ubuntu/rpb-dashboard` via subprocess. Toute modification casse la CI baseline :

| Surface | Fichier de vérité |
|---|---|
| Flags et subcommands CLI | `crates/n2b-cli/src/cli/args.rs` |
| Format JSON v2 | `schema/v2.json` (schéma gelé, bumpé en v3 si breaking) |
| Rule IDs (`cli/npm`, `imports/node-prefix`, …) | `crates/n2b-core/src/rules/*.rs` |
| Exit codes `0`/`1`/`2` | `crates/n2b-cli/src/cli/dispatch.rs` |
| ABI cdylib v1 (`find_newlines_u16`, `node2bun_abi_version`) | `crates/n2b-native/src/lib.rs` |

Le filet de sécurité est **double** :
- `tests/compare-baseline.sh` — diff octet-à-octet contre `tests/snapshots/baseline/` et `tests/rpb-dashboard-baseline/`
- `crates/n2b-cli/tests/contract.rs` — 9 tests `assert_cmd` + validation `jsonschema` contre `schema/v2.json`

Si tu changes une règle existante, il faut **soit** justifier le breaking et régénérer les baselines, **soit** ajouter une nouvelle règle. Jamais modifier silencieusement.

## Architecture — pipeline de scan

```
  entry → crates/n2b-cli/src/main.rs (dispatch only)
        → cli::dispatch::run_from_args (args.rs → enum Cmd)
        → commands/{scan,rules,audit,migrate,prompt,mui_to_md3}.rs
            → n2b_core::run::run(opts)
                → engine walk (ignore + globset + crossbeam)
                → dispatch scanner par extension/nom de fichier
                    scanners/*.rs retournent Vec<Finding>
                    rules/*.rs fournissent les regex/IDs partagés
                → report/{text,json,jsonl,markdown,sarif}.rs
        → rust_cmd.rs         # n2b rust {new,check,deps,doctor}
        → commands/mui_to_md3.rs  # n2b mui-to-md3 (MUI v9 → @md3-ui/core)
```

### Commandes ajoutées en v0.3.0

| Commande | Fichier | Description |
|---|---|---|
| `n2b mui-to-md3 [root]` | `commands/mui_to_md3.rs` | Codemod MUI v9 → @md3-ui/core. Règles YAML embarquées depuis `rules/mui-to-md3.yaml`. Flags : `--write`, `--stage-atomic`, `--only <COMPONENT>`, `--rewrite-sx`, `--rules <path>`, `--report`. |
| `n2b rust new <name>` | `rust_cmd.rs` | Scaffold Rust (flavors : bin/lib/cdylib/proc-macro/workspace/axum/discord/cli/tauri/leptos/tui/bevy/grpc). |
| `n2b rust check [path]` | `rust_cmd.rs` | `cargo check` + `cargo clippy`. |
| `n2b rust deps [path]` | `rust_cmd.rs` | `cargo outdated` + `cargo audit`. |
| `n2b rust doctor` | `rust_cmd.rs` | Vérifie la toolchain Rust (rustc, clippy, wasm-pack…). |

**Règles MUI → MD3** : `rules/mui-to-md3.yaml` est la source de vérité. Embarqué via `include_str!` au build — pas de lecture runtime si `--rules` non précisé.

**Point clé** : un scanner ne connaît pas les règles, un rule ne connaît pas les scanners. Le contrat est `Finding` (défini dans `schema/v2.json` → généré dans `schema.rs`). Pour ajouter une règle, soit tu ajoutes un scanner (nouveau type de fichier), soit tu enrichis un scanner existant avec un nouveau regex dans `rules/`.

## Codegen schema-first

`schema/v2.json` est la source de vérité unique. `scripts/generate-schema-types.ts` produit :
- `crates/n2b-core/src/schema.rs` via `cargo-typify`
- `packages/n2b/src/schema.ts` via `json-schema-to-typescript`

Les deux fichiers générés sont **commités** et la CI échoue si drift. Jamais éditer ces fichiers à la main — modifier `schema/v2.json` puis relancer le codegen.

## Rollback transactionnel des migrations

`crates/n2b-cli/src/subprocess/bun.rs` fournit `BackupGuard` avec `Drop`. Tout side-effect de `--migrate` (`bun install`, retrait lockfiles rivaux, mutations `package.json`) doit passer par ce guard pour garantir un restore sur panic ou échec subprocess. Ne jamais écrire de side-effect migration en dehors de `commands/migrate.rs`.

## Règles du dépôt parent

- **Bun uniquement** (jamais `node`/`npm`/`npx`/`pnpm`/`yarn`) — c'est ironiquement ce que n2b détecte.
- **CLI Rust au lieu des binaires GNU** (`rg` au lieu de `grep`, `fd` au lieu de `find`, `bat` au lieu de `cat`, etc.).

## Gotchas

- **proptest default = 256 cas** pour les 3 property tests sur `scanners/{package_json,source}.rs`. Si un panic historique est découvert, il faut le corriger dans le scanner plutôt que baisser le nombre de cas.
- **Bump de version** : `crates/n2b-core/Cargo.toml`, `crates/n2b-cli/Cargo.toml`, `packages/n2b/package.json` (3 fichiers). `crates/n2b-native` reste à 0.1.0 (ABI v1 gelée, ne bump que sur breaking ABI). Toujours régénérer les baselines après bump car la version apparaît dans JSON/JSONL/SARIF.
- **`n2b rules` retourne un tableau plat** (pas `{rules: [...]}`) — rpb-dashboard le parse ainsi.
- **`--report=md` et `--report=markdown` doivent être équivalents** (test contract dédié).
- Le flag `--aggressive` active les règles `api/*` (réécrit APIs Node en Bun) ; `--migrate` = `--fix --aggressive` + side-effects avec rollback.
