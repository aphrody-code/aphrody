<!-- SPDX-License-Identifier: Apache-2.0 -->

# Site, CDN & API on Vercel

Source: Vercel skills (`vercel:deployments-cicd`, `vercel:vercel-functions`,
`vercel:vercel-storage`) · the working shenron `deploy-vercel.yml` pattern.

## Dashboard (`apps/web`, Next.js 16)

Create a Vercel project (team `aphrody`, root `apps/web`). The platform builds
Next server-side — this **eliminates** the local Bun-SIGILL workaround, the
manual `standalone` copy via `ship-web.sh`/`deploy-web.sh`, and the FAILED
`rpbey-web.service`.

```bash
cd apps/web && vercel link --yes --token "$VERCEL_TOKEN"        # creates .vercel/project.json
vercel deploy --prod --yes --token "$VERCEL_TOKEN"
```

> **Why CLI-token deploy, not native Git integration (2026, verified)**: Vercel
> **Hobby can't connect to org-owned Git repos**, and rpbey lives under the
> `aphrody-code` org. So deploy with `vercel deploy --prod --token` from a GitHub
> Action (the source is uploaded, Vercel builds it) — exactly what shenron does.
> Also: Hobby is **non-commercial** (monetised rpbey → Pro), and **Vercel
> Functions can't be a WebSocket server**, so the gateway bot stays off Vercel.

### CI (mirror shenron's working workflow)
```yaml
# .github/workflows/deploy-vercel.yml
on: { push: { branches: [main], paths: ['apps/web/**','packages/**','package.json','bun.lock'] } }
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
        with: { bun-version: canary }   # VPS bun.lock is lockfileVersion 2
      - run: bun install -g vercel@latest
      - run: vercel deploy --prod --yes --token="${{ secrets.VERCEL_TOKEN }}"
```
> Gotcha learned on shenron: without a `setup-bun` step the deploy died with
> `bun: command not found` (exit 127). Always install Bun first.

### Env vars (no secrets in git)
```bash
vercel env add DATABASE_URL production   # Neon pooled
vercel env add DATABASE_URL preview      # Neon branch (or via the Neon↔Vercel integration)
# + the app's other vars from apps/web/.env (NEXT_PUBLIC_*, auth, etc.)
```
Keep `process.env.NEXT_PUBLIC_*` (Next inlines at build; `Bun.env` would break it).

### Decouple from local JSON exports (the real work)
The dashboard reads `B_TS*.json` / `/var/www` written by VPS cron. On Vercel
those must come from: (a) **Neon** queried at request/build time, (b) **Vercel
Blob** (the cron writes the JSON to Blob, the app reads it), or (c) a build-time
fetch baked into the deployment. Pick per data freshness needs.

## gacha client (`apps/gacha-client`, Vite)
Static SPA → its own Vercel project (or a path in the dashboard). `vercel deploy`
serves the built `dist/` on the edge CDN.

## CDN / images (was `apps/cdn`)
Replace the Bun image server with **Vercel Blob** for storage + the Next image
optimizer / a `/api/image` function for transforms. Repoint asset URLs to the
Vercel domain. `vercel:vercel-storage` skill covers Blob (public + private).

## API
Use **Vercel Functions** (Fluid Compute): runs Express/Hono natively, 300 s
timeout, instance reuse, same repo as the dashboard. Keep the rpbey
`{ok,data}` / `{ok:false,error}` envelope (the cross-repo API contract). The bot
keeps its own gateway logic on the VM; the HTTP API lives here.

## Verify
```bash
curl -s -o /dev/null -w '%{http_code}' https://<rpbey-prod-domain>/   # expect 200
vercel ls --token "$VERCEL_TOKEN"
```
