<!-- SPDX-License-Identifier: Apache-2.0 -->
# X Pro (Gryphon) — integration summary

**Canonical reference:** [`/home/ubuntu/bxc/packages/x/docs/X_PRO.md`](file:///home/ubuntu/bxc/packages/x/docs/X_PRO.md)

Recon date: **2026-06-03**. Artifacts on VPS; no secrets in this file.

## What X Pro is

| Item | Detail |
| --- | --- |
| Product | X Pro (legacy TweetDeck lineage) — multi-column **decks** |
| Host | `pro.x.com` (Gryphon SPA, Cloudflare → Express) |
| Frontend bundle | `abs.twimg.com/gryphon-client/client-web/main.a4ab919a.js` |
| API gateway | `https://x.com/i/api/graphql/{queryId}/{OperationName}` (not `pro.x.com`) |
| Auth | Same **`.x.com`** cookies as x.com: `auth_token`, `ct0` → use `~/.aphrody/x-session.json` or `bxc cookies` |

`x.com` uses `responsive-web/client-web/`; Pro uses **Gryphon** — different bundle, shared GraphQL host.

## Recon artifacts (VPS)

| Path | Role |
| --- | --- |
| `~/bxc/storage/x-pro-recon/summary.json` | Machine summary (stack, bundle hash, deck URL, op counts) |
| `~/bxc/storage/x-pro-recon/` | HAR, detect/recon JSON, bundle extracts |

### `summary.json` highlights (2026-06-03)

```json
{
  "stack": {
    "host": "pro.x.com",
    "client": "gryphon-client",
    "api_gateway": "https://x.com/i/api/graphql/{queryId}/{OperationName}"
  },
  "bundle": { "gryphon_deck_ops": 18 },
  "recommended_modules": {
    "rust": "x_pro_deck",
    "typescript": "XProDeckService"
  }
}
```

Example deck deep link used in recon: `https://pro.x.com/i/decks/1823398034933199077`

## GraphQL — Gryphon deck operations

Catalog lives in Gryphon `main.*.js`, not responsive-web `x-graphql-catalog.json`. Sync via `sync-x-catalog` from bundle or ship `gryphon-graphql-catalog.json`.

| Operation | queryId | Type |
| --- | --- | --- |
| ViewerAccountSync | `zg67ZFVLUH0OWGwDZjhc0A` | query |
| CreateDeck | `fVIC9NDfk0-Auids8FlqQQ` | mutation |
| UpdateDeck | `XW307yOKJINBAvlwOnLteg` | mutation |
| RemoveDeck | `c20tuAQJznmUHtmOAvHLyA` | mutation |
| ReorderDecks | `u2A0QRHa7bBRBhZZSmJKXQ` | mutation |
| CreateColumn | `O4iIdjZUiZpm0KBSiftNGQ` | mutation |
| UpdateColumn | `suRGd49L2EZ0nuuU4he4aw` | mutation |
| RemoveColumn | `lfB7GP4w9oCpx5F_BxwRkw` | mutation |
| ReorderColumns | `JJpn5RKFDbYXC957QragBQ` | mutation |
| GryphonImportClientSyncColumns | `elhfTZAzxsCyjDZTlVitRw` | mutation |

Column feeds reuse standard timeline ops (`HomeTimeline`, `HomeLatestTimeline`, `SearchTimeline`, `GenericTimelineById`, etc.) — see full table in **X_PRO.md**.

## Premium+ entitlement

- SKU mapping in bundle: `BlueVerifiedPlus` → `premium_plus`
- Feature switches: `gryphon_client`, `gryphon_underground_enabled`
- Accounts without Premium+ may fail `ViewerAccountSync` server-side

Purchase/upsell GraphQL: see `bxc/packages/x/docs/PREMIUM.md`.

## bxc / aphrody tooling

```bash
cd ~/bxc && bxc detect https://pro.x.com/i/decks/<deckId> --json
cd ~/bxc && bxc recon  https://pro.x.com/i/decks/<deckId> --profile max --json
cd ~/bxc && bun run scripts/x-pro-recon.ts
cd ~/bxc && bxc har record https://pro.x.com/i/decks/<deckId> ~/bxc/storage/x-pro-recon/pro-deck.har --profile max
```

| Package | Planned surface |
| --- | --- |
| `@aphrody-code/x` | `XProDeckService` + Gryphon catalog overlay |
| `aphrody-x-client` | `x-cli pro deck` subcommands |
| `bxc-mcp` | `bxc_x_client` for cookie GraphQL (X.com, not official X API decks) |

**MCP / session:** configure `BXC_DB_PATH` and X session via `~/.aphrody/x-session.json` per [`README.md`](README.md).

## Agent stack cross-links

- Unified MCP/env: [`README.md`](README.md)
- Twitter/X env: [`../x/env-and-auth.md`](../x/env-and-auth.md)
- bxc + X client: [`../x/bxc-integration.md`](../x/bxc-integration.md)
