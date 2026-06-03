<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cloud Run

Source: `cloud-run-basics` skill (`google/skills`) · Bun guide
`bun.com/docs/guides/deployment/google-cloud-run` · Cloud Run MCP
`run.googleapis.com/mcp` (verified).

Cloud Run is a fully managed platform with **three** resource types:

| Type | Use | rpbey fit |
| --- | --- | --- |
| **Service** | Responds to HTTP on a stable URL; stateless, autoscales (incl. to zero). | API, CDN function, stateless webhooks |
| **Job** | Run-to-completion task, manual or scheduled. | recon/scrape/one-off data jobs |
| **Worker pool** | Always-on pull workloads (Pub/Sub, Kafka). | not needed |

> **Not for the gateway bot.** A Discord gateway/voice bot needs a persistent
> process + UDP → use Compute Engine ([`compute-engine-bot.md`](compute-engine-bot.md)),
> not Cloud Run. Cloud Run is for the *stateless* surfaces.

## Prerequisites

```bash
gcloud services enable run.googleapis.com cloudbuild.googleapis.com --quiet
```

### Required IAM roles (deployer)
- `roles/run.admin`, `roles/run.sourceDeveloper` (project)
- `roles/iam.serviceAccountUser` (service identity)
- `roles/logging.viewer` (project)

For `--source` builds, grant the Cloud Build SA `roles/run.builder`:
```bash
gcloud projects add-iam-policy-binding rgfr-8927d \
  --member=serviceAccount:SERVICE_ACCOUNT_EMAIL \
  --role=roles/run.builder --quiet
```

## Deploy a Bun service from source

`--source .` uses Cloud Build (no manual image push). Bun Dockerfile (canonical,
from the Bun guide):

```docker
FROM oven/bun:latest
COPY package.json bun.lock ./
RUN bun install --production --frozen-lockfile
COPY . .
CMD ["bun", "index.ts"]   # or: CMD ["bun", "run", "start"]
```

```docker
# .dockerignore
node_modules
Dockerfile*
.dockerignore
.git
.gitignore
.env
```

```bash
gcloud run deploy my-svc --source . --region=us-west1 --allow-unauthenticated
```

### The `$PORT` rule (critical)
Cloud Run **requires the container to listen on `$PORT`** (default `8080`) within
the startup timeout, or the revision never becomes READY. A Bun HTTP service:

```ts
Bun.serve({ port: Number(process.env.PORT) || 8080, fetch: handler });
```

Monorepo note (rpbey is a Bun workspace): copy the workspace and install with the
root lockfile, then run the target app, e.g.
`RUN bun install --frozen-lockfile` + `CMD ["bun","--filter","@rose-griffon/api","start"]`,
or build a standalone with `bun build --compile` and `COPY` only the binary.

## Cloud Run Jobs (batch)

```bash
gcloud run jobs deploy rpbey-recon --source . --region=us-west1 \
  --command bun --args scripts/recon.ts
gcloud run jobs execute rpbey-recon --region=us-west1
# schedule via Cloud Scheduler -> jobs.run, or just GitHub Actions cron
```

## Deploy via the Cloud Run MCP (alternative to gcloud)

The managed MCP at `https://run.googleapis.com/mcp` exposes (verified
`tools/list`): `get_service`, `list_services`, `deploy_service_from_image`,
`deploy_service_from_archive`, `deploy_service_from_file_contents`. Auth = a
bearer token:

```bash
curl -s https://run.googleapis.com/mcp \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H 'content-type: application/json' \
  -H 'accept: application/json, text/event-stream' \
  --data '{"method":"tools/list","jsonrpc":"2.0","id":1}'
```

- `deploy_service_from_image` — deploy an Artifact Registry / Docker Hub image.
- `deploy_service_from_archive` — deploy a `.tar.gz` (≤250 MiB) from a GCS object,
  with `command` + `base_image_uri` + `args`/`env`/`ports`; skips the build step.
- `deploy_service_from_file_contents` — inline source files (≤50 MiB total);
  ideal for Python/Node quick tests.

The SA token expires hourly; for repeated agent use prefer `gcloud run deploy` or
refresh the token per call. See [`mcp-and-skills.md`](mcp-and-skills.md).

## Verify / observe

```bash
gcloud run services list --region=us-west1
gcloud run services describe my-svc --region=us-west1 --format='value(status.url)'
gcloud run services logs read my-svc --region=us-west1 --limit=50
```
Or MCP `get_service` (returns the URI + whether the deploy succeeded).
