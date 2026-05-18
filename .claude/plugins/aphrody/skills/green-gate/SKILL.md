---
name: green-gate
description: Run the full n2b verification pipeline — cargo build/test/clippy/fmt + baseline diff + codegen drift check. Returns red/green per stage and aborts on first failure. Mirrors the contract gelé verrou triple from CLAUDE.md.
disable-model-invocation: true
---

# green-gate — Pipeline de vérification n2b

Lance la suite anti-régression complète, dans l'ordre, et stoppe au premier échec. C'est le filet à tirer avant tout `git push origin main` ou tout deploy.

## Usage

```
/green-gate
```

Ou directement :

```bash
bash skills/green-gate/run.sh
```

## Étapes (dans l'ordre)

| # | Étape | Commande | Échec = |
|---|---|---|---|
| 1 | Format Rust | `cargo fmt --all -- --check` | drift de format → `cargo fmt --all` |
| 2 | Build workspace | `cargo build --workspace` | erreur de compilation Rust |
| 3 | Tests workspace | `cargo test --workspace` | unit/contract/proptest cassés |
| 4 | Clippy strict | `cargo clippy --workspace --all-targets -- -D warnings` | warning lint = erreur |
| 5 | Codegen drift | `bun run codegen:schema:check` | `schema.rs` ou `index.ts` divergent de `schema/v2.json` |
| 6 | Baselines | `bash tests/compare-baseline.sh` | sortie `n2b` ≠ snapshots tracked |

## Exit codes

- `0` : tout vert — safe to push/deploy
- `1` : au moins une étape rouge (voir stderr)

## Quand l'utiliser

- **Avant `git push origin main`** : garantit qu'aucune modification ne casse le contrat externe consommé par rpb-dashboard.
- **Avant `sudo install`** : confirme que le binaire qu'on s'apprête à déployer est build-test-cliclean.
- **Après merge upstream** : vérifie qu'aucun rebase/merge n'a introduit un drift silencieux.
- **En CI fallback** : si la pipeline GitHub Actions est indisponible.

## Quand NE PAS l'utiliser

- En cours de développement actif — utilise `cargo test -p <crate>` ou `cargo check` pour la boucle rapide.
- Avant un commit purement docs/skills — overkill (le pipeline tourne 30-60s).

## Sortie attendue (succès)

```
[1/6] cargo fmt --check          ✓ OK
[2/6] cargo build --workspace    ✓ OK (12.4s)
[3/6] cargo test --workspace     ✓ OK (50+ tests, 0 failed)
[4/6] cargo clippy -D warnings   ✓ OK
[5/6] codegen drift              ✓ OK (schema.rs + index.ts in sync)
[6/6] baselines                  ✓ OK (7/7 OK, rpb-dashboard skipped)

green-gate: ALL GREEN — safe to push/deploy
```

## Ancrage CLAUDE.md

Cette suite matérialise le **filet de sécurité triple** documenté dans CLAUDE.md sous "Contrat externe gelé" :
- `tests/compare-baseline.sh` (étape 6)
- `crates/n2b-cli/tests/contract.rs` via `cargo test` (étape 3)
- `crates/n2b-cli/src/schema_test.rs` via `cargo build` (étape 2)
