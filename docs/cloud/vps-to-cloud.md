<!-- SPDX-License-Identifier: Apache-2.0 -->

# VPS → Cloud migration analysis (rpbey)

Complement to [`architecture.md`](architecture.md): the *current* VPS reality,
the per-piece move, effort/risk, and a verified cutover runbook. All facts
snapshotted from the live VPS + fact-checked against 2026 provider docs (see
[`cost-and-security.md`](cost-and-security.md) for the verified free-tier table).

## Current VPS inventory (snapshot)

| systemd unit | State | What it is | Cloud target |
| --- | --- | --- | --- |
| `rpb-bot.service` | **failed** | Discord gateway bot (Bun, voice/lavalink) | **Compute Engine `e2-micro`** |
| `rpbey-web.service` | **failed** | Next.js 16 dashboard (Bun, standalone) | **Vercel** |
| `cdn.service` | active | Bun image server | **Vercel** (Blob + image route) |
| `rg-cron.service` | active | Bun.cron daemon | **GitHub Actions** (split per job) |
| `rpbey-profile-sync.{service,timer}` | inactive | Discord→DB profile sync (05:00) | **GitHub Actions** cron |
| `rpbey-staff-sync.{service,timer}` | inactive | Discord→DB staff sync (04:30) | **GitHub Actions** cron |
| `rpbey-embed.service` | inactive | e5-small embeddings (ML, 384-d) | VM sidecar or Cloud Run Job (see note) |
| local Postgres `rpb_neon` | running | `/var/run/postgresql:5432`, **26 MB, 73 tables** | **Neon** |

Two of the user-facing units (**bot, web**) are already **FAILED** on the VPS —
the migration also *fixes* operational debt (the Next build SIGILL workaround,
the manual `standalone` deploy), not just relocates it.

## Per-piece effort & risk

| Piece | Effort | Risk | Notes |
| --- | --- | --- | --- |
| **DB → Neon** | **Low** | Low | 26 MB / 73 tables = a seconds-long `pg_dump`→restore. Only code change: `client.ts` socket→`DATABASE_URL` (already done by the migration agent, commit `82dbf32`). Verify row-count parity. |
| **Dashboard → Vercel** | **Medium-High** | Medium | Easy build; the work is **decoupling from local JSON exports** (`B_TS*.json`, `/var/www`) → Neon/Blob. Deploy via CLI token (Hobby can't Git-connect the org repo). |
| **gacha-client → Vercel** | Low | Low | Static SPA. |
| **CDN → Vercel** | Medium | Low | Rewrite the Bun image server as a Vercel function + Blob. |
| **Cron/sync → Actions** | Low-Medium | Low | Scripts must read `DATABASE_URL`/`DISCORD_TOKEN` from env + use Discord **REST** (no gateway on a runner). |
| **Bot → Compute Engine** | **Medium** | Medium | New VM + Bun container + systemd + secrets. **Voice/lavalink** needs an always-on node (RAM tight on `e2-micro` — watch the 1 GB). |
| **embed-sidecar** | Medium | Low | ML model load is heavy/cold on serverless → keep on the VM, or precompute vectors in cron and store in Neon (`pgvector`). |

## Cutover runbook (dependency-correct, reversible)

Golden rule: **never stop a VPS unit until its cloud replacement is verified
live.** Keep the local Postgres until the Neon cutover is proven.

1. **Neon** — create project, `pg_dump`→restore, point `DATABASE_URL` at the
   pooled endpoint. *Verify*: `SELECT count(*)` parity on the top tables vs local.
   *Rollback*: code still falls back to the local socket if `DATABASE_URL` unset.
2. **Vercel dashboard + API + CDN** — deploy, set env (`DATABASE_URL`, app vars).
   *Verify*: prod URL `200`, a DB-backed page renders, an API route returns the
   `{ok,data}` envelope. *Rollback*: `rpbey-web.service` still on the VPS (don't
   disable it yet); flip DNS only after Vercel is green.
3. **GitHub Actions cron** — enable the `schedule:` workflows; run each once via
   `workflow_dispatch`. *Verify*: rows updated in Neon, no errors. *Rollback*:
   re-enable the VPS timers (kept, not deleted).
4. **Compute Engine bot** — provision `e2-micro`, deploy the Bun container, load
   secrets from Secret Manager. *Verify*: bot shows online in Discord, a slash
   command replies, voice joins a channel. *Rollback*: the VPS `rpb-bot.service`
   (currently failed — fix or keep as the fallback host).
5. **Decommission** — once all four are green for a few days: `systemctl disable
   --now` the VPS units, drop the local `rpb_neon`, retire `/var/www` exports.
   Flip DNS last.

## What does NOT move
- **bxc headless** (used by the wiki-crawl) — if the crawl needs the local binary,
  keep it on the VM or containerise it as a Cloud Run Job; don't force it onto a
  GitHub runner.
- The **other VPS services** (`aphrody.service:8082`, `bxc.service:9222`,
  `bxc-crawler`) are unrelated to rpbey and stay.

## Cost outcome
26 MB DB + a small dashboard + one always-on micro VM + cron on a public repo →
**~$0/month** across Neon Free, Vercel Hobby (or Pro if monetised), Compute
Engine Always-Free `e2-micro`, and unlimited public-repo Actions. Set a GCP
**budget alert** ([`cost-and-security.md`](cost-and-security.md)) so a stray
`--min-instances` or egress spike can't surprise you.

## Honest caveats
- **Voice on a micro VM**: lavalink (JVM) + the bot on one `e2-micro` (1 GB RAM)
  is tight; if it OOMs, split lavalink to a second small VM (still cheap) — that's
  the one place "free" may become "a few dollars".
- **Vercel Hobby is non-commercial**: if rpbey takes donations/ads, you need Pro.
- **Dashboard data decoupling** is the genuine engineering effort — everything
  else is mechanical.
