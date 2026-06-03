<!-- SPDX-License-Identifier: Apache-2.0 -->

# Database — Neon (primary) + GCP alternatives

Source: `cloud-sql-basics`, `alloydb-basics` skills · `google/mcp` MCP Toolbox
for Databases (`googleapis/genai-toolbox`) · Neon docs.

## Chosen primary: Neon

rpbey already uses Drizzle (`drizzle-orm/postgres-js`) on a DB named `rpb_neon`.
Neon is serverless Postgres: managed backups, **branch-per-PR**, autoscale to
zero. It is reachable from both Vercel and the Compute Engine bot (a managed
endpoint, unlike the current local Unix socket).

**Free plan (2026, verified)**: 0.5 GB/project, **100 projects/org**, **10
branches/project**, **100 CU-h/project/month** (≈ one 0.25-CU instance always-on),
autoscale up to 2 CU, **scale-to-zero after 5 min idle**. No card required.
`rpb_neon`'s current size is small → comfortably free.

### Migrate the data
```bash
# dump local rpb_neon (owner rpb, unix socket)
pg_dump -h /var/run/postgresql -U rpb -d rpb_neon -Fc -f /tmp/rpb.dump
# restore into the Neon project's DIRECT (non-pooled) endpoint
pg_restore --no-owner --no-acl -d "$NEON_DIRECT_URL" /tmp/rpb.dump
# verify row-count parity per table (Neon MCP run_sql or psql)
```

### App wiring (`packages/db/src/client.ts`)
Read `DATABASE_URL` (Neon **pooled** `-pooler` endpoint), keep a dev fallback:
```ts
import postgres from "postgres";
import { drizzle } from "drizzle-orm/postgres-js";
const url = process.env.DATABASE_URL; // Neon pooled in prod
export const client = url
  ? postgres(url, { max: 5 })          // pooled endpoint, low max
  : postgres({ host: process.env.PGHOST ?? "/var/run/postgresql", database: "rpb_neon" });
export const db = drizzle(client, { schema });
```

### Connection strategy
- **Vercel Functions** (many short-lived invocations) → prefer the
  `@neondatabase/serverless` HTTP/WS driver, or the **pooled** `-pooler`
  endpoint, to avoid exhausting Postgres connections.
- **Compute Engine bot** (one long-lived process) → pooled endpoint, `max` ~5.
- **Migrations** → the **direct** (non-pooled) endpoint with `drizzle-kit migrate`.

### Branch-per-PR (free isolated test DBs)
Wire like shenron: the Neon↔GitHub App injects `NEON_API_KEY` + `NEON_PROJECT_ID`;
a `neon-branch.yml` workflow creates `preview/pr-<n>`, runs `drizzle-kit migrate`,
posts a schema-diff, deletes on close. The Neon↔Vercel integration auto-binds the
preview `DATABASE_URL`. (Don't double-wire the preview env if the integration
already does it.)

## GCP-native alternatives (if you ever leave Neon)

These are documented for completeness; **Neon stays the choice**.

### Cloud SQL for PostgreSQL (`cloud-sql-basics` skill)
Managed MySQL/PostgreSQL/SQL Server with backups, HA, secure connectivity.
```bash
gcloud sql instances create rpbey-pg --database-version=POSTGRES_16 \
  --tier=db-f1-micro --region=us-west1 --project=rgfr-8927d
gcloud sql databases create rpb_neon --instance=rpbey-pg
```
Connect from Cloud Run / GCE via the **Cloud SQL Auth Proxy** or a private IP.
No free tier (smallest is `db-f1-micro`, billed) → costlier than Neon for this.

### AlloyDB for PostgreSQL (`alloydb-basics` skill)
Postgres-compatible, high-performance (analytics + vector). Overkill/cost for
rpbey's size; relevant only if you need columnar/AI workloads.

### MCP Toolbox for Databases (`googleapis/genai-toolbox`)
An open-source MCP server exposing BigQuery, Cloud SQL, AlloyDB, Spanner,
Firestore as tools — deployable to Cloud Run/GKE. Useful if you want agents to
query the DB via MCP rather than direct SQL. See [`mcp-and-skills.md`](mcp-and-skills.md).
