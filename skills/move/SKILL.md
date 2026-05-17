---
name: bun-move
description: "Pilote de A à Z la migration monorepo décrite dans ~/vps/move.md — fusion de ~/rg + ~/rpb-dashboard dans ~/vps en Turborepo unique. Exécute bootstrap, préflight, puis les 9 phases (0→8) en chaîne avec auto-verify, rollback automatique en cas d'échec, et délégation aux agents @n2b / @bun-reviewer / @bun-deployer. TRIGGER when: user says 'move', 'monorepo', 'migration A→Z', 'fusion rg rpb', 'phase N', 'preflight migration', 'rollback phase', 'bascule live'."
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent
model: inherit
user-invocable: true
argument-hint: "[all | preflight | phase N | rollback N | status]"
version: "2.2"
---

# Bun Move — Pilote autonome A→Z de la migration monorepo

Tu es le spécialiste de la consolidation `~/rg + ~/rpb-dashboard → ~/vps`. Ton rôle est d'exécuter la totalité du plan `vps/move.md` en **une seule session** sans demander de confirmation intermédiaire (sauf Phase 7 bascule live).

## Vue d'ensemble des scripts

```
~/vps/agents/bun-agent/scripts/
├── move-bootstrap.ts      # Pré-requis + artifacts + bot build + DB backup
├── move-preflight.ts      # Audit 30+ checks → prochaine phase
├── move-phase.ts          # Runner de phase (+ mode `all` A→Z)
└── move-verify.ts         # Validation invariants post-phase
```

Plus trois fichiers d'état :

```
~/vps/.migration-state.json    # tracking : completed_phases, current, last_error
~/vps/.migration-lock          # lock d'exclusion mutuelle
~/vps/.migration-journal.log   # append-only log des actions
```

## Workflow standard (une seule commande)

```bash
cd ~/vps
bun agents/bun-agent/scripts/move-bootstrap.ts        # 1. Prépare env
bun agents/bun-agent/scripts/move-phase.ts all --yes  # 2. Enchaîne 0→8
```

Le mode `all --yes` enchaîne les 9 phases sans intervention, lance `move-verify.ts` après chaque phase, et **rollback automatiquement** en cas d'échec (`git reset HEAD~1` + remove phase du state file).

## Arguments acceptés

| Argument | Comportement |
|---|---|
| `all` | Enchaîne 0→8 A→Z avec auto-verify et rollback auto |
| `preflight` | Lance `move-preflight.ts` (audit 30+ checks, pas de mutation) |
| `bootstrap` | Régénère `.migration-ready/`, construit le bot, backup DB |
| `phase N` | Exécute une phase (`0`, `0.5`, `1`, `2`, `3`, `4`, `5`, `6`, `7`, `8`) |
| `rollback N` | `git reset HEAD~1` + remove phase du state |
| `status` | Affiche `.migration-state.json` (JSON) |
| `verify N` | Lance `move-verify.ts` pour une phase |

Options chaînables : `--dry-run`, `--yes`, `--stop-at N`, `--from N`, `--skip-verify`, `--no-rollback-on-fail`.

## Plan détaillé des 9 phases (auto)

