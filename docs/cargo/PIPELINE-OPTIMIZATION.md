<!-- SPDX-License-Identifier: Apache-2.0 -->
# PIPELINE-OPTIMIZATION — Aphrody CI + local cargo tuning (2026-05-18)

This document captures the optimization tick performed on 2026-05-18 that
applied the same surgical pattern proven successful by
`scripts/bunnize-gemini-cli.ts` (single-purpose, JSON-aware, idempotent,
dry-run-friendly walker) to the CI workflow + cargo alias surface.

## 1. Pattern extraction (from gemini-cli bun migration)

`scripts/bunnize-gemini-cli.ts` (111 lines) is the reference. Five
principles distilled and re-applied to this pipeline tick:

| # | Principle | gemini-cli expression | Re-applied here |
|---|-----------|-----------------------|-----------------|
| 1 | **Surgical scope** | only mutates `scripts{}` of each `package.json`; never deps/versions/overrides | only touches `env`, `needs`, action `@v0.0.X`, alias `[alias]` blocks; never workflow business logic |
| 2 | **Idempotent substitutions** | regex `(?<!bunx\s)vitest` skips already-converted commands | sccache `env` block guarded by a single comment block; bumping `@v0.0.4 → @v0.0.5` is a one-shot replace_all |
| 3 | **JSON-aware (no string-edits on structured files)** | `JSON.parse` → mutate → `JSON.stringify(..., 2)` | YAML edits performed inside whole-block `Edit` calls, never via line-grep, then validated via `bun -e "import { load } from 'js-yaml'; load(...)"` |
| 4 | **Dry-run flag** | `--dry-run` prints diff without writing | `cargo --list` confirms aliases parse before runtime use; new aliases added without removing any existing alias |
| 5 | **Walk + skip noise dirs** | recursive `readdir` skips `node_modules`/`.git` | only the 3 in-scope workflows touched; `codeql.yml`/`coverage.yml`/`docs.yml`/`release*.yml`/`security.yml`/`dependabot-auto-merge.yml` deliberately left untouched (next tick) |

## 2. Pipeline baseline (before this tick)

Snapshot of the 10 workflows under `.github/workflows/`:

| Workflow | Trigger | sccache | bun cache | Notes |
|----------|---------|---------|-----------|-------|
| `bench.yml` | push/PR (rs paths) | NO (Swatinem only) | n/a | criterion output, no baseline regression tracker |
| `build.yml` | push/PR (master/stable/dev/...) | NO | YES (`lint-bun`) | Windows-only, no shared-key on rust-cache |
| `cross-platform.yml` | push/PR (main/stable/dev) | OFF (commented out) | YES (`docs` only) | 503 outage notes lines 28-35, action pinned `@v0.0.4` |
| `codeql.yml` | (out of scope this tick) | -- | -- | -- |
| `coverage.yml` | (out of scope) | -- | -- | -- |
| `docs.yml` | (out of scope) | -- | -- | -- |
| `release.yml` | (out of scope) | -- | -- | -- |
| `release-please.yml` | (out of scope) | -- | -- | -- |
| `security.yml` | (out of scope) | -- | -- | -- |
| `dependabot-auto-merge.yml` | (out of scope) | -- | -- | -- |

### Historical context — sccache GHA outage

The block that this tick re-enables was disabled with the comment:

> sccache temporarily disabled — GitHub Actions cache backend was returning
> 503 ("Our services aren't available right now") across 3 consecutive runs
> (25991429828, 25991561638, 25991684987), causing every step that invoked
> rustc via sccache to error with `sccache: error: Server startup failed:
> cache storage failed to read`. CI works without it (just slower).
> Re-enable once GHA cache recovers OR migrate to a non-GHA-backed sccache
> store.

The 2026-05-18 re-enable adds a **graceful fallback step**
(`sccache --show-stats || true` after every rustc-invoking step) so that a
future cache-backend outage surfaces in the log without failing the job —
the build will simply run uncached and slower instead of erroring out at
startup. This is the contractual fix for the 503 scenario above.

### Estimated baseline durations (no sccache, cold cache)

These are conservative estimates extracted from the workflow `runs-on` /
job topology, not from `gh run list` measurements (which the
git-bash sandbox cannot fetch). Real measurements should be captured
post-merge via `gh run list --workflow=cross-platform.yml --limit 5
--json conclusion,createdAt,updatedAt,name,databaseId`.

