<!-- SPDX-License-Identifier: Apache-2.0 -->

# Architecture — rpbey on managed infrastructure

## Data-flow diagram

```
                          ┌──────────────────────────────────────────┐
                          │              Discord                      │
                          │  gateway WS  +  voice (UDP/RTP)           │
                          └───────▲───────────────────▲──────────────┘
                                  │ persistent         │ slash cmds (REST)
                                  │                     │
        ┌─────────────────────────┴───────┐   ┌─────────┴────────────────┐
        │  Compute Engine e2-micro (free) │   │  GitHub Actions (cron)    │
        │  apps/bot  (Bun container)      │   │  profile-sync 05:00       │
        │  discord.js gateway + lavalink  │   │  staff-sync   04:30       │
        │  systemd unit, always-on        │   │  wiki-crawl   (bxc)       │
        └───────────────┬─────────────────┘   └───────────┬──────────────┘
                        │  Postgres (pooled)               │  Postgres (pooled)
                        ▼                                  ▼
                 ┌────────────────────────────────────────────────┐
                 │                  Neon  (rpbey)                   │
                 │   default branch = prod · branch-per-PR          │
                 └───────────────▲──────────────────────────────────┘
                                 │ DATABASE_URL (pooled / serverless driver)
        ┌────────────────────────┴───────────────────────────────┐
        │                       Vercel                            │
        │  apps/web  (Next.js dashboard) — Fluid Compute + edge   │
        │  /api/*    (Vercel Functions)  — REST/GraphQL           │
        │  CDN / image route + Vercel Blob  (was apps/cdn)        │
        │  gacha-client (static)                                  │
        └─────────────────────────────────────────────────────────┘
```

## Per-service rationale

### Bot — Compute Engine `e2-micro`
A Discord **gateway** bot keeps a permanent WebSocket and (for music) a voice
UDP channel. That is the opposite of a serverless request model:
- **Cloud Run** scales to zero and CPU-throttles idle instances → the gateway WS
  is dropped; it is HTTP-only so voice UDP can't bind. You *can* force it
  (`--min-instances=1 --no-cpu-throttling` + a `$PORT` health server) but voice
  still won't work.
- **Compute Engine** is a real VM: persistent process, UDP, full control. Google's
  own "Build and run a Discord bot on Google Cloud" guide uses a Compute Engine
  VM running the bot as a long-lived process. The `e2-micro` **free tier** makes
  it cost ~0.

Run the bot as a **Bun container** (`oven/bun`) managed by **systemd** (or
Container-Optimized OS) with `Restart=always`. Details: [`compute-engine-bot.md`](compute-engine-bot.md).

### Dashboard + gacha + CDN + API — Vercel
- `apps/web` is Next.js 16 → Vercel is the native home; the platform builds it
  server-side (eliminating the local Bun-SIGILL workaround + manual `standalone`
  deploy + the two FAILED systemd units).
- `apps/gacha-client` (Vite) → static on Vercel's CDN.
- `apps/cdn` (Bun image server) → a Vercel function backed by **Vercel Blob** +
  the Next image optimizer.
- API → **Vercel Functions** (Fluid Compute runs Express/Hono natively, 300 s
  timeout, instance reuse).

The dashboard currently consumes **local JSON exports** (`B_TS*.json`, `/var/www`)
written by cron. On Vercel those must come from Neon (queried at request/build
time) or Vercel Blob. This decoupling is the main migration effort, not the DB.

### DB — Neon
Drizzle + `postgres-js` already; the DB is even named `rpb_neon`. Neon gives
managed backups, **branch-per-PR** (wired like shenron via the Neon↔GitHub +
Neon↔Vercel integrations), and scale-to-zero. The app client reads
`DATABASE_URL` (Neon **pooled** `-pooler` endpoint); Vercel Functions can use the
`@neondatabase/serverless` HTTP driver to avoid connection-pool exhaustion. GCP
alternatives (Cloud SQL, AlloyDB) are documented for completeness in
[`database.md`](database.md) but Neon is the chosen primary.

### Cron / sync / wiki-crawl — GitHub Actions
The systemd timers become `schedule:` workflows. They don't need the gateway —
member/profile data is pulled via the **Discord REST API** and written to Neon.
The wiki-crawl uses `bxc` headless; run it as an Actions job or keep it on the
VM if it needs the local headless binary. See
[`automation-github-actions.md`](automation-github-actions.md).

## What stays on the VPS (and what gets retired)
- **Retired after cutover**: `rpbey-web.service`, `cdn.service`, the local
  `rpb_neon` Postgres, the systemd timers (→ Actions).
- **Moves to GCP**: `rpb-bot.service` → Compute Engine `e2-micro`.
- **Cutover discipline**: do NOT stop a VPS service until its cloud replacement
  is verified live (HTTP 200 / row-count parity / gateway READY). DNS/flip last.

## Migration order (dependency-correct)
1. **Neon** (everything needs `DATABASE_URL`).
2. **Vercel** dashboard + API + CDN (depends on Neon).
3. **GitHub Actions** cron (depends on Neon).
4. **Compute Engine** bot (depends on Neon + Discord secrets).
5. Cutover + retire VPS units.
