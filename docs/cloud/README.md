<!-- SPDX-License-Identifier: Apache-2.0 -->

# rpbey Cloud Architecture (`docs/cloud/`)

Canonical reference for running rpbey on **managed / serverless** infrastructure,
distilled from the Google **Agent Skills** (`google/skills`), the Google **MCP**
catalog (`google/mcp`), and the official **Bun** + **Discord-on-GCP** guides.
Each page cites its source so nothing here is guessed.

rpbey is a Discord-bot-centric Bun + Turborepo monorepo: a gateway **bot**
(voice/lavalink), a Next.js **dashboard**, a Vite **gacha** client, several Bun
services, and a **Postgres** database (`rpb_neon`). The migration moves every
piece to the cheapest correct managed home.

## Target architecture (the decision matrix)

| Piece (repo) | Home | Why this home, not the others |
| --- | --- | --- |
| **Discord bot** `apps/bot` (gateway + voice) | **Compute Engine `e2-micro` (free tier)** | A gateway bot holds a permanent WebSocket; voice is UDP/RTP. Cloud Run throttles idle CPU and is HTTP-only → it drops the gateway and can't do voice. Google's own Discord-bot guide uses a Compute Engine VM. See [`compute-engine-bot.md`](compute-engine-bot.md). |
| **Dashboard** `apps/web` (Next.js 16) | **Vercel** | Native Next.js, zero-config build (kills the local Bun-SIGILL/standalone pain), preview deploys, edge CDN. See [`vercel-site-cdn.md`](vercel-site-cdn.md). |
| **gacha client** `apps/gacha-client` (Vite SPA) | **Vercel** (static) | Static assets on Vercel's CDN. |
| **DB** `rpb_neon` (Postgres + Drizzle) | **Neon** | Serverless Postgres, branch-per-PR, scale-to-zero, reachable from Vercel + the VM. GCP alternatives (Cloud SQL / AlloyDB) covered in [`database.md`](database.md). |
| **CDN / images** `apps/cdn` | **Vercel** (Blob + image route) | Fold the Bun image server into a Vercel function + Blob. See [`vercel-site-cdn.md`](vercel-site-cdn.md). |
| **Cron / sync** (profile-sync, staff-sync, wiki-crawl) | **GitHub Actions** (`schedule:`) | Pull from Discord REST + write Neon. No persistent host needed. See [`automation-github-actions.md`](automation-github-actions.md). |
| **API** | **Vercel Functions** (Fluid Compute) | Co-located with the dashboard; the bot keeps its own gateway logic. |
| stateless HTTP jobs (recon, scrape) | **Cloud Run Jobs** (optional) | Run-to-completion tasks. See [`cloud-run.md`](cloud-run.md). |

> **The load-bearing decision**: the dashboard reads Postgres over a Unix socket
> today, so moving the dashboard to Vercel *forces* the DB to Neon (Vercel can't
> reach the VPS socket). Vercel(site) and Neon(db) are one coupled decision.

## Free-tier mapping (cost = ~0)

| Service | Free allowance (2026) | rpbey fit |
| --- | --- | --- |
| Compute Engine `e2-micro` | 1 non-preemptible VM/month in `us-west1`/`us-central1`/`us-east1`, 30 GB std disk, 1 GB egress | the bot |
| Cloud Run | 2M requests, 360k GB-s, 180k vCPU-s / month | API / Jobs |
| Neon Free | 0.5 GB/project, **100 projects**, **10 branches/project**, 100 CU-h/project, scale-to-zero @5 min | `rpb_neon` |
| Vercel Hobby | 100 GB Fast Data Transfer, 4 CPU-h Active CPU, 360 GB-h mem, **1M invocations**, 6k build-min, Blob 5 GB | dashboard + CDN |
| GitHub Actions | 2,000 min/month private; **unlimited for public repos** | cron |

> **Vercel Hobby caveats (2026, verified)**: Hobby is **non-commercial** — a
> monetised rpbey needs **Pro**. Hobby **can't Git-connect org-owned repos**
> (rpbey is under the `aphrody-code` org) → deploy via the **CLI token in a
> GitHub Action** (exactly shenron's pattern). And **Vercel Functions can't be a
> WebSocket server** — another reason the gateway bot is off Vercel.

Detail + Well-Architected cost guidance: [`cost-and-security.md`](cost-and-security.md).

## This GCP environment

- **Project**: `rgfr-8927d` · **Service account**: `admin-sa@rgfr-8927d.iam.gserviceaccount.com` (gcloud already authed on the VPS).
- **Region default**: `us-west1` (free-tier eligible for Compute Engine + close to Neon `us-east-1`; pick per latency).

## Pages

- [`architecture.md`](architecture.md) — full per-service design + data flow.
- [`vps-to-cloud.md`](vps-to-cloud.md) — current VPS inventory → cloud move, effort/risk, cutover runbook, rollback.
- [`cloud-run.md`](cloud-run.md) — Cloud Run services/jobs/worker-pools, roles, deploy (source/image/MCP), Bun Dockerfile, the `$PORT` gotcha.
- [`compute-engine-bot.md`](compute-engine-bot.md) — the gateway bot on `e2-micro` free tier (Bun container + systemd, voice intact).
- [`database.md`](database.md) — Neon (primary) + Cloud SQL / AlloyDB / MCP Toolbox alternatives, Drizzle wiring, pooling.
- [`vercel-site-cdn.md`](vercel-site-cdn.md) — dashboard, CDN, API on Vercel.
- [`automation-github-actions.md`](automation-github-actions.md) — cron/sync/wiki-crawl + Neon branch-per-PR.
- [`mcp-and-skills.md`](mcp-and-skills.md) — Google MCP catalog + the loaded skills + the verified Cloud Run MCP.
- [`gcloud-runbook.md`](gcloud-runbook.md) — safe gcloud usage + the exact rpbey command sequence.
- [`cost-and-security.md`](cost-and-security.md) — free-tier budgeting + IAM/auth/secrets.

## Sources

Google Agent Skills `github.com/google/skills` (`cloud-run-basics`, `gcloud`,
`cloud-sql-basics`, `alloydb-basics`, `google-cloud-recipe-auth`, the
`google-cloud-waf-*` Well-Architected series) · Google MCP catalog
`github.com/google/mcp` · Cloud Run MCP `https://run.googleapis.com/mcp`
(verified) · Bun guide `bun.com/docs/guides/deployment/google-cloud-run` ·
Google Cloud blog "Build and run a Discord bot on Google Cloud".