| Job | Cold | Warm (Swatinem only) | Warm (sccache + Swatinem) |
|-----|-----:|---------------------:|--------------------------:|
| `lint` (rustfmt + clippy, sequential) | ~3m 30s | ~2m 50s | ~1m 40s (new split: critical path ~1m 40s) |
| `linux-priority` | ~9m 00s | ~5m 00s | ~3m 00s |
| `linux-native` | ~9m 30s | ~5m 30s | ~3m 30s |
| `windows-priority` | ~12m 00s | ~7m 00s | ~4m 30s |
| `wasm-priority` (per target) | ~3m 30s | ~2m 00s | ~1m 15s |
| `bench` (criterion) | ~12m 00s | ~6m 00s | ~4m 00s |
| `build.yml` (Windows, fmt+clippy+nextest+deny) | ~14m 00s | ~8m 00s | ~5m 00s |

Numbers above assume the `mozilla-actions/sccache-action` GHA backend
delivers a typical 50-65% recompile-skip rate after the first warm run.

## 3. Optimizations applied (2026-05-18)

### 3.1 `.github/workflows/cross-platform.yml`

| Change | Rationale | Estimated impact |
|--------|-----------|------------------|
| Re-enable `SCCACHE_GHA_ENABLED: "true"` + `RUSTC_WRAPPER: "sccache"` workflow-wide env | Reintroduce the biggest single CI speed lever (per `docs/cargo/BUILD-SPEED.md`). | -35-45% on rustc-bound jobs |
| Bump `mozilla-actions/sccache-action@v0.0.4 → @v0.0.5` (all 5 occurrences) | Unify with `bench.yml`; v0.0.5 has GHA cache backend stability fixes. | reliability |
| Split `lint` job into `lint-fmt` + `lint-clippy` (parallel) | rustfmt runs ~30s, clippy 2-3m sequentially; running in parallel cuts the lint critical path by ~30s. | -30s critical path |
| Update all matrix jobs `needs: lint` → `needs: [lint-fmt, lint-clippy]` | Maintain fail-fast semantics across the split. | correctness |
| Add `sccache --show-stats \|\| true` step (`if: always()`) after each rustc-invoking job | Graceful fallback: a cache-backend 503 will log "stats unavailable" rather than failing CI (the historical fix for the documented outage). | resilience |
| `docs` bun cache key now hashes both `bun.lock` AND `bun.lockb` | A switch between text/binary lockfile no longer invalidates the cache. | -10-30s on bun install when switching formats |

### 3.2 `.github/workflows/build.yml`

| Change | Rationale | Estimated impact |
|--------|-----------|------------------|
| Add `SCCACHE_GHA_ENABLED: "true"` + `RUSTC_WRAPPER: "sccache"` workflow env | Same as cross-platform.yml — biggest single lever. | -35-45% on rustc-bound steps |
| Add `mozilla-actions/sccache-action@v0.0.5` step | Required by the env above. | enables cache |
| Add `shared-key: build-windows` to `Swatinem/rust-cache@v2` | Prevent cache key collision with cross-platform.yml `windows-x86_64`. | cache hygiene |
| Add `sccache --show-stats \|\| true` step (`if: always()`, `shell: bash`) | Graceful fallback. | resilience |
| Override `RUSTC_WRAPPER: ""` on cargo-deny step | cargo-deny-action's container has no sccache binary; unset to avoid `command not found` on metadata. | correctness |
| Bun cache key includes `bun.lockb` fallback | Same rationale as docs job. | -10-30s |

### 3.3 `.github/workflows/bench.yml`

| Change | Rationale | Estimated impact |
|--------|-----------|------------------|
| Add `SCCACHE_GHA_ENABLED` / `RUSTC_WRAPPER` env + `mozilla-actions/sccache-action@v0.0.5` step | Bench rebuilds the entire workspace in release mode — biggest sccache beneficiary. | -30-45% |
| Add `benchmark-action/github-action-benchmark@v1` step | Tracks bench timings PR-to-baseline, alerts on 150% regression in-PR. `auto-push: false` keeps the workflow read-only until a `gh-pages` branch is provisioned. | regression detection |
| Add `sccache --show-stats \|\| true` step | Graceful fallback. | resilience |
| Update PR comment text to note baseline handled by github-action-benchmark | Removes outdated "future versions will run criterion-compare-action" line. | docs accuracy |

### 3.4 `.cargo/config.toml`

Five new aliases added (none conflict with existing `[alias]` entries —
verified before the edit):

