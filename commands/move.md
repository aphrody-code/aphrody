---
description: Drive la migration monorepo ~/rg + ~/rpb-dashboard → ~/vps (plan dans ~/vps/move.md).
argument-hint: "[bootstrap|preflight|phase N|all|verify N|rollback N|dry-run N|status] — ex: /move all --stop-at 6, /move phase 0.5, /move rollback 2"
---

Pilote la consolidation en Turborepo unique selon le plan factuel `$VPS/move.md` (**v2.2 — 2026-04-19**, ~970 lignes).

## Environment

```bash
: "${VPS:=$HOME/vps}"
: "${CLAUDE_PLUGIN_ROOT:?}"
SCRIPTS="$CLAUDE_PLUGIN_ROOT/scripts"
STATE="$VPS/.migration-state.json"
JOURNAL="$VPS/.migration-journal.log"
```

## Modes

### `bootstrap` — pré-requis avant toute phase

Régénère `.migration-ready/` (artifacts package.json/turbo.json/biome.json/tsconfig.base.json), build le bot SWC, snapshot DB Supabase (best-effort).

```bash
bun "$SCRIPTS/move-bootstrap.ts"        # idempotent
bun "$SCRIPTS/move-bootstrap.ts" --force  # force regen artifacts
```

### `preflight` (default si pas d'arg)

Audit 30+ checks des 3 repos + détection de la phase courante.

```bash
bun "$SCRIPTS/move-preflight.ts"
```

Output : `Phase courante : X · Prochaine : Y · Blocker : ...` + checklist ✓/✗.

### `phase <N>` — exécute une phase atomique

```bash
bun "$SCRIPTS/move-phase.ts" 0      # commit dirty + backups + DB snapshot
bun "$SCRIPTS/move-phase.ts" 0.5    # git init vps + submodule bun-agent
bun "$SCRIPTS/move-phase.ts" 1      # turborepo root + infra/ reorg
bun "$SCRIPTS/move-phase.ts" 2      # subtree import rg → apps/{website,azalee} + packages
bun "$SCRIPTS/move-phase.ts" 3      # subtree import rpb-dashboard → apps/{rpb-dashboard,rpb-bot}
bun "$SCRIPTS/move-phase.ts" 4      # catalog unifié (FULL AUTO) + renommages @rpb/*
bun "$SCRIPTS/move-phase.ts" 5      # paths nginx/systemd → /home/ubuntu/vps/apps/*
bun "$SCRIPTS/move-phase.ts" 6      # build offline (turbo type-check/ci/build + bot SWC) + tag
bun "$SCRIPTS/move-phase.ts" 6.5    # (optionnel) preview Vercel website — Annexe E
bun "$SCRIPTS/move-phase.ts" 7      # BASCULE LIVE (--yes requis pour skip prompt)
bun "$SCRIPTS/move-phase.ts" 8      # cleanup (rg → rg.old, --yes requis)
```

### `all` — exécution A→Z autonome (recommandé)

```bash
bun "$SCRIPTS/move-phase.ts" all --stop-at 6 --yes   # enchaîne 0→6, stop avant live
bun "$SCRIPTS/move-phase.ts" all --from 4 --yes      # reprise après crash phase 3
bun "$SCRIPTS/move-phase.ts" all --yes               # tout d'un coup (DANGER : Phase 7/8 non confirmées)
```

Skip auto les phases déjà dans `state.completed_phases`. Auto-verify après chaque phase (rollback si échec).

### `verify <N>` — re-valide les invariants d'une phase

```bash
bun "$SCRIPTS/move-verify.ts" 4    # check catalog appliqué, 0 bun.lock workspace
```

Read-only, utile après debug manuel.

### `dry-run <N>` — affiche sans exécuter

```bash
bun "$SCRIPTS/move-phase.ts" <N> --dry-run
```

### `rollback <N>` — `git reset --hard HEAD~1`

```bash
bun "$SCRIPTS/move-phase.ts" --rollback <N>
```

Si Phase 7 déjà faite → restaurer aussi `infra/systemd/.bak/*.service` + `nginx/.bak/*.conf` manuellement.

### `status` — état migration

```bash
bun "$SCRIPTS/move-phase.ts" --status               # state JSON + last_error
cd "$VPS" && git log --oneline -10 && git status --porcelain
tail -30 "$JOURNAL"                                  # journal append-only
```

## Règles dures

- **`bootstrap` une fois** avant la première phase (régénère `.migration-ready/`)
- **Toujours `preflight`** avant une phase manuelle pour confirmer l'état
- **Jamais sauter Phase 0.5** (`git init vps`) — Phase 2+ l'exige
- **Phase 7 (live)** → confirmation explicite + maintenance déjà active (cf. `state.md`)
- **Build doit être vert** (Phase 6) avant Phase 7
- **Pas de `git push --force`** sur rg/rpb pendant la migration
- **Phase 8** RENOMME en `.old`, ne supprime pas — garder `.old` pendant 30j
- **`--yes` interdit pour Phases 7 & 8** sans accord explicite utilisateur

## State files

- `$VPS/.migration-state.json` — `completed_phases`, `current_phase`, `last_error`
- `$VPS/.migration-lock` — exclusion mutuelle (auto-supprimé sur exit propre)
- `$VPS/.migration-journal.log` — journal append-only

Si lock orphelin : `rm $VPS/.migration-lock` après vérif `ps aux | grep move-phase`.

## Délégation auto

- Phase 4 (Node→Bun audit) → `@n2b`
- Review post-phase 2/3/5 → `@bun-reviewer`
- Phase 7 (sudo + healthcheck) → `@bun-deployer`
- Phase 6.5 Vercel preview → `@vercel:deployment-expert`
- Audit paths résiduels → `@bun-explorer`

## Toolchain installée

- `n2b 0.4.0` → `/usr/local/bin/n2b` (Node→Bun only — `mui-to-md3` déplacé vers crate `mui-rs`)
- `bun-agent v2.2.0` user-scope (cf. `~/.claude/settings.json:enabledPlugins`)
- `bun 1.3.12` (matche `packageManager` cible monorepo)
- Plugins migration-ready actifs : `bun-agent`, `commit-commands`, `context7`, `vercel`. Inutiles désactivés (`web3d-agent`, `rust-analyzer-lsp`, `claude-code-setup`, `plugin-dev`).

## Plan de référence

**Toujours relire** `$VPS/move.md` (v2.2, 2026-04-19) avant d'exécuter :
- §0 contraintes & versions clés (bun 1.3.12, next 16.2.4, react 19.2.5, prisma 7.7.0)
- Annexe B : 25+ commandes reproductibles
- Annexe C : checklist veille (n2b/bun-agent versions)
- Annexe D : pilotage `bun-agent` (preflight/phase/rollback/all)
- Annexe E : sanity check Vercel website (Phase 6.5)
- Annexe F : exclusions (azalee/rpbey NO-GO Vercel — bucket menu + Discord bot)
