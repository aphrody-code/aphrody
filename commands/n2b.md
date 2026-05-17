---
description: Run n2b audit and drive the Node.js → Bun-native migration on the current project.
argument-hint: "[audit|N|diff|analyze|prompt|rules|fix <path>|aggressive <path>|migrate|rollback] — ex: /n2b audit, /n2b 2, /n2b fix scripts/"
---

Drive the Node.js → Bun-native migration via the `n2b` CLI. Le binaire est détecté via `$PATH` (`command -v n2b`). Si un `bun/MIGRATION_PLAN.md` existe à la racine git, suivre le plan phasé et la matrice "migrer vs garder". Sinon mode audit/fix libre.

## Détection environnement (en premier)

```bash
N2B="$(command -v n2b)" || { echo "n2b introuvable — installer depuis ${CLAUDE_PLUGIN_ROOT}/docs/n2b/README.md"; exit 1; }
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
REPORTS_DIR="${PROJECT_ROOT}/bun/reports"
mkdir -p "$REPORTS_DIR"
```

Toujours utiliser ces variables, **jamais** de chemins absolus hardcodés.

## n2b CLI — surface complète (v0.4.0, 313 règles)

```bash
# Ignore globs canoniques (DRY — utiliser partout)
N2B_IGNORE=(--ignore 'node_modules/**' --ignore '.next/**' --ignore 'dist/**' \
            --ignore 'build/**' --ignore '.turbo/**' --ignore 'target/**' \
            --ignore '.git/**' --ignore '**/generated/**' --ignore '**/*.min.js')

# ── Scan (dry-run) ───────────────────────────────────
$N2B <path>                                      # text par défaut
$N2B <path> --report <fmt>                       # text|md|markdown|json|jsonl|sarif
$N2B <path> --quiet                              # supprime le summary
$N2B <path> --ignore <glob>                      # cumulable (+ respecte .n2bignore)
$N2B <path> --agent                              # LLM mode : ANSI off, logs stderr, stdout = payload

# ── Autofix (mutating) ───────────────────────────────
$N2B <path> --fix                                # safe : cli/*, imports/node-prefix, ci/*, shebang/*, husky/*
$N2B <path> --fix --aggressive                   # + api/* templateables + imports/bun-native
$N2B <path> --migrate                            # --fix --aggressive + side-effects

# ── Subcommands (13 actifs) ──────────────────────────
$N2B rules [--report md]                         # catalogue des 313 règles actives
$N2B prompt <path> [--max-findings N] [--include-info]
$N2B audit <path> [--state open|closed|all] [--limit N] [--term <kw>]   # GitHub issues/PRs
$N2B analyze <path> [--top-k N] [--threshold F] [--apply fix|aggressive]
$N2B rust {new,check,deps,doctor}                # scaffold Rust + toolchain
$N2B app {cli,tui,gui,bin}                       # Bun CLI/TUI/GUI/standalone
$N2B win32 <flavor>                              # Bun + Win32 (FFI windows-rs, PowerShell)
$N2B linux <flavor>                              # Bun + Linux bas-niveau
$N2B wasm {init,doctor,build,opt,size}           # Rust → WASM → Bun
$N2B bin {plugin,mdx,wasm}                       # Bun.build native plugin
$N2B patch <pkg> | --self                        # wrapper bun patch + n2b rules
$N2B bunpp                                       # couverture bun++ gaps Node
$N2B llmstxt <url>                               # génère llms.txt (siteone-crawler)
$N2B mui-to-md3 <path> [--write] [--stage-atomic] [--only Component]

# Exit codes : 0 = clean / fixed · 1 = dry-run findings · 2 = erreur
```

**Pas de `--rule <id>`** — filtrer via `<path>` ou post-filter JSONL avec `jq`.

## Modes de la commande

### `audit` (default, pas d'arg)

```bash
# Fast-path : skip si baseline plus récent que tout fichier source
needs_rescan() {
  [ ! -f "$1" ] && return 0
  [ -n "$(fd -e ts -e tsx -e js -e mjs -e cjs -e json --newer "$1" . 2>/dev/null | head -1)" ]
}

if needs_rescan "$REPORTS_DIR/n2b-baseline.jsonl"; then
  $N2B "$PROJECT_ROOT" --agent --report jsonl "${N2B_IGNORE[@]}" \
    > "$REPORTS_DIR/n2b-baseline.jsonl"
fi

jq -r '.rule_id' "$REPORTS_DIR/n2b-baseline.jsonl" | sort | uniq -c | sort -rn | head   # top règles
jq -r '.file'    "$REPORTS_DIR/n2b-baseline.jsonl" | sort | uniq -c | sort -rn | head   # top fichiers

$N2B "$PROJECT_ROOT" --report md > "$REPORTS_DIR/n2b-baseline.md"
```

Si `${PROJECT_ROOT}/bun/MIGRATION_PLAN.md` existe → recommander la prochaine phase.

### `<N>` — phase N (requiert MIGRATION_PLAN.md)

