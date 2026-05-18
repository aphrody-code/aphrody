---
name: bun-n2b
description: "Audit ou migrate Node.js → Bun natif via le CLI n2b. TRIGGER when: user says 'migrate to bun', 'node to bun', 'bun-native', 'n2b audit', 'check for node patterns', 'rewrite to bun-native APIs', or asks about replacing fs/child_process/node-fetch with Bun equivalents. Also triggers after a major refactor where Node APIs are used."
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent
model: inherit
---

# n2b — Node.js → Bun-native migration skill

Scan et réécrit automatiquement les usages Node legacy vers les APIs Bun natives. Binaire détecté via `$PATH`, jamais hardcodé.

## Invocation patterns

```bash
# Détection env (toujours en premier)
N2B="$(command -v n2b)" || { echo "n2b absent"; exit 1; }
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
REPORTS_DIR="${PROJECT_ROOT}/bun/reports"; mkdir -p "$REPORTS_DIR"
```

### Audit (non-mutating)

```bash
# Top findings
$N2B "$PROJECT_ROOT" --agent --report jsonl \
  --ignore 'node_modules/**' --ignore '.next/**' --ignore 'dist/**' --ignore '**/generated/**' \
| tee "$REPORTS_DIR/n2b-baseline.jsonl" \
| jq -r '.rule_id' | sort | uniq -c | sort -rn | head

# Rapport MD lisible
$N2B "$PROJECT_ROOT" --report md > "$REPORTS_DIR/n2b-baseline.md"
```

### Fix scopé

```bash
# Safe (jamais sur tout le repo)
$N2B scripts/ --fix                   # cli/*, imports/node-prefix, ci/*, shebang/*, husky/*

# Aggressive (scope étroit, confirmation avant)
$N2B scripts/ --fix --aggressive      # + api/* templateables (fs.readFileSync, Buffer, …)
```

### Gate post-fix (auto-détectée)

```bash
cd "$PROJECT_ROOT"
bun install --frozen-lockfile
jq -e '.scripts.build' package.json > /dev/null && bun run build
jq -e '.scripts.lint' package.json > /dev/null && bun run lint
[ -f tsconfig.json ] && bun tsc --noEmit
```

## Catalogue règles — résumé

- **Safe autofix** (`--fix`) : `cli/*` (40+ règles npm/pnpm/yarn/npx), `imports/node-prefix` (38 builtins), `ci/setup-node`, `shebang/node`, `husky/*`
- **Aggressive autofix** (`--aggressive`) : `api/fs-*` → `Bun.file/Bun.write`, `api/buffer-*`, `api/dirname-esm`, `api/sleep-promise`, `api/json-parse-readFileSync`, `imports/bun-native` (partiel — seulement `bun:*`/`node:*`)
- **Report-only** (rewrite manuel) : `api/execSync`, `api/exec`, `api/child-process-spawn`, `api/http(s)-createServer`, `api/crypto-createHash`, `api/buffer-from-base64`
- **SKIP** : `api/process-env` (portabilité SSR)

Catalogue complet : `${CLAUDE_PLUGIN_ROOT}/docs/n2b/README.md` ou `$N2B rules --report md`.

## Délégation

- Scope large / cartographie → déléguer à `bun-explorer`
- Review du diff post-fix → `bun-reviewer`
- Build + deploy post-phase → `bun-deployer`
- Multi-phase orchestration → `bun-runner` avec l'agent `n2b`

## Règles dures

1. Toujours `$N2B <path>`, jamais `$N2B .` en autofix
2. `--aggressive` sur scope étroit uniquement (scripts/, utilities)
3. `--migrate` : confirmation explicite obligatoire (supprime lockfiles rivaux)
4. 1 phase = 1 commit conventional, jamais `--no-verify`
5. Ignorer `**/generated/**`, `prisma/schema.prisma`, lockfiles

Pour le workflow complet multi-phases → invoquer l'agent `n2b` via `subagent_type: n2b`.
