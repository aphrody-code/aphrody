<!-- SPDX-License-Identifier: Apache-2.0 -->

# Google MCP servers & Agent Skills

Source: `github.com/google/mcp` (official catalog) · `github.com/google/skills`
(Agent Skills) · live verification on this VPS.

## Loaded Agent Skills (`~/.claude/skills`, from `google/skills`)

Symlinked from `~/.claude/vendor/google-skills`; auto-load each session as
`<name>@skills-dir`. The set relevant here:

| Skill | Use |
| --- | --- |
| `cloud-run-basics` | Cloud Run services/jobs/worker-pools, deploy, roles |
| `gcloud` | safe gcloud CLI usage (read this before any gcloud cmd) |
| `cloud-sql-basics` | Cloud SQL MySQL/Postgres/SQLServer |
| `alloydb-basics` | AlloyDB clusters/instances + AlloyDB MCP |
| `firebase-basics` | Firebase products |
| `gke-basics` | GKE Autopilot golden path |
| `bigquery-basics` | BigQuery + BQ ML |
| `gemini-api`, `gemini-agents-api`, `gemini-interactions-api` | Gemini on Agent Platform |
| `google-cloud-recipe-auth` / `-onboarding` | auth/ADC + first-steps |
| `google-cloud-networking-observability` | VPC/NAT/firewall diagnostics |
| `google-cloud-waf-*` | Well-Architected: cost / security / reliability / performance / operations / sustainability |
| `agent-platform-*` | model registry, tuning, eval, RAG, prompts, deploy |

Update: `git -C ~/.claude/vendor/google-skills pull`.

## Remote managed MCP servers (`*.googleapis.com/mcp`)

Managed by Google; reachable by endpoint with a bearer token. Key ones:

| Server | Endpoint | rpbey use |
| --- | --- | --- |
| **Cloud Run** (GA) | `https://run.googleapis.com/mcp` | deploy stateless services / jobs |
| Cloud SQL (Postgres) | `https://sqladmin.googleapis.com/mcp` (see docs) | DB alt |
| AlloyDB | per `alloydb-basics` | DB alt |
| BigQuery, Spanner, Bigtable, Firestore | `*.googleapis.com/mcp` | analytics/data |
| Cloud Storage | `https://storage.googleapis.com/mcp` | assets/CDN origin |
| Compute Engine | `https://compute.googleapis.com/mcp` | the bot VM |
| Cloud Resource Manager | `https://cloudresourcemanager.googleapis.com/mcp` | projects/IAM |

### Cloud Run MCP — verified on this VPS
`tools/list` returned: `get_service`, `list_services`,
`deploy_service_from_image`, `deploy_service_from_archive`,
`deploy_service_from_file_contents`.

```bash
curl -s https://run.googleapis.com/mcp \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H 'content-type: application/json' \
  -H 'accept: application/json, text/event-stream' \
  --data '{"method":"tools/list","jsonrpc":"2.0","id":1}'
```
> Auth note: the SA token (`gcloud auth print-access-token`) expires ~hourly. For
> repeated programmatic use, refresh per call, or use a stdio proxy that injects
> a fresh token. For interactive agents, `gcloud run deploy` is simpler.

Enabling/auth reference: `docs.cloud.google.com/mcp/overview`,
`.../enable-disable-mcp-servers`, `.../authenticate-mcp`.

## Open-source MCP servers (run local / deploy to GCP)

| Server | Repo | Use |
| --- | --- | --- |
| **gcloud-mcp** | `googleapis/gcloud-mcp` | gcloud CLI as MCP |
| **MCP Toolbox for Databases** | `googleapis/genai-toolbox` | BigQuery/CloudSQL/AlloyDB/Spanner/Firestore tools |
| Cloud Run (Gemini CLI ext) | `GoogleCloudPlatform/cloud-run-mcp` | deploy helper |
| GKE | `GoogleCloudPlatform/gke-mcp` | GKE ops |
| Cloud Storage | `googleapis/gcloud-mcp/packages/storage-mcp` | GCS |
| Observability | `googleapis/gcloud-mcp/packages/observability-mcp` | logs/metrics |
| Chrome DevTools | `ChromeDevTools/chrome-devtools-mcp` | browser automation |

These can run locally or be deployed to Cloud Run (see
`docs.cloud.google.com/run/docs/host-mcp-servers`).

## How rpbey uses them
- **Cloud Run MCP** / `gcloud run deploy` — deploy API + Jobs.
- **Compute Engine MCP** / `gcloud compute` — the bot VM.
- **gcloud-mcp** — general ops if agents prefer MCP over raw CLI.
- DB stays **Neon** (not a Google MCP), so no Cloud SQL MCP needed unless you
  migrate the DB to GCP.
