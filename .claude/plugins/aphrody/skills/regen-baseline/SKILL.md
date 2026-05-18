---
name: regen-baseline
description: Regenerate the 7 tracked baseline snapshots (5 fixture formats + 2 rules formats) after a legitimate output change. Required after version bumps, schema additions, or new rule registrations. Refuses to run if `n2b --version` differs from the version in package.json/Cargo.toml.
disable-model-invocation: true
---

# regen-baseline — Régénération des snapshots tracked

Régénère les 7 baselines `tests/snapshots/baseline/*` à partir de la binaire `n2b` release courante. Trace au commit ce qui a légitimement changé dans la sortie.

## Usage

```
/regen-baseline
```

Ou directement :

```bash
bash skills/regen-baseline/run.sh
```

## Quand l'utiliser

- **Bump de version** (`crates/n2b-core/Cargo.toml` + `crates/n2b-cli/Cargo.toml` + `packages/n2b/package.json`) — la version apparaît dans `meta.tool_version` du JSON/JSONL/SARIF.
- **Ajout d'une règle** — la fixture `test/fixture/` peut produire un nouveau finding, baseline change.
- **Modification d'un format de rendu** (`crates/n2b-report/src/lib.rs`) — text/json/jsonl/md/sarif.
- **Modification du registre** (`crates/n2b-registry/registry/*.toml`) qui touche les findings sur la fixture.

## Quand NE PAS l'utiliser

- **Avant** d'avoir compris pourquoi le diff existe — un baseline qui change "tout seul" indique un breaking change silencieux du contrat. Toujours diff manuellement avant de régénérer.
- Si rien n'a légitimement changé — préférer `bash tests/compare-baseline.sh` pour confirmer le green.

## Étapes (script `run.sh`)

1. Vérifie qu'on est dans le repo n2b
2. Vérifie que `target/release/n2b` existe et est à jour (sinon : `cargo build --release -p n2b`)
3. Vérifie que la version de `n2b --version` correspond à `Cargo.toml` (anti-stale binary)
4. Régénère les 5 baselines fixture (text/json/jsonl/md/sarif)
5. Régénère les 2 baselines rules (json + text)
6. Affiche `git diff --stat tests/snapshots/baseline/` pour relecture humaine

## Sortie attendue

```
=== regen-baseline ===
binary: n2b 0.5.0 (matches Cargo.toml ✓)
[1/7] tests/snapshots/baseline/fixture.txt    (regenerated)
[2/7] tests/snapshots/baseline/fixture.json   (regenerated)
[3/7] tests/snapshots/baseline/fixture.jsonl  (regenerated)
[4/7] tests/snapshots/baseline/fixture.md     (regenerated)
[5/7] tests/snapshots/baseline/fixture.sarif  (regenerated)
[6/7] tests/snapshots/baseline/rules.json     (regenerated)
[7/7] tests/snapshots/baseline/rules.txt      (regenerated)

git diff --stat:
  tests/snapshots/baseline/fixture.json   |  2 +-
  tests/snapshots/baseline/fixture.jsonl  |  2 +-
  tests/snapshots/baseline/fixture.sarif  |  2 +-

→ Relire le diff avant de commit. Si 7 fichiers diff alors qu'on attendait 3, STOP.
```

## Sécurité

- N'écrit JAMAIS dans `tests/rpb-dashboard-baseline/` (ces baselines viennent du consommateur, pas de n2b).
- Relit `n2b --version` *avant* d'écrire — refuse si le binaire est stale (mismatch Cargo.toml).
- Affiche le diff après régénération — `git add` reste manuel pour forcer la relecture humaine.