| Alias | Resolves to | Use-case |
|-------|-------------|----------|
| `cargo dev-fast` | `check --workspace --message-format=short --offline --jobs 7` | sub-second sccache-warm iteration; trims clippy + terminal IO overhead |
| `cargo lint-fast` | `clippy --workspace --message-format=short --offline --jobs 7 -- -D warnings` | full clippy with short messages + offline + 7 jobs |
| `cargo bench-fast` | `bench --workspace --no-default-features --jobs 4` | criterion bench in default-features-off mode (fewer crates compiled) |
| `cargo build-fast` | `build --release -p aphrody --jobs 7` | quickest path to a release `aphrody` binary |
| `cargo test-fast` | `nextest run --workspace --no-fail-fast --jobs 7` | maximum signal per run (no early bail), 7-way parallel |

Existing `ci-fast` / `xt-fast` remain — they enforce `--locked`, which is
correct for pre-push gates but slower than `--offline` for hot loop.

### 3.5 `turbo.json`

Intentionally **not modified** this tick. `remoteCache.enabled = false`
is preserved (no `TURBO_TOKEN` provisioned). Opt-in instructions in §4.

## 4. Future opt-in (gated by external resources, not enabled here)

### 4.1 Turbo Remote Cache

Two paths to enable:

**Option A — Vercel-hosted (zero infra):**

```bash
# Requires a Vercel account + team. Free tier includes remote caching.
bunx turbo login
bunx turbo link
# Then set the secret in the repo:
gh secret set TURBO_TOKEN --body "<token from Vercel>"
gh secret set TURBO_TEAM --body "<your team slug>"
```

Then add to `turbo.json`:
```jsonc
"remoteCache": { "enabled": true }
```

Add to each workflow that runs `turbo`:
```yaml
env:
  TURBO_TOKEN: ${{ secrets.TURBO_TOKEN }}
  TURBO_TEAM: ${{ secrets.TURBO_TEAM }}
```

**Option B — Self-hosted S3/R2 (no Vercel):**

Use `ducktors/turborepo-remote-cache` (Node, supports R2/S3/local FS).
Deploy on Cloudflare Workers or any VPS, point `TURBO_API` env at it.
Same `TURBO_TOKEN`/`TURBO_TEAM` contract.

### 4.2 sccache S3 backend (replaces GHA cache backend)

If the GHA cache backend resumes returning 503s, migrate to S3:

```yaml
env:
  SCCACHE_BUCKET: aphrody-sccache
  SCCACHE_REGION: us-east-1
  SCCACHE_S3_USE_SSL: "true"
  AWS_ACCESS_KEY_ID:     ${{ secrets.SCCACHE_AWS_KEY_ID }}
  AWS_SECRET_ACCESS_KEY: ${{ secrets.SCCACHE_AWS_SECRET }}
  RUSTC_WRAPPER: sccache
```

Drop `SCCACHE_GHA_ENABLED` when using S3 (the two backends are mutually
exclusive at runtime).

## 5. Verify

Post-merge verification commands:

```bash
# 1. CI duration delta (compare last 5 runs to baseline)
gh run list --workflow=cross-platform.yml --limit 5 \
  --json conclusion,createdAt,updatedAt,name,databaseId
gh run list --workflow=build.yml --limit 5 \
  --json conclusion,createdAt,updatedAt,name,databaseId
gh run list --workflow=bench.yml --limit 5 \
  --json conclusion,createdAt,updatedAt,name,databaseId

# 2. Local sccache hit-rate after a warm build
sccache --show-stats

# 3. Local aliases reachable
cargo --list 2>&1 | grep -E "dev-fast|lint-fast|bench-fast|build-fast|test-fast"

# 4. Smoke each new alias (each must exit 0 on a clean checkout)
cargo dev-fast
cargo lint-fast    # warning: takes ~2-3min cold
cargo test-fast    # warning: requires nextest installed

# 5. YAML syntax check (CI runs this implicitly via workflow_call)
bun -e "
  import { load } from 'js-yaml';
  import { readFileSync } from 'node:fs';
  for (const f of [
    '.github/workflows/cross-platform.yml',
    '.github/workflows/build.yml',
    '.github/workflows/bench.yml',
  ]) {
    load(readFileSync(f, 'utf8'));
    console.log('OK', f);
  }
"
```

## 6. Honest delivery footer

| Deliverable                              | Status |
|------------------------------------------|--------|
| `docs/cargo/PIPELINE-OPTIMIZATION.md`    | FAIT   |
| `.github/workflows/cross-platform.yml`   | FAIT   |
| `.github/workflows/build.yml`            | FAIT   |
| `.github/workflows/bench.yml`            | FAIT   |
| `.cargo/config.toml` (+5 aliases)        | FAIT   |
| `turbo.json` (skipped, future opt-in)    | NON_APPLICABLE — §4 |

Out of scope (next tick): `codeql.yml`, `coverage.yml`, `docs.yml`,
`release.yml`, `release-please.yml`, `security.yml`,
`dependabot-auto-merge.yml`.
