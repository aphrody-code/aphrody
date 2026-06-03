<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cost (free tier) & Security/IAM

Source: `google-cloud-waf-cost-optimization`, `google-cloud-waf-security`,
`google-cloud-recipe-auth` skills (`google/skills`).

## Free-tier budget (target ≈ $0/month)

| Service | Always-Free allowance (2026) | rpbey usage | Watch-out |
| --- | --- | --- | --- |
| **Compute Engine `e2-micro`** | 1 non-preemptible VM/mo in `us-west1`/`us-central1`/`us-east1`; 30 GB std disk; 1 GB N.A. egress | the bot | only ONE free e2-micro across the project; pick the region carefully; egress beyond 1 GB bills |
| **Cloud Run** | 2M req, 360k GB-s mem, 180k vCPU-s/mo | API + Jobs | scale-to-zero keeps it free; `--min-instances` on a service bills idle |
| **Artifact Registry** | 0.5 GB storage | bot image | prune old images |
| **Secret Manager** | 6 active secrets, 10k access ops/mo | tokens | fine |
| **Neon Free** | 0.5 GB/project, **100 projects/org, 10 branches/project**, 100 CU-h/project, scale-to-zero @5 min | `rpb_neon` + PR branches | 100 CU-h ≈ one 0.25-CU instance always-on; idle branches suspend |
| **Vercel Hobby** | 100 GB Fast Data Transfer, 4 CPU-h Active CPU, 360 GB-h mem, 1M invocations, 6k build-min, Blob 5 GB | dashboard/CDN | **non-commercial only** (monetised → Pro); **can't Git-connect org repos** → deploy via CLI token; **no WebSocket server** |
| **GitHub Actions** | 2,000 min/mo private; **unlimited for public repos** | cron | rpbey is public → free |

WAF cost guidance: right-size (e2-micro not e2-small), scale-to-zero everywhere
possible, avoid `--min-instances` on Cloud Run, prune registry/branches, set a
**budget alert**:
```bash
gcloud billing budgets create --billing-account=<ID> \
  --display-name="rpbey-zero" --budget-amount=5USD \
  --threshold-rule=percent=0.5 --threshold-rule=percent=0.9
```

## Security / IAM (`google-cloud-recipe-auth` + WAF security)

### Identity
- **Compute Engine bot** → a dedicated **service account** (least privilege:
  `secretmanager.secretAccessor` for its secrets only; nothing else).
- **CI deploys** → **Workload Identity Federation** (no long-lived SA JSON keys
  in GitHub secrets). The bot/API deploy workflows federate GitHub OIDC → GCP.
- **Local/agent** → ADC via the existing `admin-sa` (already on the VPS); scope
  down for production automation.

### Secrets — never in git/images
- Discord token, lavalink creds, `DATABASE_URL` → **Secret Manager** (GCP) /
  **Vercel env** (web) / **GitHub secrets** (Actions). Fetch at runtime.
- The repo is **public** — audit before each push: no `.env`, no tokens, no SA
  JSON committed. (rpbey's `apps/bot/src/lib/secrets.ts` is an env-driven loader,
  not a secret store — keep it that way.)

### Network / least privilege
- Bot VM: deny-all ingress except SSH (IAP-tunneled), egress to Discord + Neon.
- Neon: connect over TLS (`sslmode=require`), pooled endpoint, low `max`.
- Cloud Run API: `--allow-unauthenticated` only for genuinely public endpoints;
  otherwise require auth + a verified caller.

### Checklist before cutover
- [ ] No secrets in any tracked file (all 5 repos public).
- [ ] Budget alert set; no stray `--min-instances`/`db-f1-micro` billing.
- [ ] Bot SA scoped to `secretAccessor` only.
- [ ] WIF configured for CI (no static SA keys).
- [ ] Neon access over TLS, pooled, connection cap set.
- [ ] VPS services NOT stopped until cloud equivalents verified live.
