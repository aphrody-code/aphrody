# Changelog

Toutes les évolutions notables de n2b — format [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versionnage [SemVer](https://semver.org/lang/fr/).

## [0.3.0] — 2026-04-18

Refactor massif pour maintenabilité et précision Rust↔Bun.

### Breaking changes

- **`packages/n2b-cli/` supprimé.** La CLI TypeScript `node2bun` (binaire compilé via `bun build --compile`) est retirée. Elle dupliquait un sous-ensemble de la CLI Rust sans consommateur externe détecté. **Aucune action requise côté utilisateur** — la CLI Rust `n2b` (dans `/usr/local/bin/n2b`) reste le point d'entrée canonique.
- **`@n2b/core` passe de 0.1.0 à 0.3.0 et change d'API.** Les scanners et règles implémentés en TypeScript sont retirés : toute la logique métier vit désormais dans le binaire Rust. `@n2b/core` expose maintenant :
  - `scan()`, `rules()`, `promptMarkdown()`, `binaryVersion()` — subprocess wrappers typés.
  - `n2bPlugin()` — `Bun.plugin()` qui délègue le scan au binaire Rust.
  - `shims/{env,fs,path,shell}` — Bun-native helpers pour les patterns Node que n2b signale le plus souvent.
- **Layout Cargo workspace réorganisé** : `rust/` et `native/` remplacés par `crates/{n2b-core, n2b-cli, n2b-native}`. Les consommateurs de source directe doivent mettre à jour leurs chemins.
- **Schéma `schema/v2.json` aligné sur l'implémentation réelle.** Les champs racine sont désormais `{schema_version, tool, version, mode, root, files_scanned, findings_total, files}` au lieu de `{version, tool_version, summary, files}`. **Aucune breaking change côté payload JSON** : le binaire a toujours émis cette forme, seul le schéma documenté a été synchronisé. Les scripts qui parsaient déjà la vraie sortie ne changent pas.

### Ajouté

- **Contrat CLI-as-API formel** : `tests/compare-baseline.sh` (13 assertions snapshot) + `crates/n2b-cli/tests/contract.rs` (9 tests `assert_cmd` avec validation `jsonschema` contre `schema/v2.json`). Toute régression sur l'output JSON casse la CI.
- **Codegen de types depuis le schéma** : `scripts/generate-schema-types.ts` produit `crates/n2b-core/src/schema.rs` (via `cargo-typify`) et `packages/n2b/src/schema.ts` (via `json-schema-to-typescript`). Mode `--check` en CI détecte la dérive.
- **Tests round-trip** : `crates/n2b-cli/src/schema_test.rs` désérialise les baselines capturées sur `test/fixture/` et `rpb-dashboard/` dans `N2bReport`.
- **Rollback transactionnel de migration** : `crates/n2b-cli/src/subprocess/bun.rs` — `BackupGuard` sauvegarde `package.json`, `pnpm-workspace.yaml` et les lockfiles rivaux en `.n2b-bak` avant side-effects ; `restore_all()` sur échec `bun install`.
- **Property tests** sur les scanners critiques (`scanners/package_json.rs`, `scanners/source.rs`) via `proptest`.
- **CI** : `.github/workflows/ci.yml` matrice Ubuntu+macOS — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `bun typecheck`, `scripts/compare-baseline.sh`, `scripts/generate-schema-types.ts --check`.
- **Configs lint** : `rustfmt.toml`, `clippy.toml` (msrv 1.75, `avoid-breaking-exported-api`).
- **Shims Bun** : `packages/n2b/src/shims/env.ts` (lecture typée de `Bun.env`), `fs.ts` (wrappers `Bun.file`), `path.ts` (équivalents `fileURLToPath`/`__dirname`), `shell.ts` (alias `Bun.$`).

### Modifié

- `crates/n2b-cli/src/main.rs` : **1566 → 37 lignes** (dispatch uniquement).
- `wasm_spec.rs` (1364) → `commands/wasm_spec/{mod,parser,codegen,validator}.rs`.
- `bin_cmd.rs` (1055) → `bin_cmd.rs` + `bin_cmd_gpu.rs` + `bin_cmd_templates.rs`.
- `win32_cmd.rs` (988) → `win32_cmd.rs` + `win32_cmd_com.rs` + `win32_cmd_templates.rs`.
- `commands/audit.rs` utilise `tokio::runtime::Builder::new_current_thread()` au lieu d'un runtime multi-thread global.
- **41 `unwrap()`/`panic!` durcis** en `.context(...)?` ou `.expect("invariant: …")` documentés.

### Supprimé

- Dossiers `rust/`, `native/`, `packages/n2b-cli/`.
- `packages/n2b/src/{scanners,rules,types.ts,util.ts,report.ts}`.
- `dist/node2bun`.

### Préservé (contrat externe gelé)

- Tous les subcommands et flags de la CLI.
- Exit codes 0/1/2.
- `rule_id` (catégorie/nom) immuables.
- ABI FFI cdylib v1 (`find_newlines_u16`, `node2bun_abi_version`).
- Format JSON v2 (schéma gelé, syncé sur l'implémentation).

---

## [0.2.0] — avant le refactor

État initial figé dans le commit `ba65272` (branche `refactor/unified-rust-core`, point de départ). Voir l'historique git pour les détails.
