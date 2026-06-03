<!-- SPDX-License-Identifier: Apache-2.0 -->
# The Aphrody API Pattern — unified contract for downstream bots

**Status:** canonical. **Verified:** 2026-06-04 against `rpbey` + `shenron` source.

`rpbey` (Beyblade) and `shenron` (Dragon Ball) each expose a large HTTP surface —
REST, GraphQL, `Bun.serve` servers, Bun-native + Bun-web APIs, and periodic
data-sync jobs. They diverged on every cross-cutting decision (response
envelope, auth tiers, GraphQL/REST duplication, scheduler model). This document
promotes the **best-governed** of the two existing implementations to a
**contract**: where one repo already does it right, that becomes normative; where
both diverge, one canonical resolution is named; and the data jobs get a
resource-aware profile so they exploit the VPS without thrashing it.

It is the cross-repo seam for API + cron, the sibling of
[`rag-unified-pattern.md`](./rag-unified-pattern.md) (retrieval) and
[`PROTOCOL.md`](./PROTOCOL.md) (aphrody's own A2A/gRPC wire). This file governs
the **first-party application HTTP surfaces** of the downstream bots.

---

## 1. The reference implementation (already right in rpbey — now normative)

rpbey's `/api/v1` surface is the model. Every route is a thin wrapper over
`getRoute`/`mutationRoute` (`apps/web/src/server/api/handler.ts:20`) that:
validates input against a Zod schema (422 on failure), validates handler output
against the contract response schema (drift → 500), wraps success in `{ok,data}`
and errors in `{ok:false,error:{code,message}}`, and **never leaks `e.message`**
(generic `{code:"internal"}` 500 on throw). The 46 routes are registered in one
OpenAPI 3.1 registry (`packages/api-contract/src/openapi.ts`), served at
`GET /api/v1/openapi.json`, and feed the generated `@rpbey/api-client` SDK.

| Layer | Contract | rpbey (reference) | shenron (today) |
|---|---|---|---|
| **Success envelope** | `{ok:true, data}` | `envelope.ts:18` (`okEnvelope`) | bare object / `{ok:true}` / `{<entity>:[]}` — **4 shapes** |
| **Error envelope** | `{ok:false, error:{code,message}}`; `code` is a stable string enum, `message` human-readable, **never** raw exception text | `handler.ts:12-16` | mostly `{error:string}`, some plain-text bodies |
| **Pagination** | `{items, pagination:{total,page,pageSize,pageCount}}` | `paginated()` `envelope.ts:54` | `{rows,total,limit,offset}` |
| **Dates on the wire** | always ISO 8601 string (no `Date` objects) | `IsoDateSchema` `envelope.ts:34` | mixed (epoch + ISO) |
| **Input validation** | Zod on query + body; query coerced from `URLSearchParams`; 422 `{code:"bad_request"}` | yes, every route | per-route ad-hoc |
| **Schema → docs → client** | one Zod registry generates OpenAPI 3.1 → generates the typed client SDK | `openapi.ts` → `@rpbey/api-client` | OpenAPI hand-served, no generated client |

**Normative rules** (a surface is "pattern-conformant" iff):

1. Every JSON response is `{ok:true,data}` or `{ok:false,error:{code,message}}`.
   No `{success,...}`, no bare resource objects, no plain-text error bodies.
2. Errors carry a **stable string `code`** (caller switches on it) + a
   human-readable `message`. Server throws map to `{code:"internal"}` — never
   the exception text.
3. List endpoints return `{items, pagination:{total,page,pageSize,pageCount}}`.
   Realtime/rate-limited endpoints MAY add a sibling `retryAfterMs` field
   (gacha-server already does, `rest.ts:40`) — promote it to a first-class
   optional envelope field, not a third shape.
4. Inputs are Zod-validated; bad input is 422 `{code:"bad_request"}` with the
   prettified issue in `message`.
5. The response schemas are the single source of truth: OpenAPI is **generated**
   from them, and the client SDK is generated from the OpenAPI — no hand-written
   drift between server, doc, and client.

The canonical envelope already exists as a publishable unit:
`@rpbey/api-contract` (`envelope.ts`). The cross-repo move is to extract it to
`@aphrody-code/api-contract` (workspace package, GitHub Packages) so shenron's
`Bun.serve` router and rpbey's Next handlers import **the same** `okEnvelope` /
`jsonErr` / `paginated` / `IsoDateSchema`. Until then, each repo mirrors the
shapes above field-for-field.

---

## 2. The four divergences and their canonical resolution

### 2.1 Response envelope — rpbey `/api/v1` vs everything else
Live shapes across both repos: rpbey v1 `{ok,data}` (canonical), rpbey gacha
`{success,error:"<string>"}`, rpbey gacha-server `{ok,error:{code,message,retryInMs}}`,
shenron's four shapes, plus JSON-RPC 2.0 for A2A.

**Resolution — one envelope, named in §1.** `{ok:true,data}` /
`{ok:false,error:{code,message}}` wins (it is the only one that is schema-checked,
OpenAPI-generated, and SDK-consumed). `retryInMs`→`retryAfterMs` becomes an
optional first-class field on the error envelope. **A2A stays JSON-RPC 2.0** —
it is a distinct protocol with its own spec ([`PROTOCOL.md`](./PROTOCOL.md)), not
an application REST surface, and is exempt. GraphQL keeps its native
`{data,errors}` (§3).

### 2.2 REST vs GraphQL duplication — same read model, exposed twice
Both repos expose the **same** read DAL through two endpoints with independently
re-decided "public-safe" field sets:
- rpbey: REST `/api/v1` (Zod-OpenAPI) **and** graphql-yoga `/api/graphql`
  (`apps/web/src/app/api/graphql/schema.ts`, read-only, omits gacha
  currency/pity per a hand-maintained comment at `schema.ts:264`).
- shenron: REST `/api/public/**` **and** Pothos+yoga `/graphql`
  (`apps/bot/src/api/graphql.ts`, read-only, `ragSearch` resolver, depth-10).

**Resolution — GraphQL is a read-only projection of the same DAL, never a second
source of truth.** Both already use **graphql-yoga**; standardize on it. Both are
**read-only Query schemas** — keep it that way (mutations go through REST, which
has the validation/envelope discipline). Normative GraphQL rules:
- **graphql-yoga**, code-first schema (Pothos in shenron, raw `createSchema` in
  rpbey — both acceptable; Pothos preferred for relation-heavy schemas).
- **Depth limit** mandatory (`maxDepthRule`, shenron sets 10) **+ a cost/complexity
  limit** (neither has one yet — add it; unbounded list×relation fan-out is the
  real DoS vector, depth alone does not bound it).
- The "public-safe field set" is **derived from the same allowlist the REST layer
  uses**, not re-decided in a comment. One projection function feeds both.
- GraphiQL on in dev, **off in prod** (shenron ships it on with CORS `*` — gate it).
- Auth: see §4 — the public tier is fine for public reads, but the endpoint must
  sit behind the same tier guard as the equivalent REST read.

### 2.3 Auth tiers — overlapping, repo-specific schemes
rpbey: `better-auth` session + `isStaffUser` (admin), `x-api-key` **and** `Bearer`
(same `BOT_API_KEY`, two headers), Ed25519 (Discord webhooks), `x-api-key`
constant-time (external partner). shenron: `Bearer API_ADMIN_TOKEN` **or**
Better-Auth cookie **or** legacy HMAC cookie (admin), HMAC acting-user (games),
open public tier, open GraphQL/A2A.

**Resolution — four named tiers, one guard taxonomy.** Every route declares
exactly one:

| Tier | Who | Mechanism (canonical) | Failure |
|---|---|---|---|
| **public** | anyone | none; CORS allowlist + per-IP rate-limit | — |
| **user** | a logged-in end user acting on their own data | Better-Auth session cookie (web) / HMAC acting-user signature (cross-service) | 401 |
| **admin** | staff | Better-Auth session + `isStaff` claim; `Bearer <ADMIN_TOKEN>` constant-time for headless | 401 unauth / 403 unauthorized |
| **agent** | another service / A2A peer | `Bearer <SERVICE_TOKEN>` constant-time, or Ed25519 for signed webhooks | 401 |

Rules: **pick one header per scheme** (rpbey's dual `x-api-key`+`Bearer` for the
same secret → keep `Bearer` only). **One admin mechanism** (shenron's three →
Better-Auth cookie for humans + `Bearer` for headless; retire the legacy HMAC
cookie). Constant-time compare for every token (`crypto.timingSafeEqual` /
`Bun.CryptoHasher`) — both repos already do this in places, make it universal.
503 (not 500) when a required token env is unset.

### 2.4 Scheduler model — `Bun.cron` (rpbey) vs systemd timers (shenron)
- rpbey: **everything** in-process `Bun.cron` (`apps/bot/src/cron/index.ts`),
  UTC, **no overlap-guard / no single-flight / no nice**, per-task idempotency
  only. Doc references an `rpb-ranking-sync.timer` that **does not exist** in
  `infra/` (3 service units, **zero** `.timer`).
- shenron: heavy data movement **out-of-process** in 5 systemd timers (oneshot,
  `Nice`d, count-verified, anti-truncate-guarded, atomic-tx); only 3 lightweight
  in-process `setInterval` tickers remain.

**Resolution — split by job weight, with one discipline for each class.**

- **Lightweight, bot-state, sub-second jobs** (jail-expiry, voice-XP, role scans,
  reminder DMs, session cleanup) → **in-process** `Bun.cron`. They need the live
  bot connection and die correctly with it. **Always UTC** (document the Paris
  offset in a comment, never compute local). Each MUST be wrapped in a
  `schedule()` helper that **single-flights** (skip if the previous run of the
  same job is still in flight) and catches+logs — rpbey's wrapper does neither
  today; add the in-flight guard.
- **Heavy data movement** (DB⇄DB sync, search-vector rebuild, backups,
  stream/corpus resolution) → **out-of-process systemd timers**, `Type=oneshot`,
  `Persistent=true`, `Nice=10-15`, count-verified + anti-truncate (shenron's
  `sync-*.ts` are the reference), wrapped in `rag-nice.sh` (single-flight +
  `nice`/`ionice` + thread caps, [`rag-unified-pattern.md`](./rag-unified-pattern.md) §5).
  rpbey's 6 h/2 h `Bun.spawn` rebuilds (`Rebuild search vectors`, 120 s timeout)
  belong here, not in `Bun.cron` — they already shell out, so moving them to a
  `.timer` is mechanical and removes the in-process overlap risk.
- **Delete dead references**: rpbey's doc-claimed `rpb-ranking-sync.timer` either
  ships as a real unit or the claim is removed. No phantom schedulers.

---

## 3. GraphQL contract (read-only projection)

```
POST /graphql      (also GET for queries; graphql-yoga, fetch-native)
  body  {"query": string, "variables"?: object}
  -> 200 {"data": {...}} | {"data": null, "errors": [{message, ...}]}   # native GraphQL envelope
```

Normative: graphql-yoga; **Query-only** (no mutations — those are REST §1);
**depth limit + cost limit** both enforced; public-safe field set derived from the
REST allowlist (one projection, not two); GraphiQL **dev-only**; CORS allowlist
(not `*`) in prod; same tier guard (§2.3) as the equivalent REST read. The
endpoint path is `/graphql` (mount in the `fetch` fallback of the `Bun.serve`
router — both repos already do, `server.ts:4289` / `route.ts`).

---

## 4. `Bun.serve` + Bun-native + Bun-web conventions

Both repos are already Bun-native; this freezes the shared discipline.

**`Bun.serve`:**
- **`routes:` map for static + `:param` paths, `fetch` fallback for wildcards**
  (multi-segment, `/graphql*`, asset trees). Both repos use exactly this split.
- **Loopback bind** (`127.0.0.1`) for every internal service; the public edge
  (Next.js / Vercel) is the only externally-bound surface. Ports are
  env-overridable, never hardcoded in tests.
- **`development:false`** in prod (disables the verbose error page).
- **Graceful shutdown**: `server.stop()` on `SIGTERM`/`SIGINT`. A `EADDRINUSE`
  singleton-guard that `process.exit(13)` (rpbey `singleton-guard.ts`) is the
  reference for "only one instance".
- **Top-level `error(e)`** handler returns the §1 envelope
  `{ok:false,error:{code:"internal"}}` 500 — never the stack.
- **CORS**: explicit origin allowlist per surface (`publicCorsHeaders` /
  `corsHeadersFor`), default-deny. `ACAO:*` only for genuinely public asset
  reads, never for anything behind a tier.
- **Tests**: `Bun.serve({port:0})` (ephemeral), `await using` for RAII teardown
  (rpbey `packages/challonge/tests/proxy-smoke.test.ts:35` is the reference).

**Bun-native (prefer over Node equivalents — this is the `n2b` policy):**
`bun:sqlite` `Database` (shenron's `bot.db`; WAL + `.safeIntegers(true)` for
snowflakes), `Bun.file`/`Bun.write`, `Bun.spawn` (+ `new Response(proc.stdout).text()`),
`Bun.$` shell (`mkdir`/`rm`), `RedisClient` from `"bun"` (rpbey — **no ioredis**;
in Next routes pulled off `globalThis.Bun` since the `bun` builtin can't be
webpack-bundled), `Bun.CryptoHasher` for HMAC, `Bun.password` for any password
hashing, `Bun.Glob`. **Legitimate Node fallbacks** (do not "fix" these): `node:crypto`
`timingSafeEqual`/`randomBytes`/`randomUUID` (no Bun equivalent needed),
`node:fs/promises` `mkdir` (only when `Bun.write` can't create the dir),
`postgres`-js / Drizzle (the Postgres/Neon client — `Bun.sql` not yet adopted),
and any `node:*` inside Next.js route handlers running the `nodejs` runtime.

**Bun-web (the wire is Web-standard everywhere):**
- Handlers receive Web `Request`, return `Response`/`Response.json`. Build headers
  with `Headers`, parse with `URL`/`URLSearchParams`.
- **SSE is the canonical server-push** (not raw WS, except where realtime duplex
  is needed). One shape, used by all 4 rpbey + 3 shenron streams:
  `new ReadableStream` + `TextEncoder` writing `data: <json>\n\n`; headers
  `Content-Type: text/event-stream`, `Cache-Control: no-cache, no-transform`,
  `X-Accel-Buffering: no`; **25-30 s keepalive** (`: keepalive` comment frame);
  teardown on `req.signal` abort. **Errors are sent as an SSE event**
  (`{type:"error"}`), so the client parses one format (rpbey `chat/route.ts:33`).
- **WebSocket** only for genuine duplex / pub-sub fan-out (rpbey bot `/ws` topics
  `logs`/`bot-events`, Colyseus gacha rooms). A WS→SSE bridge
  (`api/bot/events/route.ts`) is the pattern for exposing an internal WS stream
  to a browser as SSE.

---

## 5. Cron / sync execution profile

The per-table **source-of-truth + direction** map is part of the contract — every
sync job declares it (shenron's are the reference):

| Class | Direction / authority | Guards (mandatory) | Where it runs |
|---|---|---|---|
| **Business/runtime tables** | bot SQLite authoritative → mirror **forward** to Neon | per-table tx, **count-verify** (exit 1 on mismatch), `safeIntegers` for snowflakes | systemd `.timer` oneshot |
| **Editorial/wiki tables** | Neon authoritative → pull **reverse** to SQLite read-replica | single atomic cross-table tx, **anti-truncate guard** (source=0 & dest>0 → skip+fail), write-guard at seed entry points | systemd `.timer` oneshot |
| **Derived/search artifacts** (vectors, corpus) | rebuilt from source, no authority | `rag-nice.sh` single-flight + thread caps, **incremental over full** (hash chunks, re-embed changed only) | systemd `.timer`, off-peak, `Persistent=true` |
| **Live bot state** (jail, XP, reminders) | in-memory / SQLite, no sync | in-flight single-flight guard, catch+log | in-process `Bun.cron` |

Normative job rules:
- **UTC always**; document the local offset in a comment, never compute it.
- **Single-flight every job** — in-process via an in-flight boolean in the
  `schedule()` wrapper; out-of-process via `flock` (`rag-nice.sh`). Mirrors the
  repo-wide "one heavy job at a time" discipline.
- **Idempotent + verified**: upsert, not blind insert; count-verify destructive
  syncs; anti-truncate any "replace the table" job (the single most dangerous
  pattern — a momentarily-empty source must never wipe a populated dest).
- **Heavy jobs are `Nice`d and off-peak** (`rag-nice.sh`, `Persistent=true`), so a
  rebuild yields to interactive agents and a missed slot runs once on boot, not on
  every wake.
- **No phantom timers**: a scheduler the docs name must exist as a real unit.

---

## 6. Conformance checklist (per downstream surface)

- [ ] Every JSON response is `{ok:true,data}` / `{ok:false,error:{code,message}}`; lists are `{items,pagination:{...}}`; dates are ISO strings.
- [ ] Errors carry a stable `code`; server throws map to `{code:"internal"}` (no leaked exception text).
- [ ] Inputs Zod-validated; 422 `{code:"bad_request"}` on failure. Response schemas generate OpenAPI generates the client SDK (no hand-drift).
- [ ] Every route declares one tier (public / user / admin / agent); one header per scheme; constant-time token compare; 503 when a token env is unset.
- [ ] GraphQL is read-only graphql-yoga with depth **and** cost limits, prod-CORS allowlist, GraphiQL dev-only, field set derived from the REST allowlist.
- [ ] `Bun.serve` binds loopback (internal), `routes`+`fetch` split, graceful `stop()`, env ports, `port:0` in tests; SSE is `ReadableStream`+`TextEncoder` `data:\n\n` with keepalive + abort teardown.
- [ ] Bun-native (`bun:sqlite`/`Bun.file`/`Bun.spawn`/`Bun.$`/`RedisClient`/`Bun.password`) preferred; Node fallbacks only where no Bun equivalent exists.
- [ ] Each sync job declares its source-of-truth + direction; lightweight→`Bun.cron`, heavy→systemd `.timer`; single-flight + count-verify + anti-truncate; UTC; no phantom timers.

**Per-repo gaps vs this contract:**
- **rpbey**: unify the ~95 legacy non-v1 routes onto the v1 envelope (drop
  `{success,...}` and raw shapes, 2.1); add a GraphQL **cost** limit (2.2);
  collapse dual `x-api-key`+`Bearer` to `Bearer` (2.3); add an in-flight
  single-flight guard to the `Bun.cron` `schedule()` wrapper and move the 2 h/6 h
  rebuilds to systemd `.timer`s (2.4); delete or ship the phantom
  `rpb-ranking-sync.timer` (2.4).
- **shenron**: collapse the 4 response shapes onto `{ok,data}` and the
  `{rows,total,limit,offset}` pagination onto `{items,pagination}` (2.1); retire
  the legacy HMAC admin cookie, leaving Better-Auth + `Bearer` (2.3); add a
  GraphQL cost limit and gate GraphiQL off in prod (2.2); derive the public-safe
  field set from the REST allowlist instead of a hand-maintained comment (2.2).
- **Both**: extract the canonical envelope to `@aphrody-code/api-contract` so
  the two repos import one `okEnvelope`/`jsonErr`/`paginated`/`IsoDateSchema`
  instead of mirroring it (§1).
