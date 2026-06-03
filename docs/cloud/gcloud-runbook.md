<!-- SPDX-License-Identifier: Apache-2.0 -->

# gcloud runbook (rpbey)

Source: `gcloud` skill (`google/skills`) — *read it before any gcloud command*;
it covers command validation, safety denylists, and data-reduction flags.

## Safety rules (from the skill)
- **Never** run destructive ops without confirmation: `delete`, `remove`,
  `--quiet` on a delete, project/instance deletion. Treat them as irreversible.
- Prefer read/discovery first: `... list`, `... describe`, `--format=...`,
  `--filter=...` to scope output (keeps context small, avoids guessing).
- Don't hallucinate flags — verify with `gcloud <group> <cmd> --help`.
- Use `--format='value(...)'` / `--format=json` for scriptable output.

## Environment
```bash
gcloud config set project rgfr-8927d
gcloud auth list                       # admin-sa@rgfr-8927d already active
gcloud auth print-access-token | head -c 12   # for MCP bearer auth
```

## One-time enablement + IAM (for the migration)
```bash
# APIs
gcloud services enable run.googleapis.com cloudbuild.googleapis.com \
  compute.googleapis.com secretmanager.googleapis.com artifactregistry.googleapis.com --quiet

# Cloud Build SA -> run.builder (for `--source` deploys)
PROJECT_NUMBER=$(gcloud projects describe rgfr-8927d --format='value(projectNumber)')
gcloud projects add-iam-policy-binding rgfr-8927d \
  --member="serviceAccount:${PROJECT_NUMBER}-compute@developer.gserviceaccount.com" \
  --role=roles/run.builder --quiet
```

## The bot VM (Compute Engine, free tier)
```bash
gcloud compute instances create rpbey-bot --zone=us-west1-b --machine-type=e2-micro \
  --image-family=debian-12 --image-project=debian-cloud --boot-disk-size=30GB
gcloud compute ssh rpbey-bot --zone=us-west1-b
gcloud compute instances describe rpbey-bot --zone=us-west1-b --format='value(status)'
```

## Cloud Run service / job (API, batch)
```bash
gcloud run deploy rpbey-api --source . --region=us-west1 --allow-unauthenticated
gcloud run services describe rpbey-api --region=us-west1 --format='value(status.url)'
gcloud run jobs deploy rpbey-recon --source . --region=us-west1 --command bun --args scripts/recon.ts
```

## Secrets
```bash
echo -n "$DISCORD_TOKEN" | gcloud secrets create rpbey-discord-token --data-file=-
gcloud secrets add-iam-policy-binding rpbey-discord-token \
  --member="serviceAccount:<vm-sa>" --role=roles/secretmanager.secretAccessor
```

## Observe
```bash
gcloud run services logs read rpbey-api --region=us-west1 --limit=50
gcloud compute ssh rpbey-bot --zone=us-west1-b --command='journalctl -u rpbey-bot -n 50 --no-pager'
gcloud logging read 'resource.type=cloud_run_revision' --limit=20 --format=json
```

## Denylist (do NOT run unprompted)
`gcloud projects delete`, `gcloud compute instances delete`,
`gcloud sql instances delete`, `gcloud run services delete`, any
`--quiet` paired with a delete. Always confirm + back up first.