1. Pre-flight : `git status --porcelain` doit être vide
2. Déléguer à l'agent `n2b` (`subagent_type: n2b`) avec la phase cible
3. Gate de validation (voir plus bas)
4. Commit avec message conventional **exact** du plan
5. `$N2B "$PROJECT_ROOT" --report md > "$REPORTS_DIR/n2b-after-phase-<N>.md"`

### `fix <path>` — autofix safe scopé

1. `$N2B <path> --fix` (safe uniquement)
2. Gate de validation
3. Vert → confirmer avant commit · Rouge → `git restore .` + rapport

### `aggressive <path>` — autofix agressif scopé

1. **Confirmation explicite** (templates contextuels)
2. `$N2B <path> --fix --aggressive`
3. Gate obligatoire avant commit

### `diff` — delta baseline → current

```bash
diff \
  <(jq -r .rule_id "$REPORTS_DIR/n2b-baseline.jsonl" | sort | uniq -c) \
  <($N2B "$PROJECT_ROOT" --agent --report jsonl | jq -r .rule_id | sort | uniq -c)
```

### `migrate` — full auto (danger)

1. **Confirmation explicite obligatoire**
2. Refuse si `git status` pas clean
3. `$N2B "$PROJECT_ROOT" --migrate` — supprime lockfiles rivaux, `bun install`, migre workspaces, ajoute `@types/bun`
4. Gate obligatoire

### `rollback`

- Pas de commit → `git restore -W -S -- .` + proposer `git clean -fd <scope>`
- Commit fait → afficher hash, proposer `git reset --hard HEAD~1` avec **confirmation explicite**
- Jamais de force-push sans demande

### `analyze`, `prompt`, `rules`, `github`

```bash
$N2B analyze  "$PROJECT_ROOT" --top-k 3 --threshold 0.35 --report md
$N2B prompt   "$PROJECT_ROOT" --max-findings 100 --include-info > /tmp/n2b-prompt.md
$N2B rules --report md
$N2B audit    "$PROJECT_ROOT" --state all --limit 30 --report md
```

### `monorepo` — scan cross-workspace

Pour Turborepo / Bun workspaces. Scan **isolé par workspace** + agrégation, jamais d'autofix au root.

```bash
jq -e '.workspaces' "$PROJECT_ROOT/package.json" > /dev/null || { echo "pas un monorepo"; exit 1; }

# Liste workspaces (objet ou array)
WORKSPACES="$(jq -r '.workspaces | if type=="array" then .[] else .packages[] end' "$PROJECT_ROOT/package.json" \
  | xargs -I{} bash -c "ls -d $PROJECT_ROOT/{} 2>/dev/null")"

# Scan parallèle par workspace (4-8x plus rapide qu'une boucle serial)
echo "$WORKSPACES" | xargs -P "$(nproc)" -I{} bash -c '
  ws="{}"; name="$(basename "$ws")"
  '"$N2B"' "$ws" --agent --report jsonl --quiet "${N2B_IGNORE[@]}" \
    > "'"$REPORTS_DIR"'/ws-${name}.jsonl"
  echo "✓ $name ($(wc -l < "'"$REPORTS_DIR"'/ws-${name}.jsonl") findings)"
'

# Top workspaces par volume de findings (un seul scan racine)
$N2B "$PROJECT_ROOT" --agent --report jsonl "${N2B_IGNORE[@]}" \
  > "$REPORTS_DIR/n2b-monorepo.jsonl"
jq -r '.file | split("/")[:2] | join("/")' "$REPORTS_DIR/n2b-monorepo.jsonl" \
  | sort | uniq -c | sort -rn | head
```

Si `$PROJECT_ROOT/move.md` existe → déléguer la restructuration au `@move` agent plutôt que tenter un fix global.

## Hard rules — never break

- `api/process-env` : **skip** (portabilité SSR Next.js)
- `--aggressive` : scope étroit uniquement, jamais sur tout le repo
- `--migrate` : confirmation explicite, refuse si dirty
- Scope : toujours `$N2B <path>`, jamais `$N2B .` en autofix
- 1 phase = 1 commit conventional, jamais `--no-verify`
- Ignorer : `**/generated/**`, `prisma/schema.prisma`, lockfiles

## Gate de validation (auto-détectée)

```bash
has_script() { [ -f "$PROJECT_ROOT/package.json" ] && jq -e ".scripts[\"$1\"]" "$PROJECT_ROOT/package.json" > /dev/null; }

cd "$PROJECT_ROOT"
# Dev gate : laisse bun résoudre les nouvelles deps ajoutées par la phase.
# CI gate (séparé) : utiliser --frozen-lockfile pour bloquer drift.
bun install

# Tâches en parallèle (gate plus rapide)
pids=()
has_script build      && { bun run build       & pids+=($!); }
has_script lint       && { bun run lint        & pids+=($!); }
has_script type-check && { bun run type-check  & pids+=($!); }
has_script test       && { bun test            & pids+=($!); }
wait "${pids[@]}" || { echo "✗ gate failed"; exit 1; }

[ -f tsconfig.json ] && bun tsc --noEmit
```

Échec → corriger avant commit.