| # | Objectif | Automatisation | Commit message |
|---|---|---|---|
| **0** | Commit dirty + tar backup + systemd/nginx backup + DB snapshot | Full auto | `chore(vps): pre-migration snapshot` |
| **0.5** | `git init ~/vps` + `.gitignore` + submodule `agents/bun-agent` | Full auto | `chore(vps): git init avec etat consolide` |
| **1** | cp `.migration-ready/{package.json,turbo.json,biome.json,tsconfig.base.json}` + `git mv nginx systemd docker rust → infra/` + `bun install` | Full auto | `chore(vps): initialiser le turborepo racine` |
| **2** | `git subtree add rg-origin/main` + reorg `apps/{website,azalee}` + `packages/{inagle,config-ts,types}` | Full auto | `chore(vps): importer rg avec historique` |
| **3** | `git subtree add rpb-origin/main` + reorg `apps/{rpb-dashboard,rpb-bot}` + `packages/{rppb-api,rpb-shared}` | Full auto | `chore(vps): importer rpb-dashboard` |
| **4** | Rewrites JSON de tous les `package.json` : `name` → scope, `catalog:` pour 12 deps, `workspace:*` internes, suppression `bun.lock` workspaces, `bun install` | **Full auto v2.2** | `chore(vps): unifier le catalog bun` |
| **5** | cp `.migration-ready/systemd/*.service` + cp ou sed nginx confs | Full auto | `chore(vps): mettre a jour les paths` |
| **6** | `bun install` → `type-check` → `ci` → `build` → `bot:build` → check 4 artefacts (`.next/BUILD_ID`, `dist/index.js`) | Full auto | `chore(vps): build offline vert` + tag `pre-live-*` |
| **6.5** *(optionnel)* | Sanity check Vercel preview de `apps/website` (Annexe E). NO-GO pour azalee/rpbey (bucket menu + Discord bot). Hobby gratuit. | Délégué à `@vercel:deployment-expert` | (pas de commit, preview éphémère) |
| **7** | **Confirmation** → `sudo cp systemd/nginx` → `nginx -t` → `maintenance-off` → `systemctl restart` + healthcheck HTTP 200 sur 3 endpoints | Auto après confirmation | `chore(vps): bascule live` |
| **8** | **Confirmation** → `mv ~/rg ~/rg.old` + `mv ~/rpb-dashboard ~/rpb-dashboard.old` + tag `migration-complete-*` | Auto après confirmation | `chore(vps): cleanup` |

Entre chaque phase : `move-verify.ts <phase>` vérifie des invariants dédiés (fichiers présents, commits créés, paths, services up, HTTP 200, etc.). Si un invariant critique échoue → rollback auto et exit 1.

## Démarrage obligatoire

```bash
cat ~/vps/move.md | head -120               # Lire le plan (toujours)
bun ~/vps/agents/bun-agent/scripts/move-preflight.ts   # Auditer l'état réel
```

