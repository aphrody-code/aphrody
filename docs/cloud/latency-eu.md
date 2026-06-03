<!-- SPDX-License-Identifier: Apache-2.0 -->

# Latency optimization — France / Europe focus

rpbey and shenron serve a **French** audience. Worldwide coverage is out of
scope: every region choice targets **France, else Europe**. All region IDs +
RTTs fact-checked against 2026 provider docs.

## The problem (measured from the live setup)

| Component | Current region | RTT from Paris | Verdict |
| --- | --- | --- | --- |
| **shenron** Neon | `aws-us-east-1` (Virginia) | ~85 ms | ❌ trans-Atlantic per query |
| **rpbey** Neon | `aws-us-west-2` (Oregon) | ~140 ms | ❌❌ worst |
| shenron Vercel fn | `cdg1` (Paris) | ~5 ms to user | ✅ but queries a DB 85 ms away |

A dashboard doing 5 sequential queries pays **5 × 85 ≈ 425 ms** in DB round-trips
alone. The DB region — not the Vercel edge — is the dominant cost.

## Available EU regions (verified 2026)

| Provider | Best France/EU option | RTT Paris | Notes |
| --- | --- | --- | --- |
| **Neon** | `aws-eu-central-1` (**Frankfurt**) or `aws-eu-west-2` (London) | ~10-12 ms | **No Paris region** (`eu-west-3` absent); Azure EU **deprecated**. Region is **immutable** → recreate the project + migrate. |
| **Vercel** (function) | `fra1` (Frankfurt) or `cdg1` (Paris) | <5 ms to user | Static/edge served from the **Paris PoP regardless** of function region; the function region only affects function↔DB + compute location. |
| **GCP** (bot/Cloud Run/GCE) | `europe-west3` (**Frankfurt**) or `europe-west9` (Paris) | — | **Not free-tier** (Always-Free `e2-micro` is US-only) → ~6-7 $/mo. Also: **billing must be enabled on `rgfr-8927d`** (currently NOT — blocks any GCP deploy). |
| **GitHub Actions** | n/a | — | Daily cron, latency irrelevant. |

## The plan (cheapest correct, France-first)

1. **Neon → `aws-eu-central-1` (Frankfurt)** for **both** projects. **Gain #1**:
   85→10 ms (shenron), 140→10 ms (rpbey). Region immutable → recreate + dump/restore.
   - **rpbey**: trivial (24 MB, no prod traffic) → recreate in Frankfurt now.
   - **shenron**: production (`dragonballfr.com`) → recreate in Frankfurt + cutover
     (`pg_dump`→restore → swap `DATABASE_URL` → redeploy). Short window or dual-write.
2. **Co-locate function + DB**: put the **Vercel functions in `fra1` (Frankfurt)**,
   the same region as Neon → function↔DB **~1-2 ms** (vs ~10 ms from `cdg1`). French
   users still get static from the Paris edge PoP. (`cdg1` is also fine — ~10 ms to
   a Frankfurt DB is already excellent vs 85 ms.)
3. **Bot → GCP `europe-west3` (Frankfurt)**, co-located with Neon Frankfurt →
   bot↔DB **~1 ms** (decisive for a command-heavy bot). Costs ~6-7 $/mo (no EU free
   tier) and **needs billing on `rgfr-8927d`**.
4. **Driver**: `@neondatabase/serverless` (HTTP, 1 RTT/query, no connection setup)
   on Vercel; **pooled** `-pooler` endpoint everywhere; keep `max` low on the bot.

## Latency outcome (5 sequential queries)

| Config | DB RTT total |
| --- | --- |
| Current (US) | ~425–700 ms |
| Neon Frankfurt + Vercel `cdg1` | ~50 ms |
| **All Frankfurt (Neon + Vercel `fra1` + bot `europe-west3`), co-located** | **~5–10 ms** |

## Region choice: Frankfurt vs London

Both `eu-central-1` (Frankfurt) and `eu-west-2` (London) are ~10 ms from Paris
(London marginally closer, ~7 ms). **Frankfurt wins** because it co-locates with
Vercel `fra1` and GCP `europe-west3` — putting the whole stack (DB + functions +
bot) in one metro gives intra-region <2 ms, which London can't match (no GCP/
Vercel London function region pairing as clean). Pick **Frankfurt** for the stack.

## Execution status & blockers

- **rpbey Neon → Frankfurt**: ready to do as soon as the serverless-migration agent
  releases the tree (it currently uses the `us-west-2` project). 24 MB = a
  seconds-long recreate; zero prod traffic = zero risk.
- **shenron Neon → Frankfurt**: schedule a cutover (it's live).
- **Bot on GCP (either Cloud Run or `europe-west3` GCE)**: **blocked on GCP billing**
  for `rgfr-8927d` — enable billing (human step), then deploy. Note the bot uses
  **voice/lavalink** → if you want music, prefer a **GCE VM** (Cloud Run can't do
  voice UDP); see [`compute-engine-bot.md`](compute-engine-bot.md).
- **Vercel**: set `"regions": ["fra1"]` in each app's `vercel.json` (shenron is on
  `cdg1` today — switch to `fra1` to co-locate once its Neon moves to Frankfurt).

See [`vps-to-cloud.md`](vps-to-cloud.md) for the full cutover runbook and
[`cost-and-security.md`](cost-and-security.md) for the free-tier budget.
