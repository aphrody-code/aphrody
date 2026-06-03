<!-- SPDX-License-Identifier: Apache-2.0 -->

# Automation — cron & sync via GitHub Actions

Source: GitHub Actions `schedule:` · Neon↔GitHub integration · shenron's
`neon-branch.yml` pattern.

The VPS systemd timers become Actions cron workflows. None of them need the
gateway — Discord member/profile data is pulled via the **REST API**, and writes
go to Neon `DATABASE_URL`.

## Timer → workflow map

| VPS timer | Schedule | Workflow | Notes |
| --- | --- | --- | --- |
| `rpbey-profile-sync` | 05:00 daily | `profile-sync.yml` | Discord REST (users) → Neon classement |
| `rpbey-staff-sync` | 04:30 daily | `staff-sync.yml` | staff_members avatars/pseudo → Neon |
| `rg-cron` (Bun.cron jobs) | various | split into per-job `schedule:` | enumerate the jobs, one cron each |
| wiki-crawl | daily | `wiki-crawl.yml` | uses `bxc` headless — see caveat below |

## Workflow skeleton

```yaml
name: profile-sync
on:
  schedule: [{ cron: '0 5 * * *' }]   # 05:00 UTC
  workflow_dispatch: {}
permissions: { contents: read }
jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
        with: { bun-version: canary }
      - name: Auth GitHub Packages (@aphrody/*, @rpbey/*)
        env: { GH_PKG: '${{ secrets.GH_PACKAGES_TOKEN }}' }
        run: printf '@aphrody:registry=https://registry.npmjs.org\n@rpbey:registry=https://npm.pkg.github.com\n//npm.pkg.github.com/:_authToken=%s\n' "$GH_PKG" > .npmrc
      - run: bun install --frozen-lockfile
      - name: sync
        env:
          DATABASE_URL: '${{ secrets.DATABASE_URL }}'      # Neon pooled
          DISCORD_TOKEN: '${{ secrets.DISCORD_TOKEN }}'
        run: bun apps/bot/scripts/profile-sync.ts          # REST-only path
```

> The existing sync scripts must read `DATABASE_URL`/`DISCORD_TOKEN` from env (no
> hardcode, no Unix-socket DB) — verify before moving them off the VM.

## Neon branch-per-PR (already standardized on shenron)

`neon-branch.yml`: on PR open/sync, `neondatabase/create-branch-action@v5` →
`drizzle-kit migrate` against the branch → `schema-diff-action@v1` PR comment;
on close, `delete-branch-action@v3`. Repo gets `NEON_API_KEY` (secret) +
`NEON_PROJECT_ID` (var) from the Neon GitHub App. The Neon↔Vercel integration
binds the preview `DATABASE_URL` automatically.

## Production migrations on deploy

Gate a `migrate-prod` job (`dorny/paths-filter@v3` on `schema.ts`/`migrations/**`)
before the Vercel deploy: resolve the prod connection string and run
`drizzle-kit migrate` (idempotent). Same pattern shenron uses.

## Wiki-crawl caveat

If the crawl needs the local `bxc` headless binary (not installable on a GitHub
runner), keep it on the Compute Engine VM as a systemd timer, or build a
container with `bxc` and run it as a **Cloud Run Job** on a Cloud Scheduler
trigger. Otherwise port it to a pure-Bun fetch + Actions cron.