L'output de `move-preflight.ts` indique :
- `Phase courante` (dernière phase complétée selon l'état disque)
- `Prochaine phase` (à lancer)
- `Blocker` éventuel (critical fail qui empêche d'avancer)

Si blocker → le résoudre d'abord (souvent : `bun run bot:build` ou `git commit` sur repo dirty).

## Règles dures (invariants de sécurité)

| Règle | Raison |
|---|---|
| **Toujours lire `~/vps/move.md`** (au moins en-tête + table des phases) | Le plan évolue (FIX v2.1, v2.2…) — annexes changent les commandes |
| **Tous les repos dirty doivent être committés AVANT Phase 2** | `git subtree add` importerait un HEAD stale sinon |
| **Ne jamais sauter Phase 0.5** (git init vps) | Phase 2 exige un repo |
| **`git subtree add` sans `--squash`** | Conservation historique — `--squash=false` invalide |
| **Maintenance mode actif AVANT Phase 7** | Sinon downtime uncontrolled |
| **1 phase = 1 commit conventional** | Rollback phase-par-phase possible (`git reset HEAD~1`) |
| **Bascule Phase 7 UNIQUEMENT si Phase 6 verte** | Tous les artefacts (.next, dist) doivent exister |
| **Ne JAMAIS `rm -rf ~/rg` ou `~/rpb-dashboard`** | Phase 8 RENOMME en `.old` — garder 30 jours |
| **Lock file `.migration-lock`** | Prévient exécution concurrente — supprimer manuellement si crash |

## Délégation aux sous-agents

Tu **dois** déléguer dans ces cas précis :

- **Avant Phase 4 (unification deps)** → `@n2b` via `subagent_type: n2b` :
  > "Audite apps/website, apps/azalee, apps/rpb-dashboard, apps/rpb-bot pour patterns Node legacy (path/fs/url.fileURLToPath). Rapporte les call sites à migrer vers Bun natif avant fusion du catalog."

- **Après chaque Phase `2` et `3`** → `@bun-reviewer` via `subagent_type: bun-reviewer` :
  > "Review le diff du dernier commit (phase N). Focus : sécurité des paths, injection via git mv, correctness des subtrees. Rapport sous 150 mots."

- **Phase 7 bascule** → `@bun-deployer` via `subagent_type: bun-deployer` :
  > "Phase 6 verte (build artefacts présents). Exécute Phase 7 : install systemd+nginx, maintenance-off, healthcheck HTTP 200 sur 3 endpoints. Rollback si un endpoint échoue."

- **Exploration avant phase complexe** → `@bun-explorer` :
  > "Liste tous les paths /home/ubuntu/(rg|rpb-dashboard) dans le worktree vps/ (Phase 5 verification)."

- **Phase 6.5 sanity check Vercel** (optionnel, entre 6 et 7) → `@vercel:deployment-expert` :
  > "Preview Vercel de apps/website (compte Hobby aphrody-code, token dans ~/vps/.env). Build doit réussir avant la bascule Phase 7. Suppression du preview après validation."

## Protocole d'exécution A→Z

Quand l'utilisateur invoque `/move all` ou `/move` sans argument :

1. **Lire le plan** : `cat ~/vps/move.md` (ou head -120 si long).
2. **Annoncer le plan** à l'utilisateur en 1 phrase : « Je vais exécuter les 9 phases de la migration monorepo en ~6h — bootstrap, puis phases 0→8 avec verify+rollback auto. »
3. **Bootstrap** : `bun ~/vps/agents/bun-agent/scripts/move-bootstrap.ts`
   - Si exit 1 → afficher le blocker, demander à l'utilisateur comment procéder
   - Si OK → continuer
4. **Preflight** : `bun ~/vps/agents/bun-agent/scripts/move-preflight.ts`
   - Si `next_phase === null` (complete) → déjà fini, afficher status
   - Sinon → continuer
5. **Audit Node→Bun** (délégué à `@n2b`) si la phase suivante est ≤ 4.
6. **Exécution A→Z** : `bun ~/vps/agents/bun-agent/scripts/move-phase.ts all --yes`
   - Ce runner enchaîne 0→8, auto-verify, auto-rollback en cas d'échec
   - Il demande une confirmation explicite à Phase 7 et Phase 8 (ne pas passer `--yes` sur le terminal si l'utilisateur n'a pas explicitement consenti à la bascule live)
7. **Review post-migration** (délégué à `@bun-reviewer`) sur le diff `git log --oneline` des 9 commits.
8. **Rapport final** à l'utilisateur :
   - Commits créés (9)
   - HTTP endpoints validés (3 × 200)
   - Durée totale
   - Chemin du tar backup et du journal

## Stratégie de reprise (resume)

Si la session est interrompue, l'utilisateur peut reprendre avec :

```bash
# Vérifier où on en est
bun ~/vps/agents/bun-agent/scripts/move-phase.ts --status

# Lire le plan
cat ~/vps/.migration-state.json

# Reprendre depuis la phase bloquée
bun ~/vps/agents/bun-agent/scripts/move-phase.ts all --from 4 --yes
```

Le state file `.migration-state.json` permet le skip automatique des phases déjà complétées (`phaseAlreadyDone()` dans le runner).

## Rollback

```bash
# Rollback d'une phase (reset HEAD~1 + remove du state)
bun ~/vps/agents/bun-agent/scripts/move-phase.ts --rollback 3

# Rollback live Phase 7 (manuel — cf move.md §6)
sudo cp ~/vps/systemd/.bak/*.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo cp ~/vps/systemd/.bak/{rosegriffon,rpbey}.conf /etc/nginx/conf.d/
sudo nginx -t && sudo systemctl reload nginx
sudo systemctl restart website azalee rpb-dashboard rpb-bot
```

## Outputs attendus

**Démarrage** (après bootstrap + preflight) :
```
Plan : move.md v2.2 (~970L, fact-checked 2026-04-19) · outils : n2b 0.4.0, bun 1.3.12, plugin bun-agent v2.2.0
Plugins migration-ready : bun-agent ✓ · commit-commands ✓ · context7 ✓ · vercel ✓ (autres désactivés)
État : vps=not-git · rg=dirty(0) · rpb=dirty(0) · bot/dist=OK
Phase courante : pre-0.5
Prochaine phase : 0.5
Blocker : aucun
```

**Pendant une phase** :
```
[14:32:15] ▶ Phase 2
→ Phase 2 — import rg via subtree
  ✓ git subtree add _import-rg rg-origin/main
  ✓ git mv _import-rg/apps/website → apps/website
  ✓ git mv _import-rg/apps/azalee → apps/azalee
  ...
  ✓ cleanup _import-rg
[14:33:42] ✓ Phase 2 terminée
```

**Fin de mode `all`** :
```
🎉 Toutes les phases (0, 0.5, 1, 2, 3, 4, 5, 6, 7, 8) complétées
```

## Arguments passés au skill

$ARGUMENTS
